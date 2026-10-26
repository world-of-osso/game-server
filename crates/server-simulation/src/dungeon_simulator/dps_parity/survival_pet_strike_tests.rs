//! Actual Tip spending must dispatch Strike as One from the already owned pet.
use super::marksmanship_mechanics_tests::cast;
use super::*;
use shared::protocol::{CombatLogEvent, CombatLogKind};

fn owned_strike_outcomes(run: &Run, pet: Entity, start: usize) -> Vec<&CombatLogEvent> {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..]
        .iter()
        .filter(|event| {
            event.source == Some(pet.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(1251779)
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
        })
        .collect()
}

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_tip_impact_commands_owned_pet_strike_as_one() {
    // Exercise every seed in this fixed matrix, never retry until one passes.
    let mut positive = 0;
    for seed in 0..32 {
        positive += usize::from(assert_owned_tip_strike(seed));
    }
    assert!(
        positive > 0,
        "native owned strikes never dealt positive damage"
    );
}

fn assert_owned_tip_strike(seed: u64) -> bool {
    let _seed = crate::random::seed_scope(seed);
    // SimC76f8118 sc_hunter.cpp1367–1372: earned Tip consumption orders
    // the real main pet, preserving that pet as the native damage source.
    let mut run = prepare_literal(255, 1).unwrap();
    hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
    cast(&mut run, 883, true);
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    engage(&mut run);
    cast(&mut run, 259489, false);
    cast(&mut run, 186270, false);
    for _ in 0..40 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    // Native97ad4bea8b seed9 proves a genuine owned strike can be dodged.
    // Count the actual attack outcome, not a fabricated guaranteed hit.
    let strikes = owned_strike_outcomes(&run, pet, 0);
    assert_eq!(
        strikes.len(),
        1,
        "seed{seed}: expected one real owned strike outcome: {events:#?}"
    );
    strikes[0].kind == CombatLogKind::Damage && strikes[0].amount > 0
}

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_tipped_rank_three_swipe_adds_separate_pet_strike() {
    // SimC76f8118 sc_hunter.cpp6181–6186: actual rank3 Swipe with an
    // earned Tip supplies its additional effectiveness attack before ordinary Tip impact.
    let mut run = prepare_literal(255, 1).unwrap();
    hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
    cast(&mut run, 883, true);
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    engage(&mut run);
    cast(&mut run, 259489, false);
    cast(&mut run, 186270, false);
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    cast(&mut run, 186270, false);
    for _ in 0..40 {
        run.app.update();
    }
    let strikes = owned_strike_outcomes(&run, pet, start).len();
    assert_eq!(
        strikes, 2,
        "actual tipped rank3 Swipe must dispatch both its extra pet attack and the normal Tip attack"
    );
}

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_actual_pet_strike_crits_produce_selected_sic_em_bleed() {
    // SimC76f8118 sc_hunter.cpp2893–2898: actual owned Strike as One
    // crits execute the selected native pet bleed, without forcing a crit.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(255, 1).unwrap();
    hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
    cast(&mut run, 883, true);
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    engage(&mut run);
    for _ in 0..16 {
        cast(&mut run, 259489, false);
        cast(&mut run, 186270, false);
    }
    for _ in 0..40 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let mut saw_crit = false;
    let mut saw_bleed = false;
    for event in events {
        if event.source != Some(pet.to_bits())
            || event.target != Some(run.targets[0].to_bits())
            || event.kind != CombatLogKind::Damage
            || event.amount <= 0
        {
            continue;
        }
        if event.spell_id == Some(1251779) && event.crit {
            saw_crit = true;
        }
        if event.spell_id == Some(1253138) {
            assert!(
                saw_crit,
                "native Sic 'Em damage preceded any actual pet strike crit"
            );
            saw_bleed = true;
        }
    }
    assert!(
        saw_crit,
        "prerequisite: no actual owned Strike as One crit was observed"
    );
    assert!(
        saw_bleed,
        "actual owned strike crits produced no selected native Sic 'Em bleed"
    );
}
