//! TC a352b1fa spell_warrior.cpp1792–1856: native per-effect stun children.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::Rotation;

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
        ],
        1,
    )
    .unwrap();
    h.app.world_mut().entity_mut(h.caster).insert(Rotation {
        x: 0.0,
        y: std::f32::consts::FRAC_PI_2,
        z: 0.0,
    });
    h
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
