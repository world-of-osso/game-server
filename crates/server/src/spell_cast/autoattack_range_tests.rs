//! Native153 IsWithinMeleeRangeAt, TC a352b1fa Unit.cpp691–709.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{Npc, Position};

const EXTENDED_REACH: u32 = 326486;
const SHORT_REACH: u32 = 225319;

fn range_harness(distance: f32) -> Harness {
    let mut h = harness(Vec::new(), &[EXTENDED_REACH, SHORT_REACH], 11).unwrap();
    // A plain non-creature health target has no dodge/parry/block table.
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .remove::<Npc>()
        .insert(Position {
            x: distance,
            y: 0.0,
            z: 0.0,
        });
    h
}

fn apply_range(h: &mut Harness, target: Entity, id: u32, index: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target,
            spell_id: id,
            effect_mask: 1 << index,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn swing_once(h: &mut Harness) -> f32 {
    use bevy::ecs::system::RunSystemOnce;
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::combat::AutoAttack::new(h.npc));
    let mut mods = h
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(h.caster)
        .cloned()
        .unwrap_or_default();
    mods.melee_hit_chance_pct = 100.0;
    h.app.world_mut().entity_mut(h.caster).insert(mods);
    let before = h.health(h.npc);
    h.app
        .world_mut()
        .run_system_once(crate::combat::auto_attack_system)
        .unwrap();
    before - h.health(h.npc)
}

#[test]
fn class_completeness_native_autoattack_range_extends_actual_swing_to_ten_yards() {
    let mut h = range_harness(10.0);
    let caster = h.caster;
    apply_range(&mut h, caster, EXTENDED_REACH, 3);
    assert!(
        swing_once(&mut h) > 0.0,
        "native +5yd aura must permit real damage at 10yd"
    );
    let log = h.app.world().resource::<crate::combat::CombatLog>();
    assert!(
        log.range(0.0, f64::MAX)
            .iter()
            .any(|entry| entry.event.spell_id == 0
                && entry.event.attacker == caster.to_bits()
                && entry.event.amount > 0.0)
    );
}

#[test]
fn class_completeness_native_autoattack_range_negative_restricts_actual_swing() {
    let mut h = range_harness(3.0);
    let caster = h.caster;
    apply_range(&mut h, caster, SHORT_REACH, 2);
    assert_eq!(
        swing_once(&mut h),
        0.0,
        "native -3yd aura restricts nominal5yd range to2yd"
    );
    assert!(
        h.app
            .world()
            .get::<crate::combat::AutoAttack>(caster)
            .unwrap()
            .timer
            <= 0.0
    );
}
