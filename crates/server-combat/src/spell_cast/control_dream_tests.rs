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

#[test]
fn class_completeness_native_control_dream_whirling_stars_does_not_credit_partial_charges() {
    const ALIGNMENT: u32 = 194_223;
    const WHIRLING_STARS: u32 = 468_743;
    let mut h = harness(
        Vec::new(),
        &[CONTROL_OF_THE_DREAM, ALIGNMENT, WHIRLING_STARS],
        11,
    )
    .unwrap();
    h.advance(400);
    let first = cast_charge(&mut h, ALIGNMENT);
    assert_eq!(first.max, 2);
    assert_eq!(first.current, 1);
    assert_eq!(first.remaining_ms, first.recharge_ms - native_cap_ms());
    h.advance(400);
    let second = cast_charge(&mut h, ALIGNMENT);
    assert_eq!(second.current, 0);
    let elapsed_ms = first.remaining_ms - second.remaining_ms;
    assert!(
        (20_000..=20_250).contains(&elapsed_ms),
        "running clock: {elapsed_ms}ms"
    );
    h.advance(((second.remaining_ms + second.recharge_ms) / 50 + 60) as usize);
    let full = cast_charge(&mut h, ALIGNMENT);
    let reduction = full.recharge_ms - full.remaining_ms;
    assert!(
        (3_000..=3_250).contains(&reduction),
        "full recovery: {reduction}ms"
    );
}

#[test]
fn class_completeness_native_control_dream_newly_learned_spell_has_no_prior_credit() {
    let mut h = dream_harness(true);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            CONTROL_OF_THE_DREAM,
            BARKSKIN,
        ]));
    h.advance(400);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            CONTROL_OF_THE_DREAM,
            NATURES_SWIFTNESS,
            BARKSKIN,
        ]));
    h.advance(60);
    let receipt = cast_cooldown(&mut h, NATURES_SWIFTNESS);
    let credit = native_cooldown(NATURES_SWIFTNESS) - receipt.duration_ms;
    assert!(
        (3_000..=3_250).contains(&credit),
        "newly learned credit: {credit}ms"
    );
}

fn charged_dream_harness(spell: u32) -> Harness {
    let mut h = harness(Vec::new(), &[CONTROL_OF_THE_DREAM, spell], 11).unwrap();
    h.advance(3);
    h
}

fn cast_charge(h: &mut Harness, spell: u32) -> shared::protocol::SpellChargesUpdate {
    h.inbox.charges.clear();
    h.cast_and_deliver(spell, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.inbox
        .charges
        .iter()
        .find(|r| r.spell_id == spell)
        .unwrap()
        .clone()
}

#[test]
fn class_completeness_native_control_dream_charged_alignment_credits_only_full_time() {
    const ALIGNMENT: u32 = 194_223;
    let mut h = charged_dream_harness(ALIGNMENT);
    let source = world_db().unwrap().store.get(ALIGNMENT).unwrap();
    let category = source.categories.as_ref().unwrap();
    h.advance(400);
    let first = cast_charge(&mut h, ALIGNMENT);
    assert_eq!(u32::from(first.current), category.max_charges - 1);
    assert_eq!(first.recharge_ms, category.charge_recovery_ms);
    assert_eq!(
        first.remaining_ms,
        category.charge_recovery_ms - native_cap_ms()
    );
    h.advance((first.remaining_ms / 50 + 60) as usize);
    let next = cast_charge(&mut h, ALIGNMENT);
    let reduction = category.charge_recovery_ms - next.remaining_ms;
    assert!(
        (3_000..=3_250).contains(&reduction),
        "full-charge credit: {reduction}ms"
    );
}

#[test]
fn class_completeness_native_control_dream_restored_incarnation_starts_credit_at_restore() {
    const INCARNATION: u32 = 390_414;
    let mut h = charged_dream_harness(INCARNATION);
    let source = world_db().unwrap().store.get(INCARNATION).unwrap();
    let category = source.categories.as_ref().unwrap();
    h.advance(400);
    let first = cast_charge(&mut h, INCARNATION);
    assert_eq!(
        first.remaining_ms,
        category.charge_recovery_ms - native_cap_ms()
    );
    h.advance(400);
    super::restore_charges(
        h.app.world_mut(),
        h.caster,
        category.charge_category,
        category.max_charges.try_into().unwrap(),
    );
    h.advance(60);
    let restored = cast_charge(&mut h, INCARNATION);
    let reduction = category.charge_recovery_ms - restored.remaining_ms;
    assert!(
        (3_000..=3_250).contains(&reduction),
        "restore-time credit: {reduction}ms"
    );
}

#[test]
fn class_completeness_native_control_dream_removal_and_foreign_aura_grant_no_credit() {
    let mut h = dream_harness(true);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            NATURES_SWIFTNESS,
            BARKSKIN,
        ]));
    let owner = h.caster;
    crate::auras::remove_auras(
        h.app.world_mut(),
        owner,
        crate::auras::RemoveReason::Unlearned,
        |aura| aura.spell_id == CONTROL_OF_THE_DREAM,
    );
    let foreign = spawn_player(&mut h.app, 11, 0.0);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: foreign,
            target: owner,
            spell_id: CONTROL_OF_THE_DREAM,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.advance(400);
    assert_eq!(
        cast_cooldown(&mut h, NATURES_SWIFTNESS).duration_ms,
        native_cooldown(NATURES_SWIFTNESS)
    );
}
