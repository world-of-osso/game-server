//! FANGS_WARMUP_LOG_CORRELATED against native boss templates and instance state.
use super::*;

#[test]
fn fangs_warmup_requires_orator_and_nearby_player_then_unlocks_both() {
    let (mut h, nx, map) = bound_boss_at_difficulty(216648, 2669, 2, 205, None);
    let native = crate::spell_cast::test_harness::load_native_creature_world(&[216649]);
    {
        let mut data = h.app.world_mut().resource_mut::<WorldData>();
        data.templates.extend(native.templates);
        data.difficulty_templates
            .extend(native.difficulty_templates);
    }
    let db =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (vx, _) = spawn_authored_actor(&mut h.app, &db, 216649, map, 205);
    let other = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    let (other_nx, _) = spawn_authored_actor(&mut h.app, &db, 216648, other, 205);
    let (other_vx, _) = spawn_authored_actor(&mut h.app, &db, 216649, other, 205);
    let original = h.app.world().get::<UnitFlags>(nx).unwrap().0;
    let mask = UnitFlags::IMMUNE_TO_PC | UnitFlags::NOT_SELECTABLE;
    assert_eq!(original & mask, mask);
    assert_eq!(h.app.world().get::<UnitFlags>(vx).unwrap().0 & mask, mask);
    let pos = *h.app.world().get::<Position>(nx).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(pos);
    h.advance(26.0);
    assert_eq!(h.app.world().get::<UnitFlags>(nx).unwrap().0, original);
    let orator = instance_script::boss_id_for_encounter(h.app.world_mut(), map, 2907).unwrap();
    assert!(instance_script::set_boss_state(
        h.app.world_mut(),
        map,
        orator,
        EncounterState::Done
    ));
    h.app.world_mut().entity_mut(player).insert(Position {
        x: pos.x + 21.0,
        ..pos
    });
    h.advance(26.0);
    assert_eq!(h.app.world().get::<UnitFlags>(nx).unwrap().0, original);
    h.app.world_mut().entity_mut(player).insert(pos);
    h.advance(0.2);
    h.advance(25.4);
    for unit in [nx, vx] {
        assert_eq!(h.app.world().get::<UnitFlags>(unit).unwrap().0 & mask, mask);
    }
    h.advance(0.5);
    for unit in [nx, vx] {
        let flags = *h.app.world().get::<UnitFlags>(unit).unwrap();
        assert_eq!(flags.0 & mask, 0);
        assert_ne!(flags.0 & 0x40, 0);
        assert!(flags.can_be_attacked_by(true));
    }
    for unit in [other_nx, other_vx] {
        assert_eq!(h.app.world().get::<UnitFlags>(unit).unwrap().0 & mask, mask);
    }
}
