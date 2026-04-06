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
