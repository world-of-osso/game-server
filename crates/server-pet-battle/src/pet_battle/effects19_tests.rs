use super::*;

fn mega_enemy_battle(catalog: &Catalog) -> Battle {
    //Synthetic ordering controls are not an authored NPC team or Boss-passive proof.
    let mut initial = battle(catalog, 9, 9, [2365, 110, 123]);
    initial.teams.swap(0, 1);
    for pet in &mut initial.teams[1].pets {
        pet.stats.speed = 200;
    }
    initial
}

fn enemy_mega(initial: &Battle, catalog: &Catalog) -> (Battle, Vec<Event>) {
    initial
        .step(
            catalog,
            [Action::Pass, Action::Ability(2365)],
            &mut Fixed(500_000),
        )
        .unwrap()
}

fn damage_from(events: &[Event], team: usize) -> Vec<i32> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Damage { caster, amount, .. } if caster.team == team => Some(*amount),
            _ => None,
        })
        .collect()
}

#[test]
fn pet_battle_mega_discharge_all_active_pairs_hit_primary_and_backline_then_debuff_only_caster() {
    let catalog = catalog();
    for player_active in 0..3 {
        for npc_active in 0..3 {
            let mut initial = mega_enemy_battle(&catalog);
            initial.teams[0].active = player_active;
            initial.teams[1].active = npc_active;
            let (state, events) = enemy_mega(&initial, &catalog);
            assert_eq!(damage(&events), [180, 90, 90]);
            for slot in 0..3 {
                assert_eq!(
                    state.teams[0].pets[slot].health,
                    if slot == player_active { 9820 } else { 9910 }
                );
            }
            let applied: Vec<_> = events
                .iter()
                .filter_map(|event| match event {
                    Event::AuraApplied {
                        target,
                        ability: 542,
                    } => Some(*target),
                    _ => None,
                })
                .collect();
            assert_eq!(
                applied,
                [PetRef {
                    team: 1,
                    slot: npc_active
                }]
            );
            assert_eq!(
                state.teams[1].pets.each_ref().map(|pet| pet.health),
                [10000; 3]
            );
            assert_eq!(
                state.teams[1].pets[npc_active].cooldowns.get(&2365),
                Some(&5)
            );
            assert!(state.auras.is_empty());
            assert!(
                state
                    .teams
                    .iter()
                    .flat_map(|team| &team.pets)
                    .all(|pet| pet.state(catalog.state_id("Mod_DamageTakenPercent").unwrap()) == 0)
            );
        }
    }
}

#[test]
fn pet_battle_mega_discharge_duration_one_same_round_policy_only_boosts_attacks_after_cast() {
    let catalog = catalog();
    for (npc_speed, queued_damage) in [(200, 180), (50, 120)] {
        let mut initial = mega_enemy_battle(&catalog);
        initial.teams[1].pets[0].stats.speed = npc_speed;
        let (state, events) = initial
            .step(
                &catalog,
                [Action::Ability(110), Action::Ability(2365)],
                &mut Fixed(500_000),
            )
            .unwrap();
        assert_eq!(damage_from(&events, 0), [queued_damage]);
        assert_eq!(damage_from(&events, 1), [180, 90, 90]);
        assert_eq!(state.teams[1].pets[0].health, 10000 - queued_damage);
        assert!(state.auras.is_empty());
        //PET_BATTLE_DURATION_ONE_SAME_ROUND_POLICY counts the application round.
        //The next round's Bite is120 even when the previous queued Bite was180.
        let (next, events) = state
            .step(
                &catalog,
                [Action::Ability(110), Action::Pass],
                &mut Fixed(500_000),
            )
            .unwrap();
        assert_eq!(damage_from(&events, 0), [120]);
        assert_eq!(next.teams[1].pets[0].health, 10000 - queued_damage - 120);
    }
}

#[test]
fn pet_battle_mega_discharge_each_and_all_misses_still_apply_the_independent_self_debuff() {
    let catalog = catalog();
    for player_active in 0..3 {
        for missed in [None, Some(0), Some(1), Some(2)] {
            let mut initial = mega_enemy_battle(&catalog);
            initial.teams[0].active = player_active;
            for slot in 0..3 {
                if missed.is_none() || missed == Some(slot) {
                    initial.teams[0].pets[slot]
                        .states
                        .insert(catalog.state_id("Stat_Dodge").unwrap(), 100);
                }
            }
            //4367/50[0,100,1,0,0,0] has ChainFailure0 after damage4364–4366.
            let (state, events) = enemy_mega(&initial, &catalog);
            assert_eq!(damage(&events).len(), if missed.is_none() { 0 } else { 2 });
            for slot in 0..3 {
                let dodged = missed.is_none() || missed == Some(slot);
                let amount = if slot == player_active { 180 } else { 90 };
                assert_eq!(
                    state.teams[0].pets[slot].health,
                    10000 - if dodged { 0 } else { amount }
                );
            }
            assert!(events.contains(&Event::AuraApplied {
                target: PetRef { team: 1, slot: 0 },
                ability: 542
            }));
            assert!(state.auras.is_empty());
        }
    }
}

#[test]
fn pet_battle_mega_discharge_dead_player_reserves_stay_dead_without_redirecting_backline_hits() {
    let catalog = catalog();
    for dead in [1, 2] {
        let mut initial = mega_enemy_battle(&catalog);
        initial.teams[0].pets[dead].health = 0;
        initial.teams[0].pets[dead]
            .states
            .insert(catalog.state_id("Is_Dead").unwrap(), 1);
        let (state, events) = enemy_mega(&initial, &catalog);
        assert_eq!(damage(&events), [180, 90]);
        assert_eq!(state.teams[0].pets[dead].health, 0);
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::Revived(_)))
        );
        assert!(events.contains(&Event::AuraApplied {
            target: PetRef { team: 1, slot: 0 },
            ability: 542
        }));
    }
}

#[test]
fn pet_battle_mega_discharge_existing_terminal_policy_applies_self_debuff_after_last_player_faints()
{
    let catalog = catalog();
    let mut initial = mega_enemy_battle(&catalog);
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
    let (state, events) = enemy_mega(&initial, &catalog);
    assert_eq!(damage(&events), [1; 3]);
    assert_eq!(state.outcome, Some(Outcome::Winner(1)));
    let applied = events.iter().position(|event| matches!(event, Event::AuraApplied { target, ability: 542 } if *target == PetRef { team: 1, slot: 0 })).unwrap();
    let finished = events
        .iter()
        .position(|event| matches!(event, Event::Finished(Outcome::Winner(1))))
        .unwrap();
    assert!(applied < finished);
    assert!(state.auras.is_empty());
    //Existing action-completion and duration-one policies, not Retail timing proof.
}
