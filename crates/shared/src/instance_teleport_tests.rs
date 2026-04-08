use crate::instance::{
    Difficulty, InstanceError, InstanceManager, InstanceTemplateRegistry, WorldPosition,
    dungeon_template, raid_template,
};

use super::*;

fn sample_registry() -> InstanceTemplateRegistry {
    let mut reg = InstanceTemplateRegistry::new();
    let mut dungeon = dungeon_template(33, "Shadowfang Keep", 0);
    dungeon.entrance_pos = WorldPosition {
        map_id: 33,
        x: 100.0,
        y: 200.0,
        z: 50.0,
    };
    dungeon.exit_pos = WorldPosition {
        map_id: 0,
        x: -500.0,
        y: 300.0,
        z: 10.0,
    };
    reg.add(dungeon).unwrap();

    let mut raid = raid_template(409, "Molten Core", 230);
    raid.entrance_pos = WorldPosition {
        map_id: 409,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    raid.exit_pos = WorldPosition {
        map_id: 230,
        x: -100.0,
        y: -200.0,
        z: 30.0,
    };
    reg.add(raid).unwrap();
    reg
}

fn setup() -> (InstanceTemplateRegistry, InstanceManager, u32) {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();
    let id = mgr
        .create_instance(&reg, 33, Difficulty::Normal, 100, &[100, 101, 102], 1000)
        .unwrap();
    (reg, mgr, id)
}

// --- Enter instance ---

#[test]
fn enter_generates_teleport_per_player() {
    let (reg, mgr, id) = setup();
    let orders = enter_instance(id, &mgr, &reg, &[100, 101, 102]).unwrap();

    assert_eq!(orders.len(), 3);
    for order in &orders {
        assert_eq!(order.destination.map_id, 33);
        assert_eq!(order.destination.x, 100.0);
        assert_eq!(order.destination.y, 200.0);
        assert_eq!(order.instance_id, Some(id));
        assert_eq!(order.reason, TeleportReason::EntrancePortal);
    }
    let players: Vec<u64> = orders.iter().map(|o| o.player).collect();
    assert_eq!(players, vec![100, 101, 102]);
}

#[test]
fn enter_unknown_instance_fails() {
    let (reg, mgr, _) = setup();
    assert_eq!(
        enter_instance(999, &mgr, &reg, &[100]),
        Err(InstanceError::InstanceNotFound)
    );
}

// --- Exit instance ---

#[test]
fn exit_teleports_to_overworld() {
    let (reg, mgr, id) = setup();
    let order = exit_instance(id, &mgr, &reg, 101).unwrap();

    assert_eq!(order.player, 101);
    assert_eq!(order.destination.map_id, 0);
    assert_eq!(order.destination.x, -500.0);
    assert_eq!(order.destination.y, 300.0);
    assert_eq!(order.instance_id, None);
    assert_eq!(order.reason, TeleportReason::ExitPortal);
}

#[test]
fn exit_player_not_in_instance_fails() {
    let (reg, mgr, id) = setup();
    assert_eq!(
        exit_instance(id, &mgr, &reg, 999),
        Err(InstanceError::InstanceNotFound)
    );
}

#[test]
fn exit_unknown_instance_fails() {
    let (reg, mgr, _) = setup();
    assert_eq!(
        exit_instance(999, &mgr, &reg, 100),
        Err(InstanceError::InstanceNotFound)
    );
}

// --- Hearthstone out ---

#[test]
fn hearthstone_teleports_to_hearth() {
    let (_, mgr, id) = setup();
    let hearth = WorldPosition {
        map_id: 0,
        x: 1000.0,
        y: 2000.0,
        z: 100.0,
    };
    let order = hearthstone_out(id, &mgr, 100, hearth).unwrap();

    assert_eq!(order.player, 100);
    assert_eq!(order.destination.map_id, 0);
    assert_eq!(order.destination.x, 1000.0);
    assert_eq!(order.destination.y, 2000.0);
    assert_eq!(order.instance_id, None);
    assert_eq!(order.reason, TeleportReason::Hearthstone);
}

#[test]
fn hearthstone_player_not_in_instance_fails() {
    let (_, mgr, id) = setup();
    let hearth = WorldPosition {
        map_id: 0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    assert_eq!(
        hearthstone_out(id, &mgr, 999, hearth),
        Err(InstanceError::InstanceNotFound)
    );
}

// --- Evacuate ---

#[test]
fn evacuate_moves_all_players_out() {
    let (reg, mgr, id) = setup();
    let orders = evacuate_instance(id, &mgr, &reg).unwrap();

    assert_eq!(orders.len(), 3);
    for order in &orders {
        assert_eq!(order.destination.map_id, 0);
        assert_eq!(order.destination.x, -500.0);
        assert_eq!(order.instance_id, None);
        assert_eq!(order.reason, TeleportReason::ExitPortal);
    }
}

#[test]
fn evacuate_empty_instance_returns_empty() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();
    let id = mgr
        .create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 1000)
        .unwrap();
    mgr.get_mut(id).unwrap().remove_player(100);

    let orders = evacuate_instance(id, &mgr, &reg).unwrap();
    assert!(orders.is_empty());
}

// --- Resolve instance for entry ---

#[test]
fn resolve_finds_existing_group_instance() {
    let (_, mgr, id) = setup();
    assert_eq!(
        resolve_instance_for_entry(&mgr, 100, 33, Difficulty::Normal),
        Some(id)
    );
}

#[test]
fn resolve_returns_none_for_new_group() {
    let (_, mgr, _) = setup();
    assert_eq!(
        resolve_instance_for_entry(&mgr, 200, 33, Difficulty::Normal),
        None
    );
}

#[test]
fn resolve_returns_none_for_different_difficulty() {
    let (_, mgr, _) = setup();
    assert_eq!(
        resolve_instance_for_entry(&mgr, 100, 33, Difficulty::Heroic),
        None
    );
}

// --- Raid teleport ---

#[test]
fn enter_raid_uses_raid_entrance() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();
    let id = mgr
        .create_instance(&reg, 409, Difficulty::Normal, 200, &[200], 1000)
        .unwrap();

    let orders = enter_instance(id, &mgr, &reg, &[200]).unwrap();
    assert_eq!(orders[0].destination.map_id, 409);
}
