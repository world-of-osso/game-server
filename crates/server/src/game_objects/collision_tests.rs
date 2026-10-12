use super::*;
use std::{collections::HashMap, path::PathBuf, time::Instant};

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
    let triangles = cache.load_triangles(data, spawn.info.display_id).unwrap();
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
#[ignore = "explicit full-arena live collision measurement"]
fn arena_live_collision_full_six_player_tick_benchmark() {
    for map_id in [2759, 617, 2373] {
        let (mut app, objects) = arena_app(map_id);
        let map = InMap {
            map_id,
            instance_id: 786,
        };
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
            "arena_live_collision map={map_id} players=6 objects={} active_hulls={count} ticks=1200 floor_queries_per_tick=12 wall_movements_per_tick=6 p50_us={:.3} p95_us={:.3} p99_us={:.3} max_us={:.3} observed_hits={hits}",
            objects.len(),
            samples[600],
            samples[1140],
            samples[1188],
            samples[1199]
        );
        assert!(
            samples[1140] < 16666.0,
            "live/static six-player query+sync p95 must fit one 60Hz frame"
        );
    }
}
