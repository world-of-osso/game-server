use super::*;

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
    reg.add(dungeon_template(33, "Shadowfang Keep Revamped", 0))
        .unwrap();
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
        entrance_pos: WorldPosition {
            map_id: 1,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        exit_pos: WorldPosition {
            map_id: 0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        difficulties: vec![],
    };
    assert_eq!(reg.add(bad), Err(InstanceError::NoDifficulties));
    assert!(reg.is_empty());
}

#[test]
fn validate_entry_success() {
    let mut reg = InstanceTemplateRegistry::new();
    reg.add(sample_dungeon()).unwrap();
    assert!(reg.validate_entry(33, Difficulty::Normal, 5).is_ok());
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
