//! Real Takedown emits owner and owned-pet native damage.
use super::marksmanship_mechanics_tests::cast;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_takedown_dispatches_owner_and_owned_pet_damage() {
    // SimC76f8118 sc_hunter.cpp6396–6405: native Takedown buff/mark precede
    // both distinct real-actor impact payloads on the actual hostile primary.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(255, 1).unwrap();
    hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
    cast(&mut run, 883, true);
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    engage(&mut run);
    cast(&mut run, 1250646, false);
    for _ in 0..40 {
        run.app.update();
    }
    let events = &run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
    for (actor, spell) in [(run.actor, 1253859), (pet, 1253862)] {
        assert!(events.iter().any(|event| event.source == Some(actor.to_bits())
            && event.target == Some(run.targets[0].to_bits()) && event.spell_id == Some(spell)
            && event.kind == CombatLogKind::Damage && event.amount > 0),
            "actual Takedown must produce positive native spell{spell} from actor{actor:?}");
    }
}
