//! Player auto-attacks swing their weapon (docs/specs/player-damage.md).

use super::*;
use crate::item_stats::WeaponDamage;
use crate::level_scaling::{ArmorConstants, CreatureArmor};
use crate::stats::CharacterStats;
use shared::components::WeaponType;

/// ExpectedStat ArmorConstant (ExpansionID -2) at levels 0..=10 and 30 of
/// 12.1.0.69933; the levels between are unused here.
fn armor_constants() -> ArmorConstants {
    let mut constants = vec![0.0; 31];
    constants[10] = 210.0;
    constants[30] = 361.0;
    ArmorConstants(constants)
}

/// Hogger with ExpectedStat CreatureArmor 90 at level 10 and 155 at 30
/// (ArmorModifier 1): its own armor is the level 30 value.
fn armored_hogger(app: &mut App) -> Entity {
    let hogger = spawn_target(app, pos(1.0, 0.0, 0.0), 13_137.0);
    tuned_hogger(app, hogger);
    app.world_mut().entity_mut(hogger).insert((
        Npc {
            template_id: 448,
            name: "Hogger".into(),
        },
        CreatureArmor(155.0),
    ));
    app.insert_resource(armor_constants());
    hogger
}

fn level_10_player(app: &mut App, target: Entity, stats: CharacterStats) -> Entity {
    let player = spawn_attacker(app, pos(0.0, 0.0, 0.0), target);
    app.world_mut()
        .entity_mut(player)
        .insert((UnitLevel(10), stats));
    player
}

const HEAVY_BLADE: WeaponDamage = WeaponDamage {
    min: 100.0,
    max: 100.0,
    speed: 2.6,
    kind: WeaponType::OneHand,
};

#[test]
fn a_players_swing_is_weapon_plus_attack_power_through_the_creatures_armor() {
    let mut app = setup_app();
    let hogger = armored_hogger(&mut app);
    level_10_player(
        &mut app,
        hogger,
        CharacterStats {
            attack_power: 60.0,
            main_hand: HEAVY_BLADE,
            ..default()
        },
    );
    run_attack_tick(&mut app);
    // 100 + 60 / 6 × 2.6 = 126; a level 10 Hogger's armor 90 against the
    // level 10 constant 210 stops 30%: 88. Scaled to Hogger's pool
    // (13,137 / 1,133): 1,020. Blocked: 88 - 26 = 62 (718); crit 176 (2,040).
    let lost = 13_137.0 - app.world().get::<Health>(hogger).unwrap().current;
    assert!(
        [0.0, 1_020.0, 718.0, 2_040.0].contains(&lost),
        "health lost {lost}"
    );
}

#[test]
fn the_swing_timer_is_the_weapon_speed_over_haste() {
    for (haste_pct, expected) in [(0.0, 2.6), (30.0, 2.0)] {
        let mut app = setup_app();
        let hogger = armored_hogger(&mut app);
        let player = level_10_player(
            &mut app,
            hogger,
            CharacterStats {
                main_hand: HEAVY_BLADE,
                haste_pct,
                ..default()
            },
        );
        run_attack_tick(&mut app);
        let timer = app.world().get::<AutoAttack>(player).unwrap().timer;
        assert!(
            (timer - expected).abs() < 1e-4,
            "haste {haste_pct}: {timer}"
        );
    }
}

#[test]
fn a_full_crit_chance_swing_is_doubled_unless_avoided_or_blocked() {
    let mut app = setup_app();
    let hogger = armored_hogger(&mut app);
    level_10_player(
        &mut app,
        hogger,
        CharacterStats {
            attack_power: 60.0,
            crit_pct: 100.0,
            main_hand: HEAVY_BLADE,
            ..default()
        },
    );
    run_attack_tick(&mut app);
    // Crit 88 × 2 = 176 (2,040 of the pool); block and avoidance roll first.
    let lost = 13_137.0 - app.world().get::<Health>(hogger).unwrap().current;
    assert!([0.0, 718.0, 2_040.0].contains(&lost), "health lost {lost}");
}

#[test]
fn an_off_hand_weapon_swings_at_half_damage_200_ms_after_the_main_hand() {
    let mut app = setup_app();
    let target = spawn_target(&mut app, pos(1.0, 0.0, 0.0), 10_000.0);
    let player = level_10_player(
        &mut app,
        target,
        CharacterStats {
            main_hand: HEAVY_BLADE,
            off_hand: Some(HEAVY_BLADE),
            ..default()
        },
    );
    run_attack_tick(&mut app);
    // The main hand swings; the off hand waits ATTACK_DISPLAY_DELAY.
    let auto = app.world().get::<AutoAttack>(player).unwrap();
    assert!((auto.timer - 2.6).abs() < 1e-4 && (auto.off_hand_timer - 0.2).abs() < 1e-4);
    let after_main = app.world().get::<Health>(target).unwrap().current;
    assert!([10_000.0, 9_900.0].contains(&after_main), "{after_main}");
    app.world_mut()
        .get_mut::<AutoAttack>(player)
        .unwrap()
        .off_hand_timer = 0.0;
    run_attack_tick(&mut app);
    // 100 × 0.5 = 50 against a target without armor or avoidance (a player
    // target only misses: 5% + 19% dual wield).
    let off_hand = after_main - app.world().get::<Health>(target).unwrap().current;
    assert!([0.0, 50.0].contains(&off_hand), "{off_hand}");
    let auto = app.world().get::<AutoAttack>(player).unwrap();
    assert!((auto.off_hand_timer - 2.6).abs() < 1e-4);
}

/// Worn Shortsword (25) from world.db, the Retail warrior starting weapon.
fn worn_shortsword() -> Option<WeaponDamage> {
    let path = crate::test_data::world_db_path();
    let templates = crate::item_templates::load_item_templates(&path).unwrap();
    let tables = crate::item_stats::load_item_scaling_tables(&path).unwrap();
    crate::item_stats::item_stats(
        templates
            .get(shared::item_data::ItemDefinitionSource::Retail, 25)
            .unwrap(),
        &tables,
    )
    .weapon
}

#[test]
fn a_level_10_warrior_with_the_starting_sword_kills_hogger_in_about_133_swings() {
    let Some(sword) = worn_shortsword() else {
        return;
    };
    // ItemSparse ItemLevel 1, ItemDamageOneHand: 0-1 at 2.60 (item_stats).
    assert_eq!(
        sword,
        WeaponDamage {
            min: 0.0,
            max: 1.0,
            speed: 2.6,
            kind: WeaponType::OneHand,
        }
    );
    // Level 10 Human Warrior: Str 33 = AP 33 (AttackPowerPerStrength 1), 5% crit.
    let input = crate::stats::StatInputs {
        class: 1,
        race: 1,
        level: 10,
        primary_stat: Some(crate::stats::PrimaryStat::Strength),
        mastery_coefficient: 0.0,
        attack_power_per_strength: 1.0,
        attack_power_per_agility: 0.0,
        ranged_attack_power_per_agility: 0.0,
        gear_primary: default(),
        gear_ratings: default(),
    };
    let mut stats = crate::stats::compute_character_stats(&input, None).unwrap();
    assert_eq!((stats.attack_power, stats.crit_pct), (33.0, 5.0));
    stats.main_hand = sword;
    let mut app = setup_app();
    let hogger = armored_hogger(&mut app);
    let player = level_10_player(&mut app, hogger, stats);
    let mut swings = 0;
    while app.world().get::<Health>(hogger).unwrap().current > 0.0 && swings < 1_000 {
        app.world_mut().get_mut::<AutoAttack>(player).unwrap().timer = 0.0;
        run_attack_tick(&mut app);
        swings += 1;
    }
    // A swing rolls trunc(0-1 + 33 / 6 × 2.6) = 14-15; Hogger's level 10
    // armor 90 against 210 leaves 9-10. Hogger's pool loses 13,137 / 1,133
    // per point, truncated: 104-115 a hit (78%), 208-231 a crit (5%), 81 a
    // block (3%), 0 on a miss, dodge or parry (5 + 3 + 6%). 98.8 a swing on
    // average: 13,137 / 98.8 = 133 swings (5.8 minutes at 2.6 s), with a
    // standard deviation of about 6.
    assert!((110..=160).contains(&swings), "{swings} swings");
}
