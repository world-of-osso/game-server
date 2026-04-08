use super::*;

#[test]
fn same_instance_both_open_world() {
    assert!(same_instance(None, None));
}

#[test]
fn same_instance_both_in_same() {
    let a = InstanceId(5);
    let b = InstanceId(5);
    assert!(same_instance(Some(&a), Some(&b)));
}

#[test]
fn different_instance_ids() {
    let a = InstanceId(1);
    let b = InstanceId(2);
    assert!(!same_instance(Some(&a), Some(&b)));
}

#[test]
fn open_world_vs_instanced_not_visible() {
    let inst = InstanceId(1);
    assert!(!same_instance(None, Some(&inst)));
    assert!(!same_instance(Some(&inst), None));
}

#[test]
fn two_groups_get_separate_instances() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let group_a = [100, 101, 102, 103, 104];
    let group_b = [200, 201, 202, 203, 204];

    let id_a = mgr
        .create_instance(&reg, 33, Difficulty::Normal, 100, &group_a, 1000)
        .unwrap();
    let id_b = mgr
        .create_instance(&reg, 33, Difficulty::Normal, 200, &group_b, 1001)
        .unwrap();

    assert_ne!(id_a, id_b);

    let inst_a = mgr.get(id_a).unwrap();
    let inst_b = mgr.get(id_b).unwrap();

    assert_eq!(inst_a.map_id, inst_b.map_id);
    assert_ne!(inst_a.instance_id, inst_b.instance_id);

    assert!(!same_instance(
        Some(&inst_a.component_id()),
        Some(&inst_b.component_id())
    ));
    assert!(same_instance(
        Some(&inst_a.component_id()),
        Some(&inst_a.component_id())
    ));
}

#[test]
fn track_and_untrack_spawned_entities() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr
        .create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 1000)
        .unwrap();
    let inst = mgr.get_mut(id).unwrap();

    inst.track_entity(5000);
    inst.track_entity(5001);
    inst.track_entity(5002);
    assert_eq!(inst.spawned_entities.len(), 3);

    inst.track_entity(5000);
    assert_eq!(inst.spawned_entities.len(), 3);

    inst.untrack_entity(5001);
    assert_eq!(inst.spawned_entities.len(), 2);
    assert!(!inst.spawned_entities.contains(&5001));
}

#[test]
fn component_id_matches_instance_id() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr
        .create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 1000)
        .unwrap();
    let inst = mgr.get(id).unwrap();
    assert_eq!(inst.component_id(), InstanceId(id));
}

#[test]
fn destroyed_instance_entities_isolated_from_new() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id1 = mgr
        .create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 1000)
        .unwrap();
    let comp1 = mgr.get(id1).unwrap().component_id();

    mgr.destroy(id1);

    let id2 = mgr
        .create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 2000)
        .unwrap();
    let comp2 = mgr.get(id2).unwrap().component_id();

    assert_ne!(comp1, comp2);
    assert!(!same_instance(Some(&comp1), Some(&comp2)));
}
