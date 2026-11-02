use super::*;

#[test]
fn council_wipe_restores_the_first_phase() {
    let (mut h, kula, map) = bound_boss_at_difficulty(135475, 1762, 1, 8, None);
    h.app
        .insert_resource(crate::test_data::world_data().clone());
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (akaali, _) = spawn_authored_actor(&mut h.app, &conn, 135470, map, 8);
    let (zanazal, _) = spawn_authored_actor(&mut h.app, &conn, 135472, map, 8);
    h.advance(0.1);
    h.engage(kula);
    h.hit_pct(kula, 200.0);
    h.advance(0.1);
    h.engage(akaali);
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), akaali);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, 1),
        Some(EncounterState::NotStarted)
    );
    assert_eq!(h.health(kula).current, h.health(kula).max);
    let blocked = shared::components::UnitFlags::NON_ATTACKABLE;
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitFlags>(kula)
            .unwrap()
            .0
            & blocked,
        0
    );
    for actor in [akaali, zanazal] {
        assert_ne!(
            h.app
                .world()
                .get::<shared::components::UnitFlags>(actor)
                .unwrap()
                .0
                & blocked,
            0
        );
    }
    h.engage(kula);
    h.hit_pct(kula, 200.0);
    h.advance(0.1);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitFlags>(akaali)
            .unwrap()
            .0
            & blocked,
        0
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0
    );
}
