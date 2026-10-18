use super::effects2_tests::use_ability;
use super::*;

fn anvil_battle(catalog: &Catalog) -> Battle {
    let mut initial = battle(catalog, 9, 4, [2483, 110, 123]);
    initial.teams[0].pets[0].stats.speed = 200;
    initial
}

fn leaping_states(catalog: &Catalog, pet: &Pet) -> [i32; 2] {
    ["untargettable", "Mechanic_IsFlying"].map(|name| pet.state(catalog.state_id(name).unwrap()))
}

fn land(initial: &Battle, catalog: &Catalog) -> (Battle, Vec<Event>) {
    initial
        .step(catalog, [Action::Pass; 2], &mut Fixed(500_000))
        .unwrap()
}

#[test]
#[ignore = "PET_BATTLE_OVERRIDE_INDEX_1_SEMANTICS_UNPROVEN"]
fn pet_battle_anvil_drop_nine_active_pairs_leap_then_land_on_distinct_enemies() {
    let catalog = catalog();
    for ally_active in 0..3 {
        for enemy_active in 0..3 {
            let mut initial = anvil_battle(&catalog);
            initial.teams[0].active = ally_active;
            initial.teams[1].active = enemy_active;
            initial.teams[0].pets[ally_active].stats.speed = 200;
            let (leaping, events) = initial
                .step(
                    &catalog,
                    [Action::Ability(2483), Action::Ability(110)],
                    &mut Fixed(500_000),
                )
                .unwrap();
            assert!(damage(&events).is_empty());
            assert_eq!(leaping.auras.len(), 1);
            assert_eq!(leaping.auras[0].ability, 2339);
            assert_eq!(
                leaping.auras[0].target,
                PetRef {
                    team: 0,
                    slot: ally_active
                }
            );
            assert_eq!(
                leaping_states(&catalog, &leaping.teams[0].pets[ally_active]),
                [1, 1]
            );
            assert_eq!(
                leaping.teams[0].pets[ally_active].continuation,
                Some((2483, 2))
            );
            assert_eq!(
                leaping.teams[0].pets[ally_active].cooldowns.get(&2483),
                Some(&2)
            );
            for team in 0..2 {
                assert_eq!(
                    leaping.teams[team].pets.each_ref().map(|pet| pet.health),
                    [10000; 3]
                );
                for slot in 0..3 {
                    if (team, slot) != (0, ally_active) {
                        assert_eq!(
                            leaping_states(&catalog, &leaping.teams[team].pets[slot]),
                            [0, 0]
                        );
                    }
                }
            }
            let (landed, events) = land(&leaping, &catalog);
            assert_eq!(damage(&events), [96, 48, 48]);
            for slot in 0..3 {
                assert_eq!(
                    landed.teams[1].pets[slot].health,
                    if slot == enemy_active { 9904 } else { 9952 }
                );
            }
            assert_eq!(
                landed.teams[0].pets.each_ref().map(|pet| pet.health),
                [10000; 3]
            );
            assert!(landed.auras.is_empty());
            assert_eq!(
                leaping_states(&catalog, &landed.teams[0].pets[ally_active]),
                [0, 0]
            );
            assert_eq!(landed.teams[0].pets[ally_active].continuation, None);
            assert_eq!(
                landed.teams[0].pets[ally_active].cooldowns.get(&2483),
                Some(&1)
            );
        }
    }
}

#[test]
#[ignore = "PET_BATTLE_OVERRIDE_INDEX_1_SEMANTICS_UNPROVEN"]
fn pet_battle_anvil_drop_misses_do_not_cancel_other_victims_or_leaping_cleanup() {
    let catalog = catalog();
    for enemy_active in 0..3 {
        for missed in [None, Some(0), Some(1), Some(2)] {
            let mut initial = anvil_battle(&catalog);
            initial.teams[1].active = enemy_active;
            let (mut leaping, _) = use_ability(&initial, &catalog, 2483);
            for slot in 0..3 {
                if missed.is_none() || missed == Some(slot) {
                    leaping.teams[1].pets[slot]
                        .states
                        .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
                }
            }
            let (landed, _) = land(&leaping, &catalog);
            for slot in 0..3 {
                let expected_damage = if missed.is_none() || missed == Some(slot) {
                    0
                } else if slot == enemy_active {
                    96
                } else {
                    48
                };
                assert_eq!(landed.teams[1].pets[slot].health, 10000 - expected_damage);
            }
            assert!(landed.auras.is_empty());
            assert_eq!(leaping_states(&catalog, &landed.teams[0].pets[0]), [0, 0]);
            assert_eq!(landed.teams[0].pets[0].continuation, None);
        }
    }
}

#[test]
#[ignore = "PET_BATTLE_OVERRIDE_INDEX_1_SEMANTICS_UNPROVEN"]
fn pet_battle_anvil_drop_dead_reserves_and_primary_faint_keep_remaining_targets() {
    let catalog = catalog();
    for dead in [&[1][..], &[2][..], &[1, 2][..]] {
        let mut initial = anvil_battle(&catalog);
        for &slot in dead {
            initial.teams[1].pets[slot].health = 0;
            initial.teams[1].pets[slot]
                .states
                .insert(catalog.state_id("Is_Dead").unwrap(), 1);
        }
        let (leaping, _) = use_ability(&initial, &catalog, 2483);
        let (landed, _) = land(&leaping, &catalog);
        assert_eq!(landed.teams[1].pets[0].health, 9904);
        for slot in 1..3 {
            assert_eq!(
                landed.teams[1].pets[slot].health,
                if dead.contains(&slot) { 0 } else { 9952 }
            );
        }
        assert!(landed.auras.is_empty());
    }
    let mut initial = anvil_battle(&catalog);
    initial.teams[1].pets[0].health = 1;
    let (leaping, _) = use_ability(&initial, &catalog, 2483);
    let (landed, events) = land(&leaping, &catalog);
    assert_eq!(damage(&events), [1, 48, 48]);
    assert_eq!(
        landed.teams[1].pets.each_ref().map(|pet| pet.health),
        [0, 9952, 9952]
    );
    assert!(landed.auras.is_empty());
}

#[test]
#[ignore = "PET_BATTLE_OVERRIDE_INDEX_1_SEMANTICS_UNPROVEN"]
fn pet_battle_anvil_drop_final_faint_finishes_action_and_removes_leaping() {
    let catalog = catalog();
    let mut initial = anvil_battle(&catalog);
    initial.teams[1].pets[0].health = 1;
    for slot in 1..3 {
        initial.teams[1].pets[slot].health = 0;
        initial.teams[1].pets[slot]
            .states
            .insert(catalog.state_id("Is_Dead").unwrap(), 1);
    }
    let (leaping, _) = use_ability(&initial, &catalog, 2483);
    let (landed, events) = land(&leaping, &catalog);
    assert_eq!(damage(&events), [1]);
    assert!(landed.auras.is_empty());
    assert_eq!(leaping_states(&catalog, &landed.teams[0].pets[0]), [0, 0]);
    //Existing end-of-action completion policy, not independently captured Retail timing.
    assert!(events.contains(&Event::AuraRemoved {
        target: PetRef { team: 0, slot: 0 },
        ability: 2339
    }));
    assert_eq!(events.last(), Some(&Event::Finished(Outcome::Winner(0))));
}

#[test]
#[ignore = "PET_BATTLE_OVERRIDE_INDEX_1_SEMANTICS_UNPROVEN"]
fn pet_battle_anvil_drop_speed_order_controls_which_round_incoming_attack_is_blocked() {
    let catalog = catalog();
    for speed in [50, 200] {
        let mut initial = anvil_battle(&catalog);
        initial.teams[0].pets[0].stats.speed = speed;
        let (leaping, events) = initial
            .step(
                &catalog,
                [Action::Ability(2483), Action::Ability(110)],
                &mut Fixed(500_000),
            )
            .unwrap();
        assert_eq!(
            damage(&events),
            if speed == 50 { vec![120] } else { vec![] }
        );
        let (landed, events) = leaping
            .step(
                &catalog,
                [Action::Pass, Action::Ability(110)],
                &mut Fixed(500_000),
            )
            .unwrap();
        assert_eq!(
            damage(&events),
            if speed == 50 {
                vec![96, 48, 48]
            } else {
                vec![96, 48, 48, 120]
            }
        );
        assert_eq!(landed.teams[0].pets[0].health, 9880);
        assert!(landed.auras.is_empty());
        assert_eq!(leaping_states(&catalog, &landed.teams[0].pets[0]), [0, 0]);
    }
}
