use super::*;
use shared::components::WeaponType;

/// Worn Shortsword (25), the Retail warrior starting weapon: 1-2 at 2.60.
const WORN_SHORTSWORD: WeaponDamage = WeaponDamage {
    min: 1.0,
    max: 2.0,
    speed: 2.6,
    kind: WeaponType::OneHand,
};

const FORECAST_OUTCOMES: &[MeleeOutcome] = &[
    MeleeOutcome::Miss,
    MeleeOutcome::Dodge,
    MeleeOutcome::Parry,
    MeleeOutcome::Glancing,
    MeleeOutcome::Block,
    MeleeOutcome::Crit,
    MeleeOutcome::Hit,
];

fn count_native_forecast_outcomes(chances: &MeleeChances) -> Vec<u32> {
    let mut counts = vec![0; FORECAST_OUTCOMES.len()];
    // Exhaustive finite-domain oracle, not the production forecasting algorithm.
    for roll in 0..10_000 {
        let outcome = roll_melee_outcome(chances, roll);
        let index = FORECAST_OUTCOMES
            .iter()
            .position(|kind| *kind == outcome)
            .unwrap();
        counts[index] += 1;
    }
    counts
}

fn assert_forecast_weights_match_all_native_rolls(chances: MeleeChances) {
    let counts = count_native_forecast_outcomes(&chances);
    let weights = outcome_weights(&chances);
    let total: f64 = weights.iter().map(|(_, weight)| weight).sum();
    assert!((total - 1.0).abs() < 1e-12);
    for (&kind, count) in FORECAST_OUTCOMES.iter().zip(counts) {
        let actual: f64 = weights
            .iter()
            .filter(|(outcome, _)| *outcome == kind)
            .map(|(_, weight)| weight)
            .sum();
        assert!(
            (actual - f64::from(count) / 10_000.0).abs() < 1e-12,
            "{kind:?}"
        );
    }
}

#[test]
fn creature_forecast_weights_match_native_generic_melee_rolls() {
    let chances = swing_chances(
        Swinger::Creature {
            miss_pct: crate::combat::creature_miss_chance(None),
        },
        Victim::Creature {
            level_difference: 2,
        },
        true,
    );
    assert_forecast_weights_match_all_native_rolls(chances);
}

#[test]
fn creature_forecast_weights_preserve_native_sitting_critical_override() {
    let chances = swing_chances(
        Swinger::Creature {
            miss_pct: crate::combat::creature_miss_chance(None),
        },
        Victim::Player(PlayerDefense {
            sitting: true,
            ..Default::default()
        }),
        true,
    );
    assert_forecast_weights_match_all_native_rolls(chances);
}

#[test]
fn creature_forecast_weights_preserve_quantization_and_saturated_roll_table() {
    assert_forecast_weights_match_all_native_rolls(MeleeChances {
        miss: 90.009,
        dodge: 3.14159,
        parry: 2.71828,
        block: 1.23456,
        crit: 100.0,
        ..Default::default()
    });
}

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

fn player(crit_pct: f32, dual_wield: bool) -> Swinger {
    Swinger::Player {
        crit_pct,
        dual_wield,
    }
}

fn creature_at(level_difference: i32) -> Victim {
    Victim::Creature { level_difference }
}

#[test]
fn equal_level_creature_chances_are_trinitycore_base_values() {
    let chances = swing_chances(player(5.0, false), creature_at(0), true);
    assert_eq!(
        chances,
        MeleeChances {
            miss: 5.0,
            dodge: 3.0,
            parry: 6.0,
            glancing: 0.0,
            block: 3.0,
            crit: 5.0,
            crit_sitting: false,
        }
    );
}

#[test]
fn higher_creatures_avoid_more_and_glance_from_four_levels_up() {
    let three = swing_chances(player(5.0, false), creature_at(3), true);
    assert_eq!((three.dodge, three.parry, three.block), (7.5, 10.5, 7.5));
    assert_eq!(three.glancing, 0.0);
    let four = swing_chances(player(5.0, false), creature_at(4), true);
    assert_eq!(four.glancing, 50.0);
    // A creature below the attacker gets no level bonus.
    let lower = swing_chances(player(5.0, false), creature_at(-5), true);
    assert_eq!((lower.dodge, lower.parry, lower.block), (3.0, 6.0, 3.0));
}

#[test]
fn dual_wield_adds_19_percent_miss_against_a_player_without_avoidance() {
    let chances = swing_chances(
        player(12.0, true),
        Victim::Player(PlayerDefense::default()),
        true,
    );
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
    let chances = swing_chances(player(5.0, false), creature_at(0), true);
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
    let chances = swing_chances(player(100.0, false), creature_at(0), true);
    assert_eq!(roll_melee_outcome(&chances, 1700), MeleeOutcome::Crit);
    assert_eq!(roll_melee_outcome(&chances, 9999), MeleeOutcome::Crit);
}

#[test]
fn crit_doubles_block_stops_30_percent_and_avoidance_does_nothing() {
    assert_eq!(
        outcome_damage(11.0, MeleeOutcome::Crit, 0, CREATURE_BLOCKED_PCT),
        (22.0, 0.0)
    );
    assert_eq!(
        outcome_damage(11.0, MeleeOutcome::Hit, 0, CREATURE_BLOCKED_PCT),
        (11.0, 0.0)
    );
    assert_eq!(
        outcome_damage(11.0, MeleeOutcome::Block, 0, CREATURE_BLOCKED_PCT),
        (8.0, 3.0)
    );
    for avoided in [MeleeOutcome::Miss, MeleeOutcome::Dodge, MeleeOutcome::Parry] {
        assert_eq!(
            outcome_damage(11.0, avoided, 0, CREATURE_BLOCKED_PCT),
            (0.0, 0.0)
        );
    }
    // 10% per level, at most 30%.
    assert_eq!(
        outcome_damage(100.0, MeleeOutcome::Glancing, 4, CREATURE_BLOCKED_PCT),
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

/// Unit::RollMeleeOutcomeAgainst for a creature swinging at a player: the player's
/// avoidance, the creature's 5% crit (Unit.cpp:2885-2890) and no glancing.
#[test]
fn a_creature_swings_into_a_players_avoidance_and_crits_five_percent() {
    let defense = PlayerDefense {
        dodge: 2.0,
        parry: 5.0,
        block: 5.0,
        shield_block: 0.0,
        sitting: false,
    };
    let chances = swing_chances(
        Swinger::Creature { miss_pct: 5.0 },
        Victim::Player(defense),
        true,
    );
    assert_eq!(
        chances,
        MeleeChances {
            miss: 5.0,
            dodge: 2.0,
            parry: 5.0,
            glancing: 0.0,
            block: 5.0,
            crit: 5.0,
            crit_sitting: false,
        }
    );
    // Miss [0, 500), dodge [500, 700), parry [700, 1200), block [1200, 1700),
    // crit [1700, 2200).
    assert_eq!(roll_melee_outcome(&chances, 1199), MeleeOutcome::Parry);
    assert_eq!(roll_melee_outcome(&chances, 1200), MeleeOutcome::Block);
    assert_eq!(roll_melee_outcome(&chances, 2199), MeleeOutcome::Crit);
    // A creature 10 levels above its victim never glances.
    let far_above = swing_chances(Swinger::Creature { miss_pct: 5.0 }, creature_at(10), true);
    assert_eq!(far_above.glancing, 0.0);
}

/// A victim casting or controlled cannot dodge, parry or block (Unit.cpp:2408-2413).
#[test]
fn a_victim_that_cannot_avoid_only_meets_miss_and_crit() {
    let chances = swing_chances(player(10.0, false), creature_at(0), false);
    assert_eq!(
        chances,
        MeleeChances {
            miss: 5.0,
            crit: 10.0,
            ..MeleeChances::default()
        }
    );
}

/// Player::GetBlockPercent: ShieldBlock 250 (100 shield armor × 2.5) against a
/// 750 armor constant stops 25% of a blocked swing; capped at 85%.
#[test]
fn a_players_shield_stops_its_block_value_share_of_a_blocked_swing() {
    let defense = PlayerDefense {
        shield_block: 250.0,
        ..PlayerDefense::default()
    };
    assert_eq!(defense.blocked_pct(750.0), 25.0);
    assert_eq!(
        outcome_damage(100.0, MeleeOutcome::Block, 0, defense.blocked_pct(750.0)),
        (75.0, 25.0)
    );
    let wall = PlayerDefense {
        shield_block: 1.0e6,
        ..PlayerDefense::default()
    };
    assert!((wall.blocked_pct(750.0) - 85.0).abs() < 1e-3);
}

/// Unit::RollMeleeOutcomeAgainst (Unit.cpp:2418-2424): a swing at a player who sits,
/// sleeps or kneels crits unless it misses, ahead of dodge, parry and block; with no
/// crit chance it walks the normal table.
#[test]
fn a_swing_at_a_sitting_player_crits_unless_it_misses() {
    let sitting = PlayerDefense {
        dodge: 3.0,
        parry: 5.0,
        block: 5.0,
        sitting: true,
        ..PlayerDefense::default()
    };
    let chances = swing_chances(
        Swinger::Creature { miss_pct: 5.0 },
        Victim::Player(sitting),
        true,
    );
    assert!(chances.crit_sitting);
    assert_eq!(roll_melee_outcome(&chances, 499), MeleeOutcome::Miss);
    assert_eq!(roll_melee_outcome(&chances, 500), MeleeOutcome::Crit);
    assert_eq!(roll_melee_outcome(&chances, 9999), MeleeOutcome::Crit);
    let no_crit = MeleeChances {
        crit: 0.0,
        ..chances
    };
    assert_eq!(roll_melee_outcome(&no_crit, 500), MeleeOutcome::Dodge);
    assert_eq!(roll_melee_outcome(&no_crit, 9999), MeleeOutcome::Hit);
    let standing = swing_chances(
        Swinger::Creature { miss_pct: 5.0 },
        Victim::Player(PlayerDefense {
            sitting: false,
            ..sitting
        }),
        true,
    );
    assert_eq!(roll_melee_outcome(&standing, 500), MeleeOutcome::Dodge);
}
