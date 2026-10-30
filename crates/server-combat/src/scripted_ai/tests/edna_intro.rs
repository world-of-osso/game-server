//! TC 15e0ed76 instance_the_stonevault.cpp:54,100-121 and boss_edna.cpp:165-192.
use super::*;

#[test]
fn tagged_trash_deaths_start_copy_local_edna_walk_and_release_immunity_on_arrival() {
    let (mut h, boss, map) = bound_boss_at_difficulty(210108, 2652, 0, 205, None);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let native = crate::spell_cast::test_harness::load_native_creature_world(&[210109]);
    {
        let mut data = h.app.world_mut().resource_mut::<WorldData>();
        data.templates.extend(native.templates);
        data.difficulty_templates
            .extend(native.difficulty_templates);
        data.template_string_ids
            .extend(crate::world_data::load_template_string_ids(&conn).unwrap());
    }
    let other_map = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    let (other_boss, _) = spawn_authored_actor(&mut h.app, &conn, 210108, other_map, 205);
    let (other_trash, _) = spawn_authored_actor(&mut h.app, &conn, 210109, other_map, 205);
    let spawns: Vec<(u32, f32, f32, f32)> = conn.prepare(
        "SELECT guid,position_x,position_y,position_z FROM content_creature WHERE map=2652 AND id1=210109 ORDER BY guid"
    ).unwrap().query_map([], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).unwrap().map(Result::unwrap).collect();
    assert_eq!(spawns.len(), 5, "five source-tagged Stonevault golems");
    let mut trash = Vec::new();
    for (guid, x, y, z) in spawns {
        let mut spawn = load_authored_spawn(&conn, 210109, 2652, 205);
        spawn.guid = guid;
        spawn.position_x = x;
        spawn.position_y = y;
        spawn.position_z = z;
        let (actor, _) = spawn_prepared_actor(&mut h.app, spawn, map, 205);
        assert!(
            h.app
                .world()
                .get::<crate::zones::CreatureStringIds>(actor)
                .unwrap()
                .has("edna_intro_trash")
        );
        trash.push(actor);
    }
    let start = *h.app.world().get::<Position>(boss).unwrap();
    let kill = |h: &mut Harness, actor| {
        let damage = h.health(actor).max * 2.0;
        crate::combat::deal_direct_damage(h.app.world_mut(), actor, actor, damage);
        h.advance(0.1);
        assert_eq!(h.health(actor).current, 0.0);
    };
    kill(&mut h, other_trash);
    for &actor in &trash[..4] {
        kill(&mut h, actor);
    }
    h.advance(3.2);
    assert_eq!(*h.app.world().get::<Position>(boss).unwrap(), start);
    assert_ne!(
        h.app.world().get::<UnitFlags>(boss).unwrap().0 & UnitFlags::IMMUNE_TO_PC,
        0
    );
    kill(&mut h, trash[4]);
    h.advance(2.9);
    assert_eq!(*h.app.world().get::<Position>(boss).unwrap(), start);
    assert_ne!(
        h.app.world().get::<UnitFlags>(boss).unwrap().0 & UnitFlags::IMMUNE_TO_PC,
        0
    );
    h.advance(1.0);
    assert_ne!(*h.app.world().get::<Position>(boss).unwrap(), start);
    assert_ne!(
        h.app.world().get::<UnitFlags>(boss).unwrap().0 & UnitFlags::IMMUNE_TO_PC,
        0
    );
    for _ in 0..400 {
        h.advance(0.05);
        if h.app
            .world()
            .get::<crate::scripted_ai::movement::PointMovement>(boss)
            .is_none()
        {
            break;
        }
    }
    let arrived = h.app.world().get::<Position>(boss).unwrap();
    assert!(
        (arrived.x - 1.94097).abs() < 0.01
            && (arrived.y - 361.66537).abs() < 0.01
            && (arrived.z + 0.512153).abs() < 0.01,
        "arrival {arrived:?}"
    );
    h.advance(0.8);
    assert_ne!(
        h.app.world().get::<UnitFlags>(boss).unwrap().0 & UnitFlags::IMMUNE_TO_PC,
        0
    );
    h.advance(0.3);
    assert_eq!(
        h.app.world().get::<UnitFlags>(boss).unwrap().0 & UnitFlags::IMMUNE_TO_PC,
        0
    );
    assert_ne!(
        h.app.world().get::<UnitFlags>(other_boss).unwrap().0 & UnitFlags::IMMUNE_TO_PC,
        0
    );
}
