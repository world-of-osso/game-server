use super::*;

#[test]
fn council_sequences_lethal_hits_before_credit() {
    let (mut h, kula, map) = bound_boss_at_difficulty(135475, 1762, 1, 8, None);
    h.app
        .insert_resource(crate::test_data::world_data().clone());
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (akaali, _) = spawn_authored_actor(&mut h.app, &conn, 135470, map, 8);
    let (zanazal, _) = spawn_authored_actor(&mut h.app, &conn, 135472, map, 8);
    h.advance(0.1);
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
    assert_ne!(
        h.app
            .world()
            .get::<shared::components::UnitFlags>(akaali)
            .unwrap()
            .0
            & blocked,
        0
    );
    assert_ne!(
        h.app
            .world()
            .get::<shared::components::UnitFlags>(zanazal)
            .unwrap()
            .0
            & blocked,
        0
    );
    for (index, actor) in [kula, akaali, zanazal].into_iter().enumerate() {
        h.engage(actor);
        h.hit_pct(actor, 200.0);
        h.advance(0.1);
        assert!(h.health(actor).current > 0.0);
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            if index == 2 { 2 } else { 0 }
        );
        if index < 2 {
            let next = [akaali, zanazal][index];
            assert_eq!(
                h.app
                    .world()
                    .get::<shared::components::UnitFlags>(next)
                    .unwrap()
                    .0
                    & blocked,
                0
            );
        }
    }
    assert_eq!(
        h.inbox(0)
            .ends
            .iter()
            .filter(|e| e.encounter_id == 2140 && e.success)
            .count(),
        1
    );
}
