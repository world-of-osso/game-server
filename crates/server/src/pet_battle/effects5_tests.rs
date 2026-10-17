use super::effects2_tests::use_ability;
use super::*;

fn create_species_battle(c: &Catalog, caster: PetRef, matching: usize) -> Battle {
    let mut b = battle(c, 6, 7, [2283, 110, 123]);
    b.teams[caster.team].active = caster.slot;
    for pet in &mut b.teams[caster.team].pets {
        pet.species = 3256;
        pet.abilities[0] = 2283;
    }
    for pet in &mut b.teams[1 - caster.team].pets {
        pet.species = 2621;
    }
    b.pet_mut(caster).species = 2621;
    for slot in (0..3).filter(|slot| *slot != caster.slot).take(matching) {
        b.teams[caster.team].pets[slot].species = 2621;
    }
    b
}

#[test]
fn pet_battle_dragons_call_routes_same_species_bonus_for_both_teams_and_all_active_slots() {
    let c = catalog();
    for team in 0..2 {
        for active in 0..3 {
            for (matching, expected) in [(0, 150), (1, 192), (2, 234)] {
                let b = create_species_battle(&c, PetRef { team, slot: active }, matching);
                let target = b.active(1 - team);
                let mut actions = [Action::Pass; 2];
                actions[team] = Action::Ability(2283);
                let (b, events) = b.step(&c, actions, &mut Fixed(500_000)).unwrap();
                assert_eq!(
                    damage(&events),
                    vec![expected],
                    "team{team}/active{active}/matching{matching}"
                );
                assert!(events.iter().any(|event| matches!(event,
                    Event::Damage { target: recipient, periodic: false, .. } if *recipient == target
                )));
                assert!(b.teams[team].pets.iter().all(|pet| pet.health == 10000));
                assert_eq!(b.teams[team].pets[active].cooldowns[&2283], 3);
            }
        }
    }
}

#[test]
fn pet_battle_dragons_call_counts_fainted_roster_member_without_reviving_it() {
    let c = catalog();
    let mut b = battle(&c, 6, 7, [2283, 110, 123]);
    b.teams[0].pets[0].species = 2621;
    b.teams[0].pets[1].species = 2621;
    b.teams[0].pets[1].health = 0;
    let dead = c.state_id("Is_Dead").unwrap();
    b.teams[0].pets[1].states.insert(dead, 1);
    b.teams[0].pets[2].species = 3256;
    let (b, events) = use_ability(&b, &c, 2283);
    assert_eq!(damage(&events), vec![192]);
    assert_eq!(b.teams[0].pets[1].health, 0);
    assert_eq!(b.teams[0].pets[1].state(dead), 1);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Revived(_)))
    );
}

#[test]
fn pet_battle_dragons_call_preserves_direct_attack_miss_rules() {
    let c = catalog();
    let mut b = battle(&c, 6, 7, [2283, 110, 123]);
    b.teams[0].pets[0]
        .states
        .insert(c.state_id("Stat_Accuracy").unwrap(), -100);
    let (b, events) = use_ability(&b, &c, 2283);
    assert!(damage(&events).is_empty());
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Miss { .. }))
    );
    assert_eq!(b.teams[1].pets[0].health, 10000);
}

#[test]
fn pet_battle_dragons_call_test_only_templates_and_unestablished_parameters_reject_before_rng() {
    struct NoDraw;
    impl BattleRng for NoDraw {
        fn draw(&mut self) -> u32 {
            panic!("rejected370 must not draw RNG");
        }
    }
    let mut c = catalog();
    for ability in [2259, 2280] {
        let b = battle(&c, 6, 7, [ability, 110, 123]);
        assert!(
            b.step(&c, [Action::Ability(ability), Action::Pass], &mut NoDraw)
                .is_err()
        );
    }
    c.effects.get_mut(&4161).unwrap().params[3] = 1;
    let b = battle(&c, 6, 7, [2283, 110, 123]);
    assert!(
        b.step(&c, [Action::Ability(2283), Action::Pass], &mut NoDraw)
            .is_err()
    );
}

#[test]
fn pet_battle_dragons_call_unknown_species_identity_is_not_synthesized() {
    let c = catalog();
    let stats = calculate_stats(&c, 2621, 3, 3, 2).unwrap();
    assert!(Pet::new(&c, 0, stats, 1, [393, 2283, 0], States::new()).is_err());
    assert!(Pet::new(&c, u32::MAX, stats, 1, [393, 2283, 0], States::new()).is_err());
}

#[test]
fn pet_battle_bench_resurrection_revives_only_dead_reserves_for_every_active_slot() {
    let c = catalog();
    let dead = c.state_id("Is_Dead").unwrap();
    for ability in [2347, 2348] {
        for active in 0..3 {
            let mut b = battle(&c, 7, 7, [ability, 110, 123]);
            b.teams[0].active = active;
            for slot in 0..3 {
                let pet = &mut b.teams[0].pets[slot];
                if slot != active {
                    pet.stats.health = 10003;
                    pet.health = 0;
                    pet.states.insert(dead, 1);
                }
            }
            let (b, events) = use_ability(&b, &c, ability);
            let revived: Vec<_> = events
                .iter()
                .filter_map(|event| match event {
                    Event::Revived(pet) => Some(*pet),
                    _ => None,
                })
                .collect();
            let reserves: Vec<_> = (0..3)
                .filter(|slot| *slot != active)
                .map(|slot| PetRef { team: 0, slot })
                .collect();
            assert_eq!(revived, reserves, "ability {ability}, active {active}");
            assert_eq!(b.teams[0].pets[active].health, 10000);
            for reserve in reserves {
                assert_eq!(b.pet(reserve).health, 500);
                assert_eq!(b.pet(reserve).state(dead), 0);
            }
            assert!(b.teams[1].pets.iter().all(|pet| pet.health == 10000));
        }
    }
}

#[test]
fn pet_battle_bench_resurrection_does_not_heal_living_reserve() {
    let c = catalog();
    let dead = c.state_id("Is_Dead").unwrap();
    for ability in [2347, 2348] {
        let mut b = battle(&c, 7, 7, [ability, 110, 123]);
        b.teams[0].pets[1].health = 37;
        b.teams[0].pets[2].health = 0;
        b.teams[0].pets[2].states.insert(dead, 1);
        let (b, events) = use_ability(&b, &c, ability);
        assert_eq!(b.teams[0].pets[1].health, 37);
        assert_eq!(b.teams[0].pets[2].health, 500);
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::Revived(_)))
                .count(),
            1
        );
    }
}

#[test]
fn pet_battle_bench_resurrection_survives_old_undead_clock_without_resetting_passive() {
    let c = catalog();
    let dead = c.state_id("Is_Dead").unwrap();
    let mut b = battle(&c, 6, 7, [2347, 110, 123]);
    let reserve = PetRef { team: 0, slot: 1 };
    let pet = b.pet_mut(reserve);
    pet.health = 0;
    pet.states.insert(dead, 1);
    pet.resurrection_used = true;
    pet.undead_expires = Some(2);
    let mut b = use_ability(&b, &c, 2347).0;
    b = b
        .step(&c, [Action::Pass; 2], &mut Fixed(500_000))
        .unwrap()
        .0;
    assert_eq!(
        b.pet(reserve).health,
        500,
        "old expiry cannot kill an explicit revival"
    );
    assert!(
        b.pet(reserve).resurrection_used,
        "family passive is once per battle"
    );
    let (b, _) = b
        .step(&c, [Action::Swap(1), Action::Pass], &mut Fixed(500_000))
        .unwrap();
    let (_, events) = use_ability(&b, &c, 110);
    assert_eq!(damage(&events), vec![120], "revived reserve is usable");
}

const TWO_WAY_WALLS: [(u32, u32); 7] = [
    (325, 326),
    (433, 434),
    (444, 443),
    (465, 464),
    (479, 480),
    (2318, 464),
    (2398, 2397),
];

#[test]
fn pet_battle_two_way_walls_share_two_blocks_between_allied_and_enemy_attacks() {
    let c = catalog();
    for (ability, aura) in TWO_WAY_WALLS {
        let mut b = battle(&c, 7, 7, [ability, 110, 123]);
        b.teams[0].pets[0].stats.speed = 200;
        let (b, events) = use_ability(&b, &c, ability);
        assert!(damage(&events).is_empty());
        assert_eq!(b.auras.len(), 1);
        assert_eq!(b.auras[0].ability, aura);
        assert_eq!(b.auras[0].charges, 2);
        assert_eq!(b.auras[0].remaining, -1);
        let (b, events) = use_ability(&b, &c, 110);
        assert!(
            damage(&events).is_empty(),
            "allied attack blocked: {ability}"
        );
        assert_eq!(b.auras[0].charges, 1);
        let (b, events) = b
            .step(
                &c,
                [Action::Pass, Action::Ability(110)],
                &mut Fixed(500_000),
            )
            .unwrap();
        assert!(
            damage(&events).is_empty(),
            "enemy attack blocked: {ability}"
        );
        assert!(b.auras.is_empty());
        for pet in &b.teams[0].pets {
            assert_eq!(
                pet.state(c.state_id("Special_BlockedAttackCount").unwrap()),
                0
            );
        }
        assert_eq!(damage(&use_ability(&b, &c, 110).1), vec![120]);
    }
}

#[test]
fn pet_battle_two_way_wall_preserves_charges_when_attacks_miss() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [325, 110, 123]);
    b.teams[0].pets[0].stats.speed = 200;
    b.teams[1].pets[0]
        .states
        .insert(c.state_id("Stat_Accuracy").unwrap(), -100);
    let (mut b, events) = b
        .step(
            &c,
            [Action::Ability(325), Action::Ability(110)],
            &mut Fixed(500_000),
        )
        .unwrap();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Miss { .. }))
    );
    assert_eq!(b.auras[0].charges, 2);
    for _ in 0..3 {
        b = b
            .step(&c, [Action::Pass; 2], &mut Fixed(500_000))
            .unwrap()
            .0;
    }
    assert_eq!(b.auras[0].charges, 2);
    assert_eq!(b.auras[0].remaining, -1);
}

#[test]
fn pet_battle_two_way_wall_recast_restores_two_shared_charges() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [325, 110, 123]);
    b.teams[0].pets[0].stats.speed = 200;
    let b = use_ability(&b, &c, 325).0;
    let mut b = use_ability(&b, &c, 110).0;
    assert_eq!(b.auras[0].charges, 1);
    for _ in 0..4 {
        b = b
            .step(&c, [Action::Pass; 2], &mut Fixed(500_000))
            .unwrap()
            .0;
    }
    let b = use_ability(&b, &c, 325).0;
    assert_eq!(b.auras.len(), 1);
    assert_eq!(b.auras[0].charges, 2);
    for pet in &b.teams[0].pets {
        assert_eq!(
            pet.state(c.state_id("Special_BlockedAttackCount").unwrap()),
            2
        );
    }
}

#[test]
fn pet_battle_two_way_wall_follows_owner_team_across_swaps() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [325, 110, 123]);
    b.teams[0].pets[0].stats.speed = 200;
    let b = use_ability(&b, &c, 325).0;
    let (b, events) = b
        .step(
            &c,
            [Action::Swap(2), Action::Ability(110)],
            &mut Fixed(500_000),
        )
        .unwrap();
    assert_eq!(b.teams[0].active, 2);
    assert!(damage(&events).is_empty());
    assert_eq!(b.auras[0].charges, 1);
    for pet in &b.teams[0].pets {
        assert_eq!(
            pet.state(c.state_id("Special_BlockedAttackCount").unwrap()),
            1
        );
    }
    let (b, events) = b
        .step(
            &c,
            [Action::Pass, Action::Ability(110)],
            &mut Fixed(500_000),
        )
        .unwrap();
    assert!(damage(&events).is_empty());
    assert!(b.auras.is_empty());
    assert_eq!(b.teams[0].pets[2].health, 10000);
}

#[test]
fn pet_battle_two_way_wall_does_not_block_existing_periodic_damage() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [152, 325, 110]);
    b.teams[0].pets[0].stats.speed = 200;
    let b = use_ability(&b, &c, 152).0;
    let (b, events) = use_ability(&b, &c, 325);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Damage { periodic: true, .. }))
    );
    let wall = b.auras.iter().find(|aura| aura.ability == 326).unwrap();
    assert_eq!(wall.charges, 2);
}

#[test]
fn pet_battle_tidal_wave_clears_two_way_wall_after_its_attacks() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [325, 419, 110]);
    b.teams[0].pets[0].stats.speed = 200;
    b.teams[1].pets[1].health = 0;
    b.teams[1].pets[2].health = 0;
    let b = use_ability(&b, &c, 325).0;
    let (b, events) = use_ability(&b, &c, 419);
    assert!(damage(&events).is_empty());
    assert!(b.auras.is_empty());
    assert_eq!(damage(&use_ability(&b, &c, 110).1), vec![120]);
}

#[test]
fn pet_battle_onyx_wall_distinct_friendly_prevention_semantics_stay_rejected() {
    struct NoDraw;
    impl BattleRng for NoDraw {
        fn draw(&mut self) -> u32 {
            panic!("unestablished Onyx Wall semantics must reject before RNG");
        }
    }
    let c = catalog();
    let b = battle(&c, 7, 7, [439, 110, 123]);
    assert!(
        b.step(&c, [Action::Ability(439), Action::Pass], &mut NoDraw)
            .is_err()
    );
}
