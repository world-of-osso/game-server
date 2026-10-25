//! Native form metadata must reach actual follower swing clocks.
use super::kit_tests::at_level;
use crate::spell_cast::test_harness::Harness;
use bevy::prelude::*;
use shared::components::{Health, Position};

fn fixture(form_spell: Option<u32>) -> (Harness, Entity, f32) {
    let (mut h, actor) = at_level(232800, 90, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
    });
    h.app.world_mut().get_mut::<Position>(h.npc).unwrap().x = 1.5;
    h.app.world_mut().entity_mut(h.npc).insert(Health {
        current: 1_000_000_000.0,
        max: 1_000_000_000.0,
    });
    if let Some(spell) = form_spell {
        if !crate::auras::has_aura(h.app.world(), actor, spell) {
            crate::spell_cast::creature_cast(h.app.world_mut(), actor, spell, Some(actor), false)
                .expect("ordinary native form cast");
        }
    } else {
        crate::auras::remove_auras(
            h.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| {
                aura.effects
                    .iter()
                    .any(|effect| effect.aura_type == crate::auras::handlers::MOD_SHAPESHIFT)
            },
        );
    }
    h.advance(3);
    let form = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .and_then(crate::auras::shapeshift::current_form)
        .map(|(_, id)| id);
    assert_eq!(form.is_some(), form_spell.is_some());
    let base = if let Some(form) = form {
        let db = rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let ms: u32 = db
            .query_row(
                "SELECT CombatRoundTime FROM spell_shapeshift_form WHERE ID=?",
                [form],
                |row| row.get(0),
            )
            .unwrap();
        assert!(ms > 0);
        ms as f32 / 1000.0
    } else {
        h.app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .main_hand
            .speed
    };
    (h, actor, base)
}

fn expected_interval(world: &World, actor: Entity, base: f32) -> f64 {
    let stats = world.get::<crate::stats::CharacterStats>(actor).unwrap();
    let slow = world
        .get::<crate::auras::AuraStatMods>(actor)
        .map_or(1.0, |mods| {
            mods.attack_cast_time_mult * mods.attack_time_mult
        });
    let seconds = base / (1.0 + stats.haste_pct / 100.0) * slow;
    f64::from((seconds * 1000.0).trunc()) / 1000.0
}

fn observe_interval(form_spell: Option<u32>) {
    let _seed = crate::random::seed_scope(70448);
    let (mut h, actor, base) = fixture(form_spell);
    let target = h.npc;
    let world = h.app.world_mut();
    crate::combat::start_attack(&mut world.commands(), actor, target, None);
    world.flush();
    let mut seen = h.inbox.log.len();
    let mut first: Option<(f64, f64)> = None;
    for _ in 0..300 {
        h.advance(1);
        let now = h.app.world().resource::<Time>().elapsed_secs_f64();
        let swings = h.inbox.log[seen..]
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id.is_none()
            })
            .count();
        seen = h.inbox.log.len();
        for _ in 0..swings {
            if let Some((started, expected)) = first {
                let actual: f64 = now - started;
                let tick = h.app.world().resource::<Time>().delta_secs_f64();
                assert!(
                    (actual - expected).abs() <= tick + 0.002,
                    "native form{form_spell:?}: source base={base}s, interval={actual}s, expected={expected}s"
                );
                return;
            }
            first = Some((now, expected_interval(h.app.world(), actor, base)));
        }
    }
    panic!("native form{form_spell:?} did not deliver two actual melee swings");
}

#[test]
#[ignore = "native private DB: source Cat1000ms must drive actual follower swings"]
fn native_form_cat_uses_authored_combat_round_time() {
    observe_interval(Some(768));
}

#[test]
#[ignore = "native private DB: source Bear2500ms must drive actual follower swings"]
fn native_form_bear_uses_authored_combat_round_time() {
    observe_interval(Some(5487));
}

#[test]
#[ignore = "native private DB: leaving form must retain native weapon cadence"]
fn native_form_unshifted_retains_equipment_cadence() {
    observe_interval(None);
}

fn expected_form_ap_speed(
    weapon: &crate::item_stats::WeaponDamage,
    row: (i32, u32),
    player: bool,
    normalized: bool,
) -> f32 {
    let (id, ms) = row;
    let speed = if normalized {
        weapon.kind.normalized_speed()
    } else if !player || matches!(id, 1 | 5 | 8 | 16) {
        if weapon.kind == shared::components::WeaponType::Ranged {
            2.0 // TC InitDataForForm RANGED_ATTACK=BASE_ATTACK_TIME.
        } else {
            ms as f32 / 1000.0
        }
    } else {
        weapon.speed
    };
    speed.max(0.25) // TC StatSystem.cpp453.
}

fn assert_native_weapon_terms(
    weapon: &crate::item_stats::WeaponDamage,
    row: (i32, u32),
    player: bool,
    normalized: bool,
    form: crate::auras::shapeshift::FormAttackTiming,
) {
    let ap_speed = expected_form_ap_speed(weapon, row, player, normalized);
    let scale = row.1 as f32 / 1000.0 / ap_speed;
    let actual = crate::melee::weapon_damage_terms(weapon, normalized, Some(form));
    assert!((actual.0 - weapon.min * scale).abs() < 0.001);
    assert!((actual.1 - weapon.max * scale).abs() < 0.001);
    assert_eq!(actual.2, ap_speed);
    let regular = crate::melee::weapon_damage_terms(weapon, normalized, None);
    let regular_speed = if normalized {
        weapon.kind.normalized_speed()
    } else {
        weapon.speed
    };
    assert_eq!(regular, (weapon.min, weapon.max, regular_speed));
}

#[test]
#[ignore = "native private DB: every nonzero form uses TC weapon/AP timing, not a damage scalar"]
fn native_form_weapon_terms_cover_all_authored_forms_and_actor_kinds() {
    let (h, actor, _) = fixture(None);
    let melee = h
        .app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .main_hand;
    let path = crate::test_data::world_db_path();
    let templates = crate::item_templates::load_item_templates(&path).unwrap();
    let tables = crate::item_stats::load_item_scaling_tables(&path).unwrap();
    // CharacterLoadout158's actual native Hunter rifle, ItemDelay3000.
    let ranged = crate::item_stats::item_stats(&templates.retail[&73000], &tables)
        .weapon
        .unwrap();
    let db =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let forms = crate::auras::shapeshift::load_shapeshift_forms(&db).unwrap();
    let mut stmt = db
        .prepare("SELECT ID, CombatRoundTime FROM spell_shapeshift_form WHERE CombatRoundTime<>0")
        .unwrap();
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, i32>(0)?, row.get::<_, u32>(1)?)))
        .unwrap();
    let mut checked = Vec::new();
    for row in rows {
        let (id, ms) = row.unwrap();
        checked.push(id);
        for player in [false, true] {
            let form = forms.attack_timing(id, player).unwrap();
            assert_eq!(form.seconds, ms as f32 / 1000.0);
            for weapon in [&melee, &ranged] {
                for normalized in [false, true] {
                    assert_native_weapon_terms(weapon, (id, ms), player, normalized, form);
                }
            }
        }
    }
    checked.sort_unstable();
    assert_eq!(checked, [1, 5, 8, 44, 45, 46, 49]);
    assert!(
        forms.attack_timing(16, true).is_none(),
        "zero-time Ghost Wolf uses regular weapons"
    );
}
