use super::effects2_tests::use_ability;
use super::*;

fn sunshell_enemy_battle(catalog: &Catalog) -> Battle {
    // Synthetic enemy-side controls are not an authored Crystalsnap NPC team.
    let mut initial = battle(catalog, 7, 7, [2410, 595, 123]);
    initial.teams.swap(0, 1);
    initial
}

fn enemy_sunshell(initial: &Battle, catalog: &Catalog) -> (Battle, Vec<Event>) {
    initial
        .step(
            catalog,
            [Action::Pass, Action::Ability(2410)],
            &mut Fixed(500_000),
        )
        .unwrap()
}

#[test]
fn pet_battle_sunshell_enemy_all_active_pairs_damage_all_players_and_raise_only_maximum_hp() {
    let catalog = catalog();
    for player_active in 0..3 {
        for npc_active in 0..3 {
            let mut initial = sunshell_enemy_battle(&catalog);
            initial.teams[0].active = player_active;
            initial.teams[1].active = npc_active;
            let (state, events) = enemy_sunshell(&initial, &catalog);
            assert_eq!(damage(&events), [30; 3]);
            assert_eq!(
                state.teams[0].pets.each_ref().map(|pet| pet.health),
                [9970; 3]
            );
            assert_eq!(
                state.teams[1].pets.each_ref().map(|pet| pet.health),
                [10000; 3]
            );
            assert_eq!(state.teams[0].active, player_active);
            assert_eq!(state.teams[1].active, npc_active);
            assert_eq!(
                state.teams[1].pets[npc_active].cooldowns.get(&2410),
                Some(&3)
            );
            for team in &state.teams {
                for pet in &team.pets {
                    assert_eq!(pet.stats.health, 15000);
                    assert_eq!(pet.state(catalog.state_id("Weather_Sunlight").unwrap()), 1);
                    assert_eq!(
                        pet.state(catalog.state_id("Mod_HealingDealtPercent").unwrap()),
                        25
                    );
                }
            }
        }
    }
}

#[test]
fn pet_battle_sunshell_individual_and_all_player_misses_do_not_cancel_sunny_day() {
    let catalog = catalog();
    for player_active in 0..3 {
        for missed in [None, Some(0), Some(1), Some(2)] {
            let mut initial = sunshell_enemy_battle(&catalog);
            initial.teams[0].active = player_active;
            for slot in 0..3 {
                if missed.is_none() || missed == Some(slot) {
                    initial.teams[0].pets[slot]
                        .states
                        .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
                }
            }
            //70448 row4579/80 has Accuracy200 and ChainFailure0, independent of4576–4578.
            let (state, events) = enemy_sunshell(&initial, &catalog);
            assert_eq!(damage(&events).len(), if missed.is_none() { 0 } else { 2 });
            for slot in 0..3 {
                assert_eq!(
                    state.teams[0].pets[slot].health,
                    if missed.is_none() || missed == Some(slot) {
                        10000
                    } else {
                        9970
                    }
                );
            }
            assert!(
                state
                    .teams
                    .iter()
                    .flat_map(|team| &team.pets)
                    .all(|pet| pet.stats.health == 15000)
            );
        }
    }
}

#[test]
fn pet_battle_sunshell_dead_reserves_are_not_hit_or_resurrected() {
    let catalog = catalog();
    for dead in [1, 2] {
        let mut initial = sunshell_enemy_battle(&catalog);
        for team in &mut initial.teams {
            team.pets[dead].health = 0;
            team.pets[dead]
                .states
                .insert(catalog.state_id("Is_Dead").unwrap(), 1);
        }
        let (state, events) = enemy_sunshell(&initial, &catalog);
        assert_eq!(damage(&events), [30; 2]);
        for team in &state.teams {
            assert_eq!(team.pets[dead].health, 0);
            assert_eq!(
                team.pets[dead].state(catalog.state_id("Is_Dead").unwrap()),
                1
            );
            assert!(team.pets.iter().all(|pet| pet.stats.health == 15000));
        }
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::Revived(_)))
        );
    }
}

#[test]
fn pet_battle_sunshell_global_healing_bonus_and_expiry_reuse_sunlight_convention() {
    let catalog = catalog();
    let (mut state, _) = enemy_sunshell(&sunshell_enemy_battle(&catalog), &catalog);
    for team in &mut state.teams {
        team.pets[0].health = 5000;
    }
    let (mut state, _) = state
        .step(&catalog, [Action::Ability(123); 2], &mut Fixed(500_000))
        .unwrap();
    assert_eq!(state.teams[0].pets[0].health, 5225);
    assert_eq!(state.teams[1].pets[0].health, 5225);
    for _ in 0..7 {
        state = state
            .step(&catalog, [Action::Pass; 2], &mut Fixed(500_000))
            .unwrap()
            .0;
    }
    assert!(state.auras.is_empty());
    for pet in state.teams.iter().flat_map(|team| &team.pets) {
        assert_eq!(pet.stats.health, 10000);
        assert_eq!(pet.state(catalog.state_id("Weather_Sunlight").unwrap()), 0);
        assert_eq!(
            pet.state(catalog.state_id("Mod_HealingDealtPercent").unwrap()),
            0
        );
    }
}

#[test]
fn pet_battle_sunshell_weather_replacement_restores_maximum_and_removes_old_bonus() {
    let catalog = catalog();
    let (state, _) = enemy_sunshell(&sunshell_enemy_battle(&catalog), &catalog);
    let (state, _) = state
        .step(
            &catalog,
            [Action::Pass, Action::Ability(595)],
            &mut Fixed(500_000),
        )
        .unwrap();
    assert!(state.auras.iter().all(|aura| aura.ability != 403));
    for pet in state.teams.iter().flat_map(|team| &team.pets) {
        assert_eq!(pet.stats.health, 10000);
        assert_eq!(pet.state(catalog.state_id("Weather_Sunlight").unwrap()), 0);
        assert_eq!(
            pet.state(catalog.state_id("Mod_HealingDealtPercent").unwrap()),
            0
        );
        assert_eq!(pet.state(catalog.state_id("Weather_Moonlight").unwrap()), 1);
    }
}

fn shell_rush_battle(catalog: &Catalog) -> Battle {
    let mut initial = battle(catalog, 9, 4, [1989, 110, 123]);
    for (pet, (health, maximum)) in
        initial.teams[0]
            .pets
            .iter_mut()
            .zip([(9000, 10000), (2300, 2500), (4100, 20000)])
    {
        pet.health = health;
        pet.stats.health = maximum;
        pet.base_health = maximum;
    }
    initial
}

#[test]
fn pet_battle_shell_rush_all_active_pairs_hit_enemies_and_swap_the_casters_highest_hp_bench() {
    let catalog = catalog();
    for (ally_active, replacement) in [(0, 2), (1, 0), (2, 0)] {
        for enemy_active in 0..3 {
            let mut initial = shell_rush_battle(&catalog);
            initial.teams[0].active = ally_active;
            initial.teams[1].active = enemy_active;
            let (state, events) = use_ability(&initial, &catalog, 1989);
            assert_eq!(damage(&events), [72; 3]);
            assert_eq!(
                state.teams[1].pets.each_ref().map(|pet| pet.health),
                [9928; 3]
            );
            assert_eq!(state.teams[1].active, enemy_active);
            assert_eq!(state.teams[0].active, replacement);
            //4100/20000 beats2300/2500: selection is absolute HP, not percentage.
            assert_eq!(
                state.teams[0].pets.each_ref().map(|pet| pet.health),
                [9000, 2300, 4100]
            );
            assert_eq!(
                state.teams[0].pets.each_ref().map(|pet| pet.stats),
                initial.teams[0].pets.each_ref().map(|pet| pet.stats)
            );
            assert_eq!(
                state.teams[0].pets[ally_active].cooldowns.get(&1989),
                Some(&5)
            );
            assert!(
                !state.teams[0].pets[replacement]
                    .cooldowns
                    .contains_key(&1989)
            );
            assert!(events.iter().any(
                |event| matches!(event, Event::Swapped { team: 0, slot } if *slot == replacement)
            ));
            assert!(
                !events
                    .iter()
                    .any(|event| matches!(event, Event::Swapped { team: 1, .. }))
            );
        }
    }
}

#[test]
fn pet_battle_shell_rush_independent_swap_row_runs_after_individual_and_all_enemy_misses() {
    let catalog = catalog();
    for enemy_active in 0..3 {
        for missed in [None, Some(0), Some(1), Some(2)] {
            let mut initial = shell_rush_battle(&catalog);
            initial.teams[1].active = enemy_active;
            for slot in 0..3 {
                if missed.is_none() || missed == Some(slot) {
                    initial.teams[1].pets[slot]
                        .states
                        .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
                }
            }
            //70448 row3606 is145[100,0,0,0,0,0]: Accuracy only, no ChainFailure.
            //It is a separate effect after damage3603–3605 in turn3435, not a hit callback.
            let (state, events) = use_ability(&initial, &catalog, 1989);
            assert_eq!(state.teams[0].active, 2);
            assert_eq!(state.teams[1].active, enemy_active);
            for slot in 0..3 {
                let dodged = missed.is_none() || missed == Some(slot);
                assert_eq!(
                    state.teams[1].pets[slot].health,
                    if dodged { 10000 } else { 9928 }
                );
            }
            assert_eq!(damage(&events).len(), if missed.is_none() { 0 } else { 2 });
            assert_eq!(
                state.teams[0].pets.each_ref().map(|pet| pet.health),
                [9000, 2300, 4100]
            );
        }
    }
}

#[test]
fn pet_battle_shell_rush_skips_dead_bench_and_leaves_the_caster_when_no_bench_is_alive() {
    let catalog = catalog();
    for (dead_slots, replacement) in [(&[1][..], 2), (&[2][..], 1), (&[1, 2][..], 0)] {
        let mut initial = shell_rush_battle(&catalog);
        for &slot in dead_slots {
            initial.teams[0].pets[slot].health = 0;
            initial.teams[0].pets[slot]
                .states
                .insert(catalog.state_id("Is_Dead").unwrap(), 1);
        }
        let (state, events) = use_ability(&initial, &catalog, 1989);
        assert_eq!(damage(&events), [72; 3]);
        assert_eq!(state.teams[0].active, replacement);
        assert_eq!(
            state.teams[0].pets.each_ref().map(|pet| pet.health),
            initial.teams[0].pets.each_ref().map(|pet| pet.health)
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::Revived(_)))
        );
    }
}

#[test]
fn pet_battle_shell_rush_tied_bench_uses_existing_uniform_selection() {
    let catalog = catalog();
    for (draw, replacement) in [(0, 1), (999_999, 2)] {
        let mut initial = shell_rush_battle(&catalog);
        initial.teams[0].pets[1].health = 2400;
        initial.teams[0].pets[2].health = 2400;
        let (state, _) = initial
            .step(
                &catalog,
                [Action::Ability(1989), Action::Pass],
                &mut Fixed(draw),
            )
            .unwrap();
        assert_eq!(state.teams[0].active, replacement);
        assert_eq!(state.teams[1].active, 0);
    }
}

#[test]
fn pet_battle_shell_rush_keeps_existing_outgoing_and_incoming_swap_locks() {
    let catalog = catalog();
    for (slot, lock) in [(0, "swapOutLock"), (2, "swapInLock")] {
        let mut initial = shell_rush_battle(&catalog);
        initial.teams[0].pets[slot]
            .states
            .insert(catalog.state_id(lock).unwrap(), 1);
        let (state, events) = use_ability(&initial, &catalog, 1989);
        assert_eq!(damage(&events), [72; 3]);
        assert_eq!(state.teams[0].active, 0);
        assert_eq!(state.teams[1].active, 0);
        assert!(events.iter().any(|event| matches!(event, Event::Immune { effect: 3606, target } if *target == PetRef { team: 0, slot: 0 })));
    }
}

#[test]
fn pet_battle_shell_rush_end_of_action_policy_swaps_after_the_last_enemy_faints() {
    let catalog = catalog();
    let mut initial = shell_rush_battle(&catalog);
    initial.teams[1].pets[0].health = 1;
    for slot in 1..3 {
        initial.teams[1].pets[slot].health = 0;
        initial.teams[1].pets[slot]
            .states
            .insert(catalog.state_id("Is_Dead").unwrap(), 1);
    }
    //70448 row3606/145 has its own Accuracy100 and no link to hits3603–3605.
    //Finishing that row before Winner is existing end-of-action policy, not DB2-proven Retail timing.
    let (state, events) = use_ability(&initial, &catalog, 1989);
    assert_eq!(damage(&events), [1]);
    assert_eq!(state.teams[0].active, 2);
    assert_eq!(state.teams[1].pets.each_ref().map(|pet| pet.health), [0; 3]);
    assert_eq!(state.outcome, Some(Outcome::Winner(0)));
    let swapped = events
        .iter()
        .position(|event| matches!(event, Event::Swapped { team: 0, slot: 2 }))
        .unwrap();
    let finished = events
        .iter()
        .position(|event| matches!(event, Event::Finished(Outcome::Winner(0))))
        .unwrap();
    assert!(swapped < finished);
}
