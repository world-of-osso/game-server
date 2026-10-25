use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_bm_wrath_applies_bloodshed_from_each_owned_pet() {
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(19574, run.actor, Some(run.targets[0])),
    )
    .expect("ordinary Bestial Wrath with selected primary");
    preparation::finish_pending_setup(&mut run).unwrap();
    let pets: Vec<_> = run
        .app
        .world_mut()
        .query::<(Entity, &crate::hunter_pet::HunterPet)>()
        .iter(run.app.world())
        .filter(|(_, pet)| pet.owner == run.actor)
        .map(|(unit, _)| unit)
        .collect();
    assert!(!pets.is_empty());
    let auras = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.targets[0])
        .unwrap();
    for pet in &pets {
        assert!(
            auras
                .iter()
                .any(|aura| aura.spell_id == 321538 && aura.caster == *pet),
            "selected Bloodshed must be a real native aura owned by each pet: {pet}"
        );
    }
    for _ in 0..40 {
        run.app.update();
    }
    let combat = report::combat(run.app.world(), &run.damage_targets, 3.0);
    for pet in pets {
        assert!(
            combat["actions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|action| action["actor"] == pet.to_bits()
                    && action["spell_id"] == 321538
                    && action["damage"].as_u64().unwrap() > 0),
            "each pet's native Bloodshed must tick actual damage: {pet}; {combat}"
        );
    }
}
