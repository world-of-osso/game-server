use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_bm_animal_companion_follows_main_pet_lifecycle_and_commands() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    let main = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    let pets: Vec<_> = run
        .app
        .world_mut()
        .query::<(Entity, &crate::hunter_pet::HunterPet)>()
        .iter(run.app.world())
        .filter(|(_, pet)| pet.owner == run.actor)
        .map(|(unit, _)| unit)
        .collect();
    assert_eq!(
        pets.len(),
        2,
        "ordinary Call Pet with selected native Animal Companion must create a second permanent body"
    );
    let companion = *pets.iter().find(|&&unit| unit != main).unwrap();
    engage(&mut run);
    crate::pet_control::order_companion_pet(run.app.world_mut(), run.actor, Some(run.targets[0]));
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        34026,
        Some(run.targets[0]),
        false,
    )
    .expect("native Kill Command");
    for _ in 0..200 {
        run.app.update();
    }
    let combat = report::combat(run.app.world(), &run.damage_targets, 10.0);
    let actions = combat["actions"].as_array().unwrap();
    assert!(
        actions
            .iter()
            .any(|action| action["actor"] == companion.to_bits()
                && action["spell_id"].is_null()
                && action["damage"].as_u64().unwrap() > 0),
        "the companion must obey the main pet's native attack order and make white swings: {combat}"
    );
    assert!(
        actions
            .iter()
            .any(|action| action["actor"] == companion.to_bits() && action["spell_id"] == 83381),
        "the companion must receive actual Kill Command: {combat}"
    );
    assert!(
        !actions
            .iter()
            .any(|action| action["actor"] == companion.to_bits() && action["spell_id"] == 49966),
        "only the main pet spends Focus on its family basic attack"
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, 2641, Some(run.actor), false)
        .expect("ordinary Dismiss Pet");
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(
        run.app.world().get_entity(main).is_err(),
        "native dismissal removes the main pet"
    );
    assert!(
        run.app.world().get_entity(companion).is_err(),
        "companion lifetime ends with its main pet"
    );
    assert!(crate::hunter_pet::summoned_pet(run.app.world(), run.actor).is_none());
}
