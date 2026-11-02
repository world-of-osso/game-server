use super::*;

#[test]
fn engaging_one_bound_trinity_member_engages_its_copy_peers() {
    // WCL hbyB6K8cfCDwJvAR/3 and p3QW9RkBvK86bCga/10: all three
    // encounter members receive damage within five seconds of the first.
    let (mut h, first, map) = bound_boss_at_difficulty(243028, 2859, 0, 8, None);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (second, _) = spawn_authored_actor(&mut h.app, &conn, 243029, map, 8);
    let (third, _) = spawn_authored_actor(&mut h.app, &conn, 243030, map, 8);
    let (unrelated, _) = spawn_authored_actor(&mut h.app, &conn, 244887, map, 8);
    let other_map = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    let (other_copy, _) = spawn_authored_actor(&mut h.app, &conn, 243029, other_map, 8);
    h.engage(first);
    let world = h.app.world();
    for actor in [first, second, third] {
        assert!(
            world
                .get::<shared::components::CombatStatus>(actor)
                .is_some_and(|s| s.0)
        );
    }
    for actor in [unrelated, other_copy] {
        assert!(
            !world
                .get::<shared::components::CombatStatus>(actor)
                .is_some_and(|s| s.0)
        );
    }
}

#[test]
fn multi_boss_credit_requires_all_spawned_machinists_in_either_order() {
    for first in [213216, 213217] {
        let second = if first == 213216 { 213217 } else { 213216 };
        let (mut h, boss, map) = bound_boss_at_difficulty(first, 2652, 3, 205, None);
        h.app
            .world_mut()
            .resource_mut::<MapStore>()
            .raid_creature_encounters
            .insert((2652, second), 3);
        let conn =
            Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let (peer, _) = spawn_authored_actor(&mut h.app, &conn, second, map, 205);
        h.engage(boss);
        h.hit_pct(boss, 200.0);
        h.advance(0.1);
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map)
                & (1 << 3),
            0
        );
        h.engage(peer);
        h.hit_pct(peer, 200.0);
        h.advance(0.1);
        assert_ne!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map)
                & (1 << 3),
            0
        );
    }
}

#[test]
fn multi_boss_credit_single_spawn_still_completes() {
    let (mut h, boss, map) = bound_boss_at_difficulty(213216, 2652, 3, 205, None);
    h.app
        .world_mut()
        .resource_mut::<MapStore>()
        .raid_creature_encounters
        .insert((2652, 213217), 3);
    h.engage(boss);
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
    assert_ne!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map)
            & (1 << 3),
        0
    );
}
