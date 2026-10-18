use super::effects2_tests::use_ability;
use super::*;

const ACTIVE_PAIRS: &[(usize, usize)] = &[
    (0, 0),
    (0, 1),
    (0, 2),
    (1, 0),
    (1, 1),
    (1, 2),
    (2, 0),
    (2, 1),
    (2, 2),
];

fn cloud_battle(catalog: &Catalog, ability: u32) -> Battle {
    let mut initial = battle(catalog, 5, 5, [ability, 1954, 110]);
    for pet in &mut initial.teams[0].pets {
        pet.stats.speed = 200;
    }
    initial
}

fn cloud_values(ability: u32) -> (u32, i32, i32) {
    match ability {
        640 => (639, 96, 30),
        2525 => (2524, 78, 42),
        _ => panic!("Unexpected team-cloud fixture ability"),
    }
}

fn cloud_hits(events: &[Event], periodic: bool) -> Vec<(PetRef, PetRef, i32)> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Damage {
                caster,
                target,
                amount,
                periodic: kind,
                ..
            } if *kind == periodic => Some((*caster, *target, *amount)),
            _ => None,
        })
        .collect()
}

fn pass_round(initial: &Battle, catalog: &Catalog) -> (Battle, Vec<Event>) {
    initial
        .step(catalog, [Action::Pass; 2], &mut Fixed(500_000))
        .unwrap()
}

#[test]
#[ignore = "Approved640/2525 team-cloud RED; remove only with scoped GREEN"]
fn pet_battle_team_cloud_nine_active_pairs_hit_each_enemy_once_and_preserve_allies() {
    let catalog = catalog();
    for ability in [640, 2525] {
        let (aura, direct, periodic) = cloud_values(ability);
        for &(ally_active, enemy_active) in ACTIVE_PAIRS {
            let mut initial = cloud_battle(&catalog, ability);
            initial.teams[0].active = ally_active;
            initial.teams[1].active = enemy_active;
            for (slot, health) in [1000, 2000, 3000].into_iter().enumerate() {
                initial.teams[1].pets[slot].health = health;
            }
            let (state, events) = use_ability(&initial, &catalog, ability);
            assert_eq!(
                cloud_hits(&events, false)
                    .iter()
                    .map(|hit| hit.2)
                    .collect::<Vec<_>>(),
                [direct; 3]
            );
            assert_eq!(
                cloud_hits(&events, true)
                    .iter()
                    .map(|hit| hit.2)
                    .collect::<Vec<_>>(),
                [periodic; 3]
            );
            let mut victims: Vec<_> = cloud_hits(&events, false)
                .iter()
                .map(|hit| hit.1.slot)
                .collect();
            victims.sort_unstable();
            assert_eq!(victims, [0, 1, 2]);
            assert!(cloud_hits(&events, true).iter().all(|hit| hit.0
                == PetRef {
                    team: 0,
                    slot: ally_active
                }
                && hit.1.team == 1));
            assert_eq!(
                state.teams[1].pets.each_ref().map(|pet| pet.health),
                [
                    1000 - direct - periodic,
                    2000 - direct - periodic,
                    3000 - direct - periodic
                ]
            );
            assert_eq!(
                state.teams[0].pets.each_ref().map(|pet| pet.health),
                [10000; 3]
            );
            assert_eq!(state.auras.len(), 1);
            assert_eq!(state.auras[0].ability, aura);
            assert!(state.auras[0].team_bound);
            assert_eq!(state.auras[0].remaining, 1);
            assert_eq!(state.auras[0].target.team, 1);
            let (expired, events) = pass_round(&state, &catalog);
            assert_eq!(
                cloud_hits(&events, true)
                    .iter()
                    .map(|hit| hit.2)
                    .collect::<Vec<_>>(),
                [periodic; 3]
            );
            assert_eq!(
                expired.teams[1].pets.each_ref().map(|pet| pet.health),
                [
                    1000 - direct - 2 * periodic,
                    2000 - direct - 2 * periodic,
                    3000 - direct - 2 * periodic
                ]
            );
            assert!(expired.auras.is_empty());
        }
    }
}

#[test]
#[ignore = "Approved640/2525 team-cloud RED; remove only with scoped GREEN"]
fn pet_battle_team_cloud_each_miss_and_application_recipient_accuracy_stay_independent() {
    let catalog = catalog();
    for ability in [640, 2525] {
        let (_, direct, periodic) = cloud_values(ability);
        for &(missed, enemy_active) in ACTIVE_PAIRS {
            let mut initial = cloud_battle(&catalog, ability);
            initial.teams[1].active = enemy_active;
            initial.teams[1].pets[missed]
                .states
                .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
            let (state, _) = use_ability(&initial, &catalog, ability);
            //Application has its own existing accuracy check against the selected living recipient.
            let cloud_applied = missed != enemy_active;
            assert_eq!(state.auras.len(), usize::from(cloud_applied));
            for slot in 0..3 {
                let loss = if slot == missed {
                    0
                } else {
                    direct + if cloud_applied { periodic } else { 0 }
                };
                assert_eq!(state.teams[1].pets[slot].health, 10000 - loss);
            }
        }
    }
}

#[test]
#[ignore = "Approved640/2525 team-cloud RED; remove only with scoped GREEN"]
fn pet_battle_team_cloud_all_direct_misses_do_not_cancel_favorable_application_and_ticks() {
    struct FavorableAfterDirectMisses(u32);
    impl BattleRng for FavorableAfterDirectMisses {
        fn draw(&mut self) -> u32 {
            let miss_direct = self.0 < 3;
            self.0 += 1;
            if miss_direct { 950_000 } else { 0 }
        }
    }
    let catalog = catalog();
    for ability in [640, 2525] {
        let mut initial = cloud_battle(&catalog, ability);
        for pet in &mut initial.teams[1].pets {
            pet.states
                .insert(catalog.state_id("Stat_Dodge").unwrap(), 50);
        }
        let (state, events) = initial
            .step(
                &catalog,
                [Action::Ability(ability), Action::Pass],
                &mut FavorableAfterDirectMisses(0),
            )
            .unwrap();
        assert!(cloud_hits(&events, false).is_empty());
        assert_eq!(state.auras.len(), 1);
        assert!(state.auras[0].team_bound);
        assert_eq!(cloud_hits(&events, true).len(), 3);
        assert!(cloud_hits(&events, true).iter().all(|hit| hit.2 > 0));
        assert!(state.teams[1].pets.iter().all(|pet| pet.health < 10000));
    }
}

#[test]
#[ignore = "Approved640/2525 team-cloud RED; remove only with scoped GREEN"]
fn pet_battle_team_cloud_application_can_miss_after_all_three_direct_hits() {
    struct MissApplication(u32);
    impl BattleRng for MissApplication {
        fn draw(&mut self) -> u32 {
            let application = self.0 == 9;
            self.0 += 1;
            if application { 950_000 } else { 0 }
        }
    }
    let catalog = catalog();
    for ability in [640, 2525] {
        let mut initial = cloud_battle(&catalog, ability);
        for pet in &mut initial.teams[1].pets {
            pet.states
                .insert(catalog.state_id("Stat_Dodge").unwrap(), 50);
        }
        let (state, events) = initial
            .step(
                &catalog,
                [Action::Ability(ability), Action::Pass],
                &mut MissApplication(0),
            )
            .unwrap();
        assert_eq!(cloud_hits(&events, false).len(), 3);
        assert!(cloud_hits(&events, true).is_empty());
        assert!(state.auras.is_empty());
    }
}

#[test]
#[ignore = "Approved640/2525 team-cloud RED; remove only with scoped GREEN"]
fn pet_battle_team_cloud_dead_reserves_primary_faint_and_terminal_no_living_target() {
    let catalog = catalog();
    for ability in [640, 2525] {
        let (_, direct, periodic) = cloud_values(ability);
        for dead in [&[1][..], &[2][..], &[1, 2][..]] {
            let mut initial = cloud_battle(&catalog, ability);
            for &slot in dead {
                initial.teams[1].pets[slot].health = 0;
                initial.teams[1].pets[slot]
                    .states
                    .insert(catalog.state_id("Is_Dead").unwrap(), 1);
            }
            let (state, events) = use_ability(&initial, &catalog, ability);
            assert_eq!(cloud_hits(&events, false).len(), 3 - dead.len());
            assert_eq!(cloud_hits(&events, true).len(), 3 - dead.len());
            assert_eq!(state.teams[1].pets[0].health, 10000 - direct - periodic);
            for &slot in dead {
                assert_eq!(state.teams[1].pets[slot].health, 0);
            }
        }
        let mut initial = cloud_battle(&catalog, ability);
        initial.teams[1].pets[0].health = 1;
        let (state, events) = use_ability(&initial, &catalog, ability);
        assert_eq!(
            cloud_hits(&events, false)
                .iter()
                .map(|hit| hit.2)
                .collect::<Vec<_>>(),
            [1, direct, direct]
        );
        assert_eq!(state.auras.len(), 1);
        assert_eq!(state.auras[0].target, PetRef { team: 1, slot: 1 });
        assert_eq!(
            state.teams[1].pets.each_ref().map(|pet| pet.health),
            [0, 10000 - direct - periodic, 10000 - direct - periodic]
        );
        let (replaced, events) = state
            .step(
                &catalog,
                [Action::Pass, Action::Swap(1)],
                &mut Fixed(500_000),
            )
            .unwrap();
        assert!(cloud_hits(&events, true).is_empty());
        assert_eq!(replaced.auras[0].remaining, 1);
        let (expired, events) = pass_round(&replaced, &catalog);
        assert_eq!(cloud_hits(&events, true).len(), 2);
        assert!(expired.auras.is_empty());
        let mut one_bench_alive = cloud_battle(&catalog, ability);
        one_bench_alive.teams[1].pets[0].health = 1;
        one_bench_alive.teams[1].pets[1].health = 0;
        one_bench_alive.teams[1].pets[1]
            .states
            .insert(catalog.state_id("Is_Dead").unwrap(), 1);
        let (state, _) = use_ability(&one_bench_alive, &catalog, ability);
        assert_eq!(state.auras.len(), 1);
        assert_eq!(state.auras[0].target, PetRef { team: 1, slot: 2 });
        assert_eq!(
            state.teams[1].pets.each_ref().map(|pet| pet.health),
            [0, 0, 10000 - direct - periodic]
        );
        let mut final_enemy = cloud_battle(&catalog, ability);
        for pet in &mut final_enemy.teams[1].pets {
            pet.health = 1;
        }
        let (finished, events) = use_ability(&final_enemy, &catalog, ability);
        assert_eq!(
            cloud_hits(&events, false)
                .iter()
                .map(|hit| hit.2)
                .collect::<Vec<_>>(),
            [1; 3]
        );
        assert!(finished.auras.is_empty());
        assert_eq!(events.last(), Some(&Event::Finished(Outcome::Winner(0))));
    }
}

#[test]
#[ignore = "Approved640/2525 team-cloud RED; remove only with scoped GREEN"]
fn pet_battle_team_cloud_both_sides_swapping_preserves_original_caster_and_all_victims() {
    let catalog = catalog();
    for ability in [640, 2525] {
        let (_, direct, periodic) = cloud_values(ability);
        let mut initial = cloud_battle(&catalog, ability);
        initial.teams[0].pets[1].stats.power = 500;
        let (state, _) = use_ability(&initial, &catalog, ability);
        let (expired, events) = state
            .step(
                &catalog,
                [Action::Swap(1), Action::Swap(2)],
                &mut Fixed(500_000),
            )
            .unwrap();
        assert_eq!(expired.teams[0].active, 1);
        assert_eq!(expired.teams[1].active, 2);
        let hits = cloud_hits(&events, true);
        assert_eq!(hits.len(), 3);
        assert!(
            hits.iter()
                .all(|hit| hit.0 == PetRef { team: 0, slot: 0 } && hit.2 == periodic)
        );
        assert_eq!(
            expired.teams[1].pets.each_ref().map(|pet| pet.health),
            [10000 - direct - 2 * periodic; 3]
        );
        assert!(expired.auras.is_empty());
    }
}

#[test]
#[ignore = "Approved640/2525 team-cloud RED; remove only with scoped GREEN"]
fn pet_battle_team_cloud_recast_during_enemy_swap_refreshes_one_shared_aura() {
    let catalog = catalog();
    for ability in [640, 2525] {
        let (_, direct, periodic) = cloud_values(ability);
        let (state, _) = use_ability(&cloud_battle(&catalog, ability), &catalog, ability);
        let (refreshed, events) = state
            .step(
                &catalog,
                [Action::Ability(ability), Action::Swap(1)],
                &mut Fixed(500_000),
            )
            .unwrap();
        assert_eq!(refreshed.teams[1].active, 1);
        assert_eq!(refreshed.auras.len(), 1);
        assert!(refreshed.auras[0].team_bound);
        assert_eq!(refreshed.auras[0].remaining, 1);
        assert_eq!(cloud_hits(&events, true).len(), 3);
        assert_eq!(
            refreshed.teams[1].pets.each_ref().map(|pet| pet.health),
            [10000 - 2 * direct - 2 * periodic; 3]
        );
        let (expired, _) = pass_round(&refreshed, &catalog);
        assert!(expired.auras.is_empty());
        assert_eq!(
            expired.teams[1].pets.each_ref().map(|pet| pet.health),
            [10000 - 2 * direct - 3 * periodic; 3]
        );
    }
}

#[test]
#[ignore = "Approved640/2525 team-cloud RED; remove only with scoped GREEN"]
fn pet_battle_team_cloud_does_not_make_ordinary_poisoned_wearer_auras_team_bound() {
    let catalog = catalog();
    let (state, _) = use_ability(&cloud_battle(&catalog, 640), &catalog, 640);
    let (state, _) = use_ability(&state, &catalog, 1954);
    assert_eq!(state.auras.len(), 3);
    assert!(
        state
            .auras
            .iter()
            .all(|aura| aura.ability == 379 && !aura.team_bound)
    );
    let mut slots: Vec<_> = state.auras.iter().map(|aura| aura.target.slot).collect();
    slots.sort_unstable();
    assert_eq!(slots, [0, 1, 2]);
}
