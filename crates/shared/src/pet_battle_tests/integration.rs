use super::*;

#[test]
fn stats_from_species_through_battle_to_xp() {
    let species = PetSpecies {
        id: 42,
        family: PetFamily::Critter,
        base_health: 80.0,
        base_power: 18.0,
        base_speed: 22.0,
    };
    let stats = compute_stats(&species, 10, PetQuality::Rare);
    assert!(stats.health > 0);
    assert!(stats.power > 0);
    assert!(stats.speed > 0);

    let player_pet = BattlePet::with_family(stats, PetFamily::Critter, AbilityLoadout::default());
    let opp_pet = BattlePet::new(PetStats {
        health: 50,
        power: 5,
        speed: 10,
    });

    let mut battle = PetBattle::new(vec![player_pet], vec![opp_pet]);
    while battle.phase != BattlePhase::Ended {
        battle.submit_action(BattleSide::Player, TurnAction::UseAbility(0));
        battle.submit_action(BattleSide::Opponent, TurnAction::UseAbility(0));
    }
    assert_eq!(battle.outcome, Some(BattleOutcome::PlayerWins));

    let mut owned = OwnedPet {
        id: 1,
        species_id: 42,
        custom_name: None,
        level: 10,
        quality: PetQuality::Rare,
        xp: 0,
    };
    let xp = battle_xp(owned.level, 10);
    let result = award_xp(&mut owned, xp);
    assert!(result.xp_gained > 0);
}

#[test]
fn type_effectiveness_in_full_battle() {
    let beast_ability = PetAbility {
        id: 1,
        name: "Bite".into(),
        base_damage: 20,
        family: PetFamily::Beast,
        cooldown: 0,
    };
    let loadout = AbilityLoadout {
        known: vec![beast_ability],
        active_slots: [0, 0, 0],
    };
    let attacker = BattlePet::with_family(
        PetStats {
            health: 500,
            power: 50,
            speed: 100,
        },
        PetFamily::Beast,
        loadout,
    );
    let defender = BattlePet::with_family(
        PetStats {
            health: 1000,
            power: 10,
            speed: 50,
        },
        PetFamily::Critter,
        AbilityLoadout::default(),
    );

    let mut battle = PetBattle::new(vec![attacker], vec![defender]);
    battle.submit_action(BattleSide::Player, TurnAction::UseAbility(0));
    let events = battle.submit_action(BattleSide::Opponent, TurnAction::UseAbility(0));

    let player_event = events
        .iter()
        .find(|e| e.side == BattleSide::Player)
        .unwrap();
    assert_eq!(player_event.damage, 105);

    let opp_event = events
        .iter()
        .find(|e| e.side == BattleSide::Opponent)
        .unwrap();
    assert_eq!(opp_event.damage, 10);
}

#[test]
fn turn_order_faster_kills_before_slower_acts() {
    let fast = BattlePet::new(PetStats {
        health: 100,
        power: 200,
        speed: 100,
    });
    let slow = BattlePet::new(PetStats {
        health: 100,
        power: 200,
        speed: 50,
    });

    let mut battle = PetBattle::new(vec![fast], vec![slow]);
    battle.submit_action(BattleSide::Player, TurnAction::UseAbility(0));
    let events = battle.submit_action(BattleSide::Opponent, TurnAction::UseAbility(0));

    assert_eq!(battle.outcome, Some(BattleOutcome::PlayerWins));
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].side, BattleSide::Player);
    assert!(events[0].target_fainted);
}

#[test]
fn capture_after_weakening_in_wild_battle() {
    let player_pet = BattlePet::new(PetStats {
        health: 500,
        power: 40,
        speed: 100,
    });
    let wild_pet = BattlePet::new(PetStats {
        health: 100,
        power: 10,
        speed: 50,
    });

    let mut battle = PetBattle::new_wild(vec![player_pet], vec![wild_pet]);
    battle.submit_action(BattleSide::Player, TurnAction::UseAbility(0));
    battle.submit_action(BattleSide::Opponent, TurnAction::UseAbility(0));

    let wild_health = battle.opponent.active_pet().current_health;
    assert!(wild_health < 100, "wild pet should be damaged");

    let chance = capture_chance(wild_health, 100);
    let result = attempt_capture(&mut battle, chance - 0.01);
    assert_eq!(result, CaptureResult::Captured);
    assert_eq!(battle.phase, BattlePhase::Ended);
}

#[test]
fn full_3v3_battle_with_swaps() {
    let p1 = BattlePet::new(PetStats {
        health: 50,
        power: 30,
        speed: 100,
    });
    let p2 = BattlePet::new(PetStats {
        health: 200,
        power: 30,
        speed: 100,
    });
    let p3 = BattlePet::new(PetStats {
        health: 200,
        power: 30,
        speed: 100,
    });

    let o1 = BattlePet::new(PetStats {
        health: 100,
        power: 10,
        speed: 50,
    });
    let o2 = BattlePet::new(PetStats {
        health: 100,
        power: 10,
        speed: 50,
    });
    let o3 = BattlePet::new(PetStats {
        health: 100,
        power: 10,
        speed: 50,
    });

    let mut battle = PetBattle::new(vec![p1, p2, p3], vec![o1, o2, o3]);

    let mut turns = 0;
    while battle.phase != BattlePhase::Ended {
        battle.submit_action(BattleSide::Player, TurnAction::UseAbility(0));
        battle.submit_action(BattleSide::Opponent, TurnAction::UseAbility(0));
        turns += 1;
        assert!(turns < 100, "battle should end within 100 turns");
    }

    assert_eq!(battle.outcome, Some(BattleOutcome::PlayerWins));
    assert!(battle.player.has_alive());
}

#[test]
fn leveling_from_1_to_25_with_xp() {
    let mut pet = OwnedPet {
        id: 1,
        species_id: 1,
        custom_name: None,
        level: 1,
        quality: PetQuality::Common,
        xp: 0,
    };

    let total_needed = total_xp_for_level(25);
    let result = award_xp(&mut pet, total_needed);
    assert_eq!(result.new_level, 25);
    assert_eq!(result.levels_gained, 24);
    assert_eq!(pet.xp, 0);
}
