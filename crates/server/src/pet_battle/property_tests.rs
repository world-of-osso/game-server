use super::*;

fn use_ability(b: &Battle, c: &Catalog, id: u32) -> (Battle, Vec<Event>) {
    b.step(c, [Action::Ability(id), Action::Pass], &mut Fixed(500000))
        .unwrap()
}

#[test]
fn pet_battle_80_weather_all_slots_replace_expire_elemental() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [256, 595, 123]);
    b.teams[1].pets[2] = Pet::new(
        &c,
        b.teams[1].pets[2].stats,
        6,
        [110, 119, 123],
        States::new(),
    )
    .unwrap();
    let (b, _) = use_ability(&b, &c, 256);
    // Darkness257: -50% healing received, -10 accuracy, blind1 on all slots.
    for team in 0..2 {
        for slot in 0..3 {
            let pet = &b.teams[team].pets[slot];
            let elemental = team == 1 && slot == 2;
            assert_eq!(pet.state(c.state_id("Weather_Darkness").unwrap()), 1);
            assert_eq!(
                pet.state(c.state_id("Mod_HealingTakenPercent").unwrap()),
                if elemental { 0 } else { -50 }
            );
            assert_eq!(
                pet.state(c.state_id("Mechanic_IsBlind").unwrap()),
                if elemental { 0 } else { 1 }
            );
        }
    }
    let (mut b, _) = use_ability(&b, &c, 595);
    // Moonlight replaces rather than stacks with Darkness: +25% received,
    // +10% Magic damage; flags persist on bench pets through swaps.
    assert!(b.auras.iter().all(|a| a.ability != 257));
    let (next, _) = b
        .step(&c, [Action::Swap(1), Action::Pass], &mut Fixed(500000))
        .unwrap();
    b = next;
    for team in &b.teams {
        for pet in &team.pets {
            assert_eq!(pet.state(c.state_id("Weather_Darkness").unwrap()), 0);
            assert_eq!(
                pet.state(c.state_id("Mod_HealingTakenPercent").unwrap()),
                25
            );
        }
    }
    for _ in 0..8 {
        b = b.step(&c, [Action::Pass; 2], &mut Fixed(500000)).unwrap().0;
    }
    assert!(b.auras.is_empty());
    for team in &b.teams {
        for pet in &team.pets {
            assert_eq!(pet.state(c.state_id("Mod_HealingTakenPercent").unwrap()), 0);
            assert_eq!(pet.state(c.state_id("Mechanic_IsBlind").unwrap()), 0);
        }
    }
}

#[test]
fn pet_battle_24_sunlight_enemy_slots_and_80_max_health() {
    let c = catalog();
    let b = battle(&c, 7, 7, [404, 123, 110]);
    // Sunlight404 has three15-point enemy-slot attacks:15*6=90 each.
    // Aura403 gives +50% maximum HP=15000, +25% healing done to all6 pets.
    let (b, e) = use_ability(&b, &c, 404);
    assert_eq!(damage(&e), vec![90, 90, 90]);
    for pet in &b.teams[1].pets {
        assert_eq!(pet.health, 9910);
    }
    for team in &b.teams {
        for pet in &team.pets {
            assert_eq!(pet.stats.health, 15000);
        }
    }
    let mut b = b;
    b.teams[0].pets[0].health = 5000;
    let (mut b, _) = use_ability(&b, &c, 123);
    // Healing Wave123 base30*6=180; Sunlight180*1.25=225.
    assert_eq!(b.teams[0].pets[0].health, 5225);
    for _ in 0..8 {
        b = b.step(&c, [Action::Pass; 2], &mut Fixed(500000)).unwrap().0;
    }
    for team in &b.teams {
        for pet in &team.pets {
            assert_eq!(pet.stats.health, 10000);
        }
    }
}

#[test]
fn pet_battle_141_arcane_storm_splits_pool_and_weather_bonus() {
    let c = catalog();
    let b = battle(&c, 7, 7, [589, 110, 123]);
    // Arcane Storm589 pool24*6=144/3 living enemies=48 each.
    let (mut b, e) = use_ability(&b, &c, 589);
    assert_eq!(damage(&e), vec![48, 48, 48]);
    b.teams[0].pets[0].cooldowns.clear();
    // Arcane Winds state54 supplies bonus4: (24+4)*6/3=56 each.
    let (_, e) = use_ability(&b, &c, 589);
    assert_eq!(damage(&e), vec![56, 56, 56]);
    b.teams[1].pets[2].health = 0;
    // Split among TWO living pets: (24+4)*6/2=84 each (dead pet gets none).
    let (_, e) = use_ability(&b, &c, 589);
    assert_eq!(damage(&e), vec![84, 84]);
}

#[test]
fn pet_battle_29_takedown_adds_only_when_target_stunned() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [221, 110, 123]);
    // Takedown221: first18*6=108; conditional second18*6=108 only if state22.
    assert_eq!(damage(&use_ability(&b, &c, 221).1), vec![108]);
    b.teams[1].pets[0]
        .states
        .insert(c.state_id("Mechanic_IsStunned").unwrap(), 1);
    assert_eq!(damage(&use_ability(&b, &c, 221).1), vec![108, 108]);
}

#[test]
fn pet_battle_62_trample_percent_has_no_power_scaling() {
    let c = catalog();
    let b = battle(&c, 7, 7, [377, 110, 123]);
    // Trample377:10*6=60 ordinary damage +10% of target max10000=1000.
    assert_eq!(damage(&use_ability(&b, &c, 377).1), vec![60, 1000]);
}

#[test]
fn pet_battle_178_bash_resilient_reports_immune() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [348, 110, 123]);
    b.teams[0].pets[0].stats.speed = 200;
    b.teams[1].pets[0]
        .states
        .insert(c.state_id("resilient").unwrap(), 1);
    // Bash348 params state149,value1 blocks the1-round stun and reports immune.
    let (next, e) = use_ability(&b, &c, 348);
    assert!(next.auras.is_empty());
    assert!(e.iter().any(|e| matches!(e, Event::Immune { .. })));
    b.teams[1].pets[0].states.clear();
    let (_, e) = b
        .step(
            &c,
            [Action::Ability(348), Action::Ability(110)],
            &mut Fixed(500000),
        )
        .unwrap();
    assert!(e.contains(&Event::Skipped(PetRef { team: 1, slot: 0 })));
}

#[test]
fn pet_battle_384_385_tail_sweep_selects_first_or_last() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [122, 110, 123]);
    // Tail Sweep122:25*6=150 when user acts last;18*6=108 otherwise.
    b.teams[0].pets[0].stats.speed = 1;
    assert_eq!(damage(&use_ability(&b, &c, 122).1), vec![150]);
    b.teams[0].pets[0].stats.speed = 200;
    assert_eq!(damage(&use_ability(&b, &c, 122).1), vec![108]);
}

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
