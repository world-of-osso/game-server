//! Real pet weapon-spell attacks inherit the owner's hit/expertise mean.
use super::marksmanship_mechanics_tests::cast;
use super::*;
use shared::protocol::{CombatLogKind, MissKind};

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_pet_weapon_spells_inherit_owner_expertise_against_dodge() {
    // SimC76f8118 pet.hpp103–104/pet.cpp414–420 and attack.cpp179–181/277:
    // every pet attack's dodge roll uses the owner's hit/expertise mean.
    // Exercise every seed in this fixed matrix, never retry until one passes.
    let mut outcomes = Vec::new();
    for seed in 0..32 {
        outcomes.extend(read_pet_spell_outcomes(seed));
    }
    assert!(
        outcomes.len() >= 64,
        "real Kill Command and Strike must produce pet outcomes: {}",
        outcomes.len()
    );
    let dodged: Vec<_> = outcomes
        .iter()
        .filter(|(_, _, kind)| *kind == CombatLogKind::Miss(MissKind::Dodge))
        .collect();
    assert!(
        dodged.is_empty(),
        "pet weapon spells must not be dodged by a level-matched enemy: {dodged:?}"
    );
}

fn read_pet_spell_outcomes(seed: u64) -> Vec<(u64, u32, CombatLogKind)> {
    let _seed = crate::random::seed_scope(seed);
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
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(pet.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && matches!(event.spell_id, Some(259277 | 1251779))
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
        })
        .map(|event| (seed, event.spell_id.unwrap(), event.kind))
        .collect()
}
