use super::effects2_tests::use_ability;
use super::*;

fn poison_battle(catalog: &Catalog) -> Battle {
    battle(catalog, 9, 9, [1954, 152, 110])
}

fn damage_amounts(events: &[Event], periodic: bool) -> Vec<i32> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Damage {
                amount,
                periodic: kind,
                ..
            } if *kind == periodic => Some(*amount),
            _ => None,
        })
        .collect()
}

fn poisoned_slots(state: &Battle) -> Vec<usize> {
    let mut slots: Vec<_> = state
        .auras
        .iter()
        .filter(|aura| aura.ability == 379 && aura.target.team == 1)
        .map(|aura| aura.target.slot)
        .collect();
    slots.sort_unstable();
    slots
}

#[test]
fn pet_battle_poison_protocol_all_active_pairs_apply_distinct_poison_and_preserve_allies() {
    let catalog = catalog();
    for ally_active in 0..3 {
        for enemy_active in 0..3 {
            let mut initial = poison_battle(&catalog);
            initial.teams[0].active = ally_active;
            initial.teams[1].active = enemy_active;
            let (state, events) = use_ability(&initial, &catalog, 1954);
            assert_eq!(damage_amounts(&events, false), [30; 3]);
            //379 is Elemental6 versus Mechanical9: ordinary strong1.5 =>45.
            assert_eq!(damage_amounts(&events, true), [45; 3]);
            assert_eq!(
                state.teams[1].pets.each_ref().map(|pet| pet.health),
                [9925; 3]
            );
            assert_eq!(
                state.teams[0].pets.each_ref().map(|pet| pet.health),
                [10000; 3]
            );
            assert_eq!(poisoned_slots(&state), [0, 1, 2]);
            assert!(state.auras.iter().all(|aura| aura.caster
                == PetRef {
                    team: 0,
                    slot: ally_active
                }
                && aura.remaining == 1));
            for pet in &state.teams[1].pets {
                assert_eq!(
                    pet.state(catalog.state_id("Mechanic_IsPoisoned").unwrap()),
                    1
                );
            }
            assert_eq!(
                state.teams[0].pets[ally_active].cooldowns.get(&1954),
                Some(&5)
            );
        }
    }
}

#[test]
fn pet_battle_poison_protocol_each_miss_skips_only_its_paired_poison_and_all_misses_skip_all() {
    let catalog = catalog();
    for enemy_active in 0..3 {
        for missed in [None, Some(0), Some(1), Some(2)] {
            let mut initial = poison_battle(&catalog);
            initial.teams[1].active = enemy_active;
            for slot in 0..3 {
                if missed.is_none() || missed == Some(slot) {
                    initial.teams[1].pets[slot]
                        .states
                        .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
                }
            }
            let (state, events) = use_ability(&initial, &catalog, 1954);
            let successful: Vec<_> = (0..3)
                .filter(|slot| missed.is_some() && missed != Some(*slot))
                .collect();
            //Three damage rows then three ChainFailure1 poisons: per-victim
            //matching is an explicit source-row inference, not Retail capture.
            assert_eq!(poisoned_slots(&state), successful);
            assert_eq!(damage_amounts(&events, false).len(), successful.len());
            assert_eq!(damage_amounts(&events, true).len(), successful.len());
            for slot in 0..3 {
                assert_eq!(
                    state.teams[1].pets[slot].health,
                    if successful.contains(&slot) {
                        9925
                    } else {
                        10000
                    }
                );
            }
        }
    }
}

#[test]
fn pet_battle_poison_protocol_blocked_or_zero_direct_damage_cannot_qualify_a_poison() {
    let catalog = catalog();
    let mut initial = poison_battle(&catalog);
    initial.teams[1].pets[0].abilities[0] = 479;
    let (shielded, _) = initial
        .step(
            &catalog,
            [Action::Pass, Action::Ability(479)],
            &mut Fixed(500_000),
        )
        .unwrap();
    let (state, events) = use_ability(&shielded, &catalog, 1954);
    //Real Ice Barrier480 consumes its two shared direct-attack charges first.
    assert_eq!(damage_amounts(&events, false), [30]);
    assert_eq!(damage_amounts(&events, true), [45]);
    assert_eq!(poisoned_slots(&state), [2]);
    assert_eq!(
        state.teams[1].pets.each_ref().map(|pet| pet.health),
        [10000, 10000, 9925]
    );
    let mut initial = poison_battle(&catalog);
    initial.teams[0].pets[0]
        .states
        .insert(catalog.state_id("Add_FlatDamageDealt").unwrap(), -5);
    let (state, events) = use_ability(&initial, &catalog, 1954);
    assert_eq!(damage_amounts(&events, false), [0; 3]);
    assert!(poisoned_slots(&state).is_empty());
    assert_eq!(
        state.teams[1].pets.each_ref().map(|pet| pet.health),
        [10000; 3]
    );
}

#[test]
fn pet_battle_poison_protocol_dead_reserves_and_primary_faint_do_not_cancel_other_victims() {
    let catalog = catalog();
    for dead in [&[1][..], &[2][..], &[1, 2][..]] {
        let mut initial = poison_battle(&catalog);
        for &slot in dead {
            for team in &mut initial.teams {
                team.pets[slot].health = 0;
                team.pets[slot]
                    .states
                    .insert(catalog.state_id("Is_Dead").unwrap(), 1);
            }
        }
        let (state, events) = use_ability(&initial, &catalog, 1954);
        let expected: Vec<_> = (0..3).filter(|slot| !dead.contains(slot)).collect();
        assert_eq!(poisoned_slots(&state), expected);
        for &slot in dead {
            assert_eq!(state.teams[0].pets[slot].health, 0);
            assert_eq!(state.teams[1].pets[slot].health, 0);
        }
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::Revived(_)))
        );
    }
    let mut initial = poison_battle(&catalog);
    let stats = initial.teams[1].pets[0].stats;
    //Critter primary avoids requesting a Mechanical-revival control here.
    initial.teams[1].pets[0] =
        Pet::new(&catalog, 39, 1, stats, 4, [110, 119, 123], States::new()).unwrap();
    initial.teams[1].pets[0].health = 1;
    let (state, events) = use_ability(&initial, &catalog, 1954);
    assert_eq!(damage_amounts(&events, false), [1, 30, 30]);
    assert_eq!(poisoned_slots(&state), [1, 2]);
    assert_eq!(
        state.teams[1].pets.each_ref().map(|pet| pet.health),
        [0, 9925, 9925]
    );
}

#[test]
fn pet_battle_poison_protocol_periodic_power_stays_with_original_caster_through_swap_and_expiry() {
    let catalog = catalog();
    let mut initial = poison_battle(&catalog);
    initial.teams[0].pets[1].stats.power = 500;
    let (state, _) = use_ability(&initial, &catalog, 1954);
    let (state, events) = state
        .step(
            &catalog,
            [Action::Swap(1), Action::Pass],
            &mut Fixed(500_000),
        )
        .unwrap();
    assert_eq!(damage_amounts(&events, true), [45; 3]);
    assert_eq!(
        state.teams[1].pets.each_ref().map(|pet| pet.health),
        [9880; 3]
    );
    assert!(state.auras.is_empty());
    for pet in &state.teams[1].pets {
        assert_eq!(
            pet.state(catalog.state_id("Mechanic_IsPoisoned").unwrap()),
            0
        );
    }
}

#[test]
fn pet_battle_poison_protocol_composes_with_existing_poison_fang_refresh_and_clock() {
    let catalog = catalog();
    let (state, _) = use_ability(&poison_battle(&catalog), &catalog, 1954);
    let (mut state, events) = use_ability(&state, &catalog, 152);
    assert_eq!(damage_amounts(&events, false), [60]);
    assert_eq!(damage_amounts(&events, true), [45; 3]);
    assert_eq!(
        state.teams[1].pets.each_ref().map(|pet| pet.health),
        [9820, 9880, 9880]
    );
    assert_eq!(poisoned_slots(&state), [0]);
    assert_eq!(state.auras[0].remaining, 4);
    for _ in 0..4 {
        state = state
            .step(&catalog, [Action::Pass; 2], &mut Fixed(500_000))
            .unwrap()
            .0;
    }
    assert!(state.auras.is_empty());
    assert_eq!(
        state.teams[1].pets.each_ref().map(|pet| pet.health),
        [9640, 9880, 9880]
    );
}

#[test]
fn pet_battle_poison_protocol_does_not_reuse_a_prior_action_or_periodic_hit_after_all_misses() {
    let catalog = catalog();
    for prior_periodic in [false, true] {
        let initial = poison_battle(&catalog);
        let (mut state, _) =
            use_ability(&initial, &catalog, if prior_periodic { 1954 } else { 110 });
        if prior_periodic {
            for _ in 0..5 {
                state = state
                    .step(&catalog, [Action::Pass; 2], &mut Fixed(500_000))
                    .unwrap()
                    .0;
            }
        }
        assert!(state.auras.is_empty());
        for pet in &mut state.teams[1].pets {
            pet.states
                .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
        }
        let before = state.teams[1].pets.each_ref().map(|pet| pet.health);
        let (state, events) = use_ability(&state, &catalog, 1954);
        assert!(poisoned_slots(&state).is_empty());
        assert!(damage(&events).is_empty());
        assert_eq!(state.teams[1].pets.each_ref().map(|pet| pet.health), before);
    }
}
