//! Source camp after path367800; controlled arrival, real group0 combat.
use super::*;

#[test]
fn encounter_coverage_wailing_first_camp_waits_for_authored_ravagers() {
    let (mut h, unrelated, map) = bound_boss_at_difficulty(3669, 43, 1, 1, None);
    h.app
        .world_mut()
        .entity_mut(unrelated)
        .insert(ReactState::Passive);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (disciple, _) = spawn_authored_actor(&mut h.app, &conn, 3678, map, 1);
    let endpoint: (f32, f32, f32) = conn.query_row(
        "SELECT PositionX,PositionY,PositionZ FROM tdb_waypoint_path_node WHERE PathId=367800 ORDER BY NodeId DESC LIMIT 1",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).unwrap();
    let (x, y, z) = crate::terrain_height::wow_to_bevy(endpoint.0, endpoint.1, endpoint.2);
    h.app.world_mut().entity_mut(disciple).insert((
        Position { x, y, z },
        UnitFactionTemplate(250),
        ReactState::Passive,
        Health {
            current: 1_000_000_000.0,
            max: 1_000_000_000.0,
        },
    ));
    let player = h.player();
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(Position { x: x + 1.0, y, z });
    h.advance(0.1);
    let ravagers = |h: &mut Harness| {
        h.app
            .world_mut()
            .query::<(
                Entity,
                &crate::zones::TempSummon,
                &InMap,
                &shared::components::Npc,
            )>()
            .iter(h.app.world())
            .filter_map(|(actor, summon, copy, npc)| {
                (summon.summoner == disciple && *copy == map && npc.template_id == 3636)
                    .then_some(actor)
            })
            .collect::<Vec<_>>()
    };
    dispatch(
        h.app.world_mut(),
        disciple,
        Hook::WaypointPathEnded {
            node_id: 0,
            path_id: 367800,
        },
    );
    h.advance(0.8);
    assert!(
        !h.inbox(0)
            .chat
            .iter()
            .any(|message| message.content.contains("temple of promise"))
    );
    h.advance(0.4);
    assert!(
        h.inbox(0)
            .chat
            .iter()
            .any(|message| message.content.contains("temple of promise"))
    );
    assert!(
        !h.inbox(1)
            .chat
            .iter()
            .any(|message| message.content.contains("temple of promise")),
        "own-copy speech only"
    );
    h.advance(6.5);
    assert!(
        ravagers(&mut h).is_empty(),
        "source seven-second summon delay"
    );
    h.advance(0.5);
    let summoned = ravagers(&mut h);
    assert_eq!(summoned.len(), 2, "actual group0 has exactly two Ravagers");
    for actor in &summoned {
        assert!(
            matches!(h.app.world().get::<crate::creature_ai::CreatureState>(*actor),
            Some(crate::creature_ai::CreatureState::Pursuing { target }) if *target == disciple.to_bits())
        );
    }
    assert!(h.app.world().get::<CombatStatus>(disciple).unwrap().0);
    h.advance(20.0);
    assert!(
        !h.inbox(0)
            .chat
            .iter()
            .any(|message| message.content.contains("Come. We must continue")),
        "escort clock pauses during the real summon fight"
    );
    for actor in summoned {
        h.hit_pct(actor, 1_000.0);
    }
    h.advance(0.2);
    assert!(!h.app.world().get::<CombatStatus>(disciple).unwrap().0);
    h.advance(9.0);
    assert!(
        !h.inbox(0)
            .chat
            .iter()
            .any(|message| message.content.contains("Come. We must continue"))
    );
    h.advance(1.1);
    assert!(
        h.inbox(0)
            .chat
            .iter()
            .any(|message| message.content.contains("Come. We must continue"))
    );
    let facing = h.app.world().get::<Rotation>(disciple).unwrap().y;
    let source_facing = 5.742133 + std::f32::consts::FRAC_PI_2;
    assert!(
        (facing - source_facing).abs() < 0.0001,
        "source camp-facing angle: actual={facing} expected={source_facing}; state={:?}",
        h.app
            .world()
            .get::<crate::creature_ai::CreatureState>(disciple)
    );
}
