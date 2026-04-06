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

// --- Interaction ---

fn make_go(tmpl: &GameObjectTemplate) -> GameObject {
    GameObject::from_template(tmpl, NO_ROTATION)
}

#[test]
fn interact_chest_loots() {
    let mut go = make_go(&chest_template());
    let result = interact(&mut go).unwrap();
    assert_eq!(result, InteractionResult::LootChest { template_id: 1001 });
    assert!(go.is_in_use());
}

#[test]
fn interact_door_toggles() {
    let mut tmpl = door_template();
    tmpl.flags.remove(GameObjectFlags::LOCKED); // unlock it
    let mut go = make_go(&tmpl);
    let result = interact(&mut go).unwrap();
    assert!(matches!(result, InteractionResult::ToggleDoor { now_open: true, .. }));
}

#[test]
fn interact_mining_node_gathers() {
    let mut go = make_go(&mining_template());
    let result = interact(&mut go).unwrap();
    assert!(matches!(result, InteractionResult::GatherNode { node_type: GameObjectType::MiningNode, .. }));
}

#[test]
fn interact_herb_node() {
    let tmpl = GameObjectTemplate {
        id: 3002, object_type: GameObjectType::HerbNode, display_id: 701,
        name: "Peacebloom".to_string(), faction: 0, flags: GameObjectFlags::INTERACTABLE,
    };
    let mut go = make_go(&tmpl);
    let result = interact(&mut go).unwrap();
    assert!(matches!(result, InteractionResult::GatherNode { node_type: GameObjectType::HerbNode, .. }));
}

#[test]
fn interact_quest_object() {
    let tmpl = GameObjectTemplate {
        id: 4001, object_type: GameObjectType::QuestObject, display_id: 800,
        name: "Ancient Rune".to_string(), faction: 0, flags: GameObjectFlags::INTERACTABLE,
    };
    let mut go = make_go(&tmpl);
    let result = interact(&mut go).unwrap();
    assert_eq!(result, InteractionResult::QuestObjective { template_id: 4001 });
}

#[test]
fn interact_trap_triggers() {
    let tmpl = GameObjectTemplate {
        id: 5001, object_type: GameObjectType::Trap, display_id: 900,
        name: "Bear Trap".to_string(), faction: 0, flags: GameObjectFlags::INTERACTABLE,
    };
    let mut go = make_go(&tmpl);
    let result = interact(&mut go).unwrap();
    assert_eq!(result, InteractionResult::TriggerTrap { template_id: 5001 });
}

#[test]
fn interact_mailbox_opens() {
    let tmpl = GameObjectTemplate {
        id: 6001, object_type: GameObjectType::Mailbox, display_id: 1000,
        name: "Mailbox".to_string(), faction: 0, flags: GameObjectFlags::INTERACTABLE,
    };
    let mut go = make_go(&tmpl);
    assert_eq!(interact(&mut go).unwrap(), InteractionResult::OpenMailbox);
}

#[test]
fn interact_forge() {
    let tmpl = GameObjectTemplate {
        id: 7001, object_type: GameObjectType::Forge, display_id: 1100,
        name: "Forge".to_string(), faction: 0, flags: GameObjectFlags::INTERACTABLE,
    };
    let mut go = make_go(&tmpl);
    let result = interact(&mut go).unwrap();
    assert!(matches!(result, InteractionResult::UseCraftingStation { station_type: GameObjectType::Forge, .. }));
}

#[test]
fn interact_not_interactable() {
    let tmpl = GameObjectTemplate {
        id: 1001, object_type: GameObjectType::Chest, display_id: 500,
        name: "Broken Chest".to_string(), faction: 0, flags: GameObjectFlags::default(),
    };
    let mut go = make_go(&tmpl);
    assert_eq!(interact(&mut go), Err(InteractionError::NotInteractable));
}

#[test]
fn interact_locked_fails() {
    let mut go = make_go(&door_template()); // locked by default
    assert_eq!(interact(&mut go), Err(InteractionError::Locked));
}

#[test]
fn interact_in_use_fails() {
    let mut go = make_go(&chest_template());
    interact(&mut go).unwrap(); // sets IN_USE
    assert_eq!(interact(&mut go), Err(InteractionError::InUse));
}

#[test]
fn finish_interaction_clears_in_use() {
    let mut go = make_go(&chest_template());
    interact(&mut go).unwrap();
    assert!(go.is_in_use());
    finish_interaction(&mut go);
    assert!(!go.is_in_use());
}

#[test]
fn can_reuse_after_finish() {
    let mut go = make_go(&chest_template());
    interact(&mut go).unwrap();
    finish_interaction(&mut go);
    // Can interact again
    assert!(interact(&mut go).is_ok());
}
