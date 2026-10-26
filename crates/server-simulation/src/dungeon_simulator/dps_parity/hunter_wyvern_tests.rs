use super::*;

fn owned_cry(run: &Run) -> crate::auras::AuraInstance {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 471881 && aura.caster == run.actor)
        .expect("actual native Wyvern Cry")
        .clone()
}

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_hunter_stable_white_impacts_grow_earned_wyvern_cry_without_refresh() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    engage(&mut run);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), run.actor, run.targets[0]);
    run.app.update();
    let deadline = crate::spell_cast::now(run.app.world()) + 31.0;
    while !crate::auras::has_aura(run.app.world(), run.actor, 471878)
        && crate::spell_cast::now(run.app.world()) < deadline
    {
        run.app.update();
    }
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 471878));
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        34026,
        Some(run.targets[0]),
        false,
    )
    .expect("actual Kill Command earns real Wyvern Cry");
    // Match the ordinary literal Hunter driver's explicit pet Attack order.
    crate::pet_control::order_companion_pet(run.app.world_mut(), run.actor, Some(run.targets[0]));
    let before = owned_cry(&run);
    assert!(
        before.stacks < before.max_stacks,
        "native Cry must have room to grow"
    );
    let pets: Vec<_> = run
        .app
        .world_mut()
        .query::<(Entity, &crate::hunter_pet::HunterPet)>()
        .iter(run.app.world())
        .filter(|(_, pet)| pet.owner == run.actor)
        .map(|(pet, _)| pet.to_bits())
        .collect();
    let attempts = |run: &Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source.is_some_and(|source| pets.contains(&source))
                    && event.spell_id.is_none()
                    && matches!(
                        event.kind,
                        shared::protocol::CombatLogKind::Damage
                            | shared::protocol::CombatLogKind::Miss(_)
                    )
            })
            .count()
    };
    let before_impacts = attempts(&run);
    let deadline = crate::spell_cast::now(run.app.world()) + 5.0;
    while attempts(&run) == before_impacts && crate::spell_cast::now(run.app.world()) < deadline {
        run.app.update();
    }
    let impacts = attempts(&run) - before_impacts;
    assert!(
        impacts > 0,
        "actual owned stable bodies must swing at the native target"
    );
    let after = owned_cry(&run);
    // SimC76f8118 hunter2974–3010: every stable-body white impact increments
    // an already active Cry, using its remaining duration, not a fresh duration.
    let expected = (usize::from(before.stacks) + impacts).min(usize::from(before.max_stacks));
    assert_eq!(
        usize::from(after.stacks),
        expected,
        "native stable white impacts must grow existing Cry: before={}, impacts={impacts}",
        before.stacks
    );
    assert_eq!(
        after.expires_at, before.expires_at,
        "stack growth cannot refresh Cry's lifetime"
    );
}
