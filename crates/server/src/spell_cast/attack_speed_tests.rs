//! TC a352b1fa aura319 MOD_MELEE_HASTE_3 → HandleModMeleeSpeedPct →
//! ApplyAttackTimePercentMod (SpellAuraEffects.cpp:4564-4582, Unit.cpp:11032-11056):
//! melee attack time only, × 100 / (100 + amount) per positive aura.
use super::test_harness::*;
use bevy::prelude::*;

/// Ruthless Aggression child: aura319 +35 on the caster.
const RUTHLESS_AGGRESSION: u32 = 441_817;
/// Swift as a Coursing River child: effect1 aura319 +10.
const SWIFT_RIVER: u32 = 1_263_345;
const FIREBALL: u32 = 133;
const FRAME_SECS: f32 = 0.05;

fn native_amount(spell: u32) -> f32 {
    world_db()
        .unwrap()
        .store
        .get(spell)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.aura == 319)
        .unwrap()
        .base_points
}

fn apply_attack_speed(h: &mut Harness, unit: Entity, spell: u32) {
    let index = world_db()
        .unwrap()
        .store
        .get(spell)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.aura == 319)
        .unwrap()
        .index;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: unit,
            target: unit,
            spell_id: spell,
            effect_mask: 1 << index,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

/// Frames between the kobold's first two swings at the player.
fn kobold_swing_frames(auras: &[u32]) -> usize {
    let mut h = harness(Vec::new(), &[RUTHLESS_AGGRESSION, SWIFT_RIVER], 1).unwrap();
    let (npc, player) = (h.npc, h.caster);
    for &spell in auras {
        apply_attack_speed(&mut h, npc, spell);
    }
    h.app
        .world_mut()
        .entity_mut(npc)
        .insert(crate::combat::AutoAttack::new(player));
    let mut swings = Vec::new();
    for frame in 0..120 {
        h.app.update();
        let log = h.app.world().resource::<crate::combat::CombatLog>();
        let count = log
            .range(0.0, f64::MAX)
            .iter()
            .filter(|entry| entry.event.attacker == npc.to_bits() && entry.event.spell_id == 0)
            .count();
        if count > swings.len() {
            swings.push(frame);
        }
        if swings.len() == 2 {
            break;
        }
    }
    assert_eq!(swings.len(), 2, "two kobold swings");
    swings[1] - swings[0]
}

/// 50 ms frames, ±1 for floating-point timer accumulation.
fn assert_frames(actual: usize, factors: &[f32]) {
    let expected = expected_frames(factors);
    assert!(
        actual.abs_diff(expected) <= 1,
        "{actual} frames, expected {expected}"
    );
}

fn expected_frames(factors: &[f32]) -> usize {
    let attack_time = crate::combat::AUTO_ATTACK_INTERVAL
        * factors
            .iter()
            .map(|amount| 100.0 / (100.0 + amount))
            .product::<f32>();
    (attack_time / FRAME_SECS).ceil() as usize
}

#[test]
fn class_completeness_native_attack_speed_shortens_creature_swings() {
    assert_frames(kobold_swing_frames(&[]), &[]);
    let aggression = native_amount(RUTHLESS_AGGRESSION);
    assert_frames(kobold_swing_frames(&[RUTHLESS_AGGRESSION]), &[aggression]);
    let river = native_amount(SWIFT_RIVER);
    assert_frames(
        kobold_swing_frames(&[RUTHLESS_AGGRESSION, SWIFT_RIVER]),
        &[aggression, river],
    );
}

/// Frames from sending Fireball until its SpellGo.
fn fireball_cast_frames(attack_speed: bool) -> usize {
    let mut h = harness(Vec::new(), &[FIREBALL, RUTHLESS_AGGRESSION], 8).unwrap();
    let caster = h.caster;
    if attack_speed {
        apply_attack_speed(&mut h, caster, RUTHLESS_AGGRESSION);
    }
    let npc = h.npc;
    h.cast(FIREBALL, Some(npc));
    for frame in 1..200 {
        h.advance(1);
        if h.inbox.go.iter().any(|go| go.spell_id == FIREBALL) {
            return frame;
        }
    }
    panic!("Fireball never completed: {:?}", h.inbox.failed);
}

#[test]
fn class_completeness_native_attack_speed_leaves_cast_time_unchanged() {
    assert_eq!(fireball_cast_frames(true), fireball_cast_frames(false));
    // Ensure the aura really lands on the caster.
    let mut h = harness(Vec::new(), &[RUTHLESS_AGGRESSION], 8).unwrap();
    let caster = h.caster;
    apply_attack_speed(&mut h, caster, RUTHLESS_AGGRESSION);
    assert!(crate::auras::has_aura(
        h.app.world(),
        caster,
        RUTHLESS_AGGRESSION
    ));
}
