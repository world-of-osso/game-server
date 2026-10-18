use super::effects2_tests::use_ability;
use super::*;

const TEAM_WAVES: &[(u32, i32)] = &[(319, 42), (2309, 84)];
const TEAM_DAMAGE: &[(u32, i32)] = &[(319, 42), (2309, 84), (2523, 90)];

#[test]
fn pet_battle_team_damage_hits_each_enemy_for_all_active_slots() {
    let catalog = catalog();
    for &(ability, amount) in TEAM_DAMAGE {
        for ally_active in 0..3 {
            for enemy_active in 0..3 {
                let mut initial = battle(&catalog, 7, 7, [ability, 110, 123]);
                initial.teams[0].active = ally_active;
                initial.teams[1].active = enemy_active;
                let (state, events) = use_ability(&initial, &catalog, ability);
                assert_eq!(damage(&events), [amount; 3]);
                assert_eq!(
                    state.teams[1].pets.each_ref().map(|pet| pet.health),
                    [10000 - amount; 3]
                );
                assert_eq!(
                    state.teams[0].pets.each_ref().map(|pet| pet.health),
                    [10000; 3]
                );
            }
        }
    }
}

#[test]
fn pet_battle_team_damage_individual_misses_preserve_other_enemy_hits() {
    let catalog = catalog();
    for &(ability, amount) in TEAM_DAMAGE {
        for active in 0..3 {
            for missed in 0..3 {
                let mut initial = battle(&catalog, 7, 7, [ability, 110, 123]);
                initial.teams[1].active = active;
                initial.teams[1].pets[missed]
                    .states
                    .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
                let (state, events) = use_ability(&initial, &catalog, ability);
                assert_eq!(damage(&events), [amount; 2]);
                for slot in 0..3 {
                    assert_eq!(
                        state.teams[1].pets[slot].health,
                        if slot == missed {
                            10000
                        } else {
                            10000 - amount
                        }
                    );
                }
                assert!(events.iter().any(|event| matches!(event, Event::Miss { target, .. } if *target == PetRef { team: 1, slot: missed })));
            }
        }
    }
}

#[test]
fn pet_battle_team_damage_dead_reserve_stays_dead_without_retargeting() {
    let catalog = catalog();
    for &(ability, amount) in TEAM_DAMAGE {
        let mut initial = battle(&catalog, 7, 7, [ability, 110, 123]);
        initial.teams[1].active = 1;
        initial.teams[1].pets[0].health = 0;
        initial.teams[1].pets[0]
            .states
            .insert(catalog.state_id("Is_Dead").unwrap(), 1);
        let (state, events) = use_ability(&initial, &catalog, ability);
        assert_eq!(damage(&events), [amount; 2]);
        assert_eq!(
            state.teams[1].pets.each_ref().map(|pet| pet.health),
            [0, 10000 - amount, 10000 - amount]
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::Revived(_)))
        );
    }
}

#[test]
fn pet_battle_team_damage_shadow_bolt_volley_uses_authored_thirty_percent_variance() {
    let catalog = catalog();
    for (draw, amount) in [(0, 76), (999_999, 103)] {
        let mut initial = battle(&catalog, 7, 7, [2523, 110, 123]);
        initial.teams[0].pets[0]
            .states
            .insert(catalog.state_id("Stat_CritChance").unwrap(), 0);
        let (state, events) = initial
            .step(
                &catalog,
                [Action::Ability(2523), Action::Pass],
                &mut Fixed(draw),
            )
            .unwrap();
        // Base90 with width30 spans76..103; default width20 would give81..98.
        assert_eq!(damage(&events), [amount; 3]);
        assert_eq!(
            state.teams[1].pets.each_ref().map(|pet| pet.health),
            [10000 - amount; 3]
        );
        assert_eq!(
            state.teams[0].pets.each_ref().map(|pet| pet.health),
            [10000; 3]
        );
    }
}

#[test]
fn pet_battle_team_wave_clears_both_teams_objects_even_when_all_damage_misses() {
    let catalog = catalog();
    for &(ability, amount) in TEAM_WAVES {
        for misses in [false, true] {
            let mut initial = battle(&catalog, 7, 7, [576, 293, ability]);
            initial.teams[0].pets[0].stats.speed = 200;
            initial.teams[1].pets[0].abilities = [293, 110, 123];
            let (healthy, _) = use_ability(&initial, &catalog, 576);
            let (mut ready, _) = healthy
                .step(&catalog, [Action::Ability(293); 2], &mut Fixed(500_000))
                .unwrap();
            for team in 0..2 {
                assert!(
                    ready
                        .auras
                        .iter()
                        .any(|aura| aura.ability == 294 && aura.target == PetRef { team, slot: 0 })
                );
            }
            assert_eq!(ready.auras.len(), 3);
            if misses {
                ready.teams[0].pets[0]
                    .states
                    .insert(catalog.state_id("Stat_Accuracy").unwrap(), -100);
            }
            let (state, events) = use_ability(&ready, &catalog, ability);
            assert_eq!(
                damage(&events),
                if misses { vec![] } else { vec![amount; 3] }
            );
            for team in 0..2 {
                assert!(events.contains(&Event::AuraRemoved {
                    target: PetRef { team, slot: 0 },
                    ability: 294
                }));
                for pet in &state.teams[team].pets {
                    assert_eq!(pet.state(catalog.state_id("Mechanic_IsObject").unwrap()), 0);
                }
            }
            assert_eq!(state.auras.len(), 1);
            assert_eq!(state.auras[0].ability, 577);
            assert_eq!(state.auras[0].remaining, 7);
            assert_eq!(state.teams[0].pets[0].stats.health, 10005);
            assert_eq!(state.teams[0].pets[0].health, 10005);
        }
    }
}
