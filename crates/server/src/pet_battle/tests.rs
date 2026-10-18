use super::*;
use rusqlite::Connection;

#[path = "property_tests.rs"]
mod property_tests;

#[path = "effects2_tests.rs"]
mod effects2_tests;

#[path = "effects3_tests.rs"]
mod effects3_tests;

#[path = "effects4_tests.rs"]
mod effects4_tests;

#[path = "effects5_tests.rs"]
mod effects5_tests;

#[path = "effects6_tests.rs"]
mod effects6_tests;

#[path = "effects7_tests.rs"]
mod effects7_tests;

#[path = "effects8_tests.rs"]
mod effects8_tests;

#[path = "effects9_tests.rs"]
mod effects9_tests;

#[path = "effects10_tests.rs"]
mod effects10_tests;

#[path = "effects11_tests.rs"]
mod effects11_tests;

#[path = "effects12_tests.rs"]
mod effects12_tests;

#[path = "effects13_tests.rs"]
mod effects13_tests;

#[path = "effects14_tests.rs"]
mod effects14_tests;

#[path = "effects15_tests.rs"]
mod effects15_tests;

#[path = "effects16_tests.rs"]
mod effects16_tests;

#[path = "effects17_tests.rs"]
mod effects17_tests;

fn fixture_db() -> Connection {
    // Copy only battle tables; corruption tests must never mutate shipped world.db.
    let db = Connection::open_in_memory().unwrap();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/world.db");
    let uri = format!("file:{path}?mode=ro");
    db.execute("ATTACH DATABASE ? AS shipped", [&uri])
        .expect("open shipped world.db; run scripts/import-pet-battle-tables.py first");
    for table in [
        "battle_pet_ability",
        "battle_pet_ability_turn",
        "battle_pet_ability_effect",
        "battle_pet_effect_properties",
        "battle_pet_state",
        "battle_pet_species_state",
        "battle_pet_breed_state",
        "battle_pet_breed_quality",
        "battle_pet_species_x_ability",
        "battle_pet_ability_state",
    ] {
        db.execute_batch(&format!(
            "CREATE TABLE {table} AS SELECT * FROM shipped.{table}"
        ))
        .unwrap();
    }
    db.execute_batch("DETACH DATABASE shipped").unwrap();
    db
}

#[test]
fn pet_battle_locked_ability_slots_cannot_be_used() {
    let catalog = catalog();
    let stats = calculate_stats(&catalog, 39, 3, 3, 1).unwrap();
    let pet = Pet::new(&catalog, 39, 1, stats, 9, [110, 0, 0], States::new()).unwrap();
    let team = Team {
        pets: [pet.clone(), pet.clone(), pet],
        active: 0,
    };
    let battle = Battle::new([team.clone(), team]).unwrap();
    assert!(
        battle
            .step(&catalog, [Action::Ability(0), Action::Pass], &mut Fixed(0))
            .is_err()
    );
}

#[test]
fn pet_battle_real_species39_rare_breed3_stats() {
    let catalog = Catalog::load_world_db(&fixture_db()).unwrap();
    // CASC: species39 stamina+100, power-100, speed0; breed3 each1700.
    // Rare=.65, level25: round(1800*.65*25/20)+100=1563;
    // round(1600*.65*25/100)=260; round(1700*.65*25/100)=276.
    assert_eq!(
        calculate_stats(&catalog, 39, 3, 3, 25).unwrap(),
        Stats {
            health: 1563,
            power: 260,
            speed: 276
        }
    );
}

#[test]
fn pet_battle_missing_world_tables_are_explicit() {
    let err = Catalog::load_world_db(&Connection::open_in_memory().unwrap()).unwrap_err();
    assert!(err.contains("battle_pet_ability"), "{err}");
}

struct Fixed(u32);
impl BattleRng for Fixed {
    fn draw(&mut self) -> u32 {
        self.0
    }
}
fn catalog() -> Catalog {
    Catalog::load_world_db(&fixture_db()).unwrap()
}
fn battle(catalog: &Catalog, family: usize, target_family: usize, abilities: [u32; 3]) -> Battle {
    let stats = Stats {
        health: 10000,
        power: 100,
        speed: 100,
    };
    let caster = Pet::new(catalog, 39, 1, stats, family, abilities, States::new()).unwrap();
    let target = Pet::new(
        catalog,
        39,
        1,
        stats,
        target_family,
        [110, 119, 123],
        States::new(),
    )
    .unwrap();
    Battle::new([
        Team {
            pets: [caster.clone(), caster.clone(), caster],
            active: 0,
        },
        Team {
            pets: [target.clone(), target.clone(), target],
            active: 0,
        },
    ])
    .unwrap()
}
fn damage(events: &[Event]) -> Vec<i32> {
    events
        .iter()
        .filter_map(|e| {
            if let Event::Damage { amount, .. } = e {
                Some(*amount)
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn pet_battle_bite_strong_weak_neutral_real_id110() {
    let c = catalog();
    // Bite effect277:20points; power100 =>20*(1+.05*100)=120.
    // Beast attack vs Critter:180; Flying:floor(120*.67)=80; Humanoid:120.
    for (family, expected) in [(4, 180), (2, 80), (0, 120)] {
        let b = battle(&c, 7, family, [110, 119, 123]);
        let (next, events) = b
            .step(&c, [Action::Ability(110), Action::Pass], &mut Fixed(500000))
            .unwrap();
        assert_eq!(damage(&events), vec![expected]);
        assert_eq!(next.teams[1].pets[0].health, 10000 - expected);
        assert_eq!(b.teams[1].pets[0].health, 10000, "input state is immutable");
    }
}
#[test]
fn pet_battle_accuracy_crit_variance_and_replay() {
    let c = catalog();
    let mut b = battle(&c, 7, 0, [110, 113, 123]);
    // Burn effect280 variance40 =>+/-20%; midpoint21*6=126.
    let (next, e) = b
        .step(&c, [Action::Ability(113), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert_eq!(damage(&e), vec![126]);
    let replay = b
        .step(&c, [Action::Ability(113), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert_eq!(replay, (next, e));
    // Roll0 gives Bite lower variance bound108 and5% crit:floor(108*1.5)=162.
    let (_, e) = b
        .step(&c, [Action::Ability(110), Action::Pass], &mut Fixed(0))
        .unwrap();
    assert_eq!(damage(&e), vec![162]);
    b.teams[0].pets[0]
        .states
        .insert(c.state_id("Stat_Accuracy").unwrap(), -100);
    let (next, e) = b
        .step(&c, [Action::Ability(110), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert!(e.iter().any(|e| matches!(e, Event::Miss { .. })));
    assert_eq!(next.teams[1].pets[0].health, 10000);
}
#[test]
fn pet_battle_poison_fang_ticks_five_rounds() {
    let c = catalog();
    let b = battle(&c, 7, 0, [152, 110, 123]);
    // Ability152 direct10*6=60; aura379 tick5*6=30, duration5.
    let (mut b, e) = b
        .step(&c, [Action::Ability(152), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert_eq!(damage(&e), vec![60, 30]);
    for _ in 0..4 {
        let (next, e) = b.step(&c, [Action::Pass; 2], &mut Fixed(500000)).unwrap();
        assert_eq!(damage(&e), vec![30]);
        b = next;
    }
    assert_eq!(b.teams[1].pets[0].health, 9790);
    assert!(b.auras.is_empty());
    let (_, e) = b.step(&c, [Action::Pass; 2], &mut Fixed(500000)).unwrap();
    assert!(damage(&e).is_empty());
}
#[test]
fn pet_battle_time_stop_stun_skips_and_grants_resilient() {
    let c = catalog();
    let mut b = battle(&c, 5, 7, [600, 110, 123]);
    b.teams[0].pets[0].stats.speed = 200;
    let (b, e) = b
        .step(
            &c,
            [Action::Ability(600), Action::Ability(110)],
            &mut Fixed(500000),
        )
        .unwrap();
    assert!(e.contains(&Event::Skipped(PetRef { team: 1, slot: 0 })));
    assert!(damage(&e).is_empty());
    // aura927 states22+35; removal event5 applies aura924/state149 for2rounds.
    assert_eq!(b.teams[1].pets[0].state(c.state_id("turnLock").unwrap()), 0);
    assert_eq!(
        b.teams[1].pets[0].state(c.state_id("resilient").unwrap()),
        1
    );
    let (_, e) = b
        .step(&c, [Action::Pass, Action::Ability(110)], &mut Fixed(500000))
        .unwrap();
    assert_eq!(damage(&e), vec![120]);
}
#[test]
fn pet_battle_speed_ties_swaps_and_forfeit() {
    let c = catalog();
    let b = battle(&c, 7, 7, [110, 119, 123]);
    let (_, e) = b.step(&c, [Action::Pass; 2], &mut Fixed(0)).unwrap();
    assert_eq!(e[0], Event::Order([0, 1]));
    let (_, e) = b.step(&c, [Action::Pass; 2], &mut Fixed(999999)).unwrap();
    assert_eq!(e[0], Event::Order([1, 0]));
    let mut faster = b.clone();
    faster.teams[1].pets[0].stats.speed = 101;
    let (_, e) = faster
        .step(&c, [Action::Ability(110); 2], &mut Fixed(500000))
        .unwrap();
    assert_eq!(e[0], Event::Order([1, 0]));
    let (swapped, _) = faster
        .step(
            &c,
            [Action::Swap(1), Action::Ability(110)],
            &mut Fixed(500000),
        )
        .unwrap();
    assert_eq!(swapped.teams[0].pets[0].health, 10000);
    assert_eq!(swapped.teams[0].pets[1].health, 9880);
    let (ended, e) = b
        .step(&c, [Action::Forfeit, Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert_eq!(ended.outcome, Some(Outcome::Winner(1)));
    assert!(damage(&e).is_empty());
}
#[test]
fn pet_battle_beast_critter_flying_magic_passives() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [110, 600, 123]);
    b.teams[0].pets[0].health = 4999;
    let (_, e) = b
        .step(&c, [Action::Ability(110), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert_eq!(damage(&e), vec![150]);
    b.teams[0].pets[0].health = 5000;
    let (_, e) = b
        .step(&c, [Action::Ability(110), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert_eq!(damage(&e), vec![120]);
    let b = battle(&c, 7, 4, [600, 110, 123]);
    let (b, e) = b
        .step(&c, [Action::Ability(600), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert!(e.iter().any(|e| matches!(e, Event::Immune { .. })));
    assert!(b.auras.is_empty());
    let b = battle(&c, 7, 2, [110, 119, 123]);
    let (_, e) = b.step(&c, [Action::Pass; 2], &mut Fixed(0)).unwrap();
    assert_eq!(e[0], Event::Order([1, 0]));
    let mut b = battle(&c, 7, 5, [110, 119, 123]);
    b.teams[0].pets[0].stats.power = 10000;
    let (_, e) = b
        .step(&c, [Action::Ability(110), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert_eq!(damage(&e), vec![3500]);
}
#[test]
fn pet_battle_undead_and_mechanical_revive_once() {
    let c = catalog();
    for family in [3, 9] {
        let mut b = battle(&c, 7, family, [110, 119, 123]);
        b.teams[1].pets[0].health = 1;
        let (mut b, e) = b
            .step(&c, [Action::Ability(110), Action::Pass], &mut Fixed(500000))
            .unwrap();
        assert!(e.contains(&Event::Revived(PetRef { team: 1, slot: 0 })));
        assert_eq!(
            b.teams[1].pets[0].health,
            if family == 3 { 1 } else { 2000 }
        );
        if family == 3 {
            let (next, e) = b
                .step(&c, [Action::Pass, Action::Ability(110)], &mut Fixed(500000))
                .unwrap();
            // Undead75% outgoing damage120*.75=90; dies after its immortal round.
            assert_eq!(damage(&e), vec![90]);
            assert_eq!(next.teams[1].pets[0].health, 0);
        } else {
            b.teams[1].pets[0].health = 1;
            let (next, _) = b
                .step(&c, [Action::Ability(110), Action::Pass], &mut Fixed(500000))
                .unwrap();
            assert_eq!(next.teams[1].pets[0].health, 0);
        }
    }
}
#[test]
fn pet_battle_burrow_multiturn_unaura_and_cooldown() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [159, 110, 123]);
    b.teams[0].pets[0].stats.speed = 200;
    let (b, e) = b
        .step(
            &c,
            [Action::Ability(159), Action::Ability(110)],
            &mut Fixed(500000),
        )
        .unwrap();
    assert!(damage(&e).is_empty());
    assert_eq!(b.teams[0].pets[0].continuation, Some((159, 2)));
    let (b, e) = b
        .step(&c, [Action::Pass, Action::Ability(110)], &mut Fixed(500000))
        .unwrap();
    assert_eq!(damage(&e), vec![180, 120]);
    assert!(b.auras.is_empty());
    assert_eq!(b.teams[0].pets[0].continuation, None);
    assert!(
        b.step(&c, [Action::Ability(159), Action::Pass], &mut Fixed(500000))
            .is_err()
    );
}
#[test]
fn pet_battle_heal_modifiers_and_cooldown_expiry() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [123, 110, 119]);
    b.teams[0].pets[0].health = 1000;
    // Healing Wave30*6=180; dealt+50%, taken-50% =>135.
    b.teams[0].pets[0]
        .states
        .insert(c.state_id("Mod_HealingDealtPercent").unwrap(), 50);
    b.teams[0].pets[0]
        .states
        .insert(c.state_id("Mod_HealingTakenPercent").unwrap(), -50);
    let (mut b, e) = b
        .step(&c, [Action::Ability(123), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert!(e.contains(&Event::Heal {
        target: PetRef { team: 0, slot: 0 },
        amount: 135
    }));
    assert!(
        b.step(&c, [Action::Ability(123), Action::Pass], &mut Fixed(500000))
            .is_err()
    );
    for _ in 0..2 {
        b = b.step(&c, [Action::Pass; 2], &mut Fixed(500000)).unwrap().0;
    }
    assert!(
        b.step(&c, [Action::Ability(123), Action::Pass], &mut Fixed(500000))
            .is_ok()
    );
}
#[test]
fn pet_battle_unsupported_effect_rejects_before_rng() {
    let c = catalog();
    let b = battle(&c, 7, 7, [70, 110, 119]);
    // Replace a real row in an isolated fixture to exercise fail-closed dispatch.
    let mut c = c;
    c.effects.get_mut(&209).unwrap().property = 0;
    let result = b.step(&c, [Action::Ability(70), Action::Pass], &mut Fixed(500000));
    assert!(result.unwrap_err().contains("property 0"));
}

#[test]
fn pet_battle_curse_of_doom_waits_four_rounds_real_id218() {
    let c = catalog();
    let b = battle(&c, 3, 7, [218, 110, 123]);
    // aura217 has dummy turns1..4 then40points*6=240 onturn5.
    let (mut b, e) = b
        .step(&c, [Action::Ability(218), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert!(damage(&e).is_empty());
    for _ in 0..3 {
        let (next, e) = b.step(&c, [Action::Pass; 2], &mut Fixed(500000)).unwrap();
        assert!(damage(&e).is_empty());
        b = next;
    }
    let (b, e) = b.step(&c, [Action::Pass; 2], &mut Fixed(500000)).unwrap();
    assert_eq!(damage(&e), vec![240]);
    assert!(b.auras.is_empty());
}
#[test]
fn pet_battle_wish_heals_current_active_after_swap_real_id273() {
    let c = catalog();
    let mut b = battle(&c, 5, 7, [273, 110, 123]);
    b.teams[0].pets[0].health = 1000;
    b.teams[0].pets[1].health = 1000;
    // property63 applies aura274:dummy turn1, percent50 healturn2.
    let (b, e) = b
        .step(&c, [Action::Ability(273), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert!(!e.iter().any(|e| matches!(e, Event::Heal { .. })));
    let (b, e) = b
        .step(&c, [Action::Swap(1), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert!(e.contains(&Event::Heal {
        target: PetRef { team: 0, slot: 1 },
        amount: 5000
    }));
    assert_eq!(b.teams[0].pets[0].health, 1000);
    assert_eq!(b.teams[0].pets[1].health, 6000);
}
#[test]
fn pet_battle_fainted_replacement_does_not_advance_combat() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [110, 119, 123]);
    b.teams[0].pets[0].health = 0;
    let (next, e) = b
        .step(&c, [Action::Swap(1), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert_eq!(next.round, b.round);
    assert_eq!(e, vec![Event::Swapped { team: 0, slot: 1 }]);
}
#[test]
fn pet_battle_unimplemented_object_aura_is_rejected() {
    let c = catalog();
    // Cyclone190 is supported; Volcano176/175 still has undetermined object semantics.
    let b = battle(&c, 2, 7, [176, 110, 123]);
    let err = b
        .step(&c, [Action::Ability(176), Action::Pass], &mut NoDraw)
        .unwrap_err();
    assert!(err.contains("object"));
}
#[test]
fn pet_battle_deep_breath_state_and_humanoid_recovery() {
    let c = catalog();
    let b = battle(&c, 0, 7, [169, 110, 123]);
    let (b, e) = b
        .step(&c, [Action::Ability(169), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert!(damage(&e).is_empty());
    let mut b = b;
    b.teams[0].pets[0].health = 1000;
    let (b, e) = b.step(&c, [Action::Pass; 2], &mut Fixed(500000)).unwrap();
    assert_eq!(damage(&e), vec![300]);
    assert_eq!(b.teams[0].pets[0].health, 1400);
}

struct NoDraw;
impl BattleRng for NoDraw {
    fn draw(&mut self) -> u32 {
        panic!("rejected action consumed RNG")
    }
}
#[test]
fn pet_battle_rejected_action_preserves_rng() {
    let c = catalog();
    let b = battle(&c, 7, 7, [110, 119, 123]);
    assert!(
        b.step(&c, [Action::Ability(2551), Action::Pass], &mut NoDraw)
            .is_err()
    );
    assert!(
        b.step(&c, [Action::Swap(9), Action::Pass], &mut NoDraw)
            .is_err()
    );
    assert_eq!(
        b.step(&c, [Action::Forfeit, Action::Pass], &mut NoDraw)
            .unwrap()
            .0
            .outcome,
        Some(Outcome::Winner(1))
    );
}
#[test]
fn pet_battle_resilient_does_not_block_root_real_grasp249() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [249, 110, 123]);
    b.teams[1].pets[0]
        .states
        .insert(c.state_id("resilient").unwrap(), 1);
    let (b, e) = b
        .step(&c, [Action::Ability(249), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert!(!e.iter().any(|e| matches!(e, Event::Immune { .. })));
    assert_eq!(
        b.teams[1].pets[0].state(c.state_id("swapOutLock").unwrap()),
        1
    );
    assert!(
        b.step(&c, [Action::Pass, Action::Swap(1)], &mut NoDraw)
            .is_err()
    );
}
#[test]
fn pet_battle_aquatic_dot_and_dragon_next_round() {
    let c = catalog();
    let mut b = battle(&c, 8, 8, [152, 110, 123]);
    // Both aquatic passives: periodic30*1.25*.5=18.75 =>18; direct60 unaffected.
    let (next, e) = b
        .step(&c, [Action::Ability(152), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert_eq!(damage(&e), vec![60, 18]);
    assert_eq!(next.teams[1].pets[0].health, 9922);
    b = battle(&c, 1, 7, [110, 119, 123]);
    b.teams[1].pets[0].health = 5100;
    let (b, e) = b
        .step(&c, [Action::Ability(110), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert_eq!(damage(&e), vec![120]);
    let (b, e) = b
        .step(&c, [Action::Ability(110), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert_eq!(damage(&e), vec![180]);
    let (_, e) = b
        .step(&c, [Action::Ability(110), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert_eq!(damage(&e), vec![120]);
}

#[test]
fn pet_battle_missed_poison_chain_and_aura_refresh() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [152, 194, 110]);
    b.teams[0].pets[0]
        .states
        .insert(c.state_id("Stat_Accuracy").unwrap(), -100);
    let (b, e) = b
        .step(&c, [Action::Ability(152), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert!(damage(&e).is_empty());
    assert!(b.auras.is_empty());
    let mut b = b;
    b.teams[0].pets[0]
        .states
        .insert(c.state_id("Stat_Accuracy").unwrap(), 0);
    let (b, _) = b
        .step(&c, [Action::Ability(194), Action::Pass], &mut Fixed(500000))
        .unwrap();
    let (b, _) = b
        .step(&c, [Action::Ability(194), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert_eq!(
        b.teams[0].pets[0].state(c.state_id("Mod_SpeedPercent").unwrap()),
        100
    );
    assert_eq!(b.auras.len(), 1);
    let (b, _) = b.step(&c, [Action::Pass; 2], &mut Fixed(500000)).unwrap();
    assert_eq!(
        b.teams[0].pets[0].state(c.state_id("Mod_SpeedPercent").unwrap()),
        0
    );
    assert!(b.auras.is_empty());
}
#[test]
fn pet_battle_defeat_cancels_fainted_pet_attack() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [110, 119, 123]);
    b.teams[0].pets[0].stats.speed = 200;
    for pet in &mut b.teams[1].pets {
        pet.health = 0;
    }
    b.teams[1].pets[0].health = 1;
    let (b, e) = b
        .step(&c, [Action::Ability(110); 2], &mut Fixed(500000))
        .unwrap();
    assert_eq!(b.outcome, Some(Outcome::Winner(0)));
    assert_eq!(b.teams[0].pets[0].health, 10000);
    assert_eq!(
        e.iter().filter(|e| matches!(e, Event::Used { .. })).count(),
        1
    );
    assert!(b.step(&c, [Action::Pass; 2], &mut NoDraw).is_err());
}
#[test]
fn pet_battle_catalog_inventory_and_admission() {
    let c = catalog();
    assert_eq!(c.effects.len(), 2829);
    assert_eq!(c.abilities.len(), 1404);
    assert_eq!(c.turns.len(), 1650);
    // Undefined metadata is retained, not silently interpreted as stats/opcodes.
    let values: Vec<_> = c
        .unresolved_states
        .iter()
        .filter(|r| r.owner == 2999 && r.state == 183)
        .map(|r| r.value)
        .collect();
    assert_eq!(values, vec![2343, 1303]);
    // Identical state20=50 rows are not summed: (1700+50)*.65*25/100=>284.
    assert_eq!(calculate_stats(&c, 4597, 3, 3, 25).unwrap().speed, 284);
    let admitted: std::collections::BTreeSet<_> = c
        .abilities
        .keys()
        .copied()
        .filter(|id| validate_ability(&c, *id).is_ok())
        .collect();
    let coverage:Vec<_>=c.property_counts().into_iter().map(|(property,rows)| {
        let admitted_rows=c.effects.values().filter(|e|e.property==property && admitted.contains(&c.turns[&e.turn].ability)).count();
        serde_json::json!({"property":property,"rows":rows,"handler":supported_property(property),"admitted_rows":admitted_rows})
    }).collect();
    let admission: Vec<_> = c
        .abilities
        .keys()
        .map(|id| serde_json::json!({"ability": id, "error": validate_ability(&c, *id).err()}))
        .collect();
    println!(
        "PET_BATTLE_COVERAGE={}",
        serde_json::json!({"properties":coverage,"admitted_abilities":admitted.len(),"total_abilities":c.abilities.len(),"unresolved_states":c.unresolved_states.len(),"admission":admission})
    );
}
