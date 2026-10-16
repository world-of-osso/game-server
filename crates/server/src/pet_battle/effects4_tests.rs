use super::*;

#[test]
fn pet_battle_fade_dodges_then_swaps_to_either_living_reserve() {
    let c = catalog();
    for (draw, expected_slot) in [(0, 1), (999_999, 2)] {
        let mut b = battle(&c, 7, 7, [1084, 110, 123]);
        b.teams[0].pets[0].stats.speed = 200;
        let (b, events) = b
            .step(
                &c,
                [Action::Ability(1084), Action::Ability(110)],
                &mut Fixed(draw),
            )
            .unwrap();
        assert_eq!(b.teams[0].active, expected_slot);
        assert_eq!(b.teams[1].active, 0);
        assert_eq!(b.teams[0].pets[0].health, 10000);
        assert!(b.auras.is_empty());
        assert_eq!(
            b.teams[0].pets[0].state(c.state_id("Stat_Dodge").unwrap()),
            0
        );
        let miss = events
            .iter()
            .position(|event| matches!(event, Event::Miss { target, .. } if target.team == 0))
            .unwrap();
        let swap = events
            .iter()
            .position(|event| matches!(event, Event::Swapped { team: 0, .. }))
            .unwrap();
        assert!(
            miss < swap,
            "Fade must protect before its end-of-round swap"
        );
    }
}

#[test]
fn pet_battle_fade_does_not_select_a_fainted_reserve() {
    let c = catalog();
    for living in [1, 2] {
        let mut b = battle(&c, 7, 7, [1084, 110, 123]);
        b.teams[0].pets[0].stats.speed = 200;
        b.teams[0].pets[3 - living].health = 0;
        let (b, _) = b
            .step(
                &c,
                [Action::Ability(1084), Action::Pass],
                &mut Fixed(999_999),
            )
            .unwrap();
        assert_eq!(b.teams[0].active, living);
    }
}

#[test]
fn pet_battle_fade_stays_when_there_is_no_living_reserve() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [1084, 110, 123]);
    b.teams[0].pets[0].stats.speed = 200;
    b.teams[0].pets[1].health = 0;
    b.teams[0].pets[2].health = 0;
    let (b, events) = b
        .step(
            &c,
            [Action::Ability(1084), Action::Pass],
            &mut Fixed(500_000),
        )
        .unwrap();
    assert_eq!(b.teams[0].active, 0);
    assert!(b.auras.is_empty());
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Swapped { .. }))
    );
}
