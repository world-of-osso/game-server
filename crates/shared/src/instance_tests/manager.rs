use super::*;

#[test]
fn create_instance_assigns_sequential_ids() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id1 = mgr
        .create_instance(&reg, 33, Difficulty::Normal, 100, &[100, 101, 102], 1000)
        .unwrap();
    let id2 = mgr
        .create_instance(&reg, 33, Difficulty::Normal, 200, &[200, 201, 202], 1001)
        .unwrap();

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(mgr.len(), 2);
}

#[test]
fn create_instance_validates_template() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    assert_eq!(
        mgr.create_instance(&reg, 999, Difficulty::Normal, 100, &[100], 1000),
        Err(InstanceError::UnknownMap)
    );
    assert_eq!(
        mgr.create_instance(&reg, 33, Difficulty::MythicPlus, 100, &[100], 1000),
        Err(InstanceError::UnsupportedDifficulty)
    );

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

    mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &[100, 101], 1000)
        .unwrap();
    assert_eq!(
        mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &[100, 101], 1001),
        Err(InstanceError::AlreadyExists)
    );
}

#[test]
fn same_group_different_difficulty_allowed() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 1000)
        .unwrap();
    mgr.create_instance(&reg, 33, Difficulty::Heroic, 100, &[100], 1001)
        .unwrap();
    assert_eq!(mgr.len(), 2);
}

#[test]
fn get_or_create_returns_existing() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id1 = mgr
        .get_or_create(&reg, 33, Difficulty::Normal, 100, &[100, 101], 1000)
        .unwrap();
    let id2 = mgr
        .get_or_create(&reg, 33, Difficulty::Normal, 100, &[100, 101], 2000)
        .unwrap();
    assert_eq!(id1, id2);
    assert_eq!(mgr.len(), 1);
}

#[test]
fn get_or_create_creates_new() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr
        .get_or_create(&reg, 33, Difficulty::Normal, 100, &[100], 1000)
        .unwrap();
    assert_eq!(mgr.get(id).unwrap().map_id, 33);
}

#[test]
fn destroy_instance() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr
        .create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 1000)
        .unwrap();
    let removed = mgr.destroy(id).unwrap();
    assert_eq!(removed.instance_id, id);
    assert!(mgr.is_empty());
    assert!(mgr.get(id).is_none());

    mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 2000)
        .unwrap();
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

    let id1 = mgr
        .create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 1000)
        .unwrap();
    let id2 = mgr
        .create_instance(&reg, 33, Difficulty::Normal, 200, &[200], 1001)
        .unwrap();

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

    let id = mgr
        .create_instance(&reg, 33, Difficulty::Normal, 100, &[100, 101], 1000)
        .unwrap();
    assert_eq!(mgr.find_player_instance(101), Some(id));
    assert_eq!(mgr.find_player_instance(999), None);
}

#[test]
fn find_group_instance() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr
        .create_instance(&reg, 33, Difficulty::Heroic, 100, &[100], 1000)
        .unwrap();
    assert_eq!(
        mgr.find_group_instance(100, 33, Difficulty::Heroic),
        Some(id)
    );
    assert_eq!(mgr.find_group_instance(100, 33, Difficulty::Normal), None);
    assert_eq!(mgr.find_group_instance(200, 33, Difficulty::Heroic), None);
}

#[test]
fn instance_stores_creation_metadata() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr
        .create_instance(&reg, 409, Difficulty::Mythic, 300, &[300, 301], 5000)
        .unwrap();
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

    mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 1000)
        .unwrap();
    mgr.create_instance(&reg, 409, Difficulty::Normal, 200, &[200], 1001)
        .unwrap();

    let map_ids: Vec<u16> = mgr.iter().map(|(_, inst)| inst.map_id).collect();
    assert_eq!(map_ids.len(), 2);
    assert!(map_ids.contains(&33));
    assert!(map_ids.contains(&409));
}
