use super::*;
use rusqlite::Connection;
use std::path::PathBuf;

fn csv_records(input: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => row.push(std::mem::take(&mut field)),
            '\n' if !quoted => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            '\r' if !quoted => (),
            _ => field.push(ch),
        }
    }
    assert!(!quoted, "unterminated CSV string");
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

fn fixture_db() -> Connection {
    let root = std::env::var_os("PET_BATTLE_DB2")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933")
        });
    let db = Connection::open_in_memory().unwrap();
    for (export, table) in [
        ("BattlePetAbility", "battle_pet_ability"),
        ("BattlePetAbilityTurn", "battle_pet_ability_turn"),
        ("BattlePetAbilityEffect", "battle_pet_ability_effect"),
        ("BattlePetEffectProperties", "battle_pet_effect_properties"),
        ("BattlePetState", "battle_pet_state"),
        ("BattlePetSpeciesState", "battle_pet_species_state"),
        ("BattlePetBreedState", "battle_pet_breed_state"),
        ("BattlePetBreedQuality", "battle_pet_breed_quality"),
        ("BattlePetSpeciesXAbility", "battle_pet_species_x_ability"),
        ("BattlePetAbilityState", "battle_pet_ability_state"),
    ] {
        let path = root.join(format!("{export}.csv"));
        let input =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let rows = csv_records(&input);
        let headers = &rows[0];
        let columns = headers
            .iter()
            .map(|h| {
                format!(
                    "\"{h}\" {} NOT NULL",
                    if h.contains("lang") || h.contains("Label") || h == "LuaName" {
                        "TEXT"
                    } else if h == "StateMultiplier" {
                        "REAL"
                    } else {
                        "INTEGER"
                    }
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        db.execute_batch(&format!("CREATE TABLE {table} ({columns});"))
            .unwrap();
        let placeholders = vec!["?"; headers.len()].join(",");
        let mut insert = db
            .prepare(&format!("INSERT INTO {table} VALUES ({placeholders})"))
            .unwrap();
        for row in &rows[1..] {
            assert_eq!(row.len(), headers.len());
            insert.execute(rusqlite::params_from_iter(row)).unwrap();
        }
    }
    db
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
    let caster = Pet::new(catalog, stats, family, abilities, States::new()).unwrap();
    let target = Pet::new(
        catalog,
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
    c.effects.get_mut(&209).unwrap().property = 385;
    let result = b.step(&c, [Action::Ability(70), Action::Pass], &mut Fixed(500000));
    assert!(result.unwrap_err().contains("385"));
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
    let b = battle(&c, 2, 7, [190, 110, 123]);
    let err = b
        .step(&c, [Action::Ability(190), Action::Pass], &mut Fixed(500000))
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
