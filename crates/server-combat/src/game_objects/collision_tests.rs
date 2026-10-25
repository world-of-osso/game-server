use super::*;
use std::{collections::HashMap, path::PathBuf, time::Instant};

#[test]
fn source_ship_deck_collision_follows_the_transport_only_in_its_copy() {
    let data = PathBuf::from(std::env::var_os("GAME_SERVER_GROUND_DIR").unwrap());
    let catalog = crate::transport_catalog::fixture_catalog();
    let ship = &catalog.0[&35];
    let copy = InMap {
        map_id: 1803,
        instance_id: 786,
    };
    let other = InMap {
        instance_id: 787,
        ..copy
    };
    let mut app = App::new();
    app.add_plugins(CollisionIndexPlugin);
    app.insert_resource(GroundMap::new(&data, HashMap::new()));
    app.add_systems(FixedUpdate, sync_live_objects);
    let entity = app
        .world_mut()
        .spawn((
            copy,
            ship.info.clone(),
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            GameObjectCollision(true),
        ))
        .id();
    for time in [0, 30_000] {
        let pose = ship.route.pose_at(time);
        let [x, y, z] = pose.position;
        app.world_mut().entity_mut(entity).insert((
            Position { x, y: z, z: -y },
            Rotation {
                x: 0.0,
                y: pose.orientation + std::f32::consts::FRAC_PI_2,
                z: 0.0,
            },
        ));
        app.world_mut().run_schedule(FixedUpdate);
        let feet = pose
            .local_to_world(shared::transport_route::TransportPose {
                position: [-30.0, 0.0, 20.55],
                orientation: 0.0,
            })
            .position;
        let feet = Vec3::new(feet[0], feet[2], -feet[1]);
        let mut ground = app.world_mut().resource_mut::<GroundMap>();
        let height = match ground.ground_in(copy, feet) {
            crate::ground::GroundProbe::Supported(height) => height,
            other => panic!("authored gunship deck missing at {time}ms: {other:?}"),
        };
        assert!((height - z - 20.6032).abs() < 0.01, "{height}");
        assert!(!matches!(
            ground.ground_in(other, feet),
            crate::ground::GroundProbe::Supported(_)
        ));
    }
    app.world_mut()
        .entity_mut(entity)
        .insert(GameObjectCollision(false));
    app.world_mut().run_schedule(FixedUpdate);
    let pose = ship
        .route
        .pose_at(30_000)
        .local_to_world(shared::transport_route::TransportPose {
            position: [-30.0, 0.0, 20.55],
            orientation: 0.0,
        });
    let feet = Vec3::new(pose.position[0], pose.position[2], -pose.position[1]);
    assert!(!matches!(
        app.world_mut()
            .resource_mut::<GroundMap>()
            .ground_in(copy, feet),
        crate::ground::GroundProbe::Supported(_)
    ));
    app.world_mut()
        .entity_mut(entity)
        .insert((GameObjectCollision(true), GameObjectVisible(false)));
    app.world_mut().run_schedule(FixedUpdate);
    assert!(!matches!(
        app.world_mut()
            .resource_mut::<GroundMap>()
            .ground_in(copy, feet),
        crate::ground::GroundProbe::Supported(_)
    ));
    app.world_mut()
        .entity_mut(entity)
        .insert((GameObjectVisible(true), other));
    app.world_mut().run_schedule(FixedUpdate);
    assert!(!matches!(
        app.world_mut()
            .resource_mut::<GroundMap>()
            .ground_in(copy, feet),
        crate::ground::GroundProbe::Supported(_)
    ));
    assert!(matches!(
        app.world_mut()
            .resource_mut::<GroundMap>()
            .ground_in(other, feet),
        crate::ground::GroundProbe::Supported(_)
    ));
    let mut unrelated = ship.info.clone();
    unrelated.go_type = 5;
    app.world_mut().entity_mut(entity).insert(unrelated);
    app.world_mut().run_schedule(FixedUpdate);
    assert!(!matches!(
        app.world_mut()
            .resource_mut::<GroundMap>()
            .ground_in(other, feet),
        crate::ground::GroundProbe::Supported(_)
    ));
    app.world_mut().entity_mut(entity).insert(ship.info.clone());
    app.world_mut().run_schedule(FixedUpdate);
    assert!(matches!(
        app.world_mut()
            .resource_mut::<GroundMap>()
            .ground_in(other, feet),
        crate::ground::GroundProbe::Supported(_)
    ));
    app.world_mut().despawn(entity);
    app.world_mut().run_schedule(FixedUpdate);
    assert!(!matches!(
        app.world_mut()
            .resource_mut::<GroundMap>()
            .ground_in(other, feet),
        crate::ground::GroundProbe::Supported(_)
    ));
}

fn arena_app(map_id: u32) -> (App, Vec<(Entity, crate::game_objects::GameObjectSpawn)>) {
    let data = PathBuf::from(std::env::var_os("GAME_SERVER_GROUND_DIR").unwrap());
    let db = rusqlite::Connection::open_with_flags(
        data.join("diagnostics/arena-coverage-2026-10-12/canonical-proof-data/world.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let catalog = crate::pvp::arena::Catalog::load(&db).unwrap();
    let wdt = db
        .query_row(
            "SELECT WdtFileDataID FROM map WHERE ID=?",
            [map_id],
            |row| row.get(0),
        )
        .unwrap();
    let spawns = crate::game_objects::load_game_object_spawns(&db).unwrap();
    let mut app = App::new();
    app.add_plugins(CollisionIndexPlugin);
    app.insert_resource(GroundMap::new(&data, HashMap::from([(map_id, wdt)])));
    app.insert_resource(LineOfSight::load(&data.join("los")).unwrap());
    app.insert_resource(catalog);
    app.add_systems(FixedUpdate, sync_live_objects);
    let map = InMap {
        map_id,
        instance_id: 786,
    };
    let objects = spawns
        .0
        .into_iter()
        .filter(|spawn| spawn.map_id == map_id)
        .map(|spawn| {
            let entity = app
                .world_mut()
                .spawn((map, crate::game_objects::game_object_bundle(&spawn)))
                .id();
            (entity, spawn)
        })
        .collect();
    app.world_mut().run_schedule(FixedUpdate);
    (app, objects)
}

/// Cross an actual placed vertical face; no fabricated wall or bounding-box substitute.
fn wall_segment(spawn: &crate::game_objects::GameObjectSpawn, data: &Path) -> (Vec3, Vec3) {
    let mut cache = HullCache::default();
    let triangles = cache
        .load_triangles(data, spawn.info.display_id, spawn.info.go_type)
        .unwrap();
    let face = triangles
        .iter()
        .find(|face| {
            let normal = (face[1] - face[0])
                .cross(face[2] - face[0])
                .normalize_or_zero();
            normal.y.abs() < 0.1 && normal.length_squared() > 0.5
        })
        .expect("authored wall must have a vertical face");
    let position = crate::game_objects::bevy_position(spawn.position);
    let transform = Affine3A::from_scale_rotation_translation(
        Vec3::splat(spawn.info.scale),
        Quat::from_rotation_y(spawn.orientation),
        Vec3::new(position.x, position.y, position.z),
    );
    let face = face.map(|point| transform.transform_point3(point));
    let centre = (face[0] + face[1] + face[2]) / 3.0;
    let normal = (face[1] - face[0]).cross(face[2] - face[0]).normalize();
    (centre - normal, centre + normal)
}

#[test]
fn arena_live_walls_follow_waterfall_state_and_empyrean_despawn_per_copy() {
    for (map_id, entry) in [(617, 194395), (2373, 352927)] {
        let (mut app, objects) = arena_app(map_id);
        let (entity, spawn) = objects
            .iter()
            .find(|(_, spawn)| spawn.info.entry == entry)
            .unwrap();
        let map = InMap {
            map_id,
            instance_id: 786,
        };
        let other = InMap {
            map_id,
            instance_id: 787,
        };
        // Dalaran's source spawn is initially inactive; activate its hazard collision.
        app.world_mut()
            .entity_mut(*entity)
            .insert(GameObjectCollision(true));
        app.world_mut().run_schedule(FixedUpdate);
        let data = app.world().resource::<GroundMap>().data_dir().to_path_buf();
        let (start, end) = wall_segment(spawn, &data);
        assert!(
            !app.world()
                .resource::<GroundMap>()
                .live_clear(map, start, end)
        );
        assert!(
            app.world()
                .resource::<GroundMap>()
                .live_clear(other, start, end)
        );
        // LOS and movement use the same exact instance/state, independently of static bake.
        assert!(
            !app.world()
                .resource::<LineOfSight>()
                .live_clear(map, start, end)
        );
        app.world_mut()
            .entity_mut(*entity)
            .insert(GameObjectCollision(false));
        app.world_mut().run_schedule(FixedUpdate);
        assert!(
            app.world()
                .resource::<GroundMap>()
                .live_clear(map, start, end)
        );
        assert!(
            app.world()
                .resource::<LineOfSight>()
                .live_clear(map, start, end)
        );
        app.world_mut()
            .entity_mut(*entity)
            .insert(GameObjectCollision(true));
        app.world_mut().run_schedule(FixedUpdate);
        assert!(
            !app.world()
                .resource::<GroundMap>()
                .live_clear(map, start, end)
        );
        app.world_mut()
            .entity_mut(*entity)
            .insert(GameObjectVisible(false));
        app.world_mut().run_schedule(FixedUpdate);
        assert!(
            app.world()
                .resource::<GroundMap>()
                .live_clear(map, start, end)
        );
        app.world_mut()
            .entity_mut(*entity)
            .insert(GameObjectVisible(true));
        app.world_mut().run_schedule(FixedUpdate);
        app.world_mut().despawn(*entity);
        app.world_mut().run_schedule(FixedUpdate);
        assert!(
            app.world()
                .resource::<GroundMap>()
                .live_clear(map, start, end)
        );
    }
}

#[test]
fn arena_live_imported_objects_require_unphased_source_until_real_phasing_exists() {
    let data = PathBuf::from(std::env::var_os("GAME_SERVER_GROUND_DIR").unwrap());
    let db = rusqlite::Connection::open_with_flags(
        data.join("diagnostics/arena-coverage-2026-10-12/canonical-proof-data/world.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let total: u32 = db
        .query_row(
            "SELECT COUNT(*) FROM content_gameobject WHERE map IN (SELECT map_id FROM arena_map)",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let phased: Vec<(u32,u32,u32)> = db.prepare("SELECT guid,map,phaseMask FROM content_gameobject WHERE map IN (SELECT map_id FROM arena_map) AND phaseMask <> 1 ORDER BY guid").unwrap()
        .query_map([], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap().collect::<Result<_,_>>().unwrap();
    assert!(
        total >= 383,
        "proof must include the full arena import, not empty data"
    );
    assert!(
        phased.is_empty(),
        "collision cannot accept phased arena content until production phasing exists: {phased:?}"
    );
}

#[test]
#[ignore = "explicit full-arena collision query microbenchmark"]
fn arena_live_collision_six_position_query_microbenchmark() {
    for map_id in [2759, 617, 2373] {
        let (mut app, objects) = arena_app(map_id);
        let map = InMap {
            map_id,
            instance_id: 786,
        };
        // Include the active-waterfall worst case, not only source-initial inactive state.
        for (entity, spawn) in &objects {
            if spawn.info.entry == 194395 {
                app.world_mut()
                    .entity_mut(*entity)
                    .insert(GameObjectCollision(true));
            }
        }
        app.world_mut().run_schedule(FixedUpdate);
        let feet: Vec<_> = objects
            .iter()
            .filter(|(_, spawn)| matches!(spawn.info.go_type, 0 | 5 | 35))
            .map(|(_, spawn)| {
                let p = crate::game_objects::bevy_position(spawn.position);
                Vec3::new(p.x, p.y + 0.5, p.z)
            })
            .collect();
        let count = app.world().resource::<GroundMap>().live_hull_count(map);
        assert!(count > 0);
        let mut samples = Vec::with_capacity(1200);
        let mut hits = 0;
        // Warm all static tiles before measuring steady-state per-tick queries.
        for &point in &feet {
            app.world_mut()
                .resource_mut::<GroundMap>()
                .ground_in(map, point);
        }
        for tick in 0..1200 {
            let started = Instant::now();
            app.world_mut().run_schedule(FixedUpdate);
            let mut ground = app.world_mut().resource_mut::<GroundMap>();
            for player in 0..6 {
                let point = feet[(tick + player) % feet.len()];
                for _ in 0..2 {
                    hits += usize::from(matches!(
                        ground.ground_in(map, point),
                        crate::ground::GroundProbe::Supported(_)
                    ));
                }
                let moved = ground.clamp_live_movement(map, point, point + Vec3::X * 7.0);
                hits += usize::from(moved.x < point.x + 7.0);
            }
            samples.push(started.elapsed().as_secs_f64() * 1e6);
        }
        samples.sort_by(f64::total_cmp);
        println!(
            "arena_live_collision_query_microbenchmark map={map_id} simulated_positions=6 objects={} active_hulls={count} warmup=all_static_feet samples=1200 floor_queries_per_sample=12 wall_movements_per_sample=6 p50_us={:.3} p95_us={:.3} p99_us={:.3} max_us={:.3} observed_hits={hits}",
            objects.len(),
            samples[600],
            samples[1140],
            samples[1188],
            samples[1199]
        );
        assert!(
            samples[1140] < 16666.0,
            "live/static six-position query+sync microbenchmark p95 must fit a 16.666ms query budget; not whole-frame/tick proof"
        );
    }
}

#[test]
fn unrelated_world_objects_fit_one_catch_up_frame() {
    let data = PathBuf::from(std::env::var_os("GAME_SERVER_GROUND_DIR").unwrap());
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let spawn = crate::game_objects::load_game_object_spawns(&db)
        .unwrap()
        .0
        .into_iter()
        .find(|spawn| spawn.map_id == 0 && spawn.info.go_type != 15)
        .expect("authored nontransport world object");
    let mut app = App::new();
    app.add_plugins(CollisionIndexPlugin);
    app.insert_resource(GroundMap::new(&data, HashMap::new()));
    app.add_systems(Update, sync_live_objects);
    app.world_mut().spawn_batch((0..250_000).map(|_| {
        (
            InMap::continent(0),
            crate::game_objects::game_object_bundle(&spawn),
        )
    }));
    app.update();
    let fastest = (0..3)
        .map(|_| {
            let start = Instant::now();
            for _ in 0..5 {
                app.update();
            }
            start.elapsed()
        })
        .min()
        .unwrap();
    eprintln!("250000 unrelated objects/five collision updates: {fastest:?}");
    assert!(
        fastest < std::time::Duration::from_millis(50),
        "unrelated objects exhaust collision frame: {fastest:?}"
    );
}
