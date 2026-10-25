use super::effects2_tests::use_ability;
use super::*;

//785 is the real aura program, exercised directly at the turn-engine boundary.
//This does not claim selectable/journal785 or the rejected786 pad lifecycle.
fn frostbite_battle(catalog: &Catalog) -> Battle {
    battle(catalog, 9, 6, [785, 110, 123])
}

#[test]
fn pet_battle_frostbite_refresh_keeps_every_stack_alive_then_expires_without_reapplication() {
    let catalog = catalog();
    let initial = frostbite_battle(&catalog);
    let (first, events) = use_ability(&initial, &catalog, 785);
    assert_eq!(damage(&events), [12]);
    let (stacked, events) = use_ability(&first, &catalog, 785);
    assert_eq!(damage(&events), [12; 2]);
    assert_eq!(stacked.auras.len(), 2);
    let (one_left, events) = stacked
        .step(&catalog, [Action::Pass; 2], &mut Fixed(500000))
        .unwrap();
    assert_eq!(damage(&events), [12; 2]);
    assert_eq!(one_left.auras.len(), 2);
    let (expired, events) = one_left
        .step(&catalog, [Action::Pass; 2], &mut Fixed(500000))
        .unwrap();
    assert_eq!(damage(&events), [12; 2]);
    assert!(expired.auras.is_empty());
    assert_eq!(
        expired.teams[1].pets[0].state(catalog.state_id("Mechanic_IsChilled").unwrap()),
        0
    );
}

#[test]
fn pet_battle_frostbite_refresh_does_not_refresh_another_victims_stacks() {
    let catalog = catalog();
    let initial = frostbite_battle(&catalog);
    let (mut first, _) = use_ability(&initial, &catalog, 785);
    first.teams[1].pets[0].abilities = [785, 110, 123];
    let (opposed, _) = first
        .step(
            &catalog,
            [Action::Pass, Action::Ability(785)],
            &mut Fixed(500000),
        )
        .unwrap();
    let (refreshed, _) = use_ability(&opposed, &catalog, 785);
    assert_eq!(refreshed.auras.len(), 3);
    let (after, _) = refreshed
        .step(&catalog, [Action::Pass; 2], &mut Fixed(500000))
        .unwrap();
    assert_eq!(after.auras.len(), 2);
    assert!(
        after
            .auras
            .iter()
            .all(|aura| aura.target == PetRef { team: 1, slot: 0 })
    );
}

#[test]
fn pet_battle_frostbite_refresh_preserves_each_original_casters_damage_after_swap() {
    let catalog = catalog();
    let initial = frostbite_battle(&catalog);
    let (first, _) = use_ability(&initial, &catalog, 785);
    let (mut stacked, _) = use_ability(&first, &catalog, 785);
    stacked.teams[0].pets[1].stats.power = 200;
    let (swapped, _) = stacked
        .step(
            &catalog,
            [Action::Swap(1), Action::Pass],
            &mut Fixed(500000),
        )
        .unwrap();
    let (_, events) = use_ability(&swapped, &catalog, 785);
    assert_eq!(damage(&events), [12, 12, 22]);
    let casters: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Event::Damage { caster, .. } => Some(*caster),
            _ => None,
        })
        .collect();
    assert_eq!(
        casters,
        [
            PetRef { team: 0, slot: 0 },
            PetRef { team: 0, slot: 0 },
            PetRef { team: 0, slot: 1 }
        ]
    );
}

#[test]
fn pet_battle_frostbite_refresh_is_independent_of_new_stack_miss_and_does_not_create_stacks() {
    let catalog = catalog();
    let mut initial = frostbite_battle(&catalog);
    initial.teams[0].pets[0]
        .states
        .insert(catalog.state_id("Stat_Accuracy").unwrap(), -100);
    let (empty, _) = use_ability(&initial, &catalog, 785);
    assert!(empty.auras.is_empty());
    initial.teams[0].pets[0]
        .states
        .remove(&catalog.state_id("Stat_Accuracy").unwrap());
    let (mut applied, _) = use_ability(&initial, &catalog, 785);
    applied.teams[0].pets[0]
        .states
        .insert(catalog.state_id("Stat_Accuracy").unwrap(), -100);
    let (mut refreshed, _) = use_ability(&applied, &catalog, 785);
    assert_eq!(refreshed.auras.len(), 1);
    refreshed.teams[0].pets[0]
        .states
        .remove(&catalog.state_id("Stat_Accuracy").unwrap());
    let (still_present, events) = refreshed
        .step(&catalog, [Action::Pass; 2], &mut Fixed(500000))
        .unwrap();
    assert_eq!(damage(&events), [12]);
    assert_eq!(still_present.auras.len(), 1);
    let (expired, events) = still_present
        .step(&catalog, [Action::Pass; 2], &mut Fixed(500000))
        .unwrap();
    assert_eq!(damage(&events), [12]);
    assert!(expired.auras.is_empty());
}

#[test]
fn pet_battle_frostbite_refresh_does_not_admit_unproven_blistering_cold_pad_follow() {
    let catalog = catalog();
    let initial = battle(&catalog, 9, 6, [786, 110, 123]);
    let error = initial
        .step(&catalog, [Action::Ability(786), Action::Pass], &mut NoDraw)
        .unwrap_err();
    assert!(
        error.contains("PET_BATTLE_BLISTERING_COLD_PAD_FOLLOW_UNPROVEN"),
        "{error}"
    );
}
