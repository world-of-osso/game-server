use super::*;

#[test]
fn avanoxx_requires_three_distinct_attendant_deaths_then_waits_in_its_copy() {
    let (mut h, boss, map) = bound_boss_at_difficulty(213179, 2660, 2, 205, None);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let native =
        crate::spell_cast::test_harness::load_native_creature_world(&[217531, 218324, 217533]);
    {
        let mut data = h.app.world_mut().resource_mut::<WorldData>();
        data.templates.extend(native.templates);
        data.difficulty_templates
            .extend(native.difficulty_templates);
    }
    let other_map = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    let (other_boss, _) = spawn_authored_actor(&mut h.app, &conn, 213179, other_map, 205);
    let (other_attendant, _) = spawn_authored_actor(&mut h.app, &conn, 217533, other_map, 205);
    let actors: Vec<_> = [217531, 218324, 217533]
        .into_iter()
        .map(|entry| spawn_authored_actor(&mut h.app, &conn, entry, map, 205).0)
        .collect();
    let (duplicate, _) = spawn_authored_actor(&mut h.app, &conn, 217531, map, 205);
    let blocked = UnitFlags::IMMUNE_TO_PC | UnitFlags::NOT_SELECTABLE;
    let kill = |h: &mut Harness, actor| {
        let damage = h.health(actor).max * 2.0;
        crate::combat::deal_direct_damage(h.app.world_mut(), actor, actor, damage);
        h.advance(0.1);
        assert_eq!(h.health(actor).current, 0.0);
    };
    kill(&mut h, other_attendant);
    kill(&mut h, actors[0]);
    kill(&mut h, duplicate);
    kill(&mut h, actors[1]);
    h.advance(18.0);
    assert_eq!(
        h.app.world().get::<UnitFlags>(boss).unwrap().0 & blocked,
        blocked
    );
    kill(&mut h, actors[2]);
    h.advance(17.0);
    assert_eq!(
        h.app.world().get::<UnitFlags>(boss).unwrap().0 & blocked,
        blocked
    );
    h.advance(0.5);
    assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0 & blocked, 0);
    assert_eq!(
        h.app.world().get::<UnitFlags>(other_boss).unwrap().0 & blocked,
        blocked
    );
}
