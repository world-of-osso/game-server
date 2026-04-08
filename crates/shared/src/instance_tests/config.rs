use super::*;

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

#[test]
fn reset_timer_none_returns_zero() {
    assert_eq!(ResetTimer::None.duration_secs(), 0);
    assert_eq!(ResetTimer::None.next_reset(1000), None);
}

#[test]
fn reset_timer_daily() {
    assert_eq!(ResetTimer::Daily.duration_secs(), 86_400);
    assert_eq!(ResetTimer::Daily.next_reset(100), Some(86_400));
    assert_eq!(ResetTimer::Daily.next_reset(86_400), Some(172_800));
}

#[test]
fn reset_timer_weekly() {
    assert_eq!(ResetTimer::Weekly.duration_secs(), 604_800);
    assert_eq!(ResetTimer::Weekly.next_reset(100), Some(604_800));
}

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
    assert_eq!(
        supported,
        vec![Difficulty::Normal, Difficulty::Heroic, Difficulty::Mythic]
    );
}

#[test]
fn reset_timer_for_difficulty() {
    let tmpl = sample_dungeon();
    assert_eq!(tmpl.reset_timer(Difficulty::Normal), Some(ResetTimer::None));
    assert_eq!(
        tmpl.reset_timer(Difficulty::Heroic),
        Some(ResetTimer::Daily)
    );
    assert_eq!(
        tmpl.reset_timer(Difficulty::Mythic),
        Some(ResetTimer::Weekly)
    );
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

#[test]
fn mythic_raid_20_players() {
    let tmpl = sample_raid();
    let mythic = tmpl.difficulty_config(Difficulty::Mythic).unwrap();
    assert_eq!(mythic.max_players, 20);
    assert_eq!(mythic.reset_timer, ResetTimer::Weekly);
}
