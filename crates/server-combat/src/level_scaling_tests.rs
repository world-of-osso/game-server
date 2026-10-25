use super::*;

/// ExpectedStat rows 10 and 30 (ExpansionID -2) of 12.1.0.69933, plus a
/// made-up Midnight (11) level 30 row and ContentTuning 999 mod. Mods 4
/// (warrior) and 1 (mage) are the real ExpectedStatMod rows.
fn expected_stats() -> ExpectedStats {
    ExpectedStats {
        by_level: HashMap::from([
            (
                (10, DEFAULT_EXPANSION),
                CreatureStats {
                    health: 377.3423,
                    auto_attack_dps: 17.05213,
                    armor: 90.0,
                    armor_constant: 210.0,
                },
            ),
            (
                (30, DEFAULT_EXPANSION),
                CreatureStats {
                    health: 4378.775,
                    auto_attack_dps: 162.6229,
                    armor: 155.0,
                    armor_constant: 361.0,
                },
            ),
            (
                (30, 11),
                CreatureStats {
                    health: 5000.0,
                    auto_attack_dps: 200.0,
                    armor: 200.0,
                    armor_constant: 400.0,
                },
            ),
        ]),
        mods: HashMap::from([
            (
                4,
                CreatureStats {
                    health: 1.0,
                    auto_attack_dps: 1.0,
                    armor: 1.0,
                    armor_constant: 1.0,
                },
            ),
            (
                1,
                CreatureStats {
                    health: 0.8,
                    auto_attack_dps: 0.925,
                    armor: 0.8,
                    armor_constant: 1.0,
                },
            ),
            (
                50,
                CreatureStats {
                    health: 2.0,
                    auto_attack_dps: 0.5,
                    armor: 1.0,
                    armor_constant: 1.0,
                },
            ),
        ]),
        content_tuning_mods: HashMap::from([(999, vec![50])]),
    }
}

/// Hogger (448): ContentTuning 73 (1-30), HealthScalingExpansion 0, warrior,
/// HealthModifier 3, DamageModifier 1.7.
const HOGGER: TuningInputs = TuningInputs {
    content_tuning_id: 73,
    health_scaling_expansion: 0,
    unit_class: 1,
    health_modifier: 3.0,
    damage_modifier: 1.7,
    armor_modifier: 1.0,
};

pub(crate) const HOGGER_SCALING: LevelScaling = LevelScaling {
    content_tuning_id: 73,
    min_level: 1,
    max_level: 30,
    delta: 0,
};

pub(crate) fn hogger_stats() -> ScaledStats {
    ScaledStats(Arc::new(LevelStats::build(&expected_stats(), &HOGGER)))
}

#[test]
fn max_health_is_expected_stat_health_times_health_modifier_rounded_up() {
    let stats = LevelStats::build(&expected_stats(), &HOGGER);
    assert_eq!(stats.max_health(30), 13_137.0);
    assert_eq!(stats.max_health(10), 1_133.0);
}

#[test]
fn level_without_a_row_evaluates_to_one() {
    let stats = LevelStats::build(&expected_stats(), &HOGGER);
    assert_eq!(stats.max_health(20), 3.0);
}

#[test]
fn current_expansion_uses_its_own_row_before_the_default() {
    let current = TuningInputs {
        health_scaling_expansion: -1,
        health_modifier: 1.0,
        ..HOGGER
    };
    let stats = LevelStats::build(&expected_stats(), &current);
    assert_eq!(stats.max_health(30), 5_000.0);
    assert_eq!(stats.max_health(10), 378.0, "no Midnight row at 10");
}

#[test]
fn content_tuning_and_class_mods_multiply() {
    let mage = TuningInputs {
        content_tuning_id: 999,
        unit_class: 8,
        health_modifier: 1.0,
        ..HOGGER
    };
    let stats = LevelStats::build(&expected_stats(), &mage);
    assert_eq!(stats.max_health(30), (4378.775f32 * 2.0 * 0.8).ceil());
    assert_eq!(stats.base_damage(30), 162.6229 * 0.5 * 0.925);
}

#[test]
fn swing_damage_is_dps_times_damage_modifier_times_attack_time() {
    let stats = LevelStats::build(&expected_stats(), &HOGGER);
    let (min, max) = stats.swing_damage_range(30, 2.0);
    assert!((min - 162.6229 * 1.7 * 2.0).abs() < 0.01, "{min}");
    assert!((max - min * 1.5).abs() < 0.01, "{max}");
}

fn player(level: u8) -> UnitLevel {
    UnitLevel(level)
}

#[test]
fn level_10_player_takes_hogger_down_as_a_level_10_hogger() {
    let stats = ScaledStats(Arc::new(LevelStats::build(&expected_stats(), &HOGGER)));
    let hogger =
        UnitScaling::from_item((Some(&UnitLevel(30)), Some(&HOGGER_SCALING), Some(&stats)));
    let level_10 = player(10);
    let player = UnitScaling::from_item((Some(&level_10), None, None));
    assert_eq!(hogger.level_against(&player), 10);
    assert_eq!(player.level_against(&hogger), 10);
    // 10 of a level 10 Hogger's 1,133 health is 115 of its 13,137.
    assert_eq!(damage_dealt(&player, &hogger, 10.0), 10.0);
    assert_eq!(health_loss(&player, &hogger, 10.0), 115.0);
    // A 552 swing at level 30 is 57 at level 10.
    assert_eq!(damage_dealt(&hogger, &player, 552.0), 57.0);
    assert_eq!(health_loss(&hogger, &player, 57.0), 57.0);
}

#[test]
fn level_40_player_fights_hogger_at_its_native_level() {
    let stats = ScaledStats(Arc::new(LevelStats::build(&expected_stats(), &HOGGER)));
    let hogger =
        UnitScaling::from_item((Some(&UnitLevel(30)), Some(&HOGGER_SCALING), Some(&stats)));
    let level_40 = player(40);
    let player = UnitScaling::from_item((Some(&level_40), None, None));
    assert_eq!(hogger.level_against(&player), 30);
    assert_eq!(health_loss(&player, &hogger, 10.0), 10.0);
    assert_eq!(damage_dealt(&hogger, &player, 552.0), 552.0);
}

#[test]
fn untuned_units_deal_and_take_damage_unscaled() {
    let wolf_level = UnitLevel(7);
    let wolf = UnitScaling::from_item((Some(&wolf_level), None, None));
    let level_40 = player(40);
    let player = UnitScaling::from_item((Some(&level_40), None, None));
    assert_eq!(wolf.level_against(&player), 7);
    assert_eq!(health_loss(&player, &wolf, 10.0), 10.0);
    assert_eq!(damage_dealt(&wolf, &player, 10.0), 10.0);
}

#[test]
fn tuned_creature_armor_is_expected_stat_armor_at_the_attackers_level() {
    let stats = ScaledStats(Arc::new(LevelStats::build(&expected_stats(), &HOGGER)));
    let hogger =
        UnitScaling::from_item((Some(&UnitLevel(30)), Some(&HOGGER_SCALING), Some(&stats)));
    let (level_10, level_40) = (player(10), player(40));
    let own_armor = stats.0.armor(30);
    assert_eq!(own_armor, 155.0);
    let at = |level| UnitScaling::from_item((Some(level), None, None));
    assert_eq!(hogger.armor_against(own_armor, &at(&level_10)), 90.0);
    assert_eq!(hogger.armor_against(own_armor, &at(&level_40)), 155.0);
}

#[test]
fn untuned_armor_and_armor_constants_come_from_the_default_expansion_rows() {
    let stats = expected_stats();
    // Mage class mod 1: CreatureArmorMod 0.8; ArmorModifier 2.
    assert_eq!(stats.untuned_armor(10, 8, 2.0), 90.0 * 0.8 * 2.0);
    let constants = stats.armor_constants();
    assert_eq!(
        (constants.at(10), constants.at(30)),
        (Some(210.0), Some(361.0))
    );
}
