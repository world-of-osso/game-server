//! Controlled approach arrival; real awakening aura and source first ritual wave.
use super::*;

#[derive(Resource, Default)]
struct MoccasinBirths(Vec<(Entity, Position)>);

fn record_moccasin_births(
    trigger: On<Add, crate::zones::TempSummon>,
    units: Query<(&shared::components::Npc, &Position)>,
    mut births: ResMut<MoccasinBirths>,
) {
    if let Ok((npc, position)) = units.get(trigger.entity)
        && npc.template_id == 5762
    {
        births.0.push((trigger.entity, *position));
    }
}

fn read_approach_endpoint(conn: &Connection) -> Position {
    let end: (f32, f32, f32) = conn
        .query_row(
            "SELECT PositionX,PositionY,PositionZ FROM tdb_waypoint_path_node
         WHERE PathId=367803 ORDER BY NodeId DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    let (x, y, z) = crate::terrain_height::wow_to_bevy(end.0, end.1, end.2);
    Position { x, y, z }
}

fn read_moccasin_positions(conn: &Connection) -> Vec<Vec3> {
    let mut query = conn
        .prepare(
            "SELECT entry,position_x,position_y,position_z FROM tdb_creature_summon_groups
         WHERE summonerId=3678 AND summonerType=0 AND groupId=2 ORDER BY position_x",
        )
        .unwrap();
    let rows = query
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                Vec3::new(row.get(1)?, row.get(2)?, row.get(3)?),
            ))
        })
        .unwrap();
    rows.map(|row| {
        let (entry, position) = row.unwrap();
        assert_eq!(entry, 5762, "source group2 contains only Moccasins");
        position
    })
    .collect()
}

#[test]
fn encounter_coverage_wailing_awakening_aura_then_three_source_moccasins() {
    const BEFORE_SPEECH: f32 = 0.8;
    const CROSS_SPEECH: f32 = 0.3;
    const BEFORE_AWAKENING: f32 = 4.5;
    const CROSS_AWAKENING: f32 = 0.6;
    const BEFORE_MOCCASINS: f32 = 3.2;
    const CROSS_MOCCASINS: f32 = 0.8;
    let (mut h, prerequisite, map) = bound_boss_at_difficulty(3669, 43, 1, 1, None);
    h.hit_pct(prerequisite, 1_000.0);
    h.advance(0.05);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(prerequisite)
            .is_some(),
        "actual prerequisite death before staging"
    );
    assert!(
        h.app
            .world()
            .resource::<WorldData>()
            .templates
            .contains_key(&5762),
        "fixture retains the actual runtime Moccasin template"
    );
    h.app.init_resource::<MoccasinBirths>();
    h.app.add_observer(record_moccasin_births);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (disciple, _) = spawn_authored_actor(&mut h.app, &conn, 3678, map, 1);
    let position = read_approach_endpoint(&conn);
    // Prior arrival/source event1 faction and highHP are explicit staging limits.
    h.app.world_mut().entity_mut(disciple).insert((
        position,
        UnitFactionTemplate(250),
        Health {
            current: 1_000_000_000.0,
            max: 1_000_000_000.0,
        },
    ));
    let observer = h.player();
    h.app.world_mut().entity_mut(observer).insert(Position {
        x: position.x + 1.0,
        ..position
    });
    h.advance(0.1);
    dispatch(
        h.app.world_mut(),
        disciple,
        Hook::WaypointPathEnded {
            node_id: 0,
            path_id: 367803,
        },
    );
    let heard = |h: &Harness, copy, phrase: &str| {
        h.inbox(copy)
            .chat
            .iter()
            .any(|message| message.content.contains(phrase))
    };
    h.advance(BEFORE_SPEECH);
    assert!(!heard(&h, 0, "Protect me brave souls"));
    h.advance(CROSS_SPEECH);
    assert!(
        heard(&h, 0, "Protect me brave souls"),
        "source progress14 speech after1s"
    );
    assert!(
        !heard(&h, 1, "Protect me brave souls"),
        "own-copy ritual speech only"
    );
    h.advance(BEFORE_AWAKENING);
    assert!(
        !h.has_aura(disciple, 6271),
        "source5s wait before awakening"
    );
    assert!(
        !heard(&h, 0, "awakening ritual"),
        "Talk8 must not precede the awakening cast"
    );
    h.advance(CROSS_AWAKENING);
    assert!(
        h.has_aura(disciple, 6271),
        "actual authored6271 self aura by6.2s"
    );
    assert!(
        !h.has_aura(observer, 6271),
        "awakening targets the Disciple, not the player"
    );
    assert!(heard(&h, 0, "awakening ritual"));
    assert!(!heard(&h, 1, "awakening ritual"), "Talk8 stays in its copy");
    h.advance(BEFORE_MOCCASINS);
    assert!(
        h.app.world().resource::<MoccasinBirths>().0.is_empty(),
        "source4s delay after ritual begins"
    );
    h.advance(CROSS_MOCCASINS);
    let births = &h.app.world().resource::<MoccasinBirths>().0;
    assert_eq!(
        births.len(),
        3,
        "actual group2 consists of three Moccasins; state={:?}, combat={:?}",
        h.app
            .world()
            .get::<crate::creature_ai::CreatureState>(disciple),
        h.app.world().get::<CombatStatus>(disciple)
    );
    let source = read_moccasin_positions(&conn);
    assert_eq!(source.len(), 3, "complete authored group membership");
    let mut actual: Vec<_> = births
        .iter()
        .map(|(_, position)| {
            let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
            Vec3::new(x, y, z)
        })
        .collect();
    actual.sort_by(|a, b| a.x.total_cmp(&b.x));
    for (point, source) in actual.iter().zip(source) {
        assert!(point.distance(source) < 0.001, "authored summon position");
    }
    for (actor, _) in births {
        assert_eq!(h.app.world().get::<InMap>(*actor), Some(&map));
        let pursued = h
            .app
            .world()
            .get::<crate::creature_ai::CreatureState>(*actor);
        assert!(
            matches!(pursued, Some(crate::creature_ai::CreatureState::Pursuing { target }) if *target == disciple.to_bits()),
            "source JustSummoned targets Disciple"
        );
    }
}
