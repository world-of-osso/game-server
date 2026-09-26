use super::*;
use shared::components::WeaponType;

/// Worn Shortsword (25), the Retail warrior starting weapon: 1-2 at 2.60.
const WORN_SHORTSWORD: WeaponDamage = WeaponDamage {
    min: 1.0,
    max: 2.0,
    speed: 2.6,
    kind: WeaponType::OneHand,
};

#[test]
fn swing_damage_is_weapon_plus_attack_power_over_six_times_speed() {
    // Level 10 Human Warrior: 33 Str = 33 AP; 33 / 6 × 2.6 = 14.3.
    let (min, max) = weapon_damage_range(&WORN_SHORTSWORD, 33.0, 0.0, Hand::Main);
    assert!((min - 15.3).abs() < 1e-4, "{min}");
    assert!((max - 16.3).abs() < 1e-4, "{max}");
}

#[test]
fn off_hand_swings_at_half_and_versatility_scales_both_hands() {
    let (min, max) = weapon_damage_range(&WORN_SHORTSWORD, 33.0, 10.0, Hand::Off);
    assert!((min - 15.3 * 0.5 * 1.1).abs() < 1e-4, "{min}");
    assert!((max - 16.3 * 0.5 * 1.1).abs() < 1e-4, "{max}");
}

#[test]
fn weapon_roll_is_a_whole_number_between_the_truncated_bounds() {
    assert_eq!(roll_weapon_damage((15.3, 16.3), 0.0), 15.0);
    assert_eq!(roll_weapon_damage((15.3, 16.3), 0.49), 15.0);
    assert_eq!(roll_weapon_damage((15.3, 16.3), 0.5), 16.0);
    assert_eq!(roll_weapon_damage((15.3, 16.3), 0.999), 16.0);
}

#[test]
fn haste_shortens_the_swing() {
    assert!((swing_time(2.6, 0.0) - 2.6).abs() < 1e-6);
    assert!((swing_time(2.6, 30.0) - 2.0).abs() < 1e-6);
}

#[test]
fn equal_level_creature_chances_are_trinitycore_base_values() {
    let chances = player_swing_chances(
        Victim::Creature {
            level_difference: 0,
        },
        5.0,
        false,
    );
    assert_eq!(
        chances,
        MeleeChances {
            miss: 5.0,
            dodge: 3.0,
            parry: 6.0,
            glancing: 0.0,
            block: 3.0,
            crit: 5.0,
        }
    );
}

#[test]
fn higher_creatures_avoid_more_and_glance_from_four_levels_up() {
    let three = player_swing_chances(
        Victim::Creature {
            level_difference: 3,
        },
        5.0,
        false,
    );
    assert_eq!((three.dodge, three.parry, three.block), (7.5, 10.5, 7.5));
    assert_eq!(three.glancing, 0.0);
    let four = player_swing_chances(
        Victim::Creature {
            level_difference: 4,
        },
        5.0,
        false,
    );
    assert_eq!(four.glancing, 50.0);
    // A creature below the attacker gets no level bonus.
    let lower = player_swing_chances(
        Victim::Creature {
            level_difference: -5,
        },
        5.0,
        false,
    );
    assert_eq!((lower.dodge, lower.parry, lower.block), (3.0, 6.0, 3.0));
}

#[test]
fn dual_wield_adds_19_percent_miss_and_players_only_miss() {
    let chances = player_swing_chances(Victim::Player, 12.0, true);
    assert_eq!(
        chances,
        MeleeChances {
            miss: 24.0,
            crit: 12.0,
            ..MeleeChances::default()
        }
    );
}

#[test]
fn one_roll_walks_the_table_in_order() {
    let chances = player_swing_chances(
        Victim::Creature {
            level_difference: 0,
        },
        5.0,
        false,
    );
    // Miss [0, 500), dodge [500, 800), parry [800, 1400), block [1400, 1700),
    // crit [1700, 2200), hit above.
    let outcome = |roll| roll_melee_outcome(&chances, roll);
    assert_eq!(outcome(0), MeleeOutcome::Miss);
    assert_eq!(outcome(499), MeleeOutcome::Miss);
    assert_eq!(outcome(500), MeleeOutcome::Dodge);
    assert_eq!(outcome(800), MeleeOutcome::Parry);
    assert_eq!(outcome(1400), MeleeOutcome::Block);
    assert_eq!(outcome(1700), MeleeOutcome::Crit);
    assert_eq!(outcome(2199), MeleeOutcome::Crit);
    assert_eq!(outcome(2200), MeleeOutcome::Hit);
    assert_eq!(outcome(9999), MeleeOutcome::Hit);
}

#[test]
fn full_crit_chance_crits_every_swing_that_is_not_avoided() {
    let chances = player_swing_chances(
        Victim::Creature {
            level_difference: 0,
        },
        100.0,
        false,
    );
    assert_eq!(roll_melee_outcome(&chances, 1700), MeleeOutcome::Crit);
    assert_eq!(roll_melee_outcome(&chances, 9999), MeleeOutcome::Crit);
}

#[test]
fn crit_doubles_block_stops_30_percent_and_avoidance_does_nothing() {
    assert_eq!(outcome_damage(11.0, MeleeOutcome::Crit, 0), (22.0, 0.0));
    assert_eq!(outcome_damage(11.0, MeleeOutcome::Hit, 0), (11.0, 0.0));
    assert_eq!(outcome_damage(11.0, MeleeOutcome::Block, 0), (8.0, 3.0));
    for avoided in [MeleeOutcome::Miss, MeleeOutcome::Dodge, MeleeOutcome::Parry] {
        assert_eq!(outcome_damage(11.0, avoided, 0), (0.0, 0.0));
    }
    // 10% per level, at most 30%.
    assert_eq!(
        outcome_damage(100.0, MeleeOutcome::Glancing, 4),
        (70.0, 0.0)
    );
}

#[test]
fn armor_mitigation_is_armor_over_armor_plus_the_attackers_constant() {
    // Hogger at level 10: ExpectedStat CreatureArmor 90 vs ArmorConstant 210 = 30%.
    assert_eq!(armor_reduced_damage(16.0, 90.0, 210.0), 11.0);
    assert_eq!(armor_reduced_damage(15.0, 90.0, 210.0), 10.0);
    // Capped at 85%; 1 - 0.85 is 0.14999998 in f32, as in TrinityCore.
    assert_eq!(armor_reduced_damage(100.0, 100_000.0, 210.0), 14.0);
    assert_eq!(armor_reduced_damage(16.0, 0.0, 210.0), 16.0);
}
