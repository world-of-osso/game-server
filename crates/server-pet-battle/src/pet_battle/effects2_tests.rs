use super::*;

#[test]
fn pet_battle_consumption_speed_bonus_is_strict_and_uses_modified_speed() {
    let c = catalog();
    for (id, speed, expected) in [
        (2024, 50, 300),
        (2024, 100, 120),
        (2024, 200, 120),
        (2026, 50, 120),
        (2026, 100, 120),
        (2026, 200, 300),
    ] {
        let mut b = battle(&c, 7, 7, [id, 110, 123]);
        b.teams[0].pets[0].stats.speed = speed;
        assert_eq!(damage(&use_ability(&b, &c, id).1), vec![expected]);
    }
}

#[test]
fn pet_battle_creeping_fungus_ticks_moonlight_bonus() {
    let c = catalog();
    for (moonlight, expected) in [(0, 42), (1, 60)] {
        let mut b = battle(&c, 7, 7, [743, 110, 123]);
        b.teams[0].pets[0]
            .states
            .insert(c.state_id("Weather_Moonlight").unwrap(), moonlight);
        assert_eq!(damage(&use_ability(&b, &c, 743).1), vec![expected]);
    }
}

#[test]
fn pet_battle_weakening_blow_cannot_faint_or_trigger_mechanical_revive() {
    let c = catalog();
    let mut b = battle(&c, 7, 9, [826, 110, 123]);
    b.teams[1].pets[0].health = 100;
    let (b, e) = use_ability(&b, &c, 826);
    assert_eq!(b.teams[1].pets[0].health, 1);
    assert!(!b.teams[1].pets[0].resurrection_used);
    assert!(
        !e.iter()
            .any(|e| matches!(e, Event::Revived(_) | Event::Fainted(_)))
    );
    assert_eq!(use_ability(&b, &c, 826).0.teams[1].pets[0].health, 1);
}

#[test]
fn pet_battle_eat_the_fly_below_health_threshold_heal() {
    let c = catalog();
    for (health, expected) in [(3300, 3360), (3299, 3539)] {
        let mut b = battle(&c, 7, 7, [2206, 110, 123]);
        b.teams[0].pets[0].health = health;
        assert_eq!(
            use_ability(&b, &c, 2206).0.teams[0].pets[0].health,
            expected
        );
    }
}

#[test]
fn pet_battle_stench_applies_to_each_enemy_slot_and_expires() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [527, 110, 123]);
    b.teams[1].active = 2;
    b = use_ability(&b, &c, 527).0;
    for pet in &b.teams[1].pets {
        assert_eq!(pet.state(c.state_id("Stat_Accuracy").unwrap()), -25);
    }
    assert_eq!(b.auras.len(), 3);
    for _ in 0..3 {
        b = b.step(&c, [Action::Pass; 2], &mut Fixed(500000)).unwrap().0;
    }
    for pet in &b.teams[1].pets {
        assert_eq!(pet.state(c.state_id("Stat_Accuracy").unwrap()), 0);
    }
    assert!(b.auras.is_empty());
}

#[test]
fn pet_battle_cyclone_tracks_enemy_team_across_swap_and_tidal_removes_it() {
    let c = catalog();
    let mut b = battle(&c, 3, 7, [190, 110, 123]);
    //35% accuracy, midpoint variance and non-critical draw sequence.
    struct CycloneRng(usize);
    impl BattleRng for CycloneRng {
        fn draw(&mut self) -> u32 {
            let n = self.0;
            self.0 += 1;
            if n % 3 == 0 { 100000 } else { 500000 }
        }
    }
    b.teams[0].pets[0].stats.speed = 200;
    let (b, e) = b
        .step(&c, [Action::Ability(190), Action::Pass], &mut CycloneRng(0))
        .unwrap();
    assert_eq!(damage(&e), vec![36, 36, 36]);
    assert_eq!(b.auras.len(), 1);
    let (mut b, e) = b
        .step(&c, [Action::Pass, Action::Swap(2)], &mut CycloneRng(0))
        .unwrap();
    assert_eq!(damage(&e), vec![36, 36, 36]);
    assert_eq!(b.teams[1].active, 2);
    b.teams[1].pets[2].abilities = [419, 110, 123];
    let (b, _) = b
        .step(&c, [Action::Pass, Action::Ability(419)], &mut Fixed(500000))
        .unwrap();
    assert!(b.auras.is_empty());
}

#[test]
fn pet_battle_arcane_explosion_follows_active_slot_and_other_slots() {
    let c = catalog();
    for active in 0..3 {
        let mut b = battle(&c, 5, 7, [299, 110, 123]);
        b.teams[1].active = active;
        let (b, e) = use_ability(&b, &c, 299);
        assert_eq!(damage(&e), vec![60, 36, 36]);
        for slot in 0..3 {
            assert_eq!(
                b.teams[1].pets[slot].health,
                10000 - if slot == active { 60 } else { 36 }
            );
        }
    }
}

#[test]
fn pet_battle_deep_burn_bonus_requires_burning_target() {
    let c = catalog();
    for (burning, expected) in [(0, 78), (1, 156)] {
        let mut b = battle(&c, 6, 7, [1407, 110, 123]);
        b.teams[1].pets[0]
            .states
            .insert(c.state_id("Mechanic_IsBurning").unwrap(), burning);
        assert_eq!(damage(&use_ability(&b, &c, 1407).1), vec![expected]);
    }
}

#[test]
fn pet_battle_kick_interrupts_only_when_acting_first() {
    let c = catalog();
    for (speed, interrupted) in [(200, true), (50, false)] {
        let mut b = battle(&c, 0, 7, [307, 110, 123]);
        b.teams[0].pets[0].stats.speed = speed;
        let (_, e) = b
            .step(
                &c,
                [Action::Ability(307), Action::Ability(110)],
                &mut Fixed(500000),
            )
            .unwrap();
        assert_eq!(
            e.contains(&Event::Skipped(PetRef { team: 1, slot: 0 })),
            interrupted
        );
        assert_eq!(damage(&e).len(), if interrupted { 1 } else { 2 });
    }
}

#[test]
fn pet_battle_deep_freeze_chilled_stun_and_critter_immunity() {
    let c = catalog();
    for (family, chilled, skipped) in [(7, 0, false), (7, 1, true), (4, 1, false)] {
        let mut b = battle(&c, 6, family, [481, 110, 123]);
        b.teams[0].pets[0].stats.speed = 200;
        b.teams[1].pets[0]
            .states
            .insert(c.state_id("Mechanic_IsChilled").unwrap(), chilled);
        let (_, e) = b
            .step(
                &c,
                [Action::Ability(481), Action::Ability(110)],
                &mut Fixed(500000),
            )
            .unwrap();
        assert_eq!(
            e.contains(&Event::Skipped(PetRef { team: 1, slot: 0 })),
            skipped
        );
    }
}

#[test]
fn pet_battle_leech_life_heals_after_hit_only_and_web_doubles() {
    let c = catalog();
    for (webbed, accuracy, expected) in [(0, 0, 60), (1, 0, 120), (1, -100, 0)] {
        let mut b = battle(&c, 7, 7, [383, 110, 123]);
        b.teams[0].pets[0].health = 8000;
        b.teams[0].pets[0]
            .states
            .insert(c.state_id("Stat_Accuracy").unwrap(), accuracy);
        b.teams[1].pets[0]
            .states
            .insert(c.state_id("Mechanic_IsWebbed").unwrap(), webbed);
        let (b, e) = use_ability(&b, &c, 383);
        assert_eq!(b.teams[0].pets[0].health, 8000 + expected);
        if expected > 0 {
            let dealt = e
                .iter()
                .position(|e| matches!(e, Event::Damage { .. }))
                .unwrap();
            let healed = e
                .iter()
                .position(|e| matches!(e, Event::Heal { .. }))
                .unwrap();
            assert!(dealt < healed);
        }
    }
}

pub(super) fn use_ability(b: &Battle, c: &Catalog, id: u32) -> (Battle, Vec<Event>) {
    b.step(c, [Action::Ability(id), Action::Pass], &mut Fixed(500000))
        .unwrap()
}

#[test]
fn pet_battle_cauterize_uses_half_actual_last_hit_without_power_scaling() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [173, 110, 123]);
    b.teams[0].pets[0].health = 8000;
    b.teams[0].pets[0]
        .states
        .insert(c.state_id("Last_HitTaken").unwrap(), 84);
    let (b, events) = use_ability(&b, &c, 173);
    // Cauterize:20 points * (1+.05*100)=120, plus84*50%=42.
    assert_eq!(b.teams[0].pets[0].health, 8162);
    assert_eq!(
        events
            .iter()
            .filter_map(|e| match e {
                Event::Heal { amount, .. } => Some(*amount),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![120, 42]
    );
}

#[test]
fn pet_battle_comeback_requires_lower_health_for_additional_damage() {
    let c = catalog();
    for (health, expected) in [(10000, vec![90]), (9999, vec![90]), (9900, vec![90, 60])] {
        let mut b = battle(&c, 7, 7, [253, 110, 123]);
        b.teams[0].pets[0].health = health;
        let (_, e) = use_ability(&b, &c, 253);
        // The second row compares health AFTER the90-point first strike.
        assert_eq!(damage(&e), expected);
    }
}

#[test]
fn pet_battle_predatory_strike_quarter_health_boundary() {
    let c = catalog();
    for (health, expected) in [(2500, 180), (2499, 360)] {
        let mut b = battle(&c, 3, 7, [518, 110, 123]);
        b.teams[1].pets[0].health = health;
        let (_, e) = use_ability(&b, &c, 518);
        // Boost100 means double30 points below25%, not at25%.
        assert_eq!(damage(&e), vec![expected]);
    }
}

#[test]
fn pet_battle_nocturnal_strike_blind_guarantees_hit_but_not_targetability() {
    let c = catalog();
    let mut b = battle(&c, 3, 7, [517, 110, 123]);
    b.teams[0].pets[0]
        .states
        .insert(c.state_id("Stat_Accuracy").unwrap(), -100);
    assert!(damage(&use_ability(&b, &c, 517).1).is_empty());
    b.teams[1].pets[0]
        .states
        .insert(c.state_id("Mechanic_IsBlind").unwrap(), 1);
    b.teams[1].pets[0]
        .states
        .insert(c.state_id("Stat_Dodge").unwrap(), 100);
    assert_eq!(damage(&use_ability(&b, &c, 517).1), vec![240]);
    b.teams[1].pets[0]
        .states
        .insert(c.state_id("untargettable").unwrap(), 1);
    assert!(damage(&use_ability(&b, &c, 517).1).is_empty());
}

#[test]
fn pet_battle_wind_up_charges_fires_and_charges_again() {
    let c = catalog();
    let b = battle(&c, 9, 0, [459, 110, 123]);
    let (b, e) = use_ability(&b, &c, 459);
    assert!(damage(&e).is_empty());
    assert_eq!(
        b.teams[0].pets[0].state(c.state_id("Mod_DamageDealtPercent").unwrap()),
        10
    );
    let (b, e) = use_ability(&b, &c, 459);
    // Charged hit:45*6*1.1=297; consumes its10% charge after firing.
    assert_eq!(damage(&e), vec![297]);
    assert!(b.auras.is_empty());
    assert_eq!(
        b.teams[0].pets[0].state(c.state_id("Mod_DamageDealtPercent").unwrap()),
        0
    );
    let (_, e) = use_ability(&b, &c, 459);
    assert!(damage(&e).is_empty());
}

#[test]
fn pet_battle_rebuild_heals_only_living_other_mechanical_allies() {
    let c = catalog();
    let mut b = battle(&c, 9, 7, [533, 110, 123]);
    b.teams[0].active = 1;
    for pet in &mut b.teams[0].pets {
        pet.health = 9000;
    }
    b.teams[0].pets[2].family = 8;
    let (b, _) = use_ability(&b, &c, 533);
    //24*6 on active;12*6 on other mechanical; aquatic bench is unaffected.
    assert_eq!(
        b.teams[0].pets.iter().map(|p| p.health).collect::<Vec<_>>(),
        vec![9072, 9144, 9000]
    );
    let mut b = b;
    b.teams[0].pets[0].health = 0;
    b.teams[0].pets[1].cooldowns.clear();
    assert_eq!(use_ability(&b, &c, 533).0.teams[0].pets[0].health, 0);
}
