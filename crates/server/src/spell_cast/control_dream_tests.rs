//! Native434249: time spent ready reduces the next authored cooldown.
use super::test_harness::*;
use shared::protocol::SpellCooldownUpdate;

const CONTROL_OF_THE_DREAM: u32 = 434_249;
const NATURES_SWIFTNESS: u32 = 132_158;
const BARKSKIN: u32 = 22_812;

fn dream_harness(talent: bool) -> Harness {
    let ids = if talent {
        vec![CONTROL_OF_THE_DREAM, NATURES_SWIFTNESS, BARKSKIN]
    } else {
        vec![NATURES_SWIFTNESS, BARKSKIN]
    };
    let mut h = harness(Vec::new(), &ids, 11).unwrap();
    h.advance(3);
    h
}

fn cast_cooldown(h: &mut Harness, spell: u32) -> SpellCooldownUpdate {
    h.inbox.cooldowns.clear();
    h.cast_and_deliver(spell, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.inbox
        .cooldowns
        .iter()
        .find(|receipt| receipt.spell_id == spell && !receipt.is_gcd)
        .expect("real non-GCD cooldown receipt")
        .clone()
}

fn native_cooldown(spell: u32) -> u32 {
    world_db()
        .unwrap()
        .store
        .get(spell)
        .unwrap()
        .cooldowns
        .as_ref()
        .unwrap()
        .recovery_ms
}

fn native_cap_ms() -> u32 {
    (world_db()
        .unwrap()
        .store
        .get(CONTROL_OF_THE_DREAM)
        .unwrap()
        .effects[0]
        .base_points
        * 1_000.0) as u32
}

#[test]
fn class_completeness_native_control_dream_subtracts_capped_ready_time() {
    let mut h = dream_harness(true);
    h.advance(400);
    let receipt = cast_cooldown(&mut h, NATURES_SWIFTNESS);
    let expected = native_cooldown(NATURES_SWIFTNESS) - native_cap_ms();
    assert_eq!(receipt.duration_ms, expected);
    assert_eq!(receipt.remaining_ms, expected);
}

#[test]
fn class_completeness_native_control_dream_excludes_unrelated_spells_and_absent_talent() {
    let mut learned = dream_harness(true);
    learned.advance(400);
    assert_eq!(
        cast_cooldown(&mut learned, BARKSKIN).duration_ms,
        native_cooldown(BARKSKIN)
    );
    let mut absent = dream_harness(false);
    absent.advance(400);
    assert_eq!(
        cast_cooldown(&mut absent, NATURES_SWIFTNESS).duration_ms,
        native_cooldown(NATURES_SWIFTNESS)
    );
}

#[test]
fn class_completeness_native_control_dream_counts_only_time_after_actual_recovery() {
    let mut h = dream_harness(true);
    h.advance(400);
    let first = cast_cooldown(&mut h, NATURES_SWIFTNESS);
    assert_eq!(
        first.duration_ms,
        native_cooldown(NATURES_SWIFTNESS) - native_cap_ms()
    );
    // Wait the public recovery plus three seconds; earlier unavailable time is excluded.
    h.advance((first.duration_ms / 50 + 60) as usize);
    let second = cast_cooldown(&mut h, NATURES_SWIFTNESS);
    let reduction = native_cooldown(NATURES_SWIFTNESS) - second.duration_ms;
    assert!(
        (3_000..=3_250).contains(&reduction),
        "ready-time credit: {reduction}ms"
    );
}
