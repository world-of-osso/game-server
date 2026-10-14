//! TC a352b1fa spell_warrior.cpp1792–1856: native per-effect stun children.
use super::test_harness::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use bevy::prelude::*;
use shared::components::{Health, Rotation};
use shared::protocol::CombatLogKind;

const SHOCKWAVE: u32 = 46968;
const SHOCKWAVE_STUN: u32 = 132168;
const STORM_BOLT: u32 = 107570;
const STORM_BOLT_STUN: u32 = 132169;
const STORM_BOLTS: u32 = 436162;

fn warrior() -> Harness {
    let mut h = harness(
        Vec::new(),
        &[
            SHOCKWAVE,
            SHOCKWAVE_STUN,
            STORM_BOLT,
            STORM_BOLT_STUN,
            STORM_BOLTS,
            853,
            48792,
        ],
        1,
    )
    .unwrap();
    h.app.world_mut().entity_mut(h.caster).insert(Rotation {
        x: 0.0,
        y: std::f32::consts::FRAC_PI_2,
        z: 0.0,
    });
    // Catalog availability is not a legal loadout. Do not auto-learn this talent.
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            SHOCKWAVE, STORM_BOLT,
        ]));
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        RemoveReason::Cancelled,
        |aura| aura.spell_id == STORM_BOLTS,
    );
    h.advance(3);
    h.inbox = Inbox::default();
    h
}

fn apply_native(h: &mut Harness, spell_id: u32, target: Entity) {
    let spell = world_db().unwrap().store.get(spell_id).unwrap();
    let effect_mask = crate::auras::aura_effect_mask(spell);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: target,
            target,
            spell_id,
            effect_mask,
            source: AuraSource::Cast,
        },
    );
}

#[test]
fn class_completeness_native_shockwave_damages_and_stuns_cone_targets_not_caster() {
    let mut h = warrior();
    let second = spawn_npc(&mut h.app, 4.0);
    let behind = spawn_npc(&mut h.app, -3.0);
    h.cast_and_deliver(SHOCKWAVE, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(12);
    for target in [h.npc, second] {
        assert!(h.health(target) < 5000.0, "native cone damage absent");
        assert!(
            crate::auras::has_aura(h.app.world(), target, SHOCKWAVE_STUN),
            "Shockwave damage landed without native child stun on {target:?}"
        );
    }
    assert_eq!(h.health(behind), 5000.0);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        behind,
        SHOCKWAVE_STUN
    ));
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.caster,
        SHOCKWAVE_STUN
    ));
}

#[test]
fn class_completeness_native_storm_bolt_lands_damage_then_native_child_stun() {
    let mut h = warrior();
    h.cast_and_deliver(STORM_BOLT, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(12);
    assert!(h.health(h.npc) < 5000.0, "native missile damage absent");
    assert!(
        crate::auras::has_aura(h.app.world(), h.npc, STORM_BOLT_STUN),
        "Storm Bolt damage landed without native child stun"
    );
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.caster,
        STORM_BOLT_STUN
    ));
}

#[test]
fn class_completeness_native_storm_bolt_without_storm_bolts_does_not_splash() {
    let mut h = warrior();
    let adjacent = spawn_npc(&mut h.app, 4.0);
    h.cast_and_deliver(STORM_BOLT, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(12);
    assert!(h.health(h.npc) < 5000.0);
    assert_eq!(h.health(adjacent), 5000.0, "no native Storm Bolts talent");
    assert!(!crate::auras::has_aura(
        h.app.world(),
        adjacent,
        STORM_BOLT_STUN
    ));
}

#[test]
fn class_completeness_native_storm_bolts_talent_enables_actual_splash_and_stuns() {
    let mut h = warrior();
    let caster = h.caster;
    apply_native(&mut h, STORM_BOLTS, caster);
    let adjacent = spawn_npc(&mut h.app, 4.0);
    let distant = spawn_npc(&mut h.app, 15.0);
    h.cast_and_deliver(STORM_BOLT, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(12);
    for target in [h.npc, adjacent] {
        assert!(h.health(target) < 5000.0);
        assert!(crate::auras::has_aura(
            h.app.world(),
            target,
            STORM_BOLT_STUN
        ));
    }
    assert_eq!(h.health(distant), 5000.0);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        distant,
        STORM_BOLT_STUN
    ));
}

#[test]
fn class_completeness_native_foreign_storm_bolts_does_not_enable_caster_splash() {
    let mut h = warrior();
    let adjacent = spawn_npc(&mut h.app, 4.0);
    apply_native(&mut h, STORM_BOLTS, adjacent);
    h.cast_and_deliver(STORM_BOLT, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(12);
    assert!(h.health(h.npc) < 5000.0);
    assert_eq!(h.health(adjacent), 5000.0);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        adjacent,
        STORM_BOLT_STUN
    ));
}

#[test]
fn class_completeness_native_storm_bolts_removal_restores_single_target() {
    let mut h = warrior();
    let caster = h.caster;
    apply_native(&mut h, STORM_BOLTS, caster);
    crate::auras::remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |aura| {
        aura.spell_id == STORM_BOLTS
    });
    let adjacent = spawn_npc(&mut h.app, 4.0);
    h.cast_and_deliver(STORM_BOLT, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(12);
    assert!(h.health(h.npc) < 5000.0);
    assert_eq!(h.health(adjacent), 5000.0);
}

#[test]
fn class_completeness_native_storm_bolt_child_retains_its_own_missile_flight() {
    let mut h = warrior();
    h.cast_and_deliver(STORM_BOLT, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.health(h.npc), 5000.0);
    h.advance(4); // Native parent: minimum 5 yards / 30 yards per second.
    assert!(h.health(h.npc) < 5000.0);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.npc,
        STORM_BOLT_STUN
    ));
    h.advance(5); // Native child: another minimum 5 yards / 25 yards per second.
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.npc,
        STORM_BOLT_STUN
    ));
}

#[test]
fn class_completeness_native_shockwave_stun_is_applied_before_lethal_damage() {
    let mut h = warrior();
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = 1.0;
    h.cast_and_deliver(SHOCKWAVE, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(3);
    assert_eq!(h.health(h.npc), 0.0);
    let applied = h
        .inbox
        .log
        .iter()
        .position(|event| {
            event.spell_id == Some(SHOCKWAVE_STUN)
                && event.target == Some(h.npc.to_bits())
                && event.kind == CombatLogKind::AuraApplied
        })
        .expect("instant child must reach the living target before lethal parent damage");
    let damage = h
        .inbox
        .log
        .iter()
        .position(|event| {
            event.spell_id == Some(SHOCKWAVE)
                && event.target == Some(h.npc.to_bits())
                && event.kind == CombatLogKind::Damage
        })
        .expect("native parent damage");
    assert!(applied < damage);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.npc,
        SHOCKWAVE_STUN
    ));
}

#[test]
fn class_completeness_native_shockwave_child_keeps_caster_stun_validation() {
    let mut h = warrior();
    let caster = h.caster;
    apply_native(&mut h, 853, caster);
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(caster)
            .unwrap()
            .is_stunned()
    );
    assert!(super::try_cast_triggered(
        h.app.world_mut(),
        caster,
        SHOCKWAVE,
        None,
        1
    ));
    h.advance(12);
    assert!(h.health(h.npc) < 5000.0);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.npc,
        SHOCKWAVE_STUN
    ));
    assert!(
        h.inbox.failed.is_empty(),
        "DONT_REPORT_CAST_ERROR: {:?}",
        h.inbox.failed
    );
}

#[test]
fn class_completeness_native_storm_bolt_child_uses_full_triggered_caster_bypass() {
    let mut h = warrior();
    let caster = h.caster;
    let npc = h.npc;
    apply_native(&mut h, 853, caster);
    assert!(super::try_cast_triggered(
        h.app.world_mut(),
        caster,
        STORM_BOLT,
        Some(npc),
        1
    ));
    h.advance(12);
    assert!(h.health(npc) < 5000.0);
    assert!(crate::auras::has_aura(h.app.world(), npc, STORM_BOLT_STUN));
}

#[test]
fn class_completeness_native_warrior_stun_children_respect_mechanic_immunity() {
    for parent in [SHOCKWAVE, STORM_BOLT] {
        let mut h = warrior();
        let npc = h.npc;
        apply_native(&mut h, 48792, npc);
        h.cast_and_deliver(parent, Some(npc));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        h.advance(12);
        assert!(
            h.health(npc) < 5000.0,
            "stun immunity must not remove parent damage"
        );
        assert!(!crate::auras::has_aura(h.app.world(), npc, SHOCKWAVE_STUN));
        assert!(!crate::auras::has_aura(h.app.world(), npc, STORM_BOLT_STUN));
    }
}

#[test]
fn class_completeness_native_warrior_stun_child_expiry_restores_movement() {
    for (parent, child) in [(SHOCKWAVE, SHOCKWAVE_STUN), (STORM_BOLT, STORM_BOLT_STUN)] {
        let mut h = warrior();
        h.cast_and_deliver(parent, Some(h.npc));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        h.advance(12);
        assert!(crate::auras::has_aura(h.app.world(), h.npc, child));
        assert_eq!(
            h.app
                .world()
                .get::<crate::auras::AuraStatMods>(h.npc)
                .unwrap()
                .movement_speed,
            0.0
        );
        h.advance(100);
        assert!(!crate::auras::has_aura(h.app.world(), h.npc, child));
        assert_eq!(
            h.app
                .world()
                .get::<crate::auras::AuraStatMods>(h.npc)
                .unwrap()
                .movement_speed,
            1.0
        );
    }
}
