use super::*;

fn chest_template() -> GameObjectTemplate {
    GameObjectTemplate {
        id: 1001,
        object_type: GameObjectType::Chest,
        display_id: 500,
        name: "Treasure Chest".to_string(),
        faction: 0,
        flags: GameObjectFlags::INTERACTABLE,
    }
}

fn door_template() -> GameObjectTemplate {
    GameObjectTemplate {
        id: 2001,
        object_type: GameObjectType::Door,
        display_id: 600,
        name: "Iron Gate".to_string(),
        faction: 0,
        flags: GameObjectFlags(GameObjectFlags::INTERACTABLE.0 | GameObjectFlags::LOCKED.0),
    }
}

fn mining_template() -> GameObjectTemplate {
    GameObjectTemplate {
        id: 3001,
        object_type: GameObjectType::MiningNode,
        display_id: 700,
        name: "Copper Vein".to_string(),
        faction: 0,
        flags: GameObjectFlags::INTERACTABLE,
    }
}

const NO_ROTATION: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

// --- GameObjectFlags ---

#[test]
fn flags_contains() {
    let flags = GameObjectFlags(0x05); // INTERACTABLE | LOCKED
    assert!(flags.contains(GameObjectFlags::INTERACTABLE));
    assert!(flags.contains(GameObjectFlags::LOCKED));
    assert!(!flags.contains(GameObjectFlags::IN_USE));
}

#[test]
fn flags_insert_remove() {
    let mut flags = GameObjectFlags::default();
    assert!(!flags.contains(GameObjectFlags::INTERACTABLE));

    flags.insert(GameObjectFlags::INTERACTABLE);
    assert!(flags.contains(GameObjectFlags::INTERACTABLE));

    flags.remove(GameObjectFlags::INTERACTABLE);
    assert!(!flags.contains(GameObjectFlags::INTERACTABLE));
}

// --- GameObject from template ---

#[test]
fn from_template() {
    let tmpl = chest_template();
    let go = GameObject::from_template(&tmpl, NO_ROTATION);

    assert_eq!(go.template_id, 1001);
    assert_eq!(go.object_type, GameObjectType::Chest);
    assert_eq!(go.display_id, 500);
    assert_eq!(go.faction, 0);
    assert!(go.is_interactable());
    assert!(!go.is_locked());
    assert!(!go.is_in_use());
}

#[test]
fn locked_door() {
    let tmpl = door_template();
    let go = GameObject::from_template(&tmpl, NO_ROTATION);

    assert!(go.is_interactable());
    assert!(go.is_locked());
    assert_eq!(go.object_type, GameObjectType::Door);
}

#[test]
fn rotation_preserved() {
    let tmpl = chest_template();
    let rot = [0.1, 0.2, 0.3, 0.9];
    let go = GameObject::from_template(&tmpl, rot);
    assert_eq!(go.rotation, rot);
}

// --- GameObjectRegistry ---

#[test]
fn registry_add_and_get() {
    let mut reg = GameObjectRegistry::new();
    reg.add(chest_template());
    reg.add(door_template());
    assert_eq!(reg.len(), 2);

    let chest = reg.get(1001).unwrap();
    assert_eq!(chest.name, "Treasure Chest");
}

#[test]
fn registry_replace_existing() {
    let mut reg = GameObjectRegistry::new();
    reg.add(chest_template());
    let mut updated = chest_template();
    updated.name = "Golden Chest".to_string();
    reg.add(updated);
    assert_eq!(reg.len(), 1);
    assert_eq!(reg.get(1001).unwrap().name, "Golden Chest");
}

#[test]
fn registry_unknown_id() {
    let reg = GameObjectRegistry::new();
    assert!(reg.get(999).is_none());
}

#[test]
fn registry_by_type() {
    let mut reg = GameObjectRegistry::new();
    reg.add(chest_template());
    reg.add(door_template());
    reg.add(mining_template());

    let chests = reg.by_type(GameObjectType::Chest);
    assert_eq!(chests.len(), 1);

    let doors = reg.by_type(GameObjectType::Door);
    assert_eq!(doors.len(), 1);

    let traps = reg.by_type(GameObjectType::Trap);
    assert!(traps.is_empty());
}

#[test]
fn all_object_types_distinct() {
    let types = [
        GameObjectType::Chest,
        GameObjectType::Door,
        GameObjectType::QuestObject,
        GameObjectType::MiningNode,
        GameObjectType::HerbNode,
        GameObjectType::Trap,
        GameObjectType::Mailbox,
        GameObjectType::Forge,
        GameObjectType::Anvil,
    ];
    // All unique
    for (i, a) in types.iter().enumerate() {
        for (j, b) in types.iter().enumerate() {
            if i != j {
                assert_ne!(a, b);
            }
        }
    }
}
