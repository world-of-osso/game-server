use super::*;

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
    let mh = auto_attack_damage(200.0, 1200.0, 2.4);
    let oh = offhand_auto_attack_damage(200.0, 1200.0, 2.4);
    assert!((oh - mh * 0.5).abs() < 0.01, "oh={oh}, mh={mh}");
}

#[test]
fn offhand_damage_dagger_with_ap() {
    let oh = offhand_auto_attack_damage(80.0, 900.0, 1.7);
    assert!((oh - 167.5).abs() < 0.01, "got {oh}");
}

#[test]
fn offhand_multiplier_constant_is_half() {
    assert_eq!(OFFHAND_DAMAGE_MULTIPLIER, 0.5);
}

#[test]
fn crosscheck_normalized_ap_bonus() {
    assert!((ap_bonus_damage(600.0, 1.7) - 170.0).abs() < 0.01);
    assert!((ap_bonus_damage(600.0, 2.4) - 240.0).abs() < 0.01);
    assert!((ap_bonus_damage(600.0, 3.3) - 330.0).abs() < 0.01);
}

// --- Ability damage tests ---

#[test]
fn ability_damage_with_coefficient() {
    assert_eq!(ability_damage(500.0, 2000.0, 1.68), 3860.0);
}

#[test]
fn ability_damage_zero_ap() {
    assert_eq!(ability_damage(500.0, 0.0, 1.5), 500.0);
}

#[test]
fn ability_damage_zero_base() {
    assert_eq!(ability_damage(0.0, 1000.0, 2.0), 2000.0);
}

#[test]
fn ability_damage_zero_coefficient() {
    assert_eq!(ability_damage(300.0, 5000.0, 0.0), 300.0);
}

// --- Spell damage tests ---

#[test]
fn spell_damage_with_coefficient() {
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

#[test]
fn full_combat_round_warrior_vs_mob() {
    let attacker_level: u8 = 80;
    let attacker_class: u8 = 1;
    let target_level: u8 = 80;
    let weapon_min = 300.0_f32;
    let weapon_max = 500.0_f32;
    let weapon_speed = 3.3_f32;
    let attack_power = 1200.0_f32;
    let crit_rating = 400.0_f32;
    let target_armor = 5000.0_f32;
    let stamina = 200.0_f32;

    let max_hp = max_health(attacker_class, attacker_level, stamina).unwrap();
    let expected_hp = 8121.0 + 200.0 * 20.0;
    assert!(
        (max_hp - expected_hp).abs() < 0.01,
        "HP: {max_hp} != {expected_hp}"
    );

    let raw_min = auto_attack_damage(weapon_min, attack_power, weapon_speed);
    let raw_max = auto_attack_damage(weapon_max, attack_power, weapon_speed);
    let ap_bonus = ap_bonus_damage(attack_power, weapon_speed);
    assert!((ap_bonus - 660.0).abs() < 0.01);
    assert!((raw_min - 960.0).abs() < 0.01, "raw min: {raw_min}");
    assert!((raw_max - 1160.0).abs() < 0.01, "raw max: {raw_max}");

    let miss = melee::miss_chance(attacker_level, target_level);
    assert_eq!(miss, 300);
    let dodge = melee::dodge_chance(attacker_class, attacker_level, 0.0);
    assert_eq!(dodge, 500);
    let glancing = melee::glancing_chance(attacker_level, target_level);
    assert_eq!(glancing, 0);
    let crit = melee::crit_chance(attacker_level, crit_rating);
    assert!(crit > 0);

    let chances = melee::MeleeHitChances {
        miss,
        dodge,
        parry: 0,
        glancing,
        block: 0,
        crit,
    };
    assert_eq!(
        melee::resolve_melee_outcome(&chances, 0),
        melee::MeleeOutcome::Miss
    );
    assert_eq!(
        melee::resolve_melee_outcome(&chances, 9999),
        melee::MeleeOutcome::Hit
    );

    let mid_damage = (raw_min + raw_max) / 2.0;
    let armor_dr = armor_mitigation(target_armor, attacker_level).unwrap();
    let final_damage = mid_damage * (1.0 - armor_dr);
    assert!(
        final_damage > 100.0 && final_damage < 200.0,
        "final: {final_damage}"
    );

    let crit_damage = melee::apply_crit(mid_damage, 0.0);
    assert!((crit_damage - mid_damage * 2.0).abs() < 0.01);
    let crit_after_armor = crit_damage * (1.0 - armor_dr);
    assert!(crit_after_armor > final_damage);
}

#[test]
fn full_combat_round_rogue_vs_higher_level() {
    let attacker_level: u8 = 78;
    let target_level: u8 = 80;
    let weapon_min = 80.0_f32;
    let weapon_max = 150.0_f32;
    let weapon_speed = 1.7_f32;
    let attack_power = 900.0_f32;
    let target_armor = 3000.0_f32;

    let raw_min = auto_attack_damage(weapon_min, attack_power, weapon_speed);
    let raw_max = auto_attack_damage(weapon_max, attack_power, weapon_speed);
    assert_eq!(raw_min, 335.0);
    assert_eq!(raw_max, 405.0);

    let miss = melee::miss_chance(attacker_level, target_level);
    assert_eq!(miss, 500);
    let glancing = melee::glancing_chance(attacker_level, target_level);
    assert_eq!(glancing, 2000);
    let glancing_mult = melee::glancing_damage_multiplier(attacker_level, target_level);
    assert!((glancing_mult - 0.8).abs() < 0.001);

    let mid_damage = (raw_min + raw_max) / 2.0;
    let glancing_damage = mid_damage * glancing_mult;
    assert!((glancing_damage - 296.0).abs() < 0.01);

    let armor_dr = armor_mitigation(target_armor, attacker_level).unwrap();
    let final_glancing = glancing_damage * (1.0 - armor_dr);
    assert!(
        final_glancing > 50.0 && final_glancing < 80.0,
        "glancing: {final_glancing}"
    );
}
