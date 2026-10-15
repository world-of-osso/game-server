//! Controlled prior arrival/deaths; source later ritual timers, speakers and waves.
use super::*;

fn ritual_summons(h: &mut Harness, disciple: Entity, entry: u32) -> Vec<Entity> {
    h.app
        .world_mut()
        .query::<(
            Entity,
            &shared::components::Npc,
            &crate::zones::TempSummon,
            &Health,
        )>()
        .iter(h.app.world())
        .filter_map(|(actor, npc, summon, health)| {
            (summon.summoner == disciple && npc.template_id == entry && health.current > 0.0)
                .then_some(actor)
        })
        .collect()
}

fn kill_ritual_wave(h: &mut Harness, actors: &[Entity]) {
    for actor in actors {
        h.hit_pct(*actor, 1_000.0);
    }
    h.advance(0.1);
    for actor in actors {
        assert!(
            h.app
                .world()
                .get::<crate::combat::DeathState>(*actor)
                .is_some(),
            "actual controlled ritual-wave death"
        );
    }
}

#[test]
fn encounter_coverage_wailing_naralex_emotes_then_ectoplasms_and_mutanus() {
    let (mut h, prerequisite, map) = bound_boss_at_difficulty(3669, 43, 1, 1, None);
    kill_ritual_wave(&mut h, &[prerequisite]);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    // Register another copy's Naralex first: lookup must not pick the first entry.
    let other_map = *h.app.world().get::<InMap>(h.players[1].0).unwrap();
    let (decoy, _) = spawn_authored_actor(&mut h.app, &conn, 3679, other_map, 1);
    let (naralex, _) = spawn_authored_actor(&mut h.app, &conn, 3679, map, 1);
    let (disciple, _) = spawn_authored_actor(&mut h.app, &conn, 3678, map, 1);
    let endpoint: (f32, f32, f32) = conn
        .query_row(
            "SELECT PositionX,PositionY,PositionZ FROM tdb_waypoint_path_node
         WHERE PathId=367803 ORDER BY NodeId DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    let (x, y, z) = crate::terrain_height::wow_to_bevy(endpoint.0, endpoint.1, endpoint.2);
    let position = Position { x, y, z };
    h.app.world_mut().entity_mut(disciple).insert((
        position,
        UnitFactionTemplate(250),
        Health {
            current: 1_000_000_000.0,
            max: 1_000_000_000.0,
        },
    ));
    for (index, speaker) in [(0, naralex), (1, decoy)] {
        let position = *h.app.world().get::<Position>(speaker).unwrap();
        h.app
            .world_mut()
            .entity_mut(h.players[index].0)
            .insert(Position {
                x: position.x + 1.0,
                ..position
            });
    }
    h.advance(0.1);
    dispatch(
        h.app.world_mut(),
        disciple,
        Hook::WaypointPathEnded {
            node_id: 0,
            path_id: 367803,
        },
    );
    h.advance(10.2);
    let moccasins = ritual_summons(&mut h, disciple, 5762);
    assert_eq!(
        moccasins.len(),
        3,
        "first sourced ritual wave is the prerequisite"
    );
    kill_ritual_wave(&mut h, &moccasins);
    let heard = |h: &Harness, player, phrase: &str| {
        h.inbox(player)
            .chat
            .iter()
            .any(|m| m.content.contains(phrase))
    };
    h.advance(14.4);
    assert!(!heard(&h, 0, "fitfully"), "source15s after group2");
    h.advance(0.6);
    assert!(heard(&h, 0, "fitfully"), "same-copy Naralex group0 by25.3s");
    assert!(
        !heard(&h, 1, "fitfully"),
        "other copy's earlier Naralex is not selected"
    );
    h.advance(24.4);
    assert!(
        ritual_summons(&mut h, disciple, 5763).is_empty(),
        "source25s from first Naralex emote"
    );
    h.advance(0.6);
    let ectoplasms = ritual_summons(&mut h, disciple, 5763);
    assert_eq!(
        ectoplasms.len(),
        7,
        "actual authored group3 has seven Ectoplasms"
    );
    for actor in &ectoplasms {
        assert_eq!(h.app.world().get::<InMap>(*actor), Some(&map));
        let state = h
            .app
            .world()
            .get::<crate::creature_ai::CreatureState>(*actor);
        assert!(
            matches!(state, Some(crate::creature_ai::CreatureState::Pursuing { target }) if *target == disciple.to_bits()),
            "source JustSummoned targets Disciple"
        );
    }
    kill_ritual_wave(&mut h, &ectoplasms);
    h.advance(19.3);
    assert!(!heard(&h, 0, "writhes in agony"), "source20s after group3");
    h.advance(0.6);
    assert!(
        heard(&h, 0, "writhes in agony"),
        "same-copy Naralex group1 by70.3s"
    );
    assert!(!heard(&h, 1, "writhes in agony"));
    h.advance(29.3);
    assert!(
        ritual_summons(&mut h, disciple, 3654).is_empty(),
        "source30s from second Naralex emote"
    );
    assert!(!heard(&h, 0, "murky waters"));
    h.advance(0.8);
    assert!(
        heard(&h, 0, "murky waters"),
        "third Naralex emote precedes Mutanus"
    );
    assert!(!heard(&h, 1, "murky waters"));
    let mutanus = ritual_summons(&mut h, disciple, 3654);
    assert_eq!(
        mutanus.len(),
        1,
        "actual source group4 supplies Mutanus, not synthetic completion"
    );
    assert_eq!(h.app.world().get::<InMap>(mutanus[0]), Some(&map));
    assert!(
        h.has_aura(disciple, 6271),
        "awakening persists until source wake-up cleanup"
    );
    kill_ritual_wave(&mut h, &mutanus);
    assert_wailing_wake_up(&mut h, disciple, naralex, decoy);
    assert_wailing_owl_departure(&mut h, disciple, naralex, decoy);
    assert_wailing_real_flight_exit(&mut h, &conn, disciple, naralex, decoy);
}

fn heard_ritual_line(h: &Harness, player: usize, phrase: &str) -> bool {
    h.inbox(player)
        .chat
        .iter()
        .any(|m| m.content.contains(phrase))
}

fn naralex_stand_state(h: &Harness, actor: Entity) -> shared::components::StandState {
    h.app
        .world()
        .get::<shared::components::UnitPose>(actor)
        .unwrap()
        .stand_state
}

fn assert_wailing_wake_up(h: &mut Harness, disciple: Entity, naralex: Entity, decoy: Entity) {
    use shared::components::StandState;
    assert_eq!(naralex_stand_state(h, naralex), StandState::Sleep);
    h.advance(24.2);
    assert_eq!(
        naralex_stand_state(h, naralex),
        StandState::Sleep,
        "source25s after Mutanus, not death"
    );
    assert!(!heard_ritual_line(h, 0, "I AM AWAKE"));
    h.advance(0.8);
    assert_eq!(
        naralex_stand_state(h, naralex),
        StandState::Stand,
        "source21 wakes real same-copy Naralex"
    );
    assert!(heard_ritual_line(h, 0, "I AM AWAKE"));
    assert_eq!(naralex_stand_state(h, decoy), StandState::Sleep);
    assert!(!heard_ritual_line(h, 1, "I AM AWAKE"));
    assert!(h.has_aura(disciple, 6271));
    h.advance(2.3);
    assert!(
        h.has_aura(disciple, 6271),
        "source3s before awakening removal"
    );
    assert!(!heard_ritual_line(h, 0, "At last!"));
    h.advance(0.7);
    assert!(!h.has_aura(disciple, 6271));
    assert!(heard_ritual_line(h, 0, "At last!"));
    h.advance(2.3);
    assert!(!heard_ritual_line(h, 0, "dreaded nightmare"));
    h.advance(0.7);
    assert!(heard_ritual_line(h, 0, "dreaded nightmare"));
    h.advance(5.3);
    assert!(!heard_ritual_line(h, 0, "Farewell, brave souls"));
    h.advance(0.7);
    assert!(heard_ritual_line(h, 0, "Farewell, brave souls"));
    for phrase in ["At last!", "dreaded nightmare", "Farewell, brave souls"] {
        assert!(
            !heard_ritual_line(h, 1, phrase),
            "copy-isolated departure speech"
        );
    }
}

fn assert_wailing_owl_departure(h: &mut Harness, disciple: Entity, naralex: Entity, decoy: Entity) {
    use super::super::super::waypoint_path::GravityDisabled;
    h.advance(2.3);
    for actor in [disciple, naralex] {
        assert!(!h.has_aura(actor, 8153), "source3s after farewell");
    }
    h.advance(0.7);
    for actor in [disciple, naralex] {
        assert!(h.has_aura(actor, 8153), "ordinary source Owl Form cast");
        assert!(h.app.world().get::<GravityDisabled>(actor).is_none());
    }
    assert_eq!(
        h.app.world().get::<UnitFactionTemplate>(disciple),
        Some(&UnitFactionTemplate(35))
    );
    assert!(!h.has_aura(decoy, 8153));
    h.advance(7.3);
    for actor in [disciple, naralex] {
        assert!(
            h.app.world().get::<GravityDisabled>(actor).is_none(),
            "source8s before flight"
        );
    }
    h.advance(0.7);
    for actor in [disciple, naralex] {
        assert!(
            h.app.world().get::<GravityDisabled>(actor).is_some(),
            "source26 disables gravity for both"
        );
    }
    assert!(h.app.world().get::<GravityDisabled>(decoy).is_none());
}

fn ritual_position(h: &Harness, actor: Entity) -> Vec3 {
    let position = h.app.world().get::<Position>(actor).unwrap();
    Vec3::new(position.x, position.y, position.z)
}

fn ritual_actor_retired(h: &Harness, actor: Entity) -> bool {
    h.app.world().get_entity(actor).is_err()
        || h.app
            .world()
            .get::<crate::combat::DeathState>(actor)
            .is_some()
}

fn assert_wailing_real_flight_exit(
    h: &mut Harness,
    conn: &Connection,
    disciple: Entity,
    naralex: Entity,
    decoy: Entity,
) {
    let endpoint = read_flight_endpoint(conn, 367804);
    let naralex_endpoint = read_flight_endpoint(conn, 367900);
    let origins = [ritual_position(h, disciple), ritual_position(h, naralex)];
    let decoy_origin = ritual_position(h, decoy);
    let mut moved = [false; 2];
    for _ in 0..2400 {
        if ritual_actor_retired(h, disciple) {
            break;
        }
        h.advance(0.05);
        for (index, actor) in [disciple, naralex].into_iter().enumerate() {
            if let Some(position) = h.app.world().get::<Position>(actor) {
                let position = Vec3::new(position.x, position.y, position.z);
                assert!(position.is_finite());
                moved[index] |= position.distance(origins[index]) > 1.0;
            }
        }
        if ritual_actor_retired(h, disciple) {
            let position = ritual_position(h, disciple);
            assert!(
                position.distance(endpoint) < 0.001,
                "despawn follows real authored final flight endpoint: {position:?} vs {endpoint:?}"
            );
            break;
        }
    }
    assert_eq!(
        moved,
        [true, true],
        "both real flight paths move, no injected arrival"
    );
    assert!(
        ritual_actor_retired(h, disciple),
        "source367804-end retires ordinary-spawn Disciple"
    );
    assert!(
        ritual_actor_retired(h, naralex),
        "same callback retires Naralex"
    );
    assert!(
        ritual_position(h, naralex).distance(naralex_endpoint) < 0.001,
        "Naralex follows his distinct exact flight spline before Disciple exits"
    );
    assert!(!ritual_actor_retired(h, decoy));
    assert_eq!(ritual_position(h, decoy), decoy_origin);
}

fn read_flight_endpoint(conn: &Connection, path: u32) -> Vec3 {
    let endpoint: (f32, f32, f32) = conn.query_row(
        "SELECT PositionX,PositionY,PositionZ FROM tdb_waypoint_path_node WHERE PathId=? ORDER BY NodeId DESC LIMIT 1",
        [path], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).unwrap();
    let (x, y, z) = crate::terrain_height::wow_to_bevy(endpoint.0, endpoint.1, endpoint.2);
    Vec3::new(x, y, z)
}
