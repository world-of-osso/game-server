use super::*;
use shared::components::UnitFlags;

#[test]
fn eirich_proximity_unlock_is_copy_local_and_persists_on_reload() {
    let (mut h, _, map) = bound_boss_at_difficulty(210108, 2652, 0, 205, None);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (boss, _) = spawn_authored_actor(&mut h.app, &conn, 213119, map, 205);
    let other = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    let (other_boss, _) = spawn_authored_actor(&mut h.app, &conn, 213119, other, 205);
    let boss_position = *h.app.world().get::<Position>(boss).unwrap();
    let original_flags = h.app.world().get::<UnitFlags>(boss).unwrap().0;
    assert_ne!(original_flags & UnitFlags::NOT_SELECTABLE, 0);
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert((
        map,
        Position {
            x: boss_position.x + 30.94,
            ..boss_position
        },
    ));
    h.advance(0.2);
    assert_eq!(
        h.app.world().get::<UnitFlags>(boss).unwrap().0,
        original_flags
    );
    h.app.world_mut().entity_mut(player).insert(Position {
        x: boss_position.x + 30.92,
        ..boss_position
    });
    h.advance(0.2);
    assert_eq!(
        h.app.world().get::<UnitFlags>(boss).unwrap().0,
        original_flags & !UnitFlags::NOT_SELECTABLE
    );
    assert_ne!(
        h.app.world().get::<UnitFlags>(other_boss).unwrap().0 & UnitFlags::NOT_SELECTABLE,
        0
    );
    h.app.world_mut().entity_mut(player).insert(Position {
        x: boss_position.x + 40.0,
        ..boss_position
    });
    let (reloaded, _) = spawn_authored_actor(&mut h.app, &conn, 213119, map, 205);
    h.advance(0.2);
    assert_eq!(
        h.app.world().get::<UnitFlags>(reloaded).unwrap().0 & UnitFlags::NOT_SELECTABLE,
        0
    );
    assert_ne!(
        h.app.world().get::<UnitFlags>(other_boss).unwrap().0 & UnitFlags::NOT_SELECTABLE,
        0
    );
}
