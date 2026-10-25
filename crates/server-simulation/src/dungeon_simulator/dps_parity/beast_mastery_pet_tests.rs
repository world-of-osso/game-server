use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn beast_mastery_literal_pet_attacks_primary_target() {
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    engage(&mut run);
    for _ in 0..200 {
        run.app.update();
        drive_player(&mut run).expect("literal BM decision");
    }
    let pet = report::pet(run.app.world(), run.actor, &run.damage_targets);
    assert!(
        pet["damage"].as_u64().unwrap() > 0,
        "the native pet must attack rather than idle after literal engagement: {pet}"
    );
    assert!(
        !pet["casts"].as_array().unwrap().is_empty(),
        "pet must use its actual learned actions: {pet}"
    );
}

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn beast_mastery_literal_precombat_summons_native_pet() {
    let run = prepare_measurement(253, 1).expect("native literal BM fixture");
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor)
        .expect("literal summon_pet must acquire the authored pet before combat");
    assert_eq!(
        run.app
            .world()
            .get::<crate::hunter_pet::HunterPet>(pet)
            .unwrap()
            .owner,
        run.actor
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| event.spell_id == Some(883)
            && event.source == Some(run.actor.to_bits())
            && event.kind == shared::protocol::CombatLogKind::CastSuccess),
        "pet must be summoned by an ordinary native Call Pet cast"
    );
}
