use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_bm_wrath_applies_pet_buffs_and_delayed_apex_strike() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    engage(&mut run);
    let permanent: Vec<_> = run
        .app
        .world_mut()
        .query::<(Entity, &crate::hunter_pet::HunterPet)>()
        .iter(run.app.world())
        .filter(|(_, pet)| pet.owner == run.actor)
        .map(|(unit, _)| unit)
        .collect();
    let started = crate::spell_cast::now(run.app.world());
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(19574, run.actor, Some(run.targets[0])),
    )
    .expect("ordinary Bestial Wrath");
    let apex = run
        .app
        .world_mut()
        .query::<(Entity, &crate::hunter_pet::HunterPet)>()
        .iter(run.app.world())
        .find(|(unit, pet)| pet.owner == run.actor && !permanent.contains(unit))
        .map(|(unit, _)| unit)
        .expect("native Nature's Ally summon");
    for pet in permanent {
        assert!(
            crate::auras::has_aura(run.app.world(), pet, 186254),
            "Bestial Wrath must apply each permanent pet's native damage aura after its strike"
        );
    }
    while crate::spell_cast::now(run.app.world()) < started + 0.4 {
        run.app.update();
    }
    assert!(
        !crate::auras::has_aura(run.app.world(), apex, 1285912),
        "Apex aura must retain the sourced450ms delay"
    );
    while crate::spell_cast::now(run.app.world()) < started + 0.5 {
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), apex, 1285912),
        "Apex must receive its distinct native damage aura after450ms"
    );
    while crate::spell_cast::now(run.app.world()) < started + 1.4 {
        run.app.update();
    }
    let strikes = |run: &Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(apex.to_bits())
                    && event.spell_id == Some(344572)
                    && matches!(
                        event.kind,
                        shared::protocol::CombatLogKind::Damage
                            | shared::protocol::CombatLogKind::Miss(_)
                    )
            })
            .count()
    };
    assert_eq!(
        strikes(&run),
        0,
        "Apex strike cannot precede its sourced1.5s delay"
    );
    while crate::spell_cast::now(run.app.world()) < started + 1.6 {
        run.app.update();
    }
    assert!(
        strikes(&run) > 0,
        "Apex must execute its real native Bestial Wrath strike after1.5s"
    );
}
