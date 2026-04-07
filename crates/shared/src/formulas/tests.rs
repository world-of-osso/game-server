use super::*;

#[test]
fn warrior_level_1_base_hp() {
    assert_eq!(base_hp(1, 1), Some(20));
}

#[test]
fn warrior_level_80_base_hp() {
    assert_eq!(base_hp(1, 80), Some(8121));
}

#[test]
fn paladin_level_70_base_hp() {
    assert_eq!(base_hp(2, 70), Some(3377));
}

#[test]
fn dk_below_55_returns_none() {
    assert_eq!(base_hp(6, 54), None);
}

#[test]
fn dk_at_55_returns_data() {
    assert_eq!(base_hp(6, 55), Some(1359));
}

#[test]
fn invalid_class_returns_none() {
    assert_eq!(base_hp(10, 1), None);
    assert_eq!(base_hp(0, 1), None);
}

#[test]
fn druid_level_80_base_hp() {
    assert_eq!(base_hp(11, 80), Some(7417));
}

#[test]
fn hp_from_stamina_low_level_below_threshold() {
    // 15 stamina at level 30: 15 * 1 = 15
    assert_eq!(hp_from_stamina(15.0, 30), 15.0);
}

#[test]
fn hp_from_stamina_low_level_above_threshold() {
    // 50 stamina at level 30: 20*1 + 30*10 = 320
    assert_eq!(hp_from_stamina(50.0, 30), 320.0);
}

#[test]
fn hp_from_stamina_at_threshold_exact() {
    assert_eq!(hp_from_stamina(20.0, 30), 20.0);
}

#[test]
fn hp_from_stamina_retail_level() {
    // 150 stamina at level 70: 150 * 20 = 3000
    assert_eq!(hp_from_stamina(150.0, 70), 3000.0);
}

#[test]
fn hp_from_stamina_retail_level_80() {
    // 200 stamina at level 80: 200 * 20 = 4000
    assert_eq!(hp_from_stamina(200.0, 80), 4000.0);
}

#[test]
fn max_health_warrior_level_1() {
    // base 20 + 22 stam (WotLK: 20*1 + 2*10 = 40) = 60
    assert_eq!(max_health(1, 1, 22.0), Some(60.0));
}

#[test]
fn max_health_warrior_level_80() {
    // base 8121 + 159 stam * 20 = 8121 + 3180 = 11301
    assert_eq!(max_health(1, 80, 159.0), Some(11301.0));
}

#[test]
fn max_health_invalid_class_returns_none() {
    assert_eq!(max_health(0, 1, 100.0), None);
}

#[test]
fn hp_from_stamina_zero() {
    assert_eq!(hp_from_stamina(0.0, 1), 0.0);
    assert_eq!(hp_from_stamina(0.0, 80), 0.0);
}

// --- Mana tests ---

#[test]
fn paladin_level_1_base_mana() {
    assert_eq!(base_mana(2, 1), Some(60));
}

#[test]
fn paladin_level_80_base_mana() {
    assert_eq!(base_mana(2, 80), Some(4394));
}

#[test]
fn mage_level_70_base_mana() {
    assert_eq!(base_mana(8, 70), Some(2241));
}

#[test]
fn warrior_has_no_mana() {
    assert_eq!(base_mana(1, 1), None);
    assert_eq!(base_mana(1, 80), None);
}

#[test]
fn rogue_has_no_mana() {
    assert_eq!(base_mana(4, 1), None);
    assert_eq!(base_mana(4, 80), None);
}

#[test]
fn mana_from_intellect_low_level_below_threshold() {
    // 15 int at level 30: 15 * 1 = 15
    assert_eq!(mana_from_intellect(15.0, 30), 15.0);
}

#[test]
fn mana_from_intellect_low_level_above_threshold() {
    // 50 int at level 30: 20*1 + 30*15 = 470
    assert_eq!(mana_from_intellect(50.0, 30), 470.0);
}

#[test]
fn mana_from_intellect_at_threshold_exact() {
    assert_eq!(mana_from_intellect(20.0, 30), 20.0);
}

#[test]
fn mana_from_intellect_retail_level() {
    // 100 int at level 70: 100 * 20 = 2000
    assert_eq!(mana_from_intellect(100.0, 70), 2000.0);
}

#[test]
fn max_mana_paladin_level_80() {
    // base 4394 + 98 int * 20 = 4394 + 1960 = 6354
    assert_eq!(max_mana(2, 80, 98.0), Some(6354.0));
}

#[test]
fn max_mana_warrior_returns_none() {
    assert_eq!(max_mana(1, 80, 100.0), None);
}

#[test]
fn mana_from_intellect_zero() {
    assert_eq!(mana_from_intellect(0.0, 1), 0.0);
    assert_eq!(mana_from_intellect(0.0, 80), 0.0);
}

// --- Cross-checks against AzerothCore SQL (player_class_stats) ---
// Source: ~/Repos/azerothcore/data/sql/base/db_world/player_class_stats.sql
// Formula ref: AzerothCore src/server/game/Entities/Unit/StatSystem.cpp
//   GetHealthBonusFromStamina: first 20 stam = 1 HP, above 20 = 10 HP each
//   GetManaBonusFromIntellect: first 20 int = 1 mana, above 20 = 15 mana each

#[test]
fn crosscheck_warrior_80_hp() {
    // AzerothCore player_class_stats: class=1, level=80, basehp=8121
    // Warrior has no base mana.
    assert_eq!(base_hp(1, 80), Some(8121));
    assert_eq!(base_mana(1, 80), None);

    // With 100 stamina at L80 (retail model): 100 * 20 = 2000
    // Total: 8121 + 2000 = 10121
    assert_eq!(max_health(1, 80, 100.0), Some(10121.0));
}

#[test]
fn crosscheck_mage_70_hp_mana() {
    // AzerothCore player_class_stats: class=8, level=70, basehp=3393, basemana=2241
    assert_eq!(base_hp(8, 70), Some(3393));
    assert_eq!(base_mana(8, 70), Some(2241));

    // With 50 stamina at L70 (retail model): 50 * 20 = 1000
    // Total HP: 3393 + 1000 = 4393
    assert_eq!(max_health(8, 70, 50.0), Some(4393.0));

    // With 150 intellect at L70 (retail model): 150 * 20 = 3000
    // Total mana: 2241 + 3000 = 5241
    assert_eq!(max_mana(8, 70, 150.0), Some(5241.0));
}

#[test]
fn crosscheck_paladin_1_hp_mana() {
    // AzerothCore player_class_stats: class=2, level=1, basehp=28, basemana=60
    assert_eq!(base_hp(2, 1), Some(28));
    assert_eq!(base_mana(2, 1), Some(60));

    // With 25 stamina at L1 (WotLK model): 20*1 + 5*10 = 70
    // Total HP: 28 + 70 = 98
    assert_eq!(max_health(2, 1, 25.0), Some(98.0));

    // With 25 intellect at L1 (WotLK model): 20*1 + 5*15 = 95
    // Total mana: 60 + 95 = 155
    assert_eq!(max_mana(2, 1, 25.0), Some(155.0));
}

// --- Equipment stat aggregation tests ---

#[test]
fn sum_equipment_stats_empty() {
    let (primary, secondary) = sum_equipment_stats(&[]);
    assert_eq!(primary, UnitStats::default());
    assert_eq!(secondary, CombatRatings::default());
}

#[test]
fn sum_equipment_stats_single_item() {
    let chest = ItemStatBlock {
        primary: UnitStats {
            stamina: 50.0,
            strength: 30.0,
            ..Default::default()
        },
        secondary: CombatRatings {
            crit: 20.0,
            haste: 15.0,
            ..Default::default()
        },
    };
    let (primary, secondary) = sum_equipment_stats(&[chest]);
    assert_eq!(primary.stamina, 50.0);
    assert_eq!(primary.strength, 30.0);
    assert_eq!(secondary.crit, 20.0);
    assert_eq!(secondary.haste, 15.0);
}

#[test]
fn sum_equipment_stats_multiple_items() {
    let helm = ItemStatBlock {
        primary: UnitStats {
            stamina: 40.0,
            intellect: 35.0,
            ..Default::default()
        },
        secondary: CombatRatings {
            crit: 18.0,
            mastery: 12.0,
            ..Default::default()
        },
    };
    let chest = ItemStatBlock {
        primary: UnitStats {
            stamina: 55.0,
            intellect: 45.0,
            spirit: 10.0,
            ..Default::default()
        },
        secondary: CombatRatings {
            haste: 25.0,
            versatility: 20.0,
            armor: 500.0,
            ..Default::default()
        },
    };
    let legs = ItemStatBlock {
        primary: UnitStats {
            stamina: 48.0,
            intellect: 40.0,
            ..Default::default()
        },
        secondary: CombatRatings {
            crit: 22.0,
            mastery: 15.0,
            armor: 400.0,
            ..Default::default()
        },
    };

    let (primary, secondary) = sum_equipment_stats(&[helm, chest, legs]);

    assert_eq!(primary.stamina, 143.0);
    assert_eq!(primary.intellect, 120.0);
    assert_eq!(primary.spirit, 10.0);
    assert_eq!(primary.strength, 0.0);
    assert_eq!(secondary.crit, 40.0);
    assert_eq!(secondary.mastery, 27.0);
    assert_eq!(secondary.haste, 25.0);
    assert_eq!(secondary.versatility, 20.0);
    assert_eq!(secondary.armor, 900.0);
}

// --- Rating conversion tests ---

#[test]
fn rating_per_percent_crit_level_80() {
    // raw 10.245685040 * 100 = 1024.568504
    let per_pct = rating_per_percent(RatingType::Crit, 80).unwrap();
    assert!((per_pct - 1024.5685).abs() < 0.01);
}

#[test]
fn rating_per_percent_haste_level_70() {
    // raw 8.074973567 * 100 = 807.4973567
    let per_pct = rating_per_percent(RatingType::Haste, 70).unwrap();
    assert!((per_pct - 807.497).abs() < 0.01);
}

#[test]
fn rating_per_percent_mastery_no_multiply() {
    // Mastery uses raw value (points, not percent): 10.245685040
    let per_point = rating_per_percent(RatingType::Mastery, 80).unwrap();
    assert!((per_point - 10.2457).abs() < 0.01);
}

#[test]
fn rating_per_percent_dodge_equals_parry() {
    let dodge = rating_per_percent(RatingType::Dodge, 80).unwrap();
    let parry = rating_per_percent(RatingType::Parry, 80).unwrap();
    assert_eq!(dodge, parry);
}

#[test]
fn rating_to_percent_crit_100_at_80() {
    // 100 / 1024.568504 ≈ 0.09760%
    let pct = rating_to_percent(100.0, 80, RatingType::Crit).unwrap();
    assert!((pct - 0.09760).abs() < 0.0001);
}

#[test]
fn rating_to_percent_mastery_100_at_80() {
    // 100 / 10.245685 ≈ 9.760 mastery points
    let points = rating_to_percent(100.0, 80, RatingType::Mastery).unwrap();
    assert!((points - 9.760).abs() < 0.01);
}

#[test]
fn rating_to_percent_block_level_80() {
    // raw 5.122915645 * 100 = 512.2915645
    // 100 / 512.29 ≈ 0.19520%
    let pct = rating_to_percent(100.0, 80, RatingType::Block).unwrap();
    assert!((pct - 0.19520).abs() < 0.001);
}

#[test]
fn rating_conversion_level_0_returns_none() {
    assert_eq!(rating_to_percent(100.0, 0, RatingType::Crit), None);
}

#[test]
fn rating_conversion_level_81_returns_none() {
    assert_eq!(rating_to_percent(100.0, 81, RatingType::Crit), None);
}

#[test]
fn rating_to_percent_zero_rating() {
    let pct = rating_to_percent(0.0, 80, RatingType::Crit).unwrap();
    assert_eq!(pct, 0.0);
}

#[test]
fn higher_level_needs_more_rating() {
    let low = rating_per_percent(RatingType::Crit, 20).unwrap();
    let high = rating_per_percent(RatingType::Crit, 80).unwrap();
    assert!(high > low, "higher levels should require more rating per percent");
}

// --- Diminishing returns tests ---

#[test]
fn dr_below_threshold_is_identity() {
    // Below 30%, no DR applies
    assert_eq!(apply_secondary_dr(0.0, RatingType::Crit), 0.0);
    assert_eq!(apply_secondary_dr(15.0, RatingType::Haste), 15.0);
    assert_eq!(apply_secondary_dr(30.0, RatingType::Versatility), 30.0);
}

#[test]
fn dr_at_exact_breakpoints() {
    assert_eq!(apply_secondary_dr(40.0, RatingType::Crit), 39.0);
    assert_eq!(apply_secondary_dr(50.0, RatingType::Crit), 47.0);
    assert_eq!(apply_secondary_dr(60.0, RatingType::Crit), 54.0);
    assert_eq!(apply_secondary_dr(100.0, RatingType::Crit), 76.0);
}

#[test]
fn dr_interpolates_between_breakpoints() {
    // Midpoint between (30, 30) and (40, 39) → input 35 → output 34.5
    let result = apply_secondary_dr(35.0, RatingType::Crit);
    assert!((result - 34.5).abs() < 0.001);
}

#[test]
fn dr_mastery_uses_same_curve() {
    // 50 mastery points → 47 after DR
    assert_eq!(apply_secondary_dr(50.0, RatingType::Mastery), 47.0);
}

#[test]
fn dr_dodge_parry_block_passthrough() {
    // Avoidance stats not affected by secondary DR
    assert_eq!(apply_secondary_dr(50.0, RatingType::Dodge), 50.0);
    assert_eq!(apply_secondary_dr(50.0, RatingType::Parry), 50.0);
    assert_eq!(apply_secondary_dr(50.0, RatingType::Block), 50.0);
}

#[test]
fn dr_clamps_above_max_curve() {
    // Above 200 input, output capped at 126
    assert_eq!(apply_secondary_dr(250.0, RatingType::Crit), 126.0);
}

#[test]
fn dr_reduces_effective_value() {
    // Any value above 30% should be reduced
    for raw in [35.0, 45.0, 60.0, 80.0, 100.0] {
        let effective = apply_secondary_dr(raw, RatingType::Haste);
        assert!(
            effective < raw,
            "DR should reduce {raw}% to less, got {effective}%"
        );
    }
}

// --- Armor mitigation tests ---

#[test]
fn armor_mitigation_level_80() {
    // K(80) = 880. 880 armor vs L80: 880 / (880 + 880) = 0.5
    let dr = armor_mitigation(880.0, 80).unwrap();
    assert!((dr - 0.5).abs() < 0.001);
}

#[test]
fn armor_mitigation_level_1() {
    // K(1) = 116. 116 armor vs L1: 116 / (116 + 116) = 0.5
    let dr = armor_mitigation(116.0, 1).unwrap();
    assert!((dr - 0.5).abs() < 0.001);
}

#[test]
fn armor_mitigation_zero_armor() {
    let dr = armor_mitigation(0.0, 80).unwrap();
    assert_eq!(dr, 0.0);
}

#[test]
fn armor_mitigation_high_armor() {
    // 8800 armor vs L80 (K=880): 8800 / (8800 + 880) = 0.9090...
    let dr = armor_mitigation(8800.0, 80).unwrap();
    assert!((dr - 0.9091).abs() < 0.001);
}

#[test]
fn armor_mitigation_invalid_level() {
    assert_eq!(armor_mitigation(500.0, 0), None);
    assert_eq!(armor_mitigation(500.0, 81), None);
}

#[test]
fn armor_mitigation_increases_with_armor() {
    let low = armor_mitigation(200.0, 80).unwrap();
    let high = armor_mitigation(2000.0, 80).unwrap();
    assert!(high > low);
}

#[test]
fn armor_mitigation_decreases_vs_higher_level() {
    // Same armor is less effective against higher-level attacker
    let vs_low = armor_mitigation(500.0, 40).unwrap();
    let vs_high = armor_mitigation(500.0, 80).unwrap();
    assert!(vs_low > vs_high);
}

// --- Cross-check: armor K constants against SimC ---
// Source: SimC engine/dbc/generated/expected_stat.inc, field armor_constant
// Formula: mitigation = armor / (armor + K), same in both codebases

#[test]
fn crosscheck_armor_k_level_70() {
    // SimC expected_stat.inc line 72: level 70, armor_constant = 793.0
    // 10000 armor vs L70: 10000 / (10000 + 793) = 0.92654...
    let dr = armor_mitigation(10000.0, 70).unwrap();
    let expected = 10000.0 / (10000.0 + 793.0);
    assert!((dr - expected).abs() < 0.0001);
}

#[test]
fn crosscheck_armor_k_level_80() {
    // SimC expected_stat.inc line 82: level 80, armor_constant = 880.0
    // 5000 armor vs L80: 5000 / (5000 + 880) = 0.85034...
    let dr = armor_mitigation(5000.0, 80).unwrap();
    let expected = 5000.0 / (5000.0 + 880.0);
    assert!((dr - expected).abs() < 0.0001);
}

#[test]
fn crosscheck_armor_k_level_1() {
    // SimC expected_stat.inc line 3: level 1, armor_constant = 116.0
    // 300 armor vs L1: 300 / (300 + 116) = 0.72115...
    let dr = armor_mitigation(300.0, 1).unwrap();
    let expected = 300.0 / (300.0 + 116.0);
    assert!((dr - expected).abs() < 0.0001);
}

// --- AP contribution tests ---

#[test]
fn ap_bonus_with_2h_weapon() {
    // 1200 AP, 3.3 speed: (1200/6) * 3.3 = 200 * 3.3 = 660
    assert_eq!(ap_bonus_damage(1200.0, 3.3), 660.0);
}

#[test]
fn ap_bonus_with_1h_weapon() {
    // 1200 AP, 2.4 speed: (1200/6) * 2.4 = 200 * 2.4 = 480
    assert!((ap_bonus_damage(1200.0, 2.4) - 480.0).abs() < 0.01);
}

#[test]
fn ap_bonus_with_dagger() {
    // 1200 AP, 1.7 speed: (1200/6) * 1.7 = 200 * 1.7 = 340
    assert!((ap_bonus_damage(1200.0, 1.7) - 340.0).abs() < 0.01);
}

#[test]
fn ap_bonus_zero_ap() {
    assert_eq!(ap_bonus_damage(0.0, 2.6), 0.0);
}

#[test]
fn ap_bonus_scales_with_speed() {
    let slow = ap_bonus_damage(1000.0, 3.6);
    let fast = ap_bonus_damage(1000.0, 1.5);
    assert!(slow > fast, "slower weapons get more AP bonus per swing");
}

// --- Auto-attack damage tests ---

#[test]
fn auto_attack_min_roll() {
    // min weapon roll 100, 1200 AP, 3.3 speed → 100 + 660 = 760
    assert_eq!(auto_attack_damage(100.0, 1200.0, 3.3), 760.0);
}

#[test]
fn auto_attack_max_roll() {
    // max weapon roll 200, 1200 AP, 3.3 speed → 200 + 660 = 860
    assert_eq!(auto_attack_damage(200.0, 1200.0, 3.3), 860.0);
}

#[test]
fn auto_attack_zero_ap() {
    // No AP, just weapon damage
    assert_eq!(auto_attack_damage(150.0, 0.0, 2.6), 150.0);
}

#[test]
fn auto_attack_zero_weapon() {
    // Unarmed: 0 weapon roll, AP still contributes
    let bonus = ap_bonus_damage(1200.0, 2.0);
    assert_eq!(auto_attack_damage(0.0, 1200.0, 2.0), bonus);
}

// --- Cross-check: auto-attack damage against SimC ---
// Source: SimC engine/action/attack.hpp WEAPON_POWER_COEFFICIENT = 6
// Source: SimC engine/player/weapon.hpp normalized speeds: 1.7, 2.4, 3.3
// Formula: damage = random(min, max) + (AP / 6) * weapon_speed

#[test]
fn crosscheck_auto_attack_2h_warrior() {
    // 2H sword (3.3 speed), 600 AP, weapon rolls 300-500
    // AP bonus: (600 / 6) * 3.3 = 100 * 3.3 = 330
    // Min total: 300 + 330 = 630
    // Max total: 500 + 330 = 830
    assert_eq!(auto_attack_damage(300.0, 600.0, 3.3), 630.0);
    assert_eq!(auto_attack_damage(500.0, 600.0, 3.3), 830.0);
}

#[test]
fn crosscheck_auto_attack_1h_rogue() {
    // Dagger (1.7 speed), 900 AP, weapon rolls 80-150
    // AP bonus: (900 / 6) * 1.7 = 150 * 1.7 = 255
    // Min total: 80 + 255 = 335
    // Max total: 150 + 255 = 405
    assert_eq!(auto_attack_damage(80.0, 900.0, 1.7), 335.0);
    assert_eq!(auto_attack_damage(150.0, 900.0, 1.7), 405.0);
}

#[test]
fn offhand_damage_is_50_percent_of_mainhand() {
    // Same weapon roll, AP, speed — offhand should be exactly 50%
    let mh = auto_attack_damage(200.0, 1200.0, 2.4);
    let oh = offhand_auto_attack_damage(200.0, 1200.0, 2.4);
    assert!((oh - mh * 0.5).abs() < 0.01, "oh={oh}, mh={mh}");
}

#[test]
fn offhand_damage_dagger_with_ap() {
    // Dagger (1.7), 900 AP, roll 80
    // MH: 80 + (900/6)*1.7 = 80 + 255 = 335
    // OH: 335 * 0.5 = 167.5
    let oh = offhand_auto_attack_damage(80.0, 900.0, 1.7);
    assert!((oh - 167.5).abs() < 0.01, "got {oh}");
}

#[test]
fn offhand_multiplier_constant_is_half() {
    assert_eq!(OFFHAND_DAMAGE_MULTIPLIER, 0.5);
}

#[test]
fn crosscheck_normalized_ap_bonus() {
    // SimC normalized speeds match our WeaponType constants
    // With 600 AP:
    // Dagger (1.7): (600/6) * 1.7 = 170
    // 1H (2.4):     (600/6) * 2.4 = 240
    // 2H (3.3):     (600/6) * 3.3 = 330
    assert!((ap_bonus_damage(600.0, 1.7) - 170.0).abs() < 0.01);
    assert!((ap_bonus_damage(600.0, 2.4) - 240.0).abs() < 0.01);
    assert!((ap_bonus_damage(600.0, 3.3) - 330.0).abs() < 0.01);
}

// --- Ability damage tests ---

#[test]
fn ability_damage_with_coefficient() {
    // base 500, 2000 AP, coeff 1.68 → 500 + 3360 = 3860
    assert_eq!(ability_damage(500.0, 2000.0, 1.68), 3860.0);
}

#[test]
fn ability_damage_zero_ap() {
    assert_eq!(ability_damage(500.0, 0.0, 1.5), 500.0);
}

#[test]
fn ability_damage_zero_base() {
    // Pure AP scaling (e.g. Execute at low base)
    assert_eq!(ability_damage(0.0, 1000.0, 2.0), 2000.0);
}

#[test]
fn ability_damage_zero_coefficient() {
    // Fixed damage ability (no AP scaling)
    assert_eq!(ability_damage(300.0, 5000.0, 0.0), 300.0);
}

// --- Spell damage tests ---

#[test]
fn spell_damage_with_coefficient() {
    // Fireball: base 800, 3000 SP, coeff 1.0 → 800 + 3000 = 3800
    assert_eq!(spell_damage(800.0, 3000.0, 1.0), 3800.0);
}

#[test]
fn spell_damage_zero_sp() {
    assert_eq!(spell_damage(800.0, 0.0, 1.0), 800.0);
}

#[test]
fn spell_damage_zero_base() {
    assert_eq!(spell_damage(0.0, 2000.0, 0.5), 1000.0);
}

#[test]
fn spell_damage_zero_coefficient() {
    assert_eq!(spell_damage(500.0, 9000.0, 0.0), 500.0);
}

// --- Integration: full combat round with gear ---
// Exercises the complete formula pipeline: gear stats → hit table → damage → armor

#[test]
fn full_combat_round_warrior_vs_mob() {
    // L80 Warrior with 2H weapon attacking a L80 mob
    let attacker_level: u8 = 80;
    let attacker_class: u8 = 1; // Warrior
    let target_level: u8 = 80;

    // Gear: 2H sword + plate armor contributing stats
    let weapon_min = 300.0_f32;
    let weapon_max = 500.0_f32;
    let weapon_speed = 3.3_f32;
    let attack_power = 1200.0_f32;
    let crit_rating = 400.0_f32;
    let target_armor = 5000.0_f32;

    // Step 1: Compute HP from stats
    let stamina = 200.0_f32;
    let max_hp = max_health(attacker_class, attacker_level, stamina).unwrap();
    let expected_hp = 8121.0 + 200.0 * 20.0; // base + stam * 20 (retail)
    assert!((max_hp - expected_hp).abs() < 0.01, "HP: {max_hp} != {expected_hp}");

    // Step 2: Auto-attack damage range (before hit table)
    let raw_min = auto_attack_damage(weapon_min, attack_power, weapon_speed);
    let raw_max = auto_attack_damage(weapon_max, attack_power, weapon_speed);
    // AP bonus: (1200/6) * 3.3 = 660
    let ap_bonus = ap_bonus_damage(attack_power, weapon_speed);
    assert!((ap_bonus - 660.0).abs() < 0.01);
    assert!((raw_min - 960.0).abs() < 0.01, "raw min: {raw_min}");
    assert!((raw_max - 1160.0).abs() < 0.01, "raw max: {raw_max}");

    // Step 3: Build hit table
    let miss = melee::miss_chance(attacker_level, target_level);
    assert_eq!(miss, 300); // 3% at equal level
    let dodge = melee::dodge_chance(attacker_class, attacker_level, 0.0);
    assert_eq!(dodge, 500); // 5% base, no dodge rating
    let glancing = melee::glancing_chance(attacker_level, target_level);
    assert_eq!(glancing, 0); // 0% at equal level
    let crit = melee::crit_chance(attacker_level, crit_rating);
    assert!(crit > 0, "should have some crit from 400 rating");

    let chances = melee::MeleeHitChances {
        miss,
        dodge,
        parry: 0,
        glancing,
        block: 0,
        crit,
    };

    // Step 4: Resolve outcomes for specific rolls
    // Roll 0 = miss (first 300 of 10000)
    assert_eq!(
        melee::resolve_melee_outcome(&chances, 0),
        melee::MeleeOutcome::Miss
    );

    // Roll 9999 = normal hit (past all avoidance + crit)
    assert_eq!(
        melee::resolve_melee_outcome(&chances, 9999),
        melee::MeleeOutcome::Hit
    );

    // Step 5: Apply armor mitigation to a normal hit
    let mid_damage = (raw_min + raw_max) / 2.0; // 1060
    let armor_dr = armor_mitigation(target_armor, attacker_level).unwrap();
    let expected_dr = 5000.0 / (5000.0 + 880.0); // K(80) = 880
    assert!((armor_dr - expected_dr).abs() < 0.001);
    let final_damage = mid_damage * (1.0 - armor_dr);
    // 1060 * (1 - 0.8503) = 1060 * 0.1497 ≈ 158.7
    assert!(
        final_damage > 100.0 && final_damage < 200.0,
        "final damage {final_damage} should be 100-200 after 5000 armor"
    );

    // Step 6: Crit path — double the pre-armor damage, then mitigate
    let crit_damage = melee::apply_crit(mid_damage, 0.0); // 2.0x
    assert!((crit_damage - mid_damage * 2.0).abs() < 0.01);
    let crit_after_armor = crit_damage * (1.0 - armor_dr);
    assert!(
        crit_after_armor > final_damage,
        "crit {crit_after_armor} > normal {final_damage}"
    );
    assert!(
        (crit_after_armor - final_damage * 2.0).abs() < 1.0,
        "crit should be ~2x normal hit"
    );
}

#[test]
fn full_combat_round_rogue_vs_higher_level() {
    // L78 Rogue with dagger attacking L80 mob — tests glancing blows
    let attacker_level: u8 = 78;
    let target_level: u8 = 80;

    let weapon_min = 80.0_f32;
    let weapon_max = 150.0_f32;
    let weapon_speed = 1.7_f32;
    let attack_power = 900.0_f32;
    let target_armor = 3000.0_f32;

    // Raw damage range
    let raw_min = auto_attack_damage(weapon_min, attack_power, weapon_speed);
    let raw_max = auto_attack_damage(weapon_max, attack_power, weapon_speed);
    // AP bonus: (900/6) * 1.7 = 255
    assert_eq!(raw_min, 335.0);
    assert_eq!(raw_max, 405.0);

    // Hit table vs +2 level mob
    let miss = melee::miss_chance(attacker_level, target_level);
    assert_eq!(miss, 500); // 3% + 2% = 5%
    let glancing = melee::glancing_chance(attacker_level, target_level);
    assert_eq!(glancing, 2000); // (10 + 2*5) * 100 = 2000 = 20%
    let glancing_mult = melee::glancing_damage_multiplier(attacker_level, target_level);
    assert!((glancing_mult - 0.8).abs() < 0.001); // 2 level diff = 80%

    // Glancing blow damage after armor
    let mid_damage = (raw_min + raw_max) / 2.0; // 370
    let glancing_damage = mid_damage * glancing_mult; // 370 * 0.8 = 296
    assert!((glancing_damage - 296.0).abs() < 0.01);

    let armor_dr = armor_mitigation(target_armor, attacker_level).unwrap();
    // K(78) = 864
    let expected_dr = 3000.0 / (3000.0 + 864.0);
    assert!((armor_dr - expected_dr).abs() < 0.001);
    let final_glancing = glancing_damage * (1.0 - armor_dr);
    // 296 * (1 - 0.7764) = 296 * 0.2236 ≈ 66.2
    assert!(
        final_glancing > 50.0 && final_glancing < 80.0,
        "glancing after armor: {final_glancing}"
    );
}
