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

#[test]
fn encounter_coverage_wailing_awakening_aura_then_three_source_moccasins() {
    let (mut h, prerequisite, map) = bound_boss_at_difficulty(3669, 43, 1, 1, None);
    h.hit_pct(prerequisite, 1_000.0);
    h.advance(0.05);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(prerequisite)
            .is_some()
    );
    h.app.init_resource::<MoccasinBirths>();
    h.app.add_observer(record_moccasin_births);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (disciple, _) = spawn_authored_actor(&mut h.app, &conn, 3678, map, 1);
    let end: (f32, f32, f32) = conn.query_row(
        "SELECT PositionX,PositionY,PositionZ FROM tdb_waypoint_path_node WHERE PathId=367803 ORDER BY NodeId DESC LIMIT 1",
        [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
    ).unwrap();
    let (x, y, z) = crate::terrain_height::wow_to_bevy(end.0, end.1, end.2);
    let position = Position { x, y, z };
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
    h.app
        .world_mut()
        .entity_mut(observer)
        .insert(Position { x: x + 1.0, y, z });
    h.advance(0.1);
    dispatch(
        h.app.world_mut(),
        disciple,
        Hook::WaypointPathEnded {
            node_id: 0,
            path_id: 367803,
        },
    );
    let heard_protection = |h: &Harness, copy| {
        h.inbox(copy)
            .chat
            .iter()
            .any(|m| m.content.contains("Protect me brave souls"))
    };
    h.advance(0.8);
    assert!(!heard_protection(&h, 0));
    h.advance(0.3);
    assert!(heard_protection(&h, 0), "source progress14 speech after1s");
    assert!(!heard_protection(&h, 1), "own-copy ritual speech only");
    h.advance(4.5);
    assert!(
        !h.has_aura(disciple, 6271),
        "source5s wait before awakening"
    );
    h.advance(0.6);
    assert!(
        h.has_aura(disciple, 6271),
        "actual authored6271 self aura by6.2s"
    );
    assert!(
        !h.has_aura(observer, 6271),
        "awakening targets the Disciple, not the player"
    );
    assert!(
        h.inbox(0)
            .chat
            .iter()
            .any(|m| m.content.contains("awakening ritual"))
    );
    h.advance(3.2);
    assert!(
        h.app.world().resource::<MoccasinBirths>().0.is_empty(),
        "source4s delay after ritual begins"
    );
    h.advance(0.8);
    let births = &h.app.world().resource::<MoccasinBirths>().0;
    assert_eq!(births.len(), 3, "actual group2 consists of three Moccasins");
    let rows: Vec<(f32,f32,f32)> = conn.prepare("SELECT position_x,position_y,position_z FROM tdb_creature_summon_groups WHERE summonerId=3678 AND summonerType=0 AND groupId=2 ORDER BY position_x")
        .unwrap().query_map([],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap().map(Result::unwrap).collect();
    let mut actual: Vec<_> = births
        .iter()
        .map(|(_, position)| {
            let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
            Vec3::new(x, y, z)
        })
        .collect();
    actual.sort_by(|a, b| a.x.total_cmp(&b.x));
    for (point, source) in actual.iter().zip(rows) {
        assert!(
            point.distance(Vec3::new(source.0, source.1, source.2)) < 0.001,
            "authored summon position"
        );
    }
    for (actor, _) in births {
        assert_eq!(h.app.world().get::<InMap>(*actor), Some(&map));
        assert!(
            matches!(h.app.world().get::<crate::creature_ai::CreatureState>(*actor),Some(crate::creature_ai::CreatureState::Pursuing{target}) if *target==disciple.to_bits()),
            "source JustSummoned targets Disciple"
        );
    }
}
