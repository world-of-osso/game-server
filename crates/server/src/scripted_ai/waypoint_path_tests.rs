//! Generic movement-validation regressions, not invented encounter path data.
use super::*;
use crate::scripted_ai::{CreatureAi, CreatureAiState, Ctx};
use std::time::Duration;

#[test]
fn waypoint_source_short_initial_segment_still_moves_to_the_real_destination() {
    // TC MoveSpline.cpp::_checkPathLengths starts at segment1, not segment0.
    let spline = launch(
        vec![
            Vec3::ZERO,
            Vec3::new(0.05, 0.0, 0.0),
            Vec3::new(5.0, 0.0, 0.0),
        ],
        false,
        0.0,
        5.0,
    )
    .expect("source admits a short first crossing");
    // Native cumulative millisecond truncation permits 5mm per segment here.
    assert!((spline.position(501).x - 2.5).abs() < 0.01);
    assert_eq!(spline.position(spline.end_ms), Vec3::new(5.0, 0.0, 0.0));
}

#[test]
fn waypoint_collinear_portals_move_without_cutting_turns_or_height_changes() {
    let original = vec![
        Vec3::ZERO,
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.05, 0.0, 0.0),
        Vec3::new(1.09, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 1.0),
        Vec3::new(2.0, 1.0, 1.0),
    ];
    let controls = coalesce_collinear_crossings(original.clone());
    let spline = launch(controls, false, 0.0, 2.0).expect("collinear tiny portals are not turns");
    // The actor stays at its actual start during launch. Evaluated movement uses
    // the source spline domain beginning at its1ms minimal-duration timestamp.
    for elapsed in MINIMAL_DURATION_MS..=spline.end_ms {
        let point = spline.position(elapsed);
        let distance = original
            .windows(2)
            .map(|pair| segment_distance_squared(point, pair[0], pair[1]))
            .min_by(f32::total_cmp)
            .unwrap();
        assert!(
            distance <= PORTAL_COLLINEAR_EPSILON * PORTAL_COLLINEAR_EPSILON,
            "movement never shortcuts a turn/height transition: {point:?}"
        );
    }
    assert_eq!(spline.position(spline.end_ms), *original.last().unwrap());
    let short_height_step = vec![
        Vec3::ZERO,
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(2.0, 0.05, 0.0),
        Vec3::new(3.0, 0.05, 0.0),
    ];
    assert!(
        launch(
            coalesce_collinear_crossings(short_height_step),
            false,
            0.0,
            2.0
        )
        .is_none(),
        "a real short height corner remains rejected, never flattened"
    );
}

#[test]
#[ignore = "requires WAILING_NAV_ROOT containing the private authored map43 bake"]
fn waypoint_authored_wailing_node2_launches_and_reaches_collision_floor() {
    let root = std::path::PathBuf::from(std::env::var_os("WAILING_NAV_ROOT").unwrap());
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let paths = load_script_paths(&db).unwrap();
    let path = &paths.0[&367801];
    let node = path.nodes.iter().position(|(id, _)| *id == 2).unwrap();
    // Captured real launch boundary after walking path0 and killing its two Ravagers.
    let from = Vec3::new(-100.10522, 231.44879, -90.95079);
    let spawn = SpawnPoint {
        x: from.x,
        y: from.z,
        z: -from.y,
        wander_distance: 0.0,
        speed_walk: 1.0,
        speed_run: 1.0,
    };
    let source = Launch {
        map_id: 43,
        position: from,
        orientation: 0.0,
        spawn: &spawn,
        walking: true,
        flying: false,
    };
    let navigation = crate::navigation::Navigation::new(&root);
    let controls = query_path_controls(path, node, &source, Some(&navigation)).unwrap();
    let spline = launch(controls.clone(), false, 0.0, 2.5)
        .unwrap_or_else(|| panic!("authored node2 must launch: {controls:?}"));
    assert_eq!(spline.position(MINIMAL_DURATION_MS), from);
    let destination = spline.position(spline.end_ms);
    assert!((destination.x - path.nodes[node].1.x).abs() < 0.01);
    assert!((destination.y - path.nodes[node].1.y).abs() < 0.01);
    let maps = crate::maps::store::load_map_store(&db).unwrap();
    let mut ground = crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        HashMap::from([(43, maps.map(43).unwrap().wdt_file_data_id)]),
    );
    let feet = Vec3::new(destination.x, destination.z, -destination.y);
    let crate::ground::GroundProbe::Supported(floor) = ground.creature_ground_at(43, feet) else {
        panic!("node2 destination requires exact collision support");
    };
    assert!(
        (destination.z - floor).abs() < 0.05,
        "node2 must normalize to exact collision, not approximate nav height: {destination:?} floor={floor}"
    );
    println!("authored node2 reached {destination:?}: {controls:?}");
}

#[test]
fn waypoint_flying_and_exact_controls_do_not_require_ground_navigation() {
    let spawn = SpawnPoint {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        wander_distance: 0.0,
        speed_walk: 1.0,
        speed_run: 1.0,
    };
    let mut path = ScriptPath {
        id: 1,
        move_type: MoveType::Run,
        flags: 0,
        velocity: None,
        has_node_timing: false,
        nodes: vec![(0, Vec3::new(2.0, 3.0, 4.0)), (1, Vec3::new(4.0, 5.0, 6.0))],
    };
    let source = Launch {
        map_id: 43,
        position: Vec3::ZERO,
        orientation: 0.0,
        spawn: &spawn,
        walking: false,
        flying: true,
    };
    assert_eq!(
        query_path_controls(&path, 0, &source, None).unwrap(),
        vec![Vec3::ZERO, path.nodes[0].1]
    );
    path.flags = FLAG_EXACT_SPLINE;
    assert_eq!(
        query_path_controls(&path, 0, &source, None).unwrap(),
        vec![Vec3::ZERO, path.nodes[0].1, path.nodes[1].1]
    );
    let source = Launch {
        flying: false,
        ..source
    };
    assert_eq!(
        query_path_controls(&path, 0, &source, None).unwrap(),
        vec![Vec3::ZERO, path.nodes[0].1, path.nodes[1].1]
    );
}

#[derive(Resource, Default)]
struct ArrivalReceipts(Vec<(u32, u32)>);

struct ArrivalConsumer;
impl CreatureAi for ArrivalConsumer {
    fn waypoint_path_ended(&mut self, ctx: &mut Ctx, node: u32, path: u32) {
        ctx.world
            .resource_mut::<ArrivalReceipts>()
            .0
            .push((node, path));
    }
}

#[test]
fn waypoint_rejected_interior_segment_never_moves_or_reports_a_path_arrival() {
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    world.init_resource::<ArrivalReceipts>();
    world.insert_resource(ScriptPaths(HashMap::from([(
        1,
        Arc::new(ScriptPath {
            id: 1,
            move_type: MoveType::Run,
            flags: FLAG_EXACT_SPLINE,
            velocity: None,
            has_node_timing: false,
            nodes: vec![
                (0, Vec3::new(2.0, 0.0, 0.0)),
                (1, Vec3::new(2.05, 0.0, 0.0)),
                (2, Vec3::new(5.0, 0.0, 0.0)),
            ],
        }),
    )])));
    let actor = world
        .spawn((
            bevy(Vec3::ZERO),
            crate::maps::InMap::continent(0),
            SpawnPoint {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                wander_distance: 0.0,
                speed_walk: 1.0,
                speed_run: 1.0,
            },
            CreatureMotion::Still,
            CreatureAiState {
                ai: Some(Box::new(ArrivalConsumer)),
                pending: Default::default(),
            },
        ))
        .id();
    assert!(move_path(&mut world, actor, 1));
    for _ in 0..200 {
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(50));
        path_movement_system(&mut world);
    }
    assert_eq!(*world.get::<Position>(actor).unwrap(), bevy(Vec3::ZERO));
    assert!(
        world.resource::<ArrivalReceipts>().0.is_empty(),
        "no synthetic script arrival after rejected controls"
    );
}
