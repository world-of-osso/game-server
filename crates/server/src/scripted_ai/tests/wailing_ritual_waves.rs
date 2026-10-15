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
    let heard = |h: &Harness, copy, phrase: &str| {
        h.inbox(copy)
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
            matches!(state,Some(crate::creature_ai::CreatureState::Pursuing{target}) if *target==disciple.to_bits()),
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
        "awakening persists until the later unported cleanup"
    );
}
