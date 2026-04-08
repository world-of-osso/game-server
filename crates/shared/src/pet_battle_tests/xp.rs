use super::*;

#[test]
fn xp_to_next_level_1() {
    assert_eq!(xp_to_next_level(1), 75);
}

#[test]
fn xp_to_next_level_10() {
    assert_eq!(xp_to_next_level(10), 2550);
}

#[test]
fn xp_to_next_level_max() {
    assert_eq!(xp_to_next_level(25), 0);
}

#[test]
fn xp_curve_increases() {
    for level in 1..24 {
        assert!(
            xp_to_next_level(level + 1) > xp_to_next_level(level),
            "level {} -> {} should require more XP",
            level,
            level + 1
        );
    }
}

#[test]
fn award_xp_no_level_up() {
    let mut pet = OwnedPet {
        id: 1,
        species_id: 1,
        custom_name: None,
        level: 1,
        quality: PetQuality::Common,
        xp: 0,
    };
    let result = award_xp(&mut pet, 50);
    assert_eq!(result.levels_gained, 0);
    assert_eq!(result.new_level, 1);
    assert_eq!(result.new_xp, 50);
    assert_eq!(result.xp_gained, 50);
}

#[test]
fn award_xp_single_level_up() {
    let mut pet = OwnedPet {
        id: 1,
        species_id: 1,
        custom_name: None,
        level: 1,
        quality: PetQuality::Common,
        xp: 0,
    };
    let result = award_xp(&mut pet, 75);
    assert_eq!(result.levels_gained, 1);
    assert_eq!(result.new_level, 2);
    assert_eq!(result.new_xp, 0);
}

#[test]
fn award_xp_multiple_level_ups() {
    let mut pet = OwnedPet {
        id: 1,
        species_id: 1,
        custom_name: None,
        level: 1,
        quality: PetQuality::Common,
        xp: 0,
    };
    let result = award_xp(&mut pet, 230);
    assert_eq!(result.levels_gained, 2);
    assert_eq!(result.new_level, 3);
    assert_eq!(result.new_xp, 5);
}

#[test]
fn award_xp_caps_at_max_level() {
    let mut pet = OwnedPet {
        id: 1,
        species_id: 1,
        custom_name: None,
        level: 24,
        quality: PetQuality::Common,
        xp: 0,
    };
    let result = award_xp(&mut pet, 999_999);
    assert_eq!(result.new_level, 25);
    assert_eq!(result.new_xp, 0);
}

#[test]
fn award_xp_at_max_level_does_nothing() {
    let mut pet = OwnedPet {
        id: 1,
        species_id: 1,
        custom_name: None,
        level: 25,
        quality: PetQuality::Common,
        xp: 0,
    };
    let result = award_xp(&mut pet, 1000);
    assert_eq!(result.xp_gained, 0);
    assert_eq!(result.levels_gained, 0);
    assert_eq!(result.new_level, 25);
}

#[test]
fn total_xp_for_level_1() {
    assert_eq!(total_xp_for_level(1), 0);
}

#[test]
fn total_xp_for_level_2() {
    assert_eq!(total_xp_for_level(2), 75);
}

#[test]
fn total_xp_for_level_3() {
    assert_eq!(total_xp_for_level(3), 225);
}

#[test]
fn battle_xp_equal_level() {
    let xp = battle_xp(10, 10);
    assert!(xp > 0);
}

#[test]
fn battle_xp_higher_opponent_gives_more() {
    let low = battle_xp(10, 10);
    let high = battle_xp(10, 20);
    assert!(high > low);
}
