//! Named Warrior policies: native ranks/gear, pinned SimC relationships, no granted stats.
use super::*;
use crate::auras::ActiveAuras;
use crate::class_progression::KnownSpells;
use crate::stats::{CharacterStats, StatModifiers, WeaponAttackType};

const ARMORED_TO_THE_TEETH: u32 = 384124;
const BATTLEFIELD_COMMANDER: u32 = 424742;

fn advance_stats(run: &mut Run) {
    for _ in 0..6 {
        run.app.update();
    }
}

fn stats(run: &Run) -> CharacterStats {
    *run.app.world().get::<CharacterStats>(run.actor).unwrap()
}

fn amount(run: &Run, spell: u32, index: u8) -> f32 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .unwrap()
        .amount
}

fn unlearn(run: &mut Run, spell: u32) -> KnownSpells {
    let known = run
        .app
        .world()
        .get::<KnownSpells>(run.actor)
        .unwrap()
        .clone();
    assert!(known.contains(spell));
    let remaining = known.iter().filter(|&id| id != spell).collect();
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(KnownSpells::fixture(remaining));
    advance_stats(run);
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, spell));
    known
}

fn relearn(run: &mut Run, known: KnownSpells) {
    run.app.world_mut().entity_mut(run.actor).insert(known);
    advance_stats(run);
}

fn assert_armored_strength(spec: u32, index: u8, rank_amount: f32) {
    let mut run = prepare(spec, 1).expect("actual reference Player gear and selected traits");
    let native_amount = amount(&run, ARMORED_TO_THE_TEETH, index);
    assert_eq!(native_amount, rank_amount, "actual native rank2 amount");
    let known = unlearn(&mut run, ARMORED_TO_THE_TEETH);
    let baseline = stats(&run);
    relearn(&mut run, known);
    let current = stats(&run);
    let total_strength_pct = run
        .app
        .world()
        .get::<StatModifiers>(run.actor)
        .unwrap()
        .primary_pct
        .strength;
    let expected = baseline.primary.strength
        + current.armor * native_amount / 100.0 * (1.0 + total_strength_pct / 100.0);
    eprintln!(
        "WARRIOR_ARMORED_TO_THE_TEETH_SIMC_POLICY spec{spec}: baseline_str={}, armor={}, native_pct={native_amount}, strength={} expected={expected}",
        baseline.primary.strength, current.armor, current.primary.strength
    );
    assert!(
        (current.primary.strength - expected).abs() < 0.01,
        "native armor-derived strength: {} vs {expected}",
        current.primary.strength
    );
    if spec == 71 {
        assert_eq!(
            current.armor, 2855.0,
            "native single Reinforced Plates factor"
        );
        assert_eq!(current.armor, baseline.armor);
    } else {
        assert!(
            current.armor > baseline.armor,
            "native Vanguard strength-to-armor remains live"
        );
    }
    unlearn(&mut run, ARMORED_TO_THE_TEETH);
    assert_eq!(stats(&run).primary.strength, baseline.primary.strength);
}

fn total_attack_power(run: &Run) -> f32 {
    stats(run).total_attack_power(
        WeaponAttackType::MainHand,
        run.app.world().get::<StatModifiers>(run.actor),
    )
}

fn assert_commander_attack_power(spec: u32) {
    let mut run = prepare(spec, 1).expect("actual reference Player gear and selected traits");
    let native_amount = amount(&run, BATTLEFIELD_COMMANDER, 5);
    assert_eq!(native_amount, 3.0, "actual native dummy effect5");
    let known = unlearn(&mut run, BATTLEFIELD_COMMANDER);
    let baseline = stats(&run);
    let baseline_total = total_attack_power(&run);
    assert!(baseline.weapon_attack_power.main_hand > 0.0);
    relearn(&mut run, known);
    let factor = 1.0 + native_amount / 100.0;
    let current = stats(&run);
    eprintln!(
        "WARRIOR_BATTLEFIELD_COMMANDER_SIMC_POLICY spec{spec}: bare {}→{}, weapon_total {baseline_total}→{}, native_pct={native_amount}",
        baseline.attack_power,
        current.attack_power,
        total_attack_power(&run)
    );
    assert!((current.attack_power - baseline.attack_power * factor).abs() < 0.01);
    assert!((total_attack_power(&run) - baseline_total * factor).abs() < 0.01);
    unlearn(&mut run, BATTLEFIELD_COMMANDER);
    assert_eq!(stats(&run).attack_power, baseline.attack_power);
    assert_eq!(total_attack_power(&run), baseline_total);
}

#[test]
#[ignore = "actual Arms reference Player, native rank2 and armor; requires prepared inputs"]
fn dps_parity_warrior_stat_policy_arms_armored_strength() {
    assert_armored_strength(71, 1, 10.0);
}

#[test]
#[ignore = "actual Protection reference Player, native rank2 and Vanguard; requires prepared inputs"]
fn dps_parity_warrior_stat_policy_protection_armored_strength() {
    assert_armored_strength(73, 2, 4.0);
}

#[test]
#[ignore = "actual Arms reference Player and native weapon AP; requires prepared inputs"]
fn dps_parity_warrior_stat_policy_arms_commander_attack_power() {
    assert_commander_attack_power(71);
}

#[test]
#[ignore = "actual Protection reference Player and native weapon AP; requires prepared inputs"]
fn dps_parity_warrior_stat_policy_protection_commander_attack_power() {
    assert_commander_attack_power(73);
}
