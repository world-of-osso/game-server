use super::*;

#[test]
fn instance_add_remove_player() {
    let mut inst = Instance {
        instance_id: 1,
        map_id: 33,
        difficulty: Difficulty::Normal,
        group_leader: 100,
        players: vec![100, 101],
        spawned_entities: HashSet::new(),
        created_at: 1000,
    };
    assert!(inst.contains_player(100));
    assert!(!inst.contains_player(200));

    inst.add_player(102, 5).unwrap();
    assert!(inst.contains_player(102));
    assert_eq!(inst.players.len(), 3);

    inst.add_player(100, 5).unwrap();
    assert_eq!(inst.players.len(), 3);

    assert!(inst.remove_player(101));
    assert!(!inst.contains_player(101));
    assert!(!inst.remove_player(999));
}

#[test]
fn instance_full_rejects_player() {
    let mut inst = Instance {
        instance_id: 1,
        map_id: 33,
        difficulty: Difficulty::Normal,
        group_leader: 100,
        players: vec![100, 101, 102, 103, 104],
        spawned_entities: HashSet::new(),
        created_at: 1000,
    };
    assert_eq!(inst.add_player(105, 5), Err(InstanceError::InstanceFull));
}

#[test]
fn instance_empty_after_all_leave() {
    let mut inst = Instance {
        instance_id: 1,
        map_id: 33,
        difficulty: Difficulty::Normal,
        group_leader: 100,
        players: vec![100],
        spawned_entities: HashSet::new(),
        created_at: 1000,
    };
    assert!(!inst.is_empty());
    inst.remove_player(100);
    assert!(inst.is_empty());
}
