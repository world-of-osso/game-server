use super::effects2_tests::use_ability;
use super::*;

fn starfall_battle(
    catalog: &Catalog,
    ability: u32,
    ally_active: usize,
    enemy_active: usize,
) -> Battle {
    let mut state = battle(catalog, 9, 4, [ability, 110, 123]);
    state.teams[0].active = ally_active;
    state.teams[1].active = enemy_active;
    for (pet, health) in state.teams[0].pets.iter_mut().zip([100, 200, 9990]) {
        pet.health = health;
        pet.stats.power = 0;
    }
    state.teams[0].pets[ally_active].stats.power = 100;
    state.teams[0].pets[ally_active].stats.speed = 200;
    state
}

#[test]
fn pet_battle_starfall_all_active_slots_damage_enemies_and_heal_injured_allies_to_cap() {
    let catalog = catalog();
    for ability in [258, 979] {
        for ally_active in 0..3 {
            for enemy_active in 0..3 {
                let initial = starfall_battle(&catalog, ability, ally_active, enemy_active);
                let (state, events) = use_ability(&initial, &catalog, ability);
                assert_eq!(
                    state.teams[0].pets.each_ref().map(|p| p.health),
                    [136, 236, 10000]
                );
                assert_eq!(state.teams[1].pets.each_ref().map(|p| p.health), [9964; 3]);
                let recipients: Vec<_> = events
                    .iter()
                    .filter_map(|event| match event {
                        Event::Damage { target, .. } => Some(*target),
                        _ => None,
                    })
                    .collect();
                let mut expected = vec![PetRef {
                    team: 1,
                    slot: enemy_active,
                }];
                expected.extend(
                    (0..3)
                        .filter(|slot| *slot != enemy_active)
                        .map(|slot| PetRef { team: 1, slot }),
                );
                assert_eq!(recipients, expected);
            }
        }
    }
}

#[test]
fn pet_battle_starfall_does_not_resurrect_either_teams_dead_reserves() {
    let catalog = catalog();
    for ability in [258, 979] {
        let mut initial = starfall_battle(&catalog, ability, 0, 0);
        for team in 0..2 {
            initial.teams[team].pets[1].health = 0;
            initial.teams[team].pets[1]
                .states
                .insert(catalog.state_id("Is_Dead").unwrap(), 1);
        }
        let (state, events) = use_ability(&initial, &catalog, ability);
        assert_eq!(
            state.teams[0].pets.each_ref().map(|p| p.health),
            [136, 0, 10000]
        );
        assert_eq!(
            state.teams[1].pets.each_ref().map(|p| p.health),
            [9964, 0, 9964]
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::Revived(_)))
        );
    }
}

#[test]
fn pet_battle_starfall_individual_enemy_miss_does_not_cancel_other_hits_or_allied_healing() {
    let catalog = catalog();
    for ability in [258, 979] {
        for missed in 0..3 {
            let mut initial = starfall_battle(&catalog, ability, 2, 1);
            initial.teams[1].pets[missed]
                .states
                .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
            let (state, events) = use_ability(&initial, &catalog, ability);
            assert_eq!(
                state.teams[0].pets.each_ref().map(|p| p.health),
                [136, 236, 10000]
            );
            for slot in 0..3 {
                assert_eq!(
                    state.teams[1].pets[slot].health,
                    if slot == missed { 10000 } else { 9964 }
                );
            }
            assert!(events.iter().any(|event| matches!(event, Event::Miss { target, .. } if *target == PetRef { team: 1, slot: missed })));
        }
    }
}

#[test]
fn pet_battle_starfall_applies_moonlight_after_healing_and_existing_weather_modifies_next_cast() {
    let catalog = catalog();
    for ability in [258, 979] {
        let initial = starfall_battle(&catalog, ability, 1, 2);
        let (mut state, events) = use_ability(&initial, &catalog, ability);
        let last_heal = events
            .iter()
            .rposition(|event| matches!(event, Event::Heal { .. }))
            .unwrap();
        let weather = events
            .iter()
            .position(|event| matches!(event, Event::AuraApplied { ability: 596, .. }))
            .unwrap();
        assert!(last_heal < weather);
        assert_eq!(state.auras.len(), 1);
        assert_eq!((state.auras[0].ability, state.auras[0].remaining), (596, 9));
        for team in &state.teams {
            for pet in &team.pets {
                assert_eq!(
                    pet.state(catalog.state_id("Mod_HealingTakenPercent").unwrap()),
                    25
                );
            }
        }
        for pet in &mut state.teams[0].pets {
            pet.health = 9000;
        }
        state.teams[0].pets[1].cooldowns.clear();
        let (state, events) = use_ability(&state, &catalog, ability);
        assert_eq!(state.teams[0].pets.each_ref().map(|p| p.health), [9045; 3]);
        assert_eq!(damage(&events), [39; 3]);
        assert_eq!(state.auras.len(), 1);
    }
}

#[test]
fn pet_battle_starfall_other_ambiguous_team_selectors_remain_rejected_before_rng() {
    let catalog = catalog();
    for ability in [70, 774, 2378] {
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
