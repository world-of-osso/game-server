use super::*;

#[test]
fn nalorakk_credit_lethal_damage_completes_encounter_3209() {
    let (_directory, path) = prepare_encounter_database("nalorakk");
    let source = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let maps = crate::maps::store::load_map_store(&source).unwrap();
    assert_eq!(maps.raid_creature_encounters.get(&(2825, 246404)), Some(&2));
    let (mut h, boss, map) = bound_boss_at_difficulty(246404, 2825, 2, 205, None);
    h.app.insert_resource(maps);
    let pos = *h.app.world().get::<Position>(boss).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: pos.x + 3.0,
        ..pos
    });
    h.engage(boss);
    h.advance(0.1);
    h.hit_pct(boss, 1_000.0);
    h.advance(0.1);
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 2),
        Some(EncounterState::Done)
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map)
            & 4,
        4
    );
    assert!(
        h.inbox(0)
            .ends
            .iter()
            .any(|end| end.encounter_id == 3209 && end.success)
    );
}
