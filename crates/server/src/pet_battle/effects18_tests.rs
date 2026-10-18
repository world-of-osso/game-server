use super::*;

fn gears_enemy_battle(catalog: &Catalog) -> Battle {
    //Synthetic controls do not claim Math's authored NPC team or Elite fidelity.
    let mut initial = battle(catalog, 9, 9, [2351, 110, 123]);
    initial.teams.swap(0, 1);
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

fn enemy_gears(initial: &Battle, catalog: &Catalog) -> (Battle, Vec<Event>) {
    initial
        .step(
            catalog,
            [Action::Pass, Action::Ability(2351)],
            &mut Fixed(500_000),
        )
        .unwrap()
}

#[test]
fn pet_battle_whirling_gears_all_active_pairs_swap_the_players_lowest_absolute_hp_bench() {
    let catalog = catalog();
    for (player_active, replacement) in [(0, 1), (1, 2), (2, 1)] {
        for npc_active in 0..3 {
            let mut initial = gears_enemy_battle(&catalog);
            initial.teams[0].active = player_active;
            initial.teams[1].active = npc_active;
            let (state, events) = enemy_gears(&initial, &catalog);
            assert_eq!(damage(&events), [72; 3]);
            assert_eq!(
                state.teams[0].pets.each_ref().map(|pet| pet.health),
                [8928, 2228, 4028]
            );
            //2228/2500 is higher percentage than4028/20000, but lower absolute HP.
            assert_eq!(state.teams[0].active, replacement);
            assert_eq!(state.teams[1].active, npc_active);
            assert_eq!(
                state.teams[0].pets.each_ref().map(|pet| pet.stats),
                initial.teams[0].pets.each_ref().map(|pet| pet.stats)
            );
            assert_eq!(
                state.teams[1].pets.each_ref().map(|pet| pet.health),
                [10000; 3]
            );
            assert_eq!(
                state.teams[1].pets[npc_active].cooldowns.get(&2351),
                Some(&5)
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
fn pet_battle_whirling_gears_independent_swap_uses_post_hit_hp_after_each_or_all_misses() {
    let catalog = catalog();
    for (missed, replacement) in [(None, 1), (Some(0), 1), (Some(1), 2), (Some(2), 1)] {
        let mut initial = gears_enemy_battle(&catalog);
        initial.teams[0].pets[1].health = 300;
        initial.teams[0].pets[2].health = 320;
        for slot in 0..3 {
            if missed.is_none() || missed == Some(slot) {
                initial.teams[0].pets[slot]
                    .states
                    .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
            }
        }
        //70448 row4338/139 has Accuracy100 only, no ChainFailure linking4335–4337.
        let (state, events) = enemy_gears(&initial, &catalog);
        for slot in 0..3 {
            let dodged = missed.is_none() || missed == Some(slot);
            assert_eq!(
                state.teams[0].pets[slot].health,
                initial.teams[0].pets[slot].health - if dodged { 0 } else { 72 }
            );
        }
        assert_eq!(damage(&events).len(), if missed.is_none() { 0 } else { 2 });
        assert_eq!(state.teams[0].active, replacement);
        assert_eq!(state.teams[1].active, 0);
    }
}

#[test]
fn pet_battle_whirling_gears_skips_dead_bench_and_preserves_current_when_none_alive() {
    let catalog = catalog();
    for (dead, replacement) in [(&[1][..], 2), (&[2][..], 1), (&[1, 2][..], 0)] {
        let mut initial = gears_enemy_battle(&catalog);
        for &slot in dead {
            initial.teams[0].pets[slot].health = 0;
            initial.teams[0].pets[slot]
                .states
                .insert(catalog.state_id("Is_Dead").unwrap(), 1);
        }
        let (state, events) = enemy_gears(&initial, &catalog);
        assert_eq!(damage(&events), vec![72; 3 - dead.len()]);
        assert_eq!(state.teams[0].active, replacement);
        for &slot in dead {
            assert_eq!(state.teams[0].pets[slot].health, 0);
        }
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::Revived(_)))
        );
    }
}

#[test]
fn pet_battle_whirling_gears_tied_lowest_bench_retains_existing_uniform_selection() {
    let catalog = catalog();
    for (draw, replacement) in [(0, 1), (999_999, 2)] {
        let mut initial = gears_enemy_battle(&catalog);
        initial.teams[0].pets[1].health = 2400;
        initial.teams[0].pets[2].health = 2400;
        let (state, _) = initial
            .step(
                &catalog,
                [Action::Pass, Action::Ability(2351)],
                &mut Fixed(draw),
            )
            .unwrap();
        assert_eq!(state.teams[0].active, replacement);
        assert_eq!(state.teams[0].pets[1].health, state.teams[0].pets[2].health);
        assert_eq!(state.teams[1].active, 0);
    }
}

#[test]
fn pet_battle_whirling_gears_retains_existing_outgoing_and_selected_incoming_locks() {
    let catalog = catalog();
    for (slot, lock) in [(0, "swapOutLock"), (1, "swapInLock")] {
        let mut initial = gears_enemy_battle(&catalog);
        initial.teams[0].pets[slot]
            .states
            .insert(catalog.state_id(lock).unwrap(), 1);
        let (state, events) = enemy_gears(&initial, &catalog);
        assert_eq!(damage(&events), [72; 3]);
        assert_eq!(state.teams[0].active, 0);
        assert_eq!(state.teams[1].active, 0);
        assert!(events.iter().any(|event| matches!(event, Event::Immune { effect: 4338, target } if *target == PetRef { team: 0, slot: 0 })));
    }
}

#[test]
fn pet_battle_whirling_gears_existing_end_of_action_policy_finishes_when_no_player_pet_survives() {
    let catalog = catalog();
    let mut initial = gears_enemy_battle(&catalog);
    //Use Critter pets so this terminal-policy control does not request Mechanical revival.
    for pet in &mut initial.teams[0].pets {
        *pet = Pet::new(
            &catalog,
            39,
            1,
            pet.stats,
            4,
            [110, 119, 123],
            States::new(),
        )
        .unwrap();
        pet.health = 1;
    }
    let (state, events) = enemy_gears(&initial, &catalog);
    assert_eq!(damage(&events), [1; 3]);
    assert_eq!(state.teams[0].pets.each_ref().map(|pet| pet.health), [0; 3]);
    assert_eq!(state.outcome, Some(Outcome::Winner(1)));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Swapped { .. }))
    );
    //Resolving independent4338 before Winner is existing policy, not Retail timing proof.
}
