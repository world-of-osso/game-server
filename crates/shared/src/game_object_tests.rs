use super::*;
use crate::game_object_handlers::*;

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
fn cancel_interaction_returns_to_ready() {
    let mut go = make_go(&chest_template());
    interact(&mut go).unwrap();
    assert!(go.is_in_use());
    cancel_interaction(&mut go).unwrap();
    assert!(!go.is_in_use());
    assert_eq!(go.state, GameObjectState::Ready);
}

#[test]
fn can_reuse_after_cancel() {
    let mut go = make_go(&chest_template());
    interact(&mut go).unwrap();
    cancel_interaction(&mut go).unwrap();
    assert!(interact(&mut go).is_ok());
}

// --- Traps ---

fn trap_template() -> GameObjectTemplate {
    GameObjectTemplate {
        id: 5001, object_type: GameObjectType::Trap, display_id: 900,
        name: "Bear Trap".to_string(), faction: 0, flags: GameObjectFlags::INTERACTABLE,
    }
}

fn one_shot_trap() -> TrapData {
    TrapData { spell_id: 100, radius: 5.0, cooldown: 0 }
}

fn cooldown_trap() -> TrapData {
    TrapData { spell_id: 200, radius: 10.0, cooldown: 30 }
}

#[test]
fn trap_triggers_nearby_entities() {
    let mut go = make_go(&trap_template());
    let nearby = [(1, 2.0, 0.0, 0.0), (2, 3.0, 0.0, 0.0), (3, 100.0, 0.0, 0.0)];
    let result = trigger_trap(&mut go, &one_shot_trap(), (0.0, 0.0, 0.0), &nearby).unwrap();
    assert_eq!(result.targets.len(), 2);
    assert_eq!(result.targets[0].spell_id, 100);
    assert!(result.depleted);
    assert_eq!(go.state, GameObjectState::Depleted);
}

#[test]
fn trap_no_targets_in_range() {
    let mut go = make_go(&trap_template());
    let far = [(1, 100.0, 100.0, 0.0)];
    let result = trigger_trap(&mut go, &one_shot_trap(), (0.0, 0.0, 0.0), &far).unwrap();
    assert!(result.targets.is_empty());
    assert!(!result.depleted);
    assert_eq!(go.state, GameObjectState::Ready); // Not consumed
}

#[test]
fn trap_cooldown_not_depleted() {
    let mut go = make_go(&trap_template());
    let nearby = [(1, 1.0, 0.0, 0.0)];
    let result = trigger_trap(&mut go, &cooldown_trap(), (0.0, 0.0, 0.0), &nearby).unwrap();
    assert!(!result.depleted);
    assert!(go.flags.contains(GameObjectFlags::TRIGGERED));
    assert_eq!(go.state, GameObjectState::Ready); // Still ready, just on cooldown
}

#[test]
fn trap_triggered_cannot_retrigger() {
    let mut go = make_go(&trap_template());
    let nearby = [(1, 1.0, 0.0, 0.0)];
    trigger_trap(&mut go, &cooldown_trap(), (0.0, 0.0, 0.0), &nearby).unwrap();
    assert_eq!(
        trigger_trap(&mut go, &cooldown_trap(), (0.0, 0.0, 0.0), &nearby),
        Err(InteractionError::Depleted)
    );
}

#[test]
fn trap_reset_allows_retrigger() {
    let mut go = make_go(&trap_template());
    let nearby = [(1, 1.0, 0.0, 0.0)];
    trigger_trap(&mut go, &cooldown_trap(), (0.0, 0.0, 0.0), &nearby).unwrap();
    reset_trap(&mut go);
    assert!(!go.flags.contains(GameObjectFlags::TRIGGERED));
    let result = trigger_trap(&mut go, &cooldown_trap(), (0.0, 0.0, 0.0), &nearby).unwrap();
    assert_eq!(result.targets.len(), 1);
}

#[test]
fn trap_on_non_trap_object_fails() {
    let mut go = make_go(&chest_template());
    assert_eq!(
        trigger_trap(&mut go, &one_shot_trap(), (0.0, 0.0, 0.0), &[]),
        Err(InteractionError::NotInteractable)
    );
}

#[test]
fn in_trap_radius_boundary() {
    assert!(in_trap_radius(0.0, 0.0, 0.0, 5.0, 0.0, 0.0, 5.0));
    assert!(!in_trap_radius(0.0, 0.0, 0.0, 5.01, 0.0, 0.0, 5.0));
}

// --- Quest objects ---

fn quest_obj_template() -> GameObjectTemplate {
    GameObjectTemplate {
        id: 4001, object_type: GameObjectType::QuestObject, display_id: 800,
        name: "Ancient Rune".to_string(), faction: 0, flags: GameObjectFlags::INTERACTABLE,
    }
}

fn quest_data() -> QuestObjectData {
    QuestObjectData { quest_id: 100, objective_index: 2 }
}

#[test]
fn quest_object_completes_objective() {
    let mut go = make_go(&quest_obj_template());
    interact(&mut go).unwrap(); // Ready → InUse
    let result = use_quest_object(&mut go, &quest_data(), true).unwrap();
    assert_eq!(result.quest_id, 100);
    assert_eq!(result.objective_index, 2);
    assert_eq!(go.state, GameObjectState::Depleted);
}

#[test]
fn quest_object_no_quest_fails() {
    let mut go = make_go(&quest_obj_template());
    interact(&mut go).unwrap();
    assert_eq!(use_quest_object(&mut go, &quest_data(), false), Err(InteractionError::NotInteractable));
    assert_eq!(go.state, GameObjectState::Ready); // Returned to ready
}

#[test]
fn quest_object_not_in_use_fails() {
    let mut go = make_go(&quest_obj_template());
    assert_eq!(use_quest_object(&mut go, &quest_data(), true), Err(InteractionError::InUse));
}

#[test]
fn quest_object_wrong_type_fails() {
    let mut go = make_go(&chest_template());
    interact(&mut go).unwrap();
    assert_eq!(use_quest_object(&mut go, &quest_data(), true), Err(InteractionError::NotInteractable));
}

// --- Doors ---

fn unlocked_door_data() -> DoorData {
    DoorData { key_item_id: 0, lockpick_skill: 0, auto_close_secs: 0 }
}

fn keyed_door_data() -> DoorData {
    DoorData { key_item_id: 5000, lockpick_skill: 100, auto_close_secs: 10 }
}

fn make_unlocked_door() -> GameObject {
    let mut tmpl = door_template();
    tmpl.flags.remove(GameObjectFlags::LOCKED);
    make_go(&tmpl)
}

#[test]
fn toggle_open_unlocked_door() {
    let mut go = make_unlocked_door();
    let result = toggle_door(&mut go, &unlocked_door_data(), false, 0).unwrap();
    assert!(result.is_open);
    assert_eq!(go.state, GameObjectState::InUse);
}

#[test]
fn toggle_close_open_door() {
    let mut go = make_unlocked_door();
    toggle_door(&mut go, &unlocked_door_data(), false, 0).unwrap();
    let result = toggle_door(&mut go, &unlocked_door_data(), false, 0).unwrap();
    assert!(!result.is_open);
    assert_eq!(go.state, GameObjectState::Ready);
}

#[test]
fn toggle_locked_door_with_key() {
    let mut go = make_go(&door_template()); // locked
    let result = toggle_door(&mut go, &keyed_door_data(), true, 0).unwrap();
    assert!(result.is_open);
    assert!(!go.is_locked()); // lock cleared after use
}

#[test]
fn toggle_locked_door_with_lockpick() {
    let mut go = make_go(&door_template());
    let result = toggle_door(&mut go, &keyed_door_data(), false, 100).unwrap();
    assert!(result.is_open);
}

#[test]
fn toggle_locked_door_no_key_fails() {
    let mut go = make_go(&door_template());
    assert_eq!(
        toggle_door(&mut go, &keyed_door_data(), false, 50), // skill too low
        Err(InteractionError::Locked)
    );
}

#[test]
fn toggle_locked_door_no_lockpick_skill_config() {
    let mut go = make_go(&door_template());
    let data = DoorData { key_item_id: 5000, lockpick_skill: 0, auto_close_secs: 0 };
    // lockpick_skill=0 means can't be picked, need key
    assert_eq!(toggle_door(&mut go, &data, false, 999), Err(InteractionError::Locked));
}

#[test]
fn door_auto_close_timer() {
    let mut go = make_unlocked_door();
    let data = DoorData { key_item_id: 0, lockpick_skill: 0, auto_close_secs: 30 };
    let result = toggle_door(&mut go, &data, false, 0).unwrap();
    assert_eq!(result.auto_close_secs, 30);
}

#[test]
fn close_door_auto() {
    let mut go = make_unlocked_door();
    toggle_door(&mut go, &unlocked_door_data(), false, 0).unwrap();
    close_door(&mut go);
    assert_eq!(go.state, GameObjectState::Ready);
}

#[test]
fn close_door_already_closed_noop() {
    let mut go = make_unlocked_door();
    close_door(&mut go); // already Ready — no effect
    assert_eq!(go.state, GameObjectState::Ready);
}

#[test]
fn toggle_non_door_fails() {
    let mut go = make_go(&chest_template());
    assert_eq!(
        toggle_door(&mut go, &unlocked_door_data(), false, 0),
        Err(InteractionError::NotInteractable)
    );
}

// --- Gathering nodes ---

fn copper_gather_data() -> GatheringData {
    GatheringData {
        required_skill: 1,
        trivial_skill: 50,
        yields: vec![
            GatherYield { item_id: 2770, min_count: 1, max_count: 3 },
        ],
        skill_gain: 1,
    }
}

fn herb_gather_data() -> GatheringData {
    GatheringData {
        required_skill: 1,
        trivial_skill: 25,
        yields: vec![
            GatherYield { item_id: 2447, min_count: 1, max_count: 2 },
            GatherYield { item_id: 2448, min_count: 0, max_count: 1 },
        ],
        skill_gain: 1,
    }
}

#[test]
fn gather_mining_node_success() {
    let mut go = make_go(&mining_template());
    interact(&mut go).unwrap(); // Ready → InUse

    let result = gather_node(&mut go, &copper_gather_data(), 10, 0.5).unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].0, 2770); // Copper Ore
    assert_eq!(result.items[0].1, 2);    // min 1 + (2 * 0.5) = 2
    assert_eq!(result.skill_gained, 1);  // skill 10 < trivial 50
    assert_eq!(go.state, GameObjectState::Depleted);
}

#[test]
fn gather_herb_node_multiple_yields() {
    let tmpl = GameObjectTemplate {
        id: 3002, object_type: GameObjectType::HerbNode, display_id: 701,
        name: "Peacebloom".to_string(), faction: 0, flags: GameObjectFlags::INTERACTABLE,
    };
    let mut go = make_go(&tmpl);
    interact(&mut go).unwrap();

    let result = gather_node(&mut go, &herb_gather_data(), 5, 1.0).unwrap();
    assert_eq!(result.items.len(), 2);
    assert_eq!(result.items[0], (2447, 2)); // max
    assert_eq!(result.items[1], (2448, 1)); // max
}

#[test]
fn gather_insufficient_skill() {
    let mut go = make_go(&mining_template());
    interact(&mut go).unwrap();

    let result = gather_node(&mut go, &copper_gather_data(), 0, 0.5);
    assert_eq!(result, Err(InteractionError::MissingSkill));
    // Object returned to Ready after failed gather
    assert_eq!(go.state, GameObjectState::Ready);
}

#[test]
fn gather_trivial_no_skill_gain() {
    let mut go = make_go(&mining_template());
    interact(&mut go).unwrap();

    // Skill 100 >= trivial 50, no skill gain
    let result = gather_node(&mut go, &copper_gather_data(), 100, 0.5).unwrap();
    assert_eq!(result.skill_gained, 0);
}

#[test]
fn gather_min_roll() {
    let mut go = make_go(&mining_template());
    interact(&mut go).unwrap();

    let result = gather_node(&mut go, &copper_gather_data(), 10, 0.0).unwrap();
    assert_eq!(result.items[0].1, 1); // min_count
}

#[test]
fn gather_max_roll() {
    let mut go = make_go(&mining_template());
    interact(&mut go).unwrap();

    let result = gather_node(&mut go, &copper_gather_data(), 10, 1.0).unwrap();
    assert_eq!(result.items[0].1, 3); // max_count
}

#[test]
fn gather_non_node_fails() {
    let mut go = make_go(&chest_template());
    interact(&mut go).unwrap();

    assert_eq!(
        gather_node(&mut go, &copper_gather_data(), 10, 0.5),
        Err(InteractionError::NotInteractable)
    );
}

#[test]
fn gather_not_in_use_fails() {
    let mut go = make_go(&mining_template());
    // Don't call interact() — still Ready
    assert_eq!(
        gather_node(&mut go, &copper_gather_data(), 10, 0.5),
        Err(InteractionError::InUse)
    );
}

// --- State machine ---

#[test]
fn full_lifecycle_ready_inuse_depleted_respawning_ready() {
    let mut go = make_go(&chest_template());
    assert_eq!(go.state, GameObjectState::Ready);

    go.begin_use().unwrap();
    assert_eq!(go.state, GameObjectState::InUse);

    go.deplete().unwrap();
    assert_eq!(go.state, GameObjectState::Depleted);

    go.begin_respawn().unwrap();
    assert_eq!(go.state, GameObjectState::Respawning);

    go.respawn().unwrap();
    assert_eq!(go.state, GameObjectState::Ready);
}

#[test]
fn cannot_use_depleted_object() {
    let mut go = make_go(&chest_template());
    go.begin_use().unwrap();
    go.deplete().unwrap();
    assert_eq!(interact(&mut go), Err(InteractionError::Depleted));
}

#[test]
fn cannot_use_respawning_object() {
    let mut go = make_go(&chest_template());
    go.begin_use().unwrap();
    go.deplete().unwrap();
    go.begin_respawn().unwrap();
    assert_eq!(interact(&mut go), Err(InteractionError::Depleted));
}

#[test]
fn invalid_transition_deplete_from_ready() {
    let mut go = make_go(&chest_template());
    assert!(go.deplete().is_err());
}

#[test]
fn invalid_transition_respawn_from_inuse() {
    let mut go = make_go(&chest_template());
    go.begin_use().unwrap();
    assert!(go.respawn().is_err());
}

#[test]
fn invalid_transition_begin_respawn_from_ready() {
    let mut go = make_go(&chest_template());
    assert!(go.begin_respawn().is_err());
}

#[test]
fn cancel_use_from_inuse() {
    let mut go = make_go(&chest_template());
    go.begin_use().unwrap();
    go.cancel_use().unwrap();
    assert_eq!(go.state, GameObjectState::Ready);
}

#[test]
fn cancel_use_from_ready_fails() {
    let mut go = make_go(&chest_template());
    assert!(go.cancel_use().is_err());
}

// --- Spawn system ---

fn spawn_at(template_id: u32, map_id: u16, x: f32, respawn: u32) -> GameObjectSpawn {
    GameObjectSpawn {
        spawn_id: 0, template_id, map_id, x, y: 0.0, z: 0.0,
        rotation: NO_ROTATION, origin: SpawnOrigin::Static, respawn_time: respawn,
    }
}

#[test]
fn add_static_spawn() {
    let mut mgr = SpawnManager::new();
    let id = mgr.add_static(spawn_at(1001, 0, 100.0, 300));

    assert_eq!(id, 1);
    let spawn = mgr.get(id).unwrap();
    assert_eq!(spawn.template_id, 1001);
    assert_eq!(spawn.map_id, 0);
    assert_eq!(spawn.origin, SpawnOrigin::Static);
    assert_eq!(spawn.respawn_time, 300);
}

#[test]
fn add_dynamic_spawn() {
    let mut mgr = SpawnManager::new();
    let id = mgr.add_dynamic(spawn_at(2001, 1, 50.0, 0));

    let spawn = mgr.get(id).unwrap();
    assert_eq!(spawn.origin, SpawnOrigin::Dynamic);
}

#[test]
fn sequential_spawn_ids() {
    let mut mgr = SpawnManager::new();
    let id1 = mgr.add_static(spawn_at(1001, 0, 0.0, 0));
    let id2 = mgr.add_dynamic(spawn_at(2001, 0, 0.0, 0));
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}

#[test]
fn remove_dynamic_spawn() {
    let mut mgr = SpawnManager::new();
    mgr.add_static(spawn_at(1001, 0, 0.0, 0));
    let dyn_id = mgr.add_dynamic(spawn_at(2001, 0, 0.0, 0));

    assert!(mgr.remove_dynamic(dyn_id));
    assert_eq!(mgr.len(), 1);
    assert!(mgr.get(dyn_id).is_none());
}

#[test]
fn cannot_remove_static_spawn() {
    let mut mgr = SpawnManager::new();
    let static_id = mgr.add_static(spawn_at(1001, 0, 0.0, 0));

    assert!(!mgr.remove_dynamic(static_id));
    assert_eq!(mgr.len(), 1);
}

#[test]
fn spawns_for_map() {
    let mut mgr = SpawnManager::new();
    mgr.add_static(spawn_at(1001, 0, 0.0, 0));
    mgr.add_static(spawn_at(1002, 0, 10.0, 0));
    mgr.add_static(spawn_at(2001, 1, 0.0, 0));

    assert_eq!(mgr.spawns_for_map(0).len(), 2);
    assert_eq!(mgr.spawns_for_map(1).len(), 1);
    assert!(mgr.spawns_for_map(99).is_empty());
}

#[test]
fn spawns_for_template() {
    let mut mgr = SpawnManager::new();
    mgr.add_static(spawn_at(1001, 0, 0.0, 0));
    mgr.add_static(spawn_at(1001, 0, 10.0, 0));
    mgr.add_static(spawn_at(2001, 0, 0.0, 0));

    assert_eq!(mgr.spawns_for_template(1001).len(), 2);
    assert_eq!(mgr.spawns_for_template(2001).len(), 1);
}

#[test]
fn spawn_counts() {
    let mut mgr = SpawnManager::new();
    mgr.add_static(spawn_at(1001, 0, 0.0, 0));
    mgr.add_static(spawn_at(1002, 0, 0.0, 0));
    mgr.add_dynamic(spawn_at(3001, 0, 0.0, 0));

    let (statics, dynamics) = mgr.counts();
    assert_eq!(statics, 2);
    assert_eq!(dynamics, 1);
}

#[test]
fn get_nonexistent_spawn() {
    let mgr = SpawnManager::new();
    assert!(mgr.get(999).is_none());
}

// --- Respawn timer ---

#[test]
fn mark_despawned_and_respawn() {
    let mut tracker = RespawnTracker::new();
    tracker.mark_despawned(1, 300, 1000); // respawn at 1300

    assert!(tracker.is_despawned(1));
    assert_eq!(tracker.pending_count(), 1);

    // Not ready yet
    let ready = tracker.collect_respawns(1200);
    assert!(ready.is_empty());
    assert!(tracker.is_despawned(1));

    // Now ready
    let ready = tracker.collect_respawns(1300);
    assert_eq!(ready, vec![1]);
    assert!(!tracker.is_despawned(1));
    assert_eq!(tracker.pending_count(), 0);
}

#[test]
fn zero_respawn_time_ignored() {
    let mut tracker = RespawnTracker::new();
    tracker.mark_despawned(1, 0, 1000);
    assert!(!tracker.is_despawned(1));
    assert_eq!(tracker.pending_count(), 0);
}

#[test]
fn multiple_respawns() {
    let mut tracker = RespawnTracker::new();
    tracker.mark_despawned(1, 60, 1000);  // respawn at 1060
    tracker.mark_despawned(2, 120, 1000); // respawn at 1120
    tracker.mark_despawned(3, 300, 1000); // respawn at 1300

    let ready = tracker.collect_respawns(1100);
    assert_eq!(ready.len(), 1); // only spawn 1
    assert!(ready.contains(&1));

    let ready = tracker.collect_respawns(1200);
    assert_eq!(ready.len(), 1); // spawn 2
    assert!(ready.contains(&2));

    assert_eq!(tracker.pending_count(), 1); // spawn 3 still pending
}

#[test]
fn time_remaining() {
    let mut tracker = RespawnTracker::new();
    tracker.mark_despawned(1, 300, 1000);

    assert_eq!(tracker.time_remaining(1, 1000), 300);
    assert_eq!(tracker.time_remaining(1, 1200), 100);
    assert_eq!(tracker.time_remaining(1, 1300), 0);
    assert_eq!(tracker.time_remaining(1, 1500), 0);
    // Not tracked
    assert_eq!(tracker.time_remaining(999, 1000), 0);
}

#[test]
fn cancel_respawn() {
    let mut tracker = RespawnTracker::new();
    tracker.mark_despawned(1, 300, 1000);
    tracker.cancel(1);
    assert!(!tracker.is_despawned(1));
    assert!(tracker.collect_respawns(2000).is_empty());
}

#[test]
fn re_despawn_resets_timer() {
    let mut tracker = RespawnTracker::new();
    tracker.mark_despawned(1, 60, 1000);  // respawn at 1060
    tracker.mark_despawned(1, 60, 1050);  // re-despawned, respawn at 1110

    let ready = tracker.collect_respawns(1070);
    assert!(ready.is_empty()); // old timer would have triggered, new one hasn't

    let ready = tracker.collect_respawns(1110);
    assert_eq!(ready, vec![1]);
}
