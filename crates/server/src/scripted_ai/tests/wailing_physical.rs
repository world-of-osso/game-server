//! Actual source paths0–2 and their camp callbacks, without injected arrivals.
use super::*;

// Existing exact-ground fixture tolerance (navigation_tests/followers tests).
const GROUND_TOLERANCE: f32 = 0.05;
const NAV_FLOAT_TOLERANCE: f32 = 0.0001;
// TC MoveSpline cumulative lengths/durations are integer milliseconds.
const SPLINE_CLOCK_PRECISION: f32 = 0.001;

struct PhysicalStep {
    from: Vec3,
    to: Vec3,
    speed_before: f32,
    tick: usize,
}

fn physical_source_speed(h: &Harness, actor: Entity) -> f32 {
    let world = h.app.world();
    let spawn = world.get::<crate::creature_ai::SpawnPoint>(actor).unwrap();
    match world.get::<CreatureMotion>(actor).unwrap() {
        CreatureMotion::Still => 0.0,
        CreatureMotion::Walk => spawn.speed_walk * 2.5,
        CreatureMotion::Run => {
            let multiplier = world
                .get::<crate::auras::AuraStatMods>(actor)
                .map_or(1.0, |mods| mods.movement_speed);
            spawn.speed_run * 7.0 * multiplier
        }
    }
}

fn assert_physical_ground_step(h: &mut Harness, actor: Entity, map: InMap, step: &PhysicalStep) {
    let delta = h.app.world().resource::<Time<Fixed>>().delta_secs();
    // A handoff may change gait after the frame's sole position update.
    let speed = step.speed_before.max(physical_source_speed(h, actor));
    let horizontal = (step.to.x - step.from.x).hypot(step.to.y - step.from.y);
    let budget = speed * (delta + SPLINE_CLOCK_PRECISION);
    assert!(
        horizontal <= budget + NAV_FLOAT_TOLERANCE,
        "horizontal source budget exceeded tick{}: {}yd > {}yd, speed={} dt={}; {:?}->{:?}",
        step.tick,
        horizontal,
        budget,
        speed,
        delta,
        step.from,
        step.to
    );
    if horizontal <= NAV_FLOAT_TOLERANCE {
        assert!(
            (step.to.z - step.from.z).abs() <= GROUND_TOLERANCE,
            "vertical snap without horizontal travel tick{}: {:?}->{:?}",
            step.tick,
            step.from,
            step.to
        );
        return;
    }
    let from = Vec3::new(step.from.x, step.from.z, -step.from.y);
    let to = Vec3::new(step.to.x, step.to.z, -step.to.y);
    let floors = {
        let mut ground = h.app.world_mut().resource_mut::<crate::ground::GroundMap>();
        (
            ground.creature_ground_in(map, from),
            ground.creature_ground_in(map, to),
        )
    };
    let (
        crate::ground::GroundProbe::Supported(before_floor),
        crate::ground::GroundProbe::Supported(floor),
    ) = floors
    else {
        panic!(
            "physical step lacks collision support tick{}: {floors:?}",
            step.tick
        );
    };
    assert!(
        (step.to.z - floor).abs() <= GROUND_TOLERANCE,
        "feet must follow exact floor tick{}: {:?}->{:?}; floor={before_floor}->{floor}",
        step.tick,
        step.from,
        step.to
    );
    // Detour's WALK_FLAG ray stays on the bake's AGENT_SLOPE/AGENT_CLIMB
    // surface, not an invented slope bound on floor-snapped 3D displacement.
    let surface = h
        .app
        .world()
        .resource::<crate::navigation::Navigation>()
        .first_collision(map.map_id, from, to)
        .unwrap()
        .expect("baked walkable surface");
    let off_surface = (surface.x - to.x).hypot(surface.z - to.z);
    assert!(
        off_surface <= NAV_FLOAT_TOLERANCE,
        "movement left the walkable nav surface tick{}: {from:?}->{to:?}; clipped={surface:?}",
        step.tick
    );
}

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
    // The resource harness seeds Lord Cobrahn3669 near its observer. The source
    // escort requires all four druids defeated; leaving this prerequisite alive
    // but Passive preserves its already-started chase and blocks the walk.
    // Use the actual death boundary as pre-clear, not a stance/faction shortcut.
    h.hit_pct(unrelated, 1_000.0);
    h.advance(0.05);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(unrelated)
            .is_some(),
        "seeded Cobrahn crossed the real death boundary before escort staging"
    );
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
        let speed_before = physical_source_speed(&h, disciple);
        h.advance(0.05);
        let point = h.wow_position(disciple);
        let step = point.distance(previous);
        assert_physical_ground_step(
            &mut h,
            disciple,
            map,
            &PhysicalStep {
                from: previous,
                to: point,
                speed_before,
                tick,
            },
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
        let stopped = h.app.world().get::<CreatureMotion>(disciple) == Some(&CreatureMotion::Still);
        if camp_counts == [2, 3] && point.distance(endpoint) <= GROUND_TOLERANCE && stopped {
            arrived = true;
            break;
        }
    }
    if !arrived {
        let world = h.app.world();
        if let Some(crate::creature_ai::CreatureState::Pursuing { target }) =
            world.get::<crate::creature_ai::CreatureState>(disciple)
        {
            let target = Entity::from_bits(*target);
            println!(
                "stalled actor={disciple:?} unrelated={unrelated:?} target={target:?}: npc={:?} health={:?} map={:?} state={:?} react={:?} dead={} castercombat={:?}",
                world.get::<shared::components::Npc>(target),
                world.get::<Health>(target),
                world.get::<InMap>(target),
                world.get::<crate::creature_ai::CreatureState>(target),
                world.get::<ReactState>(target),
                world.get::<crate::combat::DeathState>(target).is_some(),
                world.get::<CombatStatus>(disciple)
            );
        }
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
