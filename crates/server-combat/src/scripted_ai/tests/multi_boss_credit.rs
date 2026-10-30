use super::*;

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
