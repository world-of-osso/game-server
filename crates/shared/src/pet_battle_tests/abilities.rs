use super::*;

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
            health: 500,
            power: 50,
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
}
