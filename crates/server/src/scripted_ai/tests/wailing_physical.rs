//! Actual source paths0–2 and their camp callbacks, without injected arrivals.
use super::*;

#[test]
#[ignore = "requires WAILING_NAV_ROOT containing the private authored map43 bake"]
fn encounter_coverage_wailing_walks_two_camps_and_onward_without_arrival_injection() {
    bevy::log::tracing_subscriber::fmt()
        .with_env_filter(
            "game_server::scripted_ai::waypoint_path=debug,game_server::creature_ai=debug",
        )
        .with_test_writer()
        .try_init()
        .expect("isolated physical fixture logger");
    let root = std::path::PathBuf::from(std::env::var_os("WAILING_NAV_ROOT").unwrap());
    let (mut h, unrelated, map) = bound_boss_at_difficulty(3669, 43, 1, 1, None);
    h.app
        .insert_resource(crate::navigation::Navigation::new(&root));
    h.app
        .world_mut()
        .entity_mut(unrelated)
        .insert(ReactState::Passive);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (disciple, _) = spawn_authored_actor(&mut h.app, &conn, 3678, map, 1);
    // Source progress event1 faction is staged; activation flag/spell gaps stay open.
    // Keep the authored/default react state: TC Creature.cpp:1304–1313 selects
    // Aggressive and Disciple never sets Passive. A fabricated Passive override
    // skips victim cleanup and leaves Pursuing after the first camp's corpses.
    // High fixture HP bounds wave survival; no NPC position or arrival is changed.
    h.app.world_mut().entity_mut(disciple).insert((
        UnitFactionTemplate(250),
        Health {
            current: 1_000_000_000.0,
            max: 1_000_000_000.0,
        },
    ));
    let endpoint: (f32, f32, f32) = conn.query_row(
        "SELECT PositionX,PositionY,PositionZ FROM tdb_waypoint_path_node WHERE PathId=367802 ORDER BY NodeId DESC LIMIT 1",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).unwrap();
    let endpoint = Vec3::new(endpoint.0, endpoint.1, endpoint.2);
    let maps = crate::maps::store::load_map_store(&conn).unwrap();
    let mut ground = crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        std::collections::HashMap::from([(43, maps.map(43).unwrap().wdt_file_data_id)]),
    );
    let projected = Vec3::new(endpoint.x, endpoint.z, -endpoint.y);
    let crate::ground::GroundProbe::Supported(floor) = ground.creature_ground_at(43, projected)
    else {
        panic!("authored path2 endpoint requires exact collision support");
    };
    let endpoint = Vec3::new(endpoint.x, endpoint.y, floor);
    h.app.insert_resource(ground);
    let mut los = los::LosMap::new(&root.parent().unwrap().parent().unwrap().join("los/43"));
    let start = h.wow_position(disciple);
    let player = h.player();
    let position = *h.app.world().get::<Position>(disciple).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: position.x + 1.0,
        ..position
    });
    h.advance(0.1);
    assert!(crate::scripted_ai::waypoint_path::move_path(
        h.app.world_mut(),
        disciple,
        367800
    ));
    let mut previous = start;
    let mut walked = 0.0;
    let mut camp_counts = [0, 0];
    let mut first_wave_at = [None, None];
    let mut saw_cleansing = false;
    let mut arrived = false;
    for tick in 0..10_000 {
        h.advance(0.05);
        let point = h.wow_position(disciple);
        let step = point.distance(previous);
        assert!(
            step < 0.5,
            "no relocation/shortcut: tick={tick} step={step} from={previous:?} to={point:?}; delta={:?} state={:?} gait={:?} speed={:?} path={}",
            h.app.world().resource::<Time>().delta(),
            h.app
                .world()
                .get::<crate::creature_ai::CreatureState>(disciple),
            h.app.world().get::<CreatureMotion>(disciple),
            h.app
                .world()
                .get::<shared::components::MovementSpeed>(disciple),
            h.app
                .world()
                .get::<crate::scripted_ai::waypoint_path::PathMovement>(disciple)
                .is_some()
        );
        let a = Vec3::new(previous.x, previous.z, -previous.y);
        let b = Vec3::new(point.x, point.z, -point.y);
        assert!(
            los.clear(a + Vec3::Y, b + Vec3::Y).unwrap(),
            "physical escort crossed collision at tick{tick}: {previous:?}->{point:?}; path={} waves={camp_counts:?} cleansing={saw_cleansing} combat={:?} motion={:?} speed={:?} state={:?} escorthome={:?} explicit_home={:?}",
            h.app
                .world()
                .get::<crate::scripted_ai::waypoint_path::PathMovement>(disciple)
                .is_some(),
            h.app.world().get::<CombatStatus>(disciple),
            h.app.world().get::<CreatureMotion>(disciple),
            h.app
                .world()
                .get::<shared::components::MovementSpeed>(disciple),
            h.app
                .world()
                .get::<crate::creature_ai::CreatureState>(disciple),
            h.app
                .world()
                .get::<crate::scripted_ai::movement::ScriptedPath>(disciple),
            h.app
                .world()
                .get::<crate::creature_ai::HomePosition>(disciple)
        );
        walked += step;
        previous = point;
        // Keep the observer player nearby without moving the escort actor.
        let position = *h.app.world().get::<Position>(disciple).unwrap();
        h.app.world_mut().entity_mut(player).insert(Position {
            x: position.x + 1.0,
            ..position
        });
        saw_cleansing |= h
            .app
            .world()
            .get::<CastState>(disciple)
            .is_some_and(|cast| cast.spell_id == 6270);
        let waves: Vec<_> = h
            .app
            .world_mut()
            .query::<(
                Entity,
                &shared::components::Npc,
                &crate::zones::TempSummon,
                &Health,
            )>()
            .iter(h.app.world())
            .filter_map(|(actor, npc, summon, health)| {
                let camp = match npc.template_id {
                    3636 => 0,
                    5048 => 1,
                    _ => return None,
                };
                (summon.summoner == disciple && health.current > 0.0).then_some((actor, camp))
            })
            .collect();
        for &(actor, camp) in &waves {
            let first = *first_wave_at[camp].get_or_insert(tick);
            if tick - first >= 200 {
                h.hit_pct(actor, 1_000.0);
                camp_counts[camp] += 1;
            }
        }
        if camp_counts == [2, 3] && point.distance(endpoint) < 0.2 {
            arrived = true;
            break;
        }
    }
    if !arrived {
        let (x, y, z): (f32, f32, f32) = conn.query_row(
            "SELECT PositionX,PositionY,PositionZ FROM tdb_waypoint_path_node WHERE PathId=367801 AND NodeId=2",
            [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap();
        let from = Vec3::new(previous.x, previous.z, -previous.y);
        let target = Vec3::new(x, z, -y);
        let navigation = h.app.world().resource::<crate::navigation::Navigation>();
        println!(
            "stopped source route {from:?} -> {target:?}: {:?}",
            navigation.query_path(43, from, target)
        );
    }
    assert!(
        arrived,
        "source three-path walk did not reach final endpoint: start={start:?} last={previous:?} end={endpoint:?} waves={camp_counts:?} cleansing={saw_cleansing}"
    );
    assert_eq!(camp_counts, [2, 3], "actual wave death boundaries both ran");
    assert!(saw_cleansing, "actual path1 arrival activated real6270");
    assert!(
        walked > start.distance(endpoint),
        "source route is not a straight shortcut"
    );
    assert!(
        h.inbox(0)
            .chat
            .iter()
            .any(|message| message.content.contains("The caverns have been purified"))
    );
    assert!(
        !h.inbox(1)
            .chat
            .iter()
            .any(|message| message.content.contains("The caverns have been purified"))
    );
    println!(
        "physical source walk {start:?} -> {previous:?}; {walked}yd; waves={camp_counts:?}; real6270={saw_cleansing}"
    );
}
