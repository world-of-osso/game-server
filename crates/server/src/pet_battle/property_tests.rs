use super::*;

#[test]
fn pet_battle_death_coil_158_stops_failed_chain_32_heals_damage() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [121, 110, 123]);
    b.teams[0].pets[0].health = 5000;
    // Death Coil121:14*(1+.05*100)=84 neutral Undead damage;
    // property32 restores50% of the last hit =42, without scaling power again.
    let (next, e) = b
        .step(&c, [Action::Ability(121), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert_eq!(damage(&e), vec![84]);
    assert_eq!(next.teams[0].pets[0].health, 5042);
    b.teams[0].pets[0]
        .states
        .insert(c.state_id("Stat_Accuracy").unwrap(), -100);
    let (next, e) = b
        .step(&c, [Action::Ability(121), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert!(damage(&e).is_empty());
    assert_eq!(next.teams[0].pets[0].health, 5000);
    assert!(!e.iter().any(|e| matches!(e, Event::Heal { .. })));
}

#[test]
fn pet_battle_quick_attack_79_116_priority_restores_speed() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [563, 110, 123]);
    b.teams[0].pets[0].stats.speed = 1;
    b.teams[1].pets[0].stats.speed = 500;
    // Quick Attack563 temporarily adds10000 speed before sorting; removes10000
    // after the round.15*6=90 damage even though base speed1 is below500.
    let (next, e) = b
        .step(
            &c,
            [Action::Ability(563), Action::Ability(110)],
            &mut Fixed(500000),
        )
        .unwrap();
    assert_eq!(e[0], Event::Order([0, 1]));
    assert_eq!(damage(&e), vec![90, 120]);
    let (_, e) = next
        .step(&c, [Action::Pass; 2], &mut Fixed(500000))
        .unwrap();
    assert_eq!(e[0], Event::Order([1, 0]));
}

#[test]
fn pet_battle_frog_kiss_27_ramps_177_blocks_resilient() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [233, 110, 123]);
    b.teams[1].pets[0]
        .states
        .insert(c.state_id("resilient").unwrap(), 1);
    // Frog Kiss233: base10 +10/use, capped30; power100 ->60,120,180,180.
    // Resilient149=1 must stop only the stun branch, not the preceding attack.
    for expected in [60, 120, 180, 180] {
        let (next, e) = b
            .step(&c, [Action::Ability(233), Action::Pass], &mut Fixed(500000))
            .unwrap();
        assert_eq!(damage(&e), vec![expected]);
        assert!(next.auras.is_empty());
        b = next;
    }
}

#[test]
fn pet_battle_22_dummy_points_are_tooltip_metadata() {
    let c = catalog();
    let b = battle(&c, 7, 7, [577, 110, 123]);
    // Healthy577 has dummy Points5 (tooltip), not a five-point heal/damage.
    // The dummy itself does not apply the aura's maxHealthBonus state.
    // Isolate it from aura admission, which requires separate stat handling.
    let mut rng = Fixed(500000);
    let mut rt = super::super::engine::Runtime {
        battle: b,
        catalog: &c,
        rng: &mut rng,
        events: Vec::new(),
    };
    rt.cast_turn(
        PetRef { team: 0, slot: 0 },
        PetRef { team: 1, slot: 0 },
        577,
        1,
        255,
    )
    .unwrap();
    assert!(damage(&rt.events).is_empty());
    assert_eq!(rt.battle.teams[0].pets[0].health, 10000);
}
