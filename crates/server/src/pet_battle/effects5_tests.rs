use super::*;

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
    let error = b
        .step(&c, [Action::Ability(439), Action::Pass], &mut NoDraw)
        .unwrap_err();
    assert!(error.contains("150"), "{error}");
}
