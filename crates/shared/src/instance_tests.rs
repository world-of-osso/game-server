use super::*;

fn sample_dungeon() -> InstanceTemplate {
    dungeon_template(33, "Shadowfang Keep", 0)
}

fn sample_raid() -> InstanceTemplate {
    raid_template(409, "Molten Core", 230)
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
