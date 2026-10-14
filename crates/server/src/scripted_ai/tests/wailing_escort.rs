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
    // Observe SetFacingTo at its callback boundary; subsequent generic chase
    // ticks replace this transient rotation in the fixture.
    dispatch(h.app.world_mut(), disciple, Hook::UpdateAi(1_000));
    let facing = h.app.world().get::<Rotation>(disciple).unwrap().y;
    let source_facing = 5.742133 + std::f32::consts::FRAC_PI_2;
    assert!(
        (facing - source_facing).abs() < 0.0001,
        "source camp-facing angle: actual={facing} expected={source_facing}; state={:?}",
        h.app
            .world()
            .get::<crate::creature_ai::CreatureState>(disciple)
    );
    h.advance(0.1);
    assert!(
        h.inbox(0)
            .chat
            .iter()
            .any(|message| message.content.contains("Come. We must continue"))
    );
}

#[derive(Resource, Default)]
struct AdderBirthPositions(Vec<(Entity, Position)>);

fn record_adder_birth_position(
    trigger: On<Add, crate::zones::TempSummon>,
    units: Query<(&shared::components::Npc, &Position)>,
    mut births: ResMut<AdderBirthPositions>,
) {
    if let Ok((npc, position)) = units.get(trigger.entity)
        && npc.template_id == 5048
    {
        births.0.push((trigger.entity, *position));
    }
}

#[test]
fn encounter_coverage_wailing_second_camp_casts_and_waits_for_three_adders() {
    let (mut h, unrelated, map) = bound_boss_at_difficulty(3669, 43, 1, 1, None);
    h.app.init_resource::<AdderBirthPositions>();
    h.app.add_observer(record_adder_birth_position);
    h.app
        .world_mut()
        .entity_mut(unrelated)
        .insert(ReactState::Passive);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (disciple, _) = spawn_authored_actor(&mut h.app, &conn, 3678, map, 1);
    let endpoint: (f32, f32, f32) = conn.query_row(
        "SELECT PositionX,PositionY,PositionZ FROM tdb_waypoint_path_node WHERE PathId=367801 ORDER BY NodeId DESC LIMIT 1",
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
    let adders = |h: &mut Harness| {
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
                (summon.summoner == disciple && *copy == map && npc.template_id == 5048)
                    .then_some(actor)
            })
            .collect::<Vec<_>>()
    };
    dispatch(
        h.app.world_mut(),
        disciple,
        Hook::WaypointPathEnded {
            node_id: 0,
            path_id: 367801,
        },
    );
    h.advance(0.1);
    let cast = h
        .app
        .world()
        .get::<CastState>(disciple)
        .expect("source cleansing cast must actually begin");
    assert_eq!(cast.spell_id, 6270);
    assert_eq!(cast.cast_type, CastType::Normal);
    assert!(
        (cast.duration - 30.0).abs() < 0.001,
        "current authored casting-time9 is30s"
    );
    assert!(
        h.inbox(0)
            .chat
            .iter()
            .any(|message| message.content.contains("Within this circle of fire"))
    );
    assert!(
        !h.inbox(1)
            .chat
            .iter()
            .any(|message| message.content.contains("Within this circle of fire")),
        "speech stays in the source copy"
    );
    h.advance(17.5);
    assert!(adders(&mut h).is_empty(), "source18s wave delay");
    h.advance(0.5);
    let summoned = adders(&mut h);
    assert_eq!(summoned.len(), 3, "actual group1 has three Adders");
    assert_eq!(
        h.app.world().get::<CastState>(disciple).unwrap().spell_id,
        6270,
        "source wave runs during the cleansing cast"
    );
    let positions = [
        (-60.2539, 273.098, -92.7609),
        (-50.1238, 274.717, -92.7609),
        (-57.5453, 280.207, -92.7609),
    ];
    for actor in &summoned {
        assert!(
            matches!(h.app.world().get::<crate::creature_ai::CreatureState>(*actor),
            Some(crate::creature_ai::CreatureState::Pursuing { target }) if *target == disciple.to_bits())
        );
        // Observe actual birth before AttackStart's movement can change it.
        let position = h
            .app
            .world()
            .resource::<AdderBirthPositions>()
            .0
            .iter()
            .find_map(|(id, position)| (*id == *actor).then_some(*position))
            .expect("actual Adder birth position");
        assert!(
            positions.iter().any(|&(sx, sy, sz)| {
                (position.x - sx).abs() < 0.001
                    && (position.z + sy).abs() < 0.001
                    && (position.y - sz).abs() < 0.001
            }),
            "wave actors start at exact authored group coordinates"
        );
    }
    assert!(h.app.world().get::<CombatStatus>(disciple).unwrap().0);
    h.advance(20.0);
    assert!(
        h.app.world().get::<CastState>(disciple).is_none(),
        "real30s cleansing cast completes during combat"
    );
    assert!(
        !h.inbox(0)
            .chat
            .iter()
            .any(|message| message.content.contains("The caverns have been purified")),
        "progress clock pauses throughout the real wave fight"
    );
    for actor in summoned {
        h.hit_pct(actor, 1_000.0);
    }
    h.advance(0.2);
    assert!(!h.app.world().get::<CombatStatus>(disciple).unwrap().0);
    h.advance(11.4);
    assert!(
        !h.inbox(0)
            .chat
            .iter()
            .any(|message| message.content.contains("The caverns have been purified")),
        "source12s delay resumes after combat"
    );
    h.advance(1.0);
    assert!(
        h.inbox(0)
            .chat
            .iter()
            .any(|message| message.content.contains("The caverns have been purified"))
    );
}
