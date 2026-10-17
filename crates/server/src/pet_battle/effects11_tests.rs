use super::effects2_tests::use_ability;
use super::*;

fn rapid_fire_battle(catalog: &Catalog, ally_active: usize, enemy_active: usize) -> Battle {
    let mut state = battle(catalog, 9, 4, [774, 110, 123]);
    state.teams[0].active = ally_active;
    state.teams[1].active = enemy_active;
    state.teams[0].pets[ally_active].stats.speed = 200;
    state.teams[0].pets[ally_active]
        .states
        .insert(catalog.state_id("Stat_CritChance").unwrap(), 0);
    state
}

#[test]
fn pet_battle_rapid_fire_two_waves_follow_all_active_slots_without_touching_allies() {
    let catalog = catalog();
    for ally_active in 0..3 {
        for enemy_active in 0..3 {
            let initial = rapid_fire_battle(&catalog, ally_active, enemy_active);
            let (state, events) = use_ability(&initial, &catalog, 774);
            assert_eq!(
                state.teams[1].pets.each_ref().map(|pet| pet.health),
                [9958; 3]
            );
            assert_eq!(
                state.teams[0].pets.each_ref().map(|pet| pet.health),
                [10000; 3]
            );
            assert_eq!(damage(&events), [24, 24, 24, 18, 18, 18]);
            let recipients: Vec<_> = events
                .iter()
                .filter_map(|event| match event {
                    Event::Damage { target, .. } => Some(*target),
                    _ => None,
                })
                .collect();
            let wave: Vec<_> = std::iter::once(enemy_active)
                .chain((0..3).filter(|slot| *slot != enemy_active))
                .map(|slot| PetRef { team: 1, slot })
                .collect();
            assert_eq!(recipients, wave.repeat(2));
        }
    }
}

#[test]
fn pet_battle_rapid_fire_independent_misses_in_either_wave_preserve_other_targets() {
    struct Draws(std::array::IntoIter<u32, 7>);
    impl BattleRng for Draws {
        fn draw(&mut self) -> u32 {
            self.0.next().expect("authored two-wave RNG fixture")
        }
    }
    let catalog = catalog();
    // Primary hits only wave1, then only wave2; other targets hit both waves.
    for (draws, primary_damage, amounts) in [
        (
            [100000, 500000, 500000, 500000, 900000, 500000, 500000],
            24,
            [24, 24, 24, 18, 18],
        ),
        (
            [900000, 500000, 500000, 100000, 500000, 500000, 500000],
            18,
            [24, 24, 18, 18, 18],
        ),
    ] {
        for active in 0..3 {
            let mut initial = rapid_fire_battle(&catalog, 0, active);
            initial.teams[1].pets[active]
                .states
                .insert(catalog.state_id("Stat_Dodge").unwrap(), 50);
            let mut rng = Draws(draws.into_iter());
            let (state, events) = initial
                .step(&catalog, [Action::Ability(774), Action::Pass], &mut rng)
                .unwrap();
            for slot in 0..3 {
                let expected = if slot == active { primary_damage } else { 42 };
                assert_eq!(state.teams[1].pets[slot].health, 10000 - expected);
            }
            assert_eq!(damage(&events), amounts);
            let misses: Vec<_> = events
                .iter()
                .filter_map(|event| match event {
                    Event::Miss { target, .. } => Some(*target),
                    _ => None,
                })
                .collect();
            assert_eq!(
                misses,
                [PetRef {
                    team: 1,
                    slot: active
                }]
            );
            assert_eq!(rng.0.len(), 0);
        }
    }
}

#[test]
fn pet_battle_rapid_fire_first_wave_faint_does_not_retarget_second_wave() {
    let catalog = catalog();
    for active in 0..3 {
        let mut initial = rapid_fire_battle(&catalog, 0, active);
        initial.teams[1].pets[active].health = 20;
        let (state, events) = use_ability(&initial, &catalog, 774);
        for slot in 0..3 {
            assert_eq!(
                state.teams[1].pets[slot].health,
                if slot == active { 0 } else { 9958 }
            );
        }
        assert_eq!(damage(&events), [20, 24, 24, 18, 18]);
        assert!(events.contains(&Event::Fainted(PetRef {
            team: 1,
            slot: active
        })));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::Swapped { .. }))
        );
        assert_eq!(state.teams[1].active, active);
        assert!(state.outcome.is_none());
    }
}

#[test]
fn pet_battle_rapid_fire_dead_reserve_stays_dead_through_both_waves() {
    let catalog = catalog();
    let mut initial = rapid_fire_battle(&catalog, 0, 1);
    initial.teams[1].pets[0].health = 0;
    initial.teams[1].pets[0]
        .states
        .insert(catalog.state_id("Is_Dead").unwrap(), 1);
    let (state, events) = use_ability(&initial, &catalog, 774);
    assert_eq!(
        state.teams[1].pets.each_ref().map(|pet| pet.health),
        [0, 9958, 9958]
    );
    assert_eq!(damage(&events), [24, 24, 18, 18]);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Revived(_)))
    );
}

#[test]
fn pet_battle_rapid_fire_other_wave_and_interleaved_selectors_remain_rejected() {
    let catalog = catalog();
    for ability in [2064, 920] {
        let initial = battle(&catalog, 9, 4, [ability, 110, 123]);
        assert!(
            initial
                .step(
                    &catalog,
                    [Action::Ability(ability), Action::Pass],
                    &mut NoDraw
                )
                .is_err()
        );
    }
}
