use super::*;

#[test]
fn capture_chance_full_health() {
    let chance = capture_chance(100, 100);
    assert!((chance - 0.20).abs() < 0.01);
}

#[test]
fn capture_chance_half_health() {
    let chance = capture_chance(50, 100);
    assert!((chance - 0.575).abs() < 0.01);
}

#[test]
fn capture_chance_near_death() {
    let chance = capture_chance(1, 100);
    assert!(chance > 0.90);
    assert!(chance <= 0.95);
}

#[test]
fn capture_chance_dead_is_zero() {
    assert_eq!(capture_chance(0, 100), 0.0);
}

#[test]
fn capture_succeeds_with_low_roll() {
    let pet = BattlePet::new(PetStats {
        health: 100,
        power: 10,
        speed: 10,
    });
    let player_pet = BattlePet::new(PetStats {
        health: 100,
        power: 10,
        speed: 10,
    });
    let mut battle = PetBattle::new_wild(vec![player_pet], vec![pet]);

    let result = attempt_capture(&mut battle, 0.1);
    assert_eq!(result, CaptureResult::Captured);
    assert_eq!(battle.phase, BattlePhase::Ended);
    assert_eq!(battle.outcome, Some(BattleOutcome::PlayerWins));
}

#[test]
fn capture_fails_with_high_roll() {
    let pet = BattlePet::new(PetStats {
        health: 100,
        power: 10,
        speed: 10,
    });
    let player_pet = BattlePet::new(PetStats {
        health: 100,
        power: 10,
        speed: 10,
    });
    let mut battle = PetBattle::new_wild(vec![player_pet], vec![pet]);

    let result = attempt_capture(&mut battle, 0.5);
    assert_eq!(result, CaptureResult::Failed);
    assert_eq!(battle.phase, BattlePhase::SelectAction);
}

#[test]
fn capture_weakened_pet_easier() {
    let mut pet = BattlePet::new(PetStats {
        health: 100,
        power: 10,
        speed: 10,
    });
    pet.take_damage(90);
    let player_pet = BattlePet::new(PetStats {
        health: 100,
        power: 10,
        speed: 10,
    });
    let mut battle = PetBattle::new_wild(vec![player_pet], vec![pet]);

    let result = attempt_capture(&mut battle, 0.8);
    assert_eq!(result, CaptureResult::Captured);
}

#[test]
fn capture_not_wild_returns_not_wild() {
    let pet = BattlePet::new(PetStats {
        health: 100,
        power: 10,
        speed: 10,
    });
    let player_pet = BattlePet::new(PetStats {
        health: 100,
        power: 10,
        speed: 10,
    });
    let mut battle = PetBattle::new(vec![player_pet], vec![pet]);

    let result = attempt_capture(&mut battle, 0.0);
    assert_eq!(result, CaptureResult::NotWild);
}

#[test]
fn capture_dead_pet_returns_dead() {
    let mut pet = BattlePet::new(PetStats {
        health: 100,
        power: 10,
        speed: 10,
    });
    pet.take_damage(100);
    let player_pet = BattlePet::new(PetStats {
        health: 100,
        power: 10,
        speed: 10,
    });
    let mut battle = PetBattle::new_wild(vec![player_pet], vec![pet]);

    let result = attempt_capture(&mut battle, 0.0);
    assert_eq!(result, CaptureResult::Dead);
}
