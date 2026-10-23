//! Spec73 missing producers, pinned TC a352b1fa plus native70448 rows.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::Position;
use shared::protocol::{CombatLogKind, EquipmentSlot};

const REND: u32 = 772;
const REND_BLEED: u32 = 388_539;
const THUNDER_CLAP: u32 = 6343;
const SHIELD_CHARGE: u32 = 385_952;
const SHIELD_CHARGE_MOVEMENT: u32 = 385_953;
const SHIELD_CHARGE_DAMAGE: u32 = 385_954;
const PHALANX_TALENT: u32 = 1_269_311;
const PHALANX_BUFF: u32 = 1_278_009;
const PHALANX_DAMAGE: u32 = 1_270_116;
const SHIELD_SLAM: u32 = 23_922;
const RAVAGER: u32 = 228_920;
const RAVAGER_TICK: u32 = 156_287;
const WHIRLING_BLADE: u32 = 1_235_113;
const DEVASTATE: u32 = 20_243;

fn fixture(rend: bool) -> Harness {
    let ids = [
        THUNDER_CLAP,
        REND_BLEED,
        435203,
        SHIELD_CHARGE,
        SHIELD_CHARGE_MOVEMENT,
        SHIELD_CHARGE_DAMAGE,
    ];
    let mut h = harness(Vec::new(), &ids, 1).unwrap();
    h.app.insert_resource(world_db().unwrap().store.clone());
    h.app.insert_resource(load_native_creature_world(&[6]));
    let mut known = ids.to_vec();
    if rend {
        known.push(REND);
    }
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(known));
    h.app.insert_resource(
        crate::item_templates::load_item_templates(&crate::test_data::world_db_path()).unwrap(),
    );
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::stats::EquippedItems {
            items: vec![(
                EquipmentSlot::OffHand,
                shared::item_data::ItemDefinitionSource::Retail,
                2362,
            )],
        });
    h.advance(3);
    h.inbox = Inbox::default();
    h
}

fn apply_passive(h: &mut Harness, unit: Entity, spell_id: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: unit,
            target: unit,
            spell_id,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(crate::auras::has_aura(h.app.world(), unit, spell_id));
}

fn phalanx_fixture(learned: bool) -> Harness {
    let mut h = fixture(false);
    let mut known = vec![THUNDER_CLAP, SHIELD_SLAM];
    if learned {
        known.push(PHALANX_TALENT);
    }
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(known));
    if learned {
        let caster = h.caster;
        apply_passive(&mut h, caster, PHALANX_TALENT);
    }
    h
}

fn clap(rend: bool) -> (Harness, Vec<Entity>) {
    // Whole native cast attempts retain actual avoidance. A hit is required before
    // inspecting its producer, never manufactured with hit flags or modified RNG.
    for _ in 0..8 {
        let mut h = fixture(rend);
        let near = spawn_npc(&mut h.app, 5.0);
        let far = spawn_npc(&mut h.app, 25.0);
        let ally = spawn_player(&mut h.app, 1, 4.0);
        h.cast_and_deliver(THUNDER_CLAP, None);
        h.advance(5);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        if h.inbox.log.iter().any(|event| {
            event.spell_id == Some(THUNDER_CLAP)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }) {
            assert!(!crate::auras::has_aura(h.app.world(), far, REND_BLEED));
            assert!(!crate::auras::has_aura(h.app.world(), ally, REND_BLEED));
            return (h, vec![near, far, ally]);
        }
    }
    panic!("eight native Thunder Clap casts produced no landed damage");
}

/// SimC76f8118 sc_warrior.cpp1684–1690: foreground special impact on
/// the primary target rolls native1235113 RPPM and emits228920.
#[test]
fn protection_producers_whirling_blade_native_rppm_emits_ravager_from_real_impacts() {
    let mut h = fixture(false);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            DEVASTATE,
            WHIRLING_BLADE,
        ]));
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(shared::components::Health {
            current: 1_000_000.0,
            max: 1_000_000.0,
        });
    let caster = h.caster;
    apply_passive(&mut h, caster, WHIRLING_BLADE);
    h.select(Some(h.npc));
    h.advance(3);
    // Real casts, native5 RPPM and real simulation time. No edited flags,
    // chances, proc timers or seeded RNG; the bounded sequence includes bad luck protection.
    for _ in 0..40 {
        h.cast_and_deliver(DEVASTATE, Some(h.npc));
        h.advance(40);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        if h.inbox.log.iter().any(|event| {
            event.spell_id == Some(RAVAGER)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }) {
            let aura = h
                .app
                .world()
                .get::<crate::auras::ActiveAuras>(h.caster)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == RAVAGER)
                .expect("the native Ravager cast retains its indicator aura");
            assert_eq!(
                aura.duration_ms, 4000,
                "Whirling Blade's native4000ms amount supplies the background lifetime"
            );
            h.advance(45);
            assert!(
                h.inbox
                    .log
                    .iter()
                    .any(|event| event.spell_id == Some(RAVAGER_TICK)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0),
                "the proc must produce Ravager's real periodic child"
            );
            h.inbox = Inbox::default();
            h.advance(70);
            assert!(
                !h.inbox
                    .log
                    .iter()
                    .any(|event| event.spell_id == Some(RAVAGER_TICK)),
                "Whirling Blade cannot leave the ordinary12s Ravager field running"
            );
            return;
        }
    }
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(DEVASTATE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    panic!(
        "native Whirling Blade failed to emit Ravager during40 actual primary-target special casts"
    );
}

/// Native228920 effect1 provides a2000ms PERIODIC_DUMMY and12s duration.
/// SimC76f8118 sc_warrior.cpp5967–5977 supplies the missing156287 pulse.
#[test]
fn protection_producers_ravager_native_periodic_damage_waits_for_tick_and_stops_at_expiry() {
    let mut h = fixture(false);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            RAVAGER,
        ]));
    let far = spawn_npc(&mut h.app, 30.0);
    h.cast_and_deliver(RAVAGER, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(15);
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(RAVAGER_TICK)),
        "the ground event has no immediate pulse"
    );
    h.advance(30);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(RAVAGER_TICK)
                && event.kind == CombatLogKind::Damage
                && event.target == Some(h.npc.to_bits())
                && event.amount > 0),
        "native PERIODIC_DUMMY must produce real156287 damage after2000ms"
    );
    assert!(
        !h.inbox.log.iter().any(
            |event| event.spell_id == Some(RAVAGER_TICK) && event.target == Some(far.to_bits())
        ),
        "a remote enemy is outside the fixed ground event"
    );
    h.advance(220);
    h.inbox = Inbox::default();
    h.advance(60);
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(RAVAGER_TICK)),
        "no pulses survive the native12s lifetime"
    );
}

/// SimC76f8118 sc_warrior.cpp4173–4174/6300–6303. Native1269311
/// supplies proc eligibility and1278009, but no native edge supplies1270116.
#[test]
fn protection_producers_phalanx_clap_buff_is_consumed_by_shield_slam_for_real_damage() {
    let mut h = phalanx_fixture(true);
    h.cast_and_deliver(THUNDER_CLAP, None);
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        crate::auras::has_aura(h.app.world(), h.caster, PHALANX_BUFF),
        "learned native Phalanx must produce its actual1278009 buff on Clap"
    );
    h.advance(40);
    h.cast_and_deliver(SHIELD_SLAM, Some(h.npc));
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        !crate::auras::has_aura(h.app.world(), h.caster, PHALANX_BUFF),
        "Shield Slam consumes the owned Phalanx buff"
    );
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(PHALANX_DAMAGE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "consumption must emit damage, not only remove the buff"
    );
    h.inbox = Inbox::default();
    // Native23922 RecoveryTime=9000ms; preserve the real cooldown.
    h.advance(200);
    h.cast_and_deliver(SHIELD_SLAM, Some(h.npc));
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(PHALANX_DAMAGE)),
        "an already-consumed buff cannot produce a second child"
    );
}

#[test]
fn protection_producers_phalanx_other_melee_abilities_do_not_produce_the_clap_buff() {
    for _ in 0..8 {
        let mut h = phalanx_fixture(true);
        h.cast_and_deliver(SHIELD_SLAM, Some(h.npc));
        h.advance(5);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        if h.inbox.log.iter().any(|event| {
            event.spell_id == Some(SHIELD_SLAM)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }) {
            assert!(
                !crate::auras::has_aura(h.app.world(), h.caster, PHALANX_BUFF),
                "Phalanx is produced by Clap/Blast, not by Shield Slam itself"
            );
            return;
        }
    }
    panic!("eight native Shield Slams produced no landed damage");
}

#[test]
fn protection_producers_phalanx_catalog_presence_does_not_grant_the_talent() {
    let mut h = phalanx_fixture(false);
    h.cast_and_deliver(THUNDER_CLAP, None);
    h.advance(40);
    h.cast_and_deliver(SHIELD_SLAM, Some(h.npc));
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.caster,
        PHALANX_BUFF
    ));
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(PHALANX_DAMAGE))
    );
}

#[test]
fn protection_producers_phalanx_foreign_buff_does_not_empower_the_caster() {
    let mut h = phalanx_fixture(true);
    let other = spawn_player(&mut h.app, 1, 1.0);
    apply_passive(&mut h, other, PHALANX_BUFF);
    h.cast_and_deliver(SHIELD_SLAM, Some(h.npc));
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), other, PHALANX_BUFF));
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(PHALANX_DAMAGE))
    );
}

#[test]
fn protection_producers_tc_thunder_clap_emits_owned_rend_on_each_landed_enemy() {
    let (mut h, _) = clap(true);
    let landed: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(THUNDER_CLAP)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .map(|event| Entity::try_from_bits(event.target.unwrap()).unwrap())
        .collect();
    assert!(!landed.is_empty());
    for target in &landed {
        assert!(
            crate::auras::has_aura(h.app.world(), *target, REND_BLEED),
            "actual native Clap hit must produce its Rend child"
        );
        assert!(crate::auras::has_aura(h.app.world(), *target, 435203));
        let aura = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(*target)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == REND_BLEED)
            .unwrap();
        assert_eq!(aura.caster, h.caster);
    }
    h.advance(65);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(REND_BLEED)
                && event.kind == CombatLogKind::Damage
                && event.periodic
                && event.amount > 0),
        "Rend must tick real damage, not just register an aura"
    );
}

#[test]
fn protection_producers_tc_thunder_clap_without_known_rend_preserves_slow_only() {
    let (h, _) = clap(false);
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(REND_BLEED))
    );
    assert!(h.inbox.log.iter().any(|event| event.spell_id == Some(435203)
        && event.kind == CombatLogKind::AuraApplied));
}

#[test]
fn protection_producers_tc_shield_charge_moves_then_native_landing_children_deal_damage() {
    let mut h = fixture(false);
    let target = h.npc;
    // The ordinary spell fixture NPC omits display identity. Native charge
    // requires real target reach before it can launch; load its actual catalog.
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::scripted_ai::model_reach::load_model_reaches(&conn).unwrap());
    let model = &crate::test_data::world_data().templates[&6].models[0];
    h.app
        .world_mut()
        .entity_mut(target)
        .insert(shared::components::ModelDisplay {
            display_id: model.display_id,
        });
    assert!(crate::scripted_ai::model_reach::combat_reach(h.app.world(), target).is_some());
    h.app.world_mut().entity_mut(target).insert(Position {
        x: 10.0,
        y: 0.0,
        z: 0.0,
    });
    let near = spawn_npc(&mut h.app, 12.0);
    let far = spawn_npc(&mut h.app, 30.0);
    h.cast_and_deliver(SHIELD_CHARGE, Some(target));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(SHIELD_CHARGE_DAMAGE)
                && event.kind == CombatLogKind::Damage),
        "child damage waits for native charge flight"
    );
    h.advance(20);
    assert!(
        h.app.world().get::<Position>(h.caster).unwrap().x > 5.0,
        "TC Dummy must cast native385953 effect96 movement"
    );
    let damage: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(SHIELD_CHARGE_DAMAGE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .collect();
    assert!(
        damage
            .iter()
            .any(|event| event.target == Some(target.to_bits())),
        "landing must cast native385954 primary damage"
    );
    assert!(
        damage
            .iter()
            .any(|event| event.target == Some(near.to_bits())),
        "native landing child preserves its area damage"
    );
    assert!(
        !damage
            .iter()
            .any(|event| event.target == Some(far.to_bits()))
    );
    assert!(
        crate::auras::has_aura(h.app.world(), target, SHIELD_CHARGE_DAMAGE),
        "native child stun must accompany damage"
    );
}
