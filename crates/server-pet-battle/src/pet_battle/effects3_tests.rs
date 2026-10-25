use super::*;

#[test]
fn pet_battle_effects3_next_used_ability_lock_is_wearer_owned_and_one_shot() {
    let c = catalog();
    for root in [522, 1858] {
        let mut b = battle(&c, 7, 7, [root, 110, 123]);
        b.teams[0].pets[0].stats.speed = 200;
        let (b, events) = b
            .step(
                &c,
                [Action::Ability(root), Action::Ability(110)],
                &mut Fixed(500000),
            )
            .unwrap();
        assert_eq!(damage(&events), vec![120], "next Bite110 still executes");
        assert_eq!(b.teams[0].pets[0].health, 9880);
        assert_eq!(b.teams[1].pets[0].lockouts, [5, 0, 0]);
        assert_eq!(b.teams[0].pets[0].lockouts, [0; 3]);
        assert!(
            b.auras.is_empty(),
            "one-shot aura removes itself from wearer"
        );
        assert!(
            b.step(&c, [Action::Pass, Action::Ability(110)], &mut NoDraw)
                .unwrap_err()
                .contains("locked out")
        );
        let (b, _) = b
            .step(&c, [Action::Pass, Action::Ability(119)], &mut Fixed(500000))
            .unwrap();
        let (b, _) = b
            .step(&c, [Action::Pass, Action::Swap(2)], &mut Fixed(500000))
            .unwrap();
        // The lock belongs to the original wearer, not its team or the aura caster.
        let (b, _) = b
            .step(
                &c,
                [Action::Ability(110), Action::Ability(110)],
                &mut Fixed(500000),
            )
            .unwrap();
        let (b, _) = b
            .step(&c, [Action::Pass, Action::Swap(0)], &mut Fixed(500000))
            .unwrap();
        assert!(
            b.step(&c, [Action::Pass, Action::Ability(110)], &mut Fixed(500000))
                .is_ok()
        );
    }
}

#[test]
fn pet_battle_effects3_next_ability_aura_waits_through_pass_and_swap() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [522, 110, 123]);
    b.teams[0].pets[0].stats.speed = 200;
    let (b, _) = b
        .step(&c, [Action::Ability(522), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert_eq!(b.auras.len(), 1);
    let (b, _) = b
        .step(&c, [Action::Pass, Action::Swap(1)], &mut Fixed(500000))
        .unwrap();
    assert!(
        b.auras.is_empty(),
        "two-round aura expires without fabricating an ability use"
    );
    assert!(
        b.step(&c, [Action::Pass, Action::Ability(110)], &mut Fixed(500000))
            .is_ok()
    );
}

#[test]
fn pet_battle_effects3_lock_does_not_abort_committed_deep_breath() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [522, 110, 123]);
    b.teams[0].pets[0].stats.speed = 200;
    b.teams[1].pets[0].abilities = [169, 110, 123];
    let (b, _) = b
        .step(
            &c,
            [Action::Ability(522), Action::Ability(169)],
            &mut Fixed(500000),
        )
        .unwrap();
    assert_eq!(b.teams[1].pets[0].lockouts, [5, 0, 0]);
    assert_eq!(b.teams[1].pets[0].continuation, Some((169, 2)));
    let (b, events) = b.step(&c, [Action::Pass; 2], &mut Fixed(500000)).unwrap();
    assert_eq!(b.teams[1].pets[0].lockouts, [4, 0, 0]);
    assert_eq!(damage(&events), vec![300]);
    assert_eq!(b.teams[1].pets[0].continuation, None);
    assert!(
        b.step(&c, [Action::Pass, Action::Ability(169)], &mut NoDraw)
            .unwrap_err()
            .contains("locked out")
    );
}

#[test]
fn pet_battle_effects3_missed_lock_aura_does_not_lock_next_ability() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [1858, 110, 123]);
    b.teams[0].pets[0].stats.speed = 200;
    b.teams[0].pets[0]
        .states
        .insert(c.state_id("Stat_Accuracy").unwrap(), -100);
    let (b, events) = b
        .step(
            &c,
            [Action::Ability(1858), Action::Ability(110)],
            &mut Fixed(500000),
        )
        .unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Miss { effect: 3404, .. }))
    );
    assert!(b.auras.is_empty());
    assert!(
        b.step(&c, [Action::Pass, Action::Ability(110)], &mut Fixed(500000))
            .is_ok()
    );
}

#[test]
fn pet_battle_effects3_mind_games_locks_own_one_based_slots_not_opponent() {
    let c = catalog();
    let b = battle(&c, 7, 7, [110, 123, 2390]);
    let (b, events) = b
        .step(
            &c,
            [Action::Ability(2390), Action::Pass],
            &mut Fixed(500000),
        )
        .unwrap();
    assert_eq!(damage(&events), Vec::<i32>::new());
    assert_eq!(b.teams[0].pets[0].lockouts, [9, 9, 0]);
    let mut waiting = b.clone();
    for remaining in (1..9).rev() {
        assert!(
            waiting
                .step(&c, [Action::Ability(110), Action::Pass], &mut NoDraw)
                .unwrap_err()
                .contains("locked out")
        );
        waiting = waiting
            .step(&c, [Action::Pass; 2], &mut Fixed(500000))
            .unwrap()
            .0;
        assert_eq!(waiting.teams[0].pets[0].lockouts, [remaining, remaining, 0]);
    }
    assert!(
        waiting
            .step(&c, [Action::Ability(110), Action::Pass], &mut Fixed(500000))
            .is_ok()
    );
    for id in [110, 123] {
        assert!(
            b.step(&c, [Action::Ability(id), Action::Pass], &mut NoDraw)
                .unwrap_err()
                .contains("locked out")
        );
    }
    assert_eq!(b.teams[0].pets[0].state(c.state_id("turnLock").unwrap()), 0);
    assert!(
        b.step(&c, [Action::Pass, Action::Ability(110)], &mut Fixed(500000))
            .is_ok()
    );
    let (b, _) = b
        .step(&c, [Action::Swap(1), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert!(
        b.step(&c, [Action::Ability(110), Action::Pass], &mut Fixed(500000))
            .is_ok()
    );
}

#[test]
fn pet_battle_effects3_ambiguous_persistent_spores_and_override_still_reject_without_rng() {
    let c = catalog();
    for id in [2213, 163] {
        let b = battle(&c, 7, 7, [id, 110, 123]);
        assert!(
            b.step(&c, [Action::Ability(id), Action::Pass], &mut NoDraw)
                .is_err()
        );
    }
}
