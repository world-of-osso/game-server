use super::effects2_tests::use_ability;
use super::*;

fn primal_cry_battle(catalog: &Catalog, ally_active: usize, enemy_active: usize) -> Battle {
    let mut state = battle(catalog, 9, 0, [920, 110, 123]);
    state.teams[0].active = ally_active;
    state.teams[1].active = enemy_active;
    state.teams[0].pets[ally_active].stats.speed = 200;
    state
}

#[test]
fn pet_battle_primal_cry_pairs_damage_and_distinct_slows_for_all_active_slots() {
    let catalog = catalog();
    for ally_active in 0..3 {
        for enemy_active in 0..3 {
            let initial = primal_cry_battle(&catalog, ally_active, enemy_active);
            let (mut state, events) = use_ability(&initial, &catalog, 920);
            assert_eq!(
                state.teams[1].pets.each_ref().map(|pet| pet.health),
                [9940; 3]
            );
            assert_eq!(
                state.teams[0].pets.each_ref().map(|pet| pet.health),
                [10000; 3]
            );
            assert_eq!(damage(&events), [60; 3]);
            assert_eq!(state.auras.len(), 3);
            for slot in 0..3 {
                let target = PetRef { team: 1, slot };
                let aura = state.auras.iter().find(|a| a.target == target).unwrap();
                assert_eq!(aura.ability, 154);
                assert_eq!(
                    aura.caster,
                    PetRef {
                        team: 0,
                        slot: ally_active
                    }
                );
                assert_eq!(
                    state
                        .pet(target)
                        .state(catalog.state_id("Mod_SpeedPercent").unwrap()),
                    -25
                );
            }
            // Actual next-round order proves100-speed foes became75, below our80.
            state.teams[0].pets[ally_active].stats.speed = 80;
            let (_, events) = state
                .step(&catalog, [Action::Pass; 2], &mut Fixed(500_000))
                .unwrap();
            assert!(events.contains(&Event::Order([0, 1])));
        }
    }
}

#[test]
fn pet_battle_primal_cry_damage_miss_skips_only_matching_slow_and_next_pair_still_hits() {
    let catalog = catalog();
    for active in 0..3 {
        for missed in 0..3 {
            let mut initial = primal_cry_battle(&catalog, 2, active);
            initial.teams[1].pets[missed]
                .states
                .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
            let (state, events) = use_ability(&initial, &catalog, 920);
            for slot in 0..3 {
                let pet = &state.teams[1].pets[slot];
                assert_eq!(pet.health, if slot == missed { 10000 } else { 9940 });
                assert_eq!(
                    pet.state(catalog.state_id("Mod_SpeedPercent").unwrap()),
                    if slot == missed { 0 } else { -25 }
                );
            }
            assert_eq!(state.auras.len(), 2);
            assert!(!events.iter().any(|event| matches!(event, Event::AuraApplied { target, .. } if *target == PetRef { team: 1, slot: missed })));
        }
    }
}

#[test]
fn pet_battle_primal_cry_refreshes_live_slows_without_stacking() {
    let catalog = catalog();
    let initial = primal_cry_battle(&catalog, 0, 2);
    let (mut state, _) = use_ability(&initial, &catalog, 920);
    for _ in 0..2 {
        state = state
            .step(&catalog, [Action::Pass; 2], &mut Fixed(500_000))
            .unwrap()
            .0;
    }
    // Existing enemy-aura timing includes the application round; refresh before expiry.
    assert!(state.auras.iter().all(|aura| aura.remaining == 1));
    let (state, _) = use_ability(&state, &catalog, 920);
    assert_eq!(
        state.teams[1].pets.each_ref().map(|pet| pet.health),
        [9880; 3]
    );
    assert_eq!(state.auras.len(), 3);
    for pet in &state.teams[1].pets {
        assert_eq!(
            pet.state(catalog.state_id("Mod_SpeedPercent").unwrap()),
            -25
        );
    }
    assert!(state.auras.iter().all(|aura| aura.remaining == 3));
}

#[test]
fn pet_battle_primal_cry_dead_reserve_is_neither_damaged_slowed_nor_revived() {
    let catalog = catalog();
    let mut initial = primal_cry_battle(&catalog, 0, 1);
    initial.teams[1].pets[0].health = 0;
    initial.teams[1].pets[0]
        .states
        .insert(catalog.state_id("Is_Dead").unwrap(), 1);
    let (state, events) = use_ability(&initial, &catalog, 920);
    assert_eq!(
        state.teams[1].pets.each_ref().map(|pet| pet.health),
        [0, 9940, 9940]
    );
    assert_eq!(
        state.teams[1].pets[0].state(catalog.state_id("Mod_SpeedPercent").unwrap()),
        0
    );
    assert_eq!(state.auras.len(), 2);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Revived(_)))
    );
}
