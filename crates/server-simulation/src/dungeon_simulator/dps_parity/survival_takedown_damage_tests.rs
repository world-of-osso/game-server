//! Real Takedown emits owner and owned-pet native damage.
use super::marksmanship_mechanics_tests::cast;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_takedown_dispatches_owner_and_owned_pet_damage() {
    // SimC76f8118 sc_hunter.cpp6396–6405: native Takedown buff/mark precede
    // both distinct real-actor impact payloads on the actual hostile primary.
    let mut player_hits = 0;
    let mut pet_hits = 0;
    for seed in [70448, 0, 1, 2, 3, 4, 5, 6] {
        let (player, pet) = assert_takedown_outcomes(seed);
        player_hits += usize::from(player);
        pet_hits += usize::from(pet);
    }
    assert!(
        player_hits > 0 && pet_hits > 0,
        "both actual actors must deal positive native damage across the fixed matrix"
    );
}

#[test]
#[ignore = "native Takedown pet impact after lethal owner damage"]
fn survival_takedown_pet_cancels_after_owner_kills_primary() {
    use shared::components::Health;
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(255, 3).unwrap();
    hunter_fixture::require_profile_pet(run.app.world(), run.actor).unwrap();
    cast(&mut run, 883, true);
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let primary = run.targets[0];
    run.app
        .world_mut()
        .entity_mut(primary)
        .remove::<crate::creature_scripts::TrainingDummy>();
    run.app
        .world_mut()
        .get_mut::<Health>(primary)
        .unwrap()
        .current = 1.0;
    cast(&mut run, 1250646, false);
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .get::<Health>(primary)
            .is_none_or(|health| health.current <= 0.0)
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(primary.to_bits())
                && event.spell_id == Some(1253859)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "the owner impact must kill the primary"
    );
    assert!(
        !events
            .iter()
            .any(|event| event.source == Some(pet.to_bits())
                && event.spell_id == Some(1253862)
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))),
        "the single-target pet impact must cancel after its primary dies"
    );
}

fn assert_takedown_outcomes(seed: u64) -> (bool, bool) {
    let _seed = crate::random::seed_scope(seed);
    let mut run = prepare_literal(255, 1).unwrap();
    hunter_fixture::require_profile_pet(run.app.world(), run.actor).unwrap();
    cast(&mut run, 883, true);
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    engage(&mut run);
    cast(&mut run, 1250646, false);
    for _ in 0..40 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let mut positive = [false; 2];
    for (index, (actor, spell)) in [(run.actor, 1253859), (pet, 1253862)]
        .into_iter()
        .enumerate()
    {
        // Native fc87ee307c shows the real pet child can be dodged. Require
        // exactly one actual owned outcome for every seed, not a guaranteed hit.
        let outcomes: Vec<_> = events
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(spell)
                    && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
            })
            .collect();
        assert_eq!(
            outcomes.len(),
            1,
            "seed{seed}: actual actor{actor:?} must emit one native spell{spell} outcome"
        );
        positive[index] = outcomes[0].kind == CombatLogKind::Damage && outcomes[0].amount > 0;
    }
    (positive[0], positive[1])
}
