use super::*;

fn sample_dungeon() -> InstanceTemplate {
    dungeon_template(33, "Shadowfang Keep", 0)
}

fn sample_raid() -> InstanceTemplate {
    raid_template(409, "Molten Core", 230)
}

fn sample_registry() -> InstanceTemplateRegistry {
    let mut reg = InstanceTemplateRegistry::new();
    reg.add(sample_dungeon()).unwrap();
    reg.add(sample_raid()).unwrap();
    reg
}

// --- Difficulty enum ---

#[test]
fn difficulty_config_lookup() {
    let tmpl = sample_dungeon();
    assert!(tmpl.supports_difficulty(Difficulty::Normal));
    assert!(tmpl.supports_difficulty(Difficulty::Heroic));
    assert!(tmpl.supports_difficulty(Difficulty::Mythic));
    assert!(!tmpl.supports_difficulty(Difficulty::MythicPlus));
}

#[test]
fn max_players_by_difficulty() {
    let tmpl = sample_dungeon();
    assert_eq!(tmpl.max_players(Difficulty::Normal), Some(5));
    assert_eq!(tmpl.max_players(Difficulty::Heroic), Some(5));
    assert_eq!(tmpl.max_players(Difficulty::MythicPlus), None);
}

#[test]
fn raid_max_players() {
    let tmpl = sample_raid();
    assert_eq!(tmpl.max_players(Difficulty::Normal), Some(25));
    assert_eq!(tmpl.max_players(Difficulty::Mythic), Some(20));
}

// --- ResetTimer ---

#[test]
fn reset_timer_none_returns_zero() {
    assert_eq!(ResetTimer::None.duration_secs(), 0);
    assert_eq!(ResetTimer::None.next_reset(1000), None);
}

#[test]
fn reset_timer_daily() {
    assert_eq!(ResetTimer::Daily.duration_secs(), 86_400);
    // Acquired at t=100 → next daily reset at 86400
    assert_eq!(ResetTimer::Daily.next_reset(100), Some(86_400));
    // Acquired at t=86400 → next reset at 172800
    assert_eq!(ResetTimer::Daily.next_reset(86_400), Some(172_800));
}

#[test]
fn reset_timer_weekly() {
    assert_eq!(ResetTimer::Weekly.duration_secs(), 604_800);
    assert_eq!(ResetTimer::Weekly.next_reset(100), Some(604_800));
}

// --- InstanceTemplate ---

#[test]
fn dungeon_template_defaults() {
    let tmpl = sample_dungeon();
    assert_eq!(tmpl.map_id, 33);
    assert_eq!(tmpl.name, "Shadowfang Keep");
    assert_eq!(tmpl.instance_type, InstanceType::Dungeon);
    assert_eq!(tmpl.parent_map_id, 0);
    assert!(!tmpl.allow_mount);
    assert_eq!(tmpl.difficulties.len(), 3);
}

#[test]
fn raid_template_defaults() {
    let tmpl = sample_raid();
    assert_eq!(tmpl.instance_type, InstanceType::Raid);
    assert_eq!(tmpl.difficulties.len(), 3);
}

#[test]
fn supported_difficulties_list() {
    let tmpl = sample_dungeon();
    let supported = tmpl.supported_difficulties();
    assert_eq!(supported, vec![Difficulty::Normal, Difficulty::Heroic, Difficulty::Mythic]);
}

#[test]
fn reset_timer_for_difficulty() {
    let tmpl = sample_dungeon();
    assert_eq!(tmpl.reset_timer(Difficulty::Normal), Some(ResetTimer::None));
    assert_eq!(tmpl.reset_timer(Difficulty::Heroic), Some(ResetTimer::Daily));
    assert_eq!(tmpl.reset_timer(Difficulty::Mythic), Some(ResetTimer::Weekly));
}

#[test]
fn creature_multipliers() {
    let tmpl = sample_dungeon();
    let normal = tmpl.difficulty_config(Difficulty::Normal).unwrap();
    assert_eq!(normal.creature_health_multiplier, 1.0);
    assert_eq!(normal.creature_damage_multiplier, 1.0);

    let heroic = tmpl.difficulty_config(Difficulty::Heroic).unwrap();
    assert_eq!(heroic.creature_health_multiplier, 2.0);
    assert_eq!(heroic.creature_damage_multiplier, 1.5);
}

// --- InstanceTemplateRegistry ---

#[test]
fn registry_add_and_get() {
    let mut reg = InstanceTemplateRegistry::new();
    assert!(reg.is_empty());

    reg.add(sample_dungeon()).unwrap();
    assert_eq!(reg.len(), 1);

    let tmpl = reg.get(33).unwrap();
    assert_eq!(tmpl.name, "Shadowfang Keep");
}

#[test]
fn registry_unknown_map() {
    let reg = InstanceTemplateRegistry::new();
    assert!(reg.get(999).is_none());
}

#[test]
fn registry_replace_existing() {
    let mut reg = InstanceTemplateRegistry::new();
    reg.add(dungeon_template(33, "Shadowfang Keep", 0)).unwrap();
    reg.add(dungeon_template(33, "Shadowfang Keep Revamped", 0)).unwrap();
    assert_eq!(reg.len(), 1);
    assert_eq!(reg.get(33).unwrap().name, "Shadowfang Keep Revamped");
}

#[test]
fn registry_rejects_no_difficulties() {
    let mut reg = InstanceTemplateRegistry::new();
    let bad = InstanceTemplate {
        map_id: 1,
        name: "Empty".to_string(),
        instance_type: InstanceType::Dungeon,
        parent_map_id: 0,
        allow_mount: false,
        entrance_pos: WorldPosition { map_id: 1, x: 0.0, y: 0.0, z: 0.0 },
        exit_pos: WorldPosition { map_id: 0, x: 0.0, y: 0.0, z: 0.0 },
        difficulties: vec![],
    };
    assert_eq!(reg.add(bad), Err(InstanceError::NoDifficulties));
    assert!(reg.is_empty());
}

#[test]
fn validate_entry_success() {
    let mut reg = InstanceTemplateRegistry::new();
    reg.add(sample_dungeon()).unwrap();
    let result = reg.validate_entry(33, Difficulty::Normal, 5);
    assert!(result.is_ok());
}

#[test]
fn validate_entry_too_many_players() {
    let mut reg = InstanceTemplateRegistry::new();
    reg.add(sample_dungeon()).unwrap();
    assert_eq!(
        reg.validate_entry(33, Difficulty::Normal, 6),
        Err(InstanceError::TooManyPlayers)
    );
}

#[test]
fn validate_entry_unsupported_difficulty() {
    let mut reg = InstanceTemplateRegistry::new();
    reg.add(sample_dungeon()).unwrap();
    assert_eq!(
        reg.validate_entry(33, Difficulty::MythicPlus, 5),
        Err(InstanceError::UnsupportedDifficulty)
    );
}

#[test]
fn validate_entry_unknown_map() {
    let reg = InstanceTemplateRegistry::new();
    assert_eq!(
        reg.validate_entry(999, Difficulty::Normal, 5),
        Err(InstanceError::UnknownMap)
    );
}

#[test]
fn by_type_filters_correctly() {
    let mut reg = InstanceTemplateRegistry::new();
    reg.add(sample_dungeon()).unwrap();
    reg.add(sample_raid()).unwrap();
    reg.add(dungeon_template(36, "Deadmines", 0)).unwrap();

    let dungeons = reg.by_type(InstanceType::Dungeon);
    assert_eq!(dungeons.len(), 2);

    let raids = reg.by_type(InstanceType::Raid);
    assert_eq!(raids.len(), 1);
    assert_eq!(raids[0].name, "Molten Core");

    let bgs = reg.by_type(InstanceType::Battleground);
    assert!(bgs.is_empty());
}

#[test]
fn registry_iter() {
    let mut reg = InstanceTemplateRegistry::new();
    reg.add(sample_dungeon()).unwrap();
    reg.add(sample_raid()).unwrap();
    let names: Vec<&str> = reg.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"Shadowfang Keep"));
    assert!(names.contains(&"Molten Core"));
}

#[test]
fn mythic_raid_20_players() {
    let tmpl = sample_raid();
    let mythic = tmpl.difficulty_config(Difficulty::Mythic).unwrap();
    // Mythic raids are always 20-player (retail standard)
    assert_eq!(mythic.max_players, 20);
    assert_eq!(mythic.reset_timer, ResetTimer::Weekly);
}

// --- Instance (runtime) ---

#[test]
fn instance_add_remove_player() {
    let mut inst = Instance {
        instance_id: 1,
        map_id: 33,
        difficulty: Difficulty::Normal,
        group_leader: 100,
        players: vec![100, 101],
        spawned_entities: Vec::new(),
        created_at: 1000,
    };
    assert!(inst.contains_player(100));
    assert!(!inst.contains_player(200));

    inst.add_player(102, 5).unwrap();
    assert!(inst.contains_player(102));
    assert_eq!(inst.players.len(), 3);

    // Adding duplicate is a no-op
    inst.add_player(100, 5).unwrap();
    assert_eq!(inst.players.len(), 3);

    assert!(inst.remove_player(101));
    assert!(!inst.contains_player(101));
    assert!(!inst.remove_player(999));
}

#[test]
fn instance_full_rejects_player() {
    let mut inst = Instance {
        instance_id: 1,
        map_id: 33,
        difficulty: Difficulty::Normal,
        group_leader: 100,
        players: vec![100, 101, 102, 103, 104],
        spawned_entities: Vec::new(),
        created_at: 1000,
    };
    assert_eq!(inst.add_player(105, 5), Err(InstanceError::InstanceFull));
}

#[test]
fn instance_empty_after_all_leave() {
    let mut inst = Instance {
        instance_id: 1,
        map_id: 33,
        difficulty: Difficulty::Normal,
        group_leader: 100,
        players: vec![100],
        spawned_entities: Vec::new(),
        created_at: 1000,
    };
    assert!(!inst.is_empty());
    inst.remove_player(100);
    assert!(inst.is_empty());
}

// --- InstanceManager ---

#[test]
fn create_instance_assigns_sequential_ids() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id1 = mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &[100, 101, 102], 1000).unwrap();
    let id2 = mgr.create_instance(&reg, 33, Difficulty::Normal, 200, &[200, 201, 202], 1001).unwrap();

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(mgr.len(), 2);
}

#[test]
fn create_instance_validates_template() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    // Unknown map
    assert_eq!(
        mgr.create_instance(&reg, 999, Difficulty::Normal, 100, &[100], 1000),
        Err(InstanceError::UnknownMap)
    );

    // Unsupported difficulty
    assert_eq!(
        mgr.create_instance(&reg, 33, Difficulty::MythicPlus, 100, &[100], 1000),
        Err(InstanceError::UnsupportedDifficulty)
    );

    // Too many players (dungeon max 5)
    let six = [100, 101, 102, 103, 104, 105];
    assert_eq!(
        mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &six, 1000),
        Err(InstanceError::TooManyPlayers)
    );
}

#[test]
fn duplicate_group_instance_rejected() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &[100, 101], 1000).unwrap();
    assert_eq!(
        mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &[100, 101], 1001),
        Err(InstanceError::AlreadyExists)
    );
}

#[test]
fn same_group_different_difficulty_allowed() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 1000).unwrap();
    mgr.create_instance(&reg, 33, Difficulty::Heroic, 100, &[100], 1001).unwrap();
    assert_eq!(mgr.len(), 2);
}

#[test]
fn get_or_create_returns_existing() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id1 = mgr.get_or_create(&reg, 33, Difficulty::Normal, 100, &[100, 101], 1000).unwrap();
    let id2 = mgr.get_or_create(&reg, 33, Difficulty::Normal, 100, &[100, 101], 2000).unwrap();
    assert_eq!(id1, id2);
    assert_eq!(mgr.len(), 1);
}

#[test]
fn get_or_create_creates_new() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr.get_or_create(&reg, 33, Difficulty::Normal, 100, &[100], 1000).unwrap();
    assert_eq!(mgr.get(id).unwrap().map_id, 33);
}

#[test]
fn destroy_instance() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 1000).unwrap();
    let removed = mgr.destroy(id).unwrap();
    assert_eq!(removed.instance_id, id);
    assert!(mgr.is_empty());
    assert!(mgr.get(id).is_none());

    // Group can now create a new instance for the same map
    mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 2000).unwrap();
}

#[test]
fn destroy_nonexistent_returns_none() {
    let mut mgr = InstanceManager::new();
    assert!(mgr.destroy(99).is_none());
}

#[test]
fn cleanup_empty_instances() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id1 = mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 1000).unwrap();
    let id2 = mgr.create_instance(&reg, 33, Difficulty::Normal, 200, &[200], 1001).unwrap();

    // Empty out instance 1
    mgr.get_mut(id1).unwrap().remove_player(100);

    let removed = mgr.cleanup_empty();
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].instance_id, id1);
    assert_eq!(mgr.len(), 1);
    assert!(mgr.get(id2).is_some());
}

#[test]
fn find_player_instance() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &[100, 101], 1000).unwrap();
    assert_eq!(mgr.find_player_instance(101), Some(id));
    assert_eq!(mgr.find_player_instance(999), None);
}

#[test]
fn find_group_instance() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr.create_instance(&reg, 33, Difficulty::Heroic, 100, &[100], 1000).unwrap();
    assert_eq!(mgr.find_group_instance(100, 33, Difficulty::Heroic), Some(id));
    assert_eq!(mgr.find_group_instance(100, 33, Difficulty::Normal), None);
    assert_eq!(mgr.find_group_instance(200, 33, Difficulty::Heroic), None);
}

#[test]
fn instance_stores_creation_metadata() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr.create_instance(&reg, 409, Difficulty::Mythic, 300, &[300, 301], 5000).unwrap();
    let inst = mgr.get(id).unwrap();
    assert_eq!(inst.map_id, 409);
    assert_eq!(inst.difficulty, Difficulty::Mythic);
    assert_eq!(inst.group_leader, 300);
    assert_eq!(inst.players, vec![300, 301]);
    assert_eq!(inst.created_at, 5000);
}

#[test]
fn manager_iter() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 1000).unwrap();
    mgr.create_instance(&reg, 409, Difficulty::Normal, 200, &[200], 1001).unwrap();

    let map_ids: Vec<u16> = mgr.iter().map(|(_, inst)| inst.map_id).collect();
    assert_eq!(map_ids.len(), 2);
    assert!(map_ids.contains(&33));
    assert!(map_ids.contains(&409));
}

// --- Per-group isolation ---

#[test]
fn same_instance_both_open_world() {
    assert!(same_instance(None, None));
}

#[test]
fn same_instance_both_in_same() {
    let a = InstanceId(5);
    let b = InstanceId(5);
    assert!(same_instance(Some(&a), Some(&b)));
}

#[test]
fn different_instance_ids() {
    let a = InstanceId(1);
    let b = InstanceId(2);
    assert!(!same_instance(Some(&a), Some(&b)));
}

#[test]
fn open_world_vs_instanced_not_visible() {
    let inst = InstanceId(1);
    assert!(!same_instance(None, Some(&inst)));
    assert!(!same_instance(Some(&inst), None));
}

#[test]
fn two_groups_get_separate_instances() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let group_a = [100, 101, 102, 103, 104];
    let group_b = [200, 201, 202, 203, 204];

    let id_a = mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &group_a, 1000).unwrap();
    let id_b = mgr.create_instance(&reg, 33, Difficulty::Normal, 200, &group_b, 1001).unwrap();

    // Different instance IDs for the same map
    assert_ne!(id_a, id_b);

    let inst_a = mgr.get(id_a).unwrap();
    let inst_b = mgr.get(id_b).unwrap();

    // Same map, but isolated
    assert_eq!(inst_a.map_id, inst_b.map_id);
    assert_ne!(inst_a.instance_id, inst_b.instance_id);

    // Component IDs differ, so entities won't see each other
    assert!(!same_instance(Some(&inst_a.component_id()), Some(&inst_b.component_id())));

    // Players within the same instance share visibility
    assert!(same_instance(Some(&inst_a.component_id()), Some(&inst_a.component_id())));
}

#[test]
fn track_and_untrack_spawned_entities() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 1000).unwrap();
    let inst = mgr.get_mut(id).unwrap();

    // Spawn creatures for this instance
    inst.track_entity(5000);
    inst.track_entity(5001);
    inst.track_entity(5002);
    assert_eq!(inst.spawned_entities.len(), 3);

    // Duplicate tracking is a no-op
    inst.track_entity(5000);
    assert_eq!(inst.spawned_entities.len(), 3);

    // Untrack on despawn
    inst.untrack_entity(5001);
    assert_eq!(inst.spawned_entities.len(), 2);
    assert!(!inst.spawned_entities.contains(&5001));
}

#[test]
fn component_id_matches_instance_id() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 1000).unwrap();
    let inst = mgr.get(id).unwrap();
    assert_eq!(inst.component_id(), InstanceId(id));
}

#[test]
fn destroyed_instance_entities_isolated_from_new() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id1 = mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 1000).unwrap();
    let comp1 = mgr.get(id1).unwrap().component_id();

    mgr.destroy(id1);

    // Same group creates a new instance — gets a new ID
    let id2 = mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 2000).unwrap();
    let comp2 = mgr.get(id2).unwrap().component_id();

    // Old entities (with comp1) won't match new instance
    assert_ne!(comp1, comp2);
    assert!(!same_instance(Some(&comp1), Some(&comp2)));
}

// --- Difficulty scaling ---

fn base_creature() -> CreatureBaseStats {
    CreatureBaseStats {
        health: 10000.0,
        damage_min: 100.0,
        damage_max: 200.0,
    }
}

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

    // Raid mythic: 2.5x health, 2.0x damage
    assert_eq!(scaled.health, 25000.0);
    assert_eq!(scaled.damage_min, 200.0);
    assert_eq!(scaled.damage_max, 400.0);
}

#[test]
fn loot_variant_for_each_difficulty() {
    let tmpl = sample_dungeon();
    assert_eq!(loot_variant_for_difficulty(&tmpl, Difficulty::Normal), Some(0));
    assert_eq!(loot_variant_for_difficulty(&tmpl, Difficulty::Heroic), Some(1));
    assert_eq!(loot_variant_for_difficulty(&tmpl, Difficulty::Mythic), Some(2));
    assert_eq!(loot_variant_for_difficulty(&tmpl, Difficulty::MythicPlus), None);
}

#[test]
fn manager_scale_creature() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr.create_instance(&reg, 33, Difficulty::Heroic, 100, &[100], 1000).unwrap();
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

    let id = mgr.create_instance(&reg, 409, Difficulty::Mythic, 100, &[100], 1000).unwrap();
    let config = mgr.difficulty_config(id, &reg).unwrap();

    assert_eq!(config.difficulty, Difficulty::Mythic);
    assert_eq!(config.max_players, 20);
    assert_eq!(config.creature_health_multiplier, 2.5);
}

include!("instance_reset_tests.rs");
