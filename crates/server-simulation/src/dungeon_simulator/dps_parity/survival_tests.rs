//! Native Survival fixture contracts; no follower kit or substitute priorities.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_literal_precombat_summons_native_pet() {
    let run = prepare_measurement(255, 1).expect("native Survival player fixture");
    let driver = run.spec_driver.as_ref().expect("literal Survival driver");
    assert_eq!(driver.label(), "LITERAL_SIMC_SURVIVAL_76f8118");
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor)
        .expect("precombat Call Pet must summon the native stable pet");
    assert_eq!(
        run.app.world().get::<crate::hunter_pet::HunterPet>(pet).unwrap().owner,
        run.actor
    );
    let events = &run.app.world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
    assert!(events.iter().any(|event| event.source == Some(run.actor.to_bits())
        && event.spell_id == Some(883) && event.kind == CombatLogKind::CastSuccess));
}

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_literal_actions_and_pet_damage_actual_primary() {
    let mut run = prepare_measurement(255, 1).expect("native Survival player fixture");
    assert!(run.spec_driver.is_some(), "Survival cannot use AssistedCombat");
    engage(&mut run);
    for _ in 0..200 {
        run.app.update();
        drive_player(&mut run).expect("literal Survival decision");
    }
    let events = &run.app.world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
    assert!(events.iter().any(|event| event.source == Some(run.actor.to_bits())
        && event.spell_id == Some(259489) && event.kind == CombatLogKind::CastSuccess),
        "literal APL must execute native Survival Kill Command");
    let pet = report::pet(run.app.world(), run.actor, &run.damage_targets);
    assert!(pet["damage"].as_u64().unwrap() > 0,
        "the native summoned pet must attack the actual primary: {pet}");
}
