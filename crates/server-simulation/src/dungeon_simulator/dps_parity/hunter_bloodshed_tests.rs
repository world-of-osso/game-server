use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_bm_wrath_applies_bloodshed_from_each_owned_pet() {
    let _seed = crate::random::seed_scope(70_448);
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
    let landed: Vec<_> = pets
        .iter()
        .copied()
        .filter(|pet| {
            auras
                .iter()
                .any(|aura| aura.spell_id == 321538 && aura.caster == *pet)
        })
        .collect();
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for pet in &pets {
        // Native physical application can be dodged. Every actual owned pet
        // must attempt Bloodshed; only landed applications can own its DoT.
        let missed = events.iter().any(|event| {
            event.source == Some(pet.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(321538)
                && matches!(event.kind, shared::protocol::CombatLogKind::Miss(_))
        });
        assert!(
            landed.contains(pet) || missed,
            "each owned pet must actually apply or miss native Bloodshed: pet={pet}, events={events:?}"
        );
    }
    assert!(
        !landed.is_empty(),
        "fixture must exercise actual native Bloodshed ticks"
    );
    eprintln!(
        "NATIVE_BLOODSHED_OUTCOMES attempts={} landed={}",
        pets.len(),
        landed.len()
    );
    for _ in 0..40 {
        run.app.update();
    }
    let combat = report::combat(run.app.world(), &run.damage_targets, 3.0);
    for pet in landed {
        assert!(
            combat["actions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|action| action["actor"] == pet.to_bits()
                    && action["spell_id"] == 321538
                    && action["damage"].as_u64().unwrap() > 0),
            "each landed pet Bloodshed must tick actual damage: pet={pet}; {combat}"
        );
    }
}
