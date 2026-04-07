use super::*;

// -- Journal tests --

#[test]
fn quality_from_id() {
    assert_eq!(PetQuality::from_id(0), Some(PetQuality::Poor));
    assert_eq!(PetQuality::from_id(1), Some(PetQuality::Common));
    assert_eq!(PetQuality::from_id(2), Some(PetQuality::Uncommon));
    assert_eq!(PetQuality::from_id(3), Some(PetQuality::Rare));
    assert_eq!(PetQuality::from_id(4), Some(PetQuality::Epic));
    assert_eq!(PetQuality::from_id(5), Some(PetQuality::Legendary));
    assert_eq!(PetQuality::from_id(6), None);
}

#[test]
fn add_pet() {
    let mut journal = PetJournal::default();
    let id = journal.add(1, 5, PetQuality::Common).unwrap();
    assert_eq!(journal.count(), 1);
    let pet = journal.get(id).unwrap();
    assert_eq!(pet.species_id, 1);
    assert_eq!(pet.level, 5);
    assert_eq!(pet.quality, PetQuality::Common);
}

#[test]
fn add_pet_clamps_level() {
    let mut journal = PetJournal::default();
    let id = journal.add(1, 99, PetQuality::Common).unwrap();
    let pet = journal.get(id).unwrap();
    assert_eq!(pet.level, MAX_PET_LEVEL);
}

#[test]
fn add_pet_unique_ids() {
    let mut journal = PetJournal::default();
    let id1 = journal.add(1, 1, PetQuality::Common).unwrap();
    let id2 = journal.add(2, 1, PetQuality::Common).unwrap();
    assert_ne!(id1, id2);
}

#[test]
fn species_limit() {
    let mut journal = PetJournal::default();
    journal.add(42, 1, PetQuality::Common).unwrap();
    journal.add(42, 1, PetQuality::Common).unwrap();
    journal.add(42, 1, PetQuality::Common).unwrap();
    let result = journal.add(42, 1, PetQuality::Common);
    assert_eq!(result, Err(PetJournalError::SpeciesLimitReached));
}

#[test]
fn remove_pet() {
    let mut journal = PetJournal::default();
    let id = journal.add(1, 1, PetQuality::Common).unwrap();
    journal.remove(id).unwrap();
    assert_eq!(journal.count(), 0);
    assert!(journal.get(id).is_none());
}

#[test]
fn remove_nonexistent() {
    let mut journal = PetJournal::default();
    let result = journal.remove(999);
    assert_eq!(result, Err(PetJournalError::PetNotFound));
}

#[test]
fn rename_pet() {
    let mut journal = PetJournal::default();
    let id = journal.add(1, 1, PetQuality::Common).unwrap();
    journal.rename(id, "Fluffy".into()).unwrap();
    let pet = journal.get(id).unwrap();
    assert_eq!(pet.custom_name, Some("Fluffy".into()));
}

#[test]
fn rename_empty_fails() {
    let mut journal = PetJournal::default();
    let id = journal.add(1, 1, PetQuality::Common).unwrap();
    let result = journal.rename(id, "".into());
    assert_eq!(result, Err(PetJournalError::EmptyName));
}

#[test]
fn rename_nonexistent() {
    let mut journal = PetJournal::default();
    let result = journal.rename(999, "Name".into());
    assert_eq!(result, Err(PetJournalError::PetNotFound));
}

#[test]
fn unique_species_count() {
    let mut journal = PetJournal::default();
    journal.add(1, 1, PetQuality::Common).unwrap();
    journal.add(1, 1, PetQuality::Common).unwrap();
    journal.add(2, 1, PetQuality::Common).unwrap();
    assert_eq!(journal.unique_species(), 2);
}

#[test]
fn by_species() {
    let mut journal = PetJournal::default();
    journal.add(1, 1, PetQuality::Common).unwrap();
    journal.add(1, 2, PetQuality::Rare).unwrap();
    journal.add(2, 1, PetQuality::Common).unwrap();
    let pets = journal.by_species(1);
    assert_eq!(pets.len(), 2);
    assert!(pets.iter().all(|p| p.species_id == 1));
}

#[test]
fn highest_level_empty() {
    let journal = PetJournal::default();
    assert_eq!(journal.highest_level(), 0);
}

#[test]
fn highest_level() {
    let mut journal = PetJournal::default();
    journal.add(1, 5, PetQuality::Common).unwrap();
    journal.add(2, 20, PetQuality::Common).unwrap();
    journal.add(3, 12, PetQuality::Common).unwrap();
    assert_eq!(journal.highest_level(), 20);
}

#[test]
fn serialization_round_trip() {
    let mut journal = PetJournal::default();
    let id = journal.add(7, 15, PetQuality::Rare).unwrap();
    journal.rename(id, "Sparky".into()).unwrap();
    let json = serde_json::to_string(&journal).unwrap();
    let restored: PetJournal = serde_json::from_str(&json).unwrap();
    assert_eq!(journal, restored);
}

#[test]
fn remove_frees_species_slot() {
    let mut journal = PetJournal::default();
    let id1 = journal.add(42, 1, PetQuality::Common).unwrap();
    journal.add(42, 1, PetQuality::Common).unwrap();
    journal.add(42, 1, PetQuality::Common).unwrap();
    journal.remove(id1).unwrap();
    // Should succeed now that one slot is freed.
    journal.add(42, 1, PetQuality::Common).unwrap();
    assert_eq!(journal.count_species(42), 3);
}

// -- Stat tests --

fn test_species() -> PetSpecies {
    PetSpecies {
        id: 1,
        family: PetFamily::Beast,
        base_health: 100.0,
        base_power: 20.0,
        base_speed: 20.0,
    }
}

#[test]
fn level_multiplier_at_level_1() {
    let stats = compute_stats(&test_species(), 1, PetQuality::Common);
    // multiplier = 1.0, quality = 1.0, health bonus = 5
    // health = round(100 * 1.0 * 1.0) + 5 = 105
    // power  = round(20  * 1.0 * 1.0)     = 20
    assert_eq!(stats.health, 105);
    assert_eq!(stats.power, 20);
}

#[test]
fn level_multiplier_at_level_25() {
    let stats = compute_stats(&test_species(), 25, PetQuality::Common);
    // multiplier = 1.0 + 24*(4/24) = 5.0, quality = 1.0, health bonus = 125
    // health = round(100 * 5.0 * 1.0) + 125 = 625
    // power  = round(20  * 5.0 * 1.0)       = 100
    assert_eq!(stats.health, 625);
    assert_eq!(stats.power, 100);
    assert_eq!(stats.speed, 100);
}

#[test]
fn level_multiplier_mid_level() {
    let stats = compute_stats(&test_species(), 13, PetQuality::Common);
    // multiplier = 1.0 + 12*(4/24) = 3.0, health bonus = 65
    // health = round(100 * 3.0 * 1.0) + 65 = 365
    assert_eq!(stats.health, 365);
}

#[test]
fn stats_level1_common() {
    let stats = compute_stats(&test_species(), 1, PetQuality::Common);
    assert_eq!(stats.health, 105);
    assert_eq!(stats.power, 20);
    assert_eq!(stats.speed, 20);
}

#[test]
fn stats_level25_common() {
    let stats = compute_stats(&test_species(), 25, PetQuality::Common);
    assert_eq!(stats.health, 625);
    assert_eq!(stats.power, 100);
    assert_eq!(stats.speed, 100);
}

#[test]
fn stats_rare_better_than_common() {
    let common = compute_stats(&test_species(), 10, PetQuality::Common);
    let rare = compute_stats(&test_species(), 10, PetQuality::Rare);
    assert!(rare.health > common.health);
    assert!(rare.power > common.power);
    assert!(rare.speed > common.speed);
}

#[test]
fn stats_higher_level_better() {
    let low = compute_stats(&test_species(), 5, PetQuality::Common);
    let high = compute_stats(&test_species(), 20, PetQuality::Common);
    assert!(high.health > low.health);
    assert!(high.power > low.power);
    assert!(high.speed > low.speed);
}

#[test]
fn stats_quality_ordering() {
    let species = test_species();
    let level = 10;
    let poor = compute_stats(&species, level, PetQuality::Poor);
    let common = compute_stats(&species, level, PetQuality::Common);
    let uncommon = compute_stats(&species, level, PetQuality::Uncommon);
    let rare = compute_stats(&species, level, PetQuality::Rare);
    let epic = compute_stats(&species, level, PetQuality::Epic);
    let legendary = compute_stats(&species, level, PetQuality::Legendary);
    assert!(poor.power < common.power);
    assert!(common.power < uncommon.power);
    assert!(uncommon.power < rare.power);
    assert!(rare.power < epic.power);
    assert!(epic.power < legendary.power);
}

#[test]
fn stats_poor_quality_penalty() {
    let common = compute_stats(&test_species(), 10, PetQuality::Common);
    let poor = compute_stats(&test_species(), 10, PetQuality::Poor);
    assert!(poor.power < common.power);
    assert!(poor.speed < common.speed);
}

#[test]
fn stats_level25_rare_concrete() {
    let stats = compute_stats(&test_species(), 25, PetQuality::Rare);
    // multiplier = 5.0, quality = 1.2, health bonus = 125
    // health = round(100 * 5.0 * 1.2) + 125 = 600 + 125 = 725
    // power  = round(20  * 5.0 * 1.2)       = 120
    // speed  = round(20  * 5.0 * 1.2)       = 120
    assert_eq!(stats.health, 725);
    assert_eq!(stats.power, 120);
    assert_eq!(stats.speed, 120);
}

// -- Battle system tests --

fn make_pet(health: u32, power: u32, speed: u32) -> BattlePet {
    BattlePet::new(PetStats {
        health,
        power,
        speed,
    })
}

#[test]
fn battle_starts_in_select_action() {
    let battle = PetBattle::new(
        vec![make_pet(100, 10, 10)],
        vec![make_pet(100, 10, 10)],
    );
    assert_eq!(battle.phase, BattlePhase::SelectAction);
    assert_eq!(battle.turn, 1);
}

#[test]
fn faster_pet_acts_first() {
    // Player speed=100, opponent speed=50 → player acts first.
    let player_pet = make_pet(200, 30, 100);
    let opp_pet = make_pet(200, 10, 50);
    let opp_initial_health = opp_pet.stats.health;
    let mut battle = PetBattle::new(vec![player_pet], vec![opp_pet]);
    battle.submit_action(BattleSide::Player, TurnAction::UseAbility(0));
    battle.submit_action(BattleSide::Opponent, TurnAction::UseAbility(0));
    // Opponent should have taken 30 damage from the faster player pet.
    assert_eq!(
        battle.opponent.active_pet().current_health,
        opp_initial_health - 30
    );
}

#[test]
fn slower_pet_acts_second() {
    // Opponent speed=100, player speed=50 → opponent acts first.
    let player_pet = make_pet(200, 10, 50);
    let opp_pet = make_pet(200, 30, 100);
    let player_initial_health = player_pet.stats.health;
    let mut battle = PetBattle::new(vec![player_pet], vec![opp_pet]);
    battle.submit_action(BattleSide::Player, TurnAction::UseAbility(0));
    battle.submit_action(BattleSide::Opponent, TurnAction::UseAbility(0));
    // Player should have taken 30 damage from the faster opponent pet.
    assert_eq!(
        battle.player.active_pet().current_health,
        player_initial_health - 30
    );
}

#[test]
fn pet_faints_and_auto_swaps() {
    // pet0 has only 10 hp, opponent has power=50 → pet0 faints; auto-swap to pet1.
    let pets = vec![
        make_pet(10, 5, 10),
        make_pet(500, 5, 10),
        make_pet(500, 5, 10),
    ];
    let opp_pets = vec![make_pet(500, 50, 5)];
    let mut battle = PetBattle::new(pets, opp_pets);
    // Player acts first (speed 10 > 5), then opponent kills pet0.
    battle.submit_action(BattleSide::Player, TurnAction::UseAbility(0));
    battle.submit_action(BattleSide::Opponent, TurnAction::UseAbility(0));
    // pet0 fainted; should have swapped to pet1 (index 1).
    assert_eq!(battle.player.active, 1);
    assert_eq!(battle.phase, BattlePhase::SelectAction);
}

#[test]
fn all_pets_faint_ends_battle() {
    let player_pets = vec![make_pet(1, 5, 5)];
    let opp_pets = vec![make_pet(500, 50, 10)];
    let mut battle = PetBattle::new(player_pets, opp_pets);
    // Opponent is faster (speed 10 > 5), kills player's only pet.
    battle.submit_action(BattleSide::Player, TurnAction::Pass);
    battle.submit_action(BattleSide::Opponent, TurnAction::UseAbility(0));
    assert_eq!(battle.phase, BattlePhase::Ended);
    assert_eq!(battle.outcome, Some(BattleOutcome::OpponentWins));
}

#[test]
fn swap_action_changes_active() {
    let pets = vec![
        make_pet(200, 10, 10),
        make_pet(200, 10, 10),
        make_pet(200, 10, 10),
    ];
    let opp_pets = vec![make_pet(200, 10, 10)];
    let mut battle = PetBattle::new(pets, opp_pets);
    battle.submit_action(BattleSide::Player, TurnAction::Swap(1));
    battle.submit_action(BattleSide::Opponent, TurnAction::Pass);
    assert_eq!(battle.player.active, 1);
}

#[test]
fn pass_does_nothing() {
    let player_pet = make_pet(200, 10, 10);
    let opp_pet = make_pet(200, 10, 10);
    let p_health = player_pet.stats.health;
    let o_health = opp_pet.stats.health;
    let mut battle = PetBattle::new(vec![player_pet], vec![opp_pet]);
    let events = battle.submit_action(BattleSide::Player, TurnAction::Pass);
    // Only player submitted; no events yet.
    assert!(events.is_empty());
    let events = battle.submit_action(BattleSide::Opponent, TurnAction::Pass);
    // Both passed — no damage events.
    assert!(events.is_empty());
    assert_eq!(battle.player.active_pet().current_health, p_health);
    assert_eq!(battle.opponent.active_pet().current_health, o_health);
    assert_eq!(battle.turn, 2);
}

#[test]
fn turn_counter_increments() {
    let mut battle = PetBattle::new(
        vec![make_pet(500, 5, 5)],
        vec![make_pet(500, 5, 5)],
    );
    for _ in 0..3 {
        battle.submit_action(BattleSide::Player, TurnAction::Pass);
        battle.submit_action(BattleSide::Opponent, TurnAction::Pass);
    }
    assert_eq!(battle.turn, 4);
}

#[test]
fn submit_after_ended_returns_empty() {
    // End the battle by having opponent kill player's only pet.
    let player_pets = vec![make_pet(1, 5, 5)];
    let opp_pets = vec![make_pet(500, 50, 10)];
    let mut battle = PetBattle::new(player_pets, opp_pets);
    battle.submit_action(BattleSide::Player, TurnAction::Pass);
    battle.submit_action(BattleSide::Opponent, TurnAction::UseAbility(0));
    assert_eq!(battle.phase, BattlePhase::Ended);
    // Any further submission should return empty.
    let events = battle.submit_action(BattleSide::Player, TurnAction::UseAbility(0));
    assert!(events.is_empty());
}

#[test]
fn speed_tie_favors_player() {
    // Equal speed → player_first = true → player acts first.
    let player_pet = make_pet(200, 30, 50);
    let opp_pet = make_pet(200, 10, 50);
    let opp_initial_health = opp_pet.stats.health;
    let mut battle = PetBattle::new(vec![player_pet], vec![opp_pet]);
    battle.submit_action(BattleSide::Player, TurnAction::UseAbility(0));
    battle.submit_action(BattleSide::Opponent, TurnAction::UseAbility(0));
    // Player dealt damage first: opponent lost 30 hp.
    assert_eq!(
        battle.opponent.active_pet().current_health,
        opp_initial_health - 30
    );
}

// -- Type effectiveness tests --

#[test]
fn type_strong_is_1_5() {
    assert!((type_effectiveness(PetFamily::Beast, PetFamily::Critter) - 1.5).abs() < 0.01);
}

#[test]
fn type_weak_is_0_67() {
    assert!((type_effectiveness(PetFamily::Beast, PetFamily::Flying) - 0.67).abs() < 0.01);
}

#[test]
fn type_neutral_is_1_0() {
    assert!((type_effectiveness(PetFamily::Beast, PetFamily::Beast) - 1.0).abs() < 0.01);
}

#[test]
fn all_families_have_one_strong_and_one_weak() {
    let families = [
        PetFamily::Aquatic,
        PetFamily::Beast,
        PetFamily::Critter,
        PetFamily::Dragonkin,
        PetFamily::Elemental,
        PetFamily::Flying,
        PetFamily::Humanoid,
        PetFamily::Magic,
        PetFamily::Mechanical,
        PetFamily::Undead,
    ];
    for &attacker in &families {
        let strong_count = families
            .iter()
            .filter(|&&def| type_effectiveness(attacker, def) > 1.0)
            .count();
        let weak_count = families
            .iter()
            .filter(|&&def| type_effectiveness(attacker, def) < 1.0)
            .count();
        assert_eq!(strong_count, 1, "{attacker:?} should have 1 strong matchup");
        assert_eq!(weak_count, 1, "{attacker:?} should have 1 weak matchup");
    }
}

// -- Ability tests --

fn test_ability(family: PetFamily, base_damage: u32) -> PetAbility {
    PetAbility {
        id: 1,
        name: "Test".into(),
        base_damage,
        family,
        cooldown: 0,
    }
}

#[test]
fn ability_damage_neutral() {
    let ability = test_ability(PetFamily::Beast, 20);
    assert_eq!(ability_damage(&ability, 50, PetFamily::Beast), 70);
}

#[test]
fn ability_damage_strong() {
    let ability = test_ability(PetFamily::Beast, 20);
    assert_eq!(ability_damage(&ability, 50, PetFamily::Critter), 105);
}

#[test]
fn ability_damage_weak() {
    let ability = test_ability(PetFamily::Beast, 20);
    assert_eq!(ability_damage(&ability, 50, PetFamily::Flying), 47);
}

#[test]
fn ability_loadout_active_slots() {
    let a0 = test_ability(PetFamily::Beast, 10);
    let a1 = test_ability(PetFamily::Aquatic, 15);
    let a2 = test_ability(PetFamily::Flying, 20);
    let a3 = test_ability(PetFamily::Critter, 25);
    let loadout = AbilityLoadout {
        known: vec![a0, a1, a2, a3],
        active_slots: [0, 2, 3],
    };
    assert_eq!(loadout.active_ability(0).unwrap().base_damage, 10);
    assert_eq!(loadout.active_ability(1).unwrap().base_damage, 20);
    assert_eq!(loadout.active_ability(2).unwrap().base_damage, 25);
    assert!(loadout.active_ability(3).is_none());
}

#[test]
fn battle_with_type_effectiveness() {
    let beast_ability = test_ability(PetFamily::Beast, 20);
    let loadout = AbilityLoadout {
        known: vec![beast_ability],
        active_slots: [0, 0, 0],
    };

    let attacker = BattlePet::with_family(
        PetStats { health: 500, power: 50, speed: 100 },
        PetFamily::Beast,
        loadout,
    );
    let defender = BattlePet::with_family(
        PetStats { health: 500, power: 50, speed: 50 },
        PetFamily::Critter,
        AbilityLoadout::default(),
    );

    let mut battle = PetBattle::new(vec![attacker], vec![defender]);
    battle.submit_action(BattleSide::Player, TurnAction::UseAbility(0));
    let events = battle.submit_action(BattleSide::Opponent, TurnAction::UseAbility(0));
    // Beast ability vs Critter: (20 + 50) * 1.5 = 105
    let player_event = events.iter().find(|e| e.side == BattleSide::Player).unwrap();
    assert_eq!(player_event.damage, 105);
}
