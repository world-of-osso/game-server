use super::effects2_tests::use_ability;
use super::*;

fn winter_battle(catalog: &Catalog) -> Battle {
    battle(catalog, 7, 7, [1013, 595, 123])
}

#[test]
fn pet_battle_call_of_winter_all_active_pairs_test_each_recipients_existing_chill() {
    let catalog = catalog();
    let chilled = catalog.state_id("Mechanic_IsChilled").unwrap();
    for ally_active in 0..3 {
        for enemy_active in 0..3 {
            for chilled_slots in [&[][..], &[0][..], &[1][..], &[2][..], &[0, 1, 2][..]] {
                let mut initial = winter_battle(&catalog);
                initial.teams[0].active = ally_active;
                initial.teams[1].active = enemy_active;
                for &slot in chilled_slots {
                    initial.teams[1].pets[slot].states.insert(chilled, 1);
                }
                let (state, events) = use_ability(&initial, &catalog, 1013);
                assert_eq!(damage(&events).len(), 3 + chilled_slots.len());
                for slot in 0..3 {
                    let expected = if chilled_slots.contains(&slot) {
                        9940
                    } else {
                        9952
                    };
                    assert_eq!(state.teams[1].pets[slot].health, expected);
                    assert_eq!(state.teams[0].pets[slot].health, 10000);
                    //Weather adds its delta without replacing an existing Chilled state.
                    assert_eq!(
                        state.teams[1].pets[slot].state(chilled),
                        if chilled_slots.contains(&slot) { 2 } else { 1 }
                    );
                }
                assert_eq!(state.teams[0].active, ally_active);
                assert_eq!(state.teams[1].active, enemy_active);
                assert_eq!(
                    state.teams[0].pets[ally_active].cooldowns.get(&1013),
                    Some(&3)
                );
                //Weather205 is applied by row2301 AFTER all six damage rows.
                //It cannot supply Chilled or its25% bonus retroactively to these hits.
                for pet in state.teams.iter().flat_map(|team| &team.pets) {
                    assert!(pet.state(chilled) > 0);
                    assert_eq!(pet.state(catalog.state_id("Weather_Blizzard").unwrap()), 1);
                    assert_eq!(
                        pet.state(catalog.state_id("Mod_PetTypeDamageDealtPercent").unwrap()),
                        25
                    );
                }
            }
        }
    }
}

#[test]
fn pet_battle_call_of_winter_individual_and_all_misses_still_apply_blizzard() {
    let catalog = catalog();
    for enemy_active in 0..3 {
        for missed in [None, Some(0), Some(1), Some(2)] {
            let mut initial = winter_battle(&catalog);
            initial.teams[1].active = enemy_active;
            for slot in 0..3 {
                initial.teams[1].pets[slot]
                    .states
                    .insert(catalog.state_id("Mechanic_IsChilled").unwrap(), 1);
                if missed.is_none() || missed == Some(slot) {
                    initial.teams[1].pets[slot]
                        .states
                        .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
                }
            }
            let (state, events) = use_ability(&initial, &catalog, 1013);
            assert_eq!(damage(&events).len(), if missed.is_none() { 0 } else { 4 });
            for slot in 0..3 {
                let dodged = missed.is_none() || missed == Some(slot);
                assert_eq!(
                    state.teams[1].pets[slot].health,
                    if dodged { 10000 } else { 9940 }
                );
            }
            assert!(
                state
                    .teams
                    .iter()
                    .flat_map(|team| &team.pets)
                    .all(|pet| pet.state(catalog.state_id("Weather_Blizzard").unwrap()) == 1)
            );
        }
    }
}

#[test]
fn pet_battle_call_of_winter_skips_dead_backlines_without_redirecting_their_bonus() {
    let catalog = catalog();
    for dead in [1, 2] {
        let mut initial = winter_battle(&catalog);
        for team in &mut initial.teams {
            team.pets[dead].health = 0;
            team.pets[dead]
                .states
                .insert(catalog.state_id("Is_Dead").unwrap(), 1);
            team.pets[dead]
                .states
                .insert(catalog.state_id("Mechanic_IsChilled").unwrap(), 1);
        }
        let (state, events) = use_ability(&initial, &catalog, 1013);
        assert_eq!(damage(&events), [48; 2]);
        for team in &state.teams {
            assert_eq!(team.pets[dead].health, 0);
            assert_eq!(
                team.pets[dead].state(catalog.state_id("Is_Dead").unwrap()),
                1
            );
        }
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::Revived(_)))
        );
    }
}

#[test]
fn pet_battle_call_of_winter_recast_uses_prior_blizzard_bonus_and_chill() {
    let catalog = catalog();
    let (mut state, events) = use_ability(&winter_battle(&catalog), &catalog, 1013);
    assert_eq!(damage(&events), [48; 3]);
    for _ in 0..3 {
        state = state
            .step(&catalog, [Action::Pass; 2], &mut Fixed(500_000))
            .unwrap()
            .0;
    }
    let (state, events) = use_ability(&state, &catalog, 1013);
    assert_eq!(damage(&events), [60, 15, 60, 15, 60, 15]);
    assert_eq!(
        state.teams[1].pets.each_ref().map(|pet| pet.health),
        [9877; 3]
    );
}

#[test]
fn pet_battle_call_of_winter_weather_replaces_and_expires_with_existing_clock() {
    let catalog = catalog();
    let (initial, _) = use_ability(&winter_battle(&catalog), &catalog, 1013);
    let mut expired = initial.clone();
    for _ in 0..8 {
        expired = expired
            .step(&catalog, [Action::Pass; 2], &mut Fixed(500_000))
            .unwrap()
            .0;
    }
    assert!(expired.auras.is_empty());
    for pet in expired.teams.iter().flat_map(|team| &team.pets) {
        assert_eq!(pet.state(catalog.state_id("Weather_Blizzard").unwrap()), 0);
        assert_eq!(
            pet.state(catalog.state_id("Mechanic_IsChilled").unwrap()),
            0
        );
        assert_eq!(
            pet.state(catalog.state_id("Mod_PetTypeDamageDealtPercent").unwrap()),
            0
        );
    }
    let (replaced, _) = use_ability(&initial, &catalog, 595);
    assert!(replaced.auras.iter().all(|aura| aura.ability != 205));
    for pet in replaced.teams.iter().flat_map(|team| &team.pets) {
        assert_eq!(pet.state(catalog.state_id("Weather_Blizzard").unwrap()), 0);
        assert_eq!(
            pet.state(catalog.state_id("Mechanic_IsChilled").unwrap()),
            0
        );
        assert_eq!(pet.state(catalog.state_id("Weather_Moonlight").unwrap()), 1);
    }
}

#[test]
fn pet_battle_call_of_winter_existing_end_of_action_policy_applies_weather_before_winner() {
    let catalog = catalog();
    let mut initial = winter_battle(&catalog);
    initial.teams[1].pets[0].health = 1;
    for slot in 1..3 {
        initial.teams[1].pets[slot].health = 0;
        initial.teams[1].pets[slot]
            .states
            .insert(catalog.state_id("Is_Dead").unwrap(), 1);
    }
    //Row2301 has Accuracy200/ChainFailure0. Finishing it before Winner is the
    //existing end-of-action policy, not independently proven Retail terminal timing.
    let (state, events) = use_ability(&initial, &catalog, 1013);
    assert_eq!(damage(&events), [1]);
    assert_eq!(state.outcome, Some(Outcome::Winner(0)));
    assert_eq!(
        state.teams[0].pets[0].state(catalog.state_id("Weather_Blizzard").unwrap()),
        1
    );
}
