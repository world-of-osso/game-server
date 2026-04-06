// State machine, spawn system, and respawn timer tests
// (extracted from game_object_tests.rs)

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
    assert_eq!(spawn.origin, SpawnOrigin::Static);
    assert_eq!(spawn.respawn_time, 300);
}

#[test]
fn add_dynamic_spawn() {
    let mut mgr = SpawnManager::new();
    let id = mgr.add_dynamic(spawn_at(2001, 1, 50.0, 0));
    assert_eq!(mgr.get(id).unwrap().origin, SpawnOrigin::Dynamic);
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
    tracker.mark_despawned(1, 300, 1000);
    assert!(tracker.is_despawned(1));
    assert_eq!(tracker.pending_count(), 1);
    assert!(tracker.collect_respawns(1200).is_empty());
    let ready = tracker.collect_respawns(1300);
    assert_eq!(ready, vec![1]);
    assert!(!tracker.is_despawned(1));
}

#[test]
fn zero_respawn_time_ignored() {
    let mut tracker = RespawnTracker::new();
    tracker.mark_despawned(1, 0, 1000);
    assert!(!tracker.is_despawned(1));
}

#[test]
fn multiple_respawns() {
    let mut tracker = RespawnTracker::new();
    tracker.mark_despawned(1, 60, 1000);
    tracker.mark_despawned(2, 120, 1000);
    tracker.mark_despawned(3, 300, 1000);
    assert_eq!(tracker.collect_respawns(1100).len(), 1);
    assert_eq!(tracker.collect_respawns(1200).len(), 1);
    assert_eq!(tracker.pending_count(), 1);
}

#[test]
fn time_remaining() {
    let mut tracker = RespawnTracker::new();
    tracker.mark_despawned(1, 300, 1000);
    assert_eq!(tracker.time_remaining(1, 1000), 300);
    assert_eq!(tracker.time_remaining(1, 1200), 100);
    assert_eq!(tracker.time_remaining(1, 1300), 0);
    assert_eq!(tracker.time_remaining(999, 1000), 0);
}

#[test]
fn cancel_respawn() {
    let mut tracker = RespawnTracker::new();
    tracker.mark_despawned(1, 300, 1000);
    tracker.cancel(1);
    assert!(!tracker.is_despawned(1));
}

#[test]
fn re_despawn_resets_timer() {
    let mut tracker = RespawnTracker::new();
    tracker.mark_despawned(1, 60, 1000);
    tracker.mark_despawned(1, 60, 1050);
    assert!(tracker.collect_respawns(1070).is_empty());
    assert_eq!(tracker.collect_respawns(1110), vec![1]);
}
