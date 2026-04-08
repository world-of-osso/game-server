use super::*;

#[test]
fn normal_scaling_is_identity() {
    let tmpl = sample_dungeon();
    let config = tmpl.difficulty_config(Difficulty::Normal).unwrap();
    let scaled = scale_creature_stats(&base_creature(), config);

    assert_eq!(scaled.health, 10000.0);
    assert_eq!(scaled.damage_min, 100.0);
    assert_eq!(scaled.damage_max, 200.0);
    assert_eq!(scaled.loot_table_variant, 0);
}

#[test]
fn heroic_doubles_health() {
    let tmpl = sample_dungeon();
    let config = tmpl.difficulty_config(Difficulty::Heroic).unwrap();
    let scaled = scale_creature_stats(&base_creature(), config);

    assert_eq!(scaled.health, 20000.0);
    assert_eq!(scaled.damage_min, 150.0);
    assert_eq!(scaled.damage_max, 300.0);
    assert_eq!(scaled.loot_table_variant, 1);
}

#[test]
fn mythic_triples_health() {
    let tmpl = sample_dungeon();
    let config = tmpl.difficulty_config(Difficulty::Mythic).unwrap();
    let scaled = scale_creature_stats(&base_creature(), config);

    assert_eq!(scaled.health, 30000.0);
    assert_eq!(scaled.damage_min, 200.0);
    assert_eq!(scaled.damage_max, 400.0);
    assert_eq!(scaled.loot_table_variant, 2);
}

#[test]
fn raid_mythic_scaling() {
    let tmpl = sample_raid();
    let config = tmpl.difficulty_config(Difficulty::Mythic).unwrap();
    let scaled = scale_creature_stats(&base_creature(), config);

    assert_eq!(scaled.health, 25000.0);
    assert_eq!(scaled.damage_min, 200.0);
    assert_eq!(scaled.damage_max, 400.0);
}

#[test]
fn loot_variant_for_each_difficulty() {
    let tmpl = sample_dungeon();
    assert_eq!(
        loot_variant_for_difficulty(&tmpl, Difficulty::Normal),
        Some(0)
    );
    assert_eq!(
        loot_variant_for_difficulty(&tmpl, Difficulty::Heroic),
        Some(1)
    );
    assert_eq!(
        loot_variant_for_difficulty(&tmpl, Difficulty::Mythic),
        Some(2)
    );
    assert_eq!(
        loot_variant_for_difficulty(&tmpl, Difficulty::MythicPlus),
        None
    );
}

#[test]
fn manager_scale_creature() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr
        .create_instance(&reg, 33, Difficulty::Heroic, 100, &[100], 1000)
        .unwrap();
    let scaled = mgr.scale_creature(id, &reg, &base_creature()).unwrap();

    assert_eq!(scaled.health, 20000.0);
    assert_eq!(scaled.loot_table_variant, 1);
}

#[test]
fn manager_scale_creature_unknown_instance() {
    let reg = sample_registry();
    let mgr = InstanceManager::new();

    assert!(mgr.scale_creature(999, &reg, &base_creature()).is_none());
}

#[test]
fn manager_difficulty_config() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr
        .create_instance(&reg, 409, Difficulty::Mythic, 100, &[100], 1000)
        .unwrap();
    let config = mgr.difficulty_config(id, &reg).unwrap();

    assert_eq!(config.difficulty, Difficulty::Mythic);
    assert_eq!(config.max_players, 20);
    assert_eq!(config.creature_health_multiplier, 2.5);
}
