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
fn attack_power_multiplier_also_scales_the_weapon_roll() {
    let _seed = crate::random::seed_scope(70_448);
    let mut app = setup_app();
    let target = spawn_target(&mut app, pos(1.0, 0.0, 0.0), 10_000.0);
    let attacker = level_10_player(
        &mut app,
        target,
        CharacterStats {
            attack_power: 60.0,
            main_hand: HEAVY_BLADE,
            ..default()
        },
    );
    app.world_mut()
        .entity_mut(attacker)
        .insert(crate::auras::AuraStatMods {
            attack_power_pct: 30.0,
            ..default()
        });
    run_attack_tick(&mut app);
    // AP is already scaled in CharacterStats: 100 * 1.3 + 60 / 6 * 2.6 = 156.
    assert_eq!(app.world().get::<Health>(target).unwrap().current, 9_844.0);
}

#[test]
fn source_bound_absorb_is_consumed_once_by_real_dual_wield_swings() {
    let _seed = crate::random::seed_scope(70448);
    let mut app = setup_app();
    let target = spawn_target(&mut app, pos(1.0, 0.0, 0.0), 10_000.0);
    let attacker = level_10_player(
        &mut app,
        target,
        CharacterStats {
            main_hand: HEAVY_BLADE,
            off_hand: Some(HEAVY_BLADE),
            ..default()
        },
    );
    crate::auras::source_absorb::grant(app.world_mut(), target, attacker, 204301, 40.0, 10_000);
    run_attack_tick(&mut app);
    assert_eq!(app.world().get::<Health>(target).unwrap().current, 9890.0);
}

#[test]
fn the_swing_timer_is_the_weapon_speed_over_haste() {
    for (haste_pct, expected_millis) in [(0.0, 2600), (30.0, 2000)] {
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
        assert_eq!(
            timer,
            std::time::Duration::from_millis(expected_millis),
            "haste {haste_pct}"
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
fn both_hands_swing_when_due_without_delaying_each_other() {
    let _seed = crate::random::seed_scope(70448);
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
    assert_eq!(
        app.world().get::<Health>(target).unwrap().current,
        9850.0,
        "both due hands must deal their native 100 and 50 damage"
    );
    let auto = app.world().get::<AutoAttack>(player).unwrap();
    assert_eq!(auto.timer, std::time::Duration::from_millis(2600));
    assert_eq!(auto.off_hand_timer, std::time::Duration::from_millis(2600));
}

#[test]
fn swing_clocks_preserve_fractional_tick_remainder_per_hand() {
    let mut app = setup_app();
    let target = spawn_target(&mut app, pos(1.0, 0.0, 0.0), 1_000_000.0);
    let player = level_10_player(
        &mut app,
        target,
        CharacterStats {
            main_hand: WeaponDamage {
                speed: 0.63,
                ..HEAVY_BLADE
            },
            off_hand: Some(WeaponDamage {
                speed: 0.77,
                ..HEAVY_BLADE
            }),
            ..default()
        },
    );
    auto_attack_system(app.world_mut());
    let mut counts = [1, 1];
    for _ in 0..200 {
        let before = app.world().get::<AutoAttack>(player).unwrap().clone();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        auto_attack_system(app.world_mut());
        let after = app.world().get::<AutoAttack>(player).unwrap();
        counts[0] += u32::from(after.timer > before.timer);
        counts[1] += u32::from(after.off_hand_timer > before.off_hand_timer);
    }
    assert_eq!(
        counts,
        [16, 13],
        "ten seconds must follow each weapon's continuous deadline"
    );
}

#[test]
fn swing_ready_while_out_of_range_does_not_accumulate_credit() {
    let mut app = setup_app();
    let target = spawn_target(&mut app, pos(100.0, 0.0, 0.0), 1_000_000.0);
    let player = level_10_player(
        &mut app,
        target,
        CharacterStats {
            main_hand: HEAVY_BLADE,
            ..default()
        },
    );
    for _ in 0..4 {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        auto_attack_system(app.world_mut());
    }
    *app.world_mut().get_mut::<Position>(target).unwrap() = pos(1.0, 0.0, 0.0);
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(50));
    auto_attack_system(app.world_mut());
    assert_eq!(
        app.world().get::<AutoAttack>(player).unwrap().timer,
        std::time::Duration::from_millis(2600)
    );
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
        app.world_mut().get_mut::<AutoAttack>(player).unwrap().timer = std::time::Duration::ZERO;
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
