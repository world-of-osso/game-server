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
