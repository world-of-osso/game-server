use super::*;
use shared::components::{Npc, Position, ThreatTable, UnitFlags};

fn npc(world: &mut World, map: crate::maps::InMap, entry: u32, point: Vec3) -> Entity {
    world
        .spawn((
            Npc {
                template_id: entry,
                name: String::new(),
            },
            map,
            Position {
                x: point.x,
                y: point.y,
                z: point.z,
            },
            crate::creature_ai::SpawnPoint {
                x: point.x,
                y: point.y,
                z: point.z,
                wander_distance: 0.0,
                speed_walk: 1.0,
                speed_run: 1.0,
            },
        ))
        .id()
}

fn die(world: &mut World, entity: Entity) {
    world
        .entity_mut(entity)
        .insert(crate::combat::DeathState { respawn_timer: 0.0 });
    on_unit_death(world, entity);
}

#[test]
#[ignore = "requires real Academy baked LOS geometry"]
fn academy_crawth_descent_clears_real_geometry() {
    let root = std::path::PathBuf::from(
        std::env::var_os("GAME_SERVER_GROUND_DIR").expect("ground assets"),
    );
    let mut map = los::LosMap::new(&root.join("los/2526"));
    let start = Vec3::new(2430.3, 959.735, 2978.93);
    let end = Vec3::new(2369.31, 912.2291, 3041.47);
    let duration = start.distance(end) / 50.0;
    let height = shared::skyriding::GRAVITY * duration * duration / 8.0;
    let at = |t: f32| start.lerp(end, t) + Vec3::Y * (height * 4.0 * t * (1.0 - t) + 1.0);
    for i in 0..100 {
        let from = at(i as f32 / 100.0);
        let to = at((i + 1) as f32 / 100.0);
        assert!(
            map.clear(from, to).unwrap(),
            "collision on Crawth descent segment {i}: {from:?} -> {to:?}"
        );
    }
}

#[test]
fn academy_intro_unlocks_only_after_local_pack_dies_and_crawth_lands() {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default());
    app.add_systems(
        Update,
        (
            super::super::jump_movement::release_finished_jumps,
            finish_crawth_descent,
        )
            .chain(),
    );
    let map = crate::maps::InMap {
        map_id: 2526,
        instance_id: 7,
    };
    let other = crate::maps::InMap {
        instance_id: 8,
        ..map
    };
    let world = app.world_mut();
    let ancient = npc(world, map, 196482, Vec3::new(1766.25, 913.659, 3086.41));
    let crawth = npc(world, map, 191736, Vec3::new(2430.3, 959.735, 2978.93));
    for boss in [ancient, crawth] {
        world.entity_mut(boss).insert(UnitFlags(832));
    }
    let lasher1 = npc(world, map, 197219, Vec3::new(1757.89, 914.589, 3049.78));
    let lasher2 = npc(world, map, 197219, Vec3::new(1760.64, 914.614, 3124.68));
    let foreign = npc(world, other, 197219, Vec3::new(1757.89, 914.589, 3049.78));
    let eagle1 = npc(world, map, 192333, Vec3::new(2358.73, 912.28156, 3047.32));
    let eagle2 = npc(world, map, 192333, Vec3::new(2371.6, 912.34436, 3051.36));
    die(world, foreign);
    die(world, lasher1);
    assert_ne!(world.get::<UnitFlags>(ancient).unwrap().0 & 0x300, 0);
    die(world, lasher2);
    assert_eq!(world.get::<UnitFlags>(ancient).unwrap().0 & 0x300, 0);
    assert!(world.get::<ThreatTable>(ancient).is_some());
    die(world, eagle1);
    assert!(world.get::<CrawthDescent>(crawth).is_none());
    die(world, eagle2);
    assert_ne!(world.get::<UnitFlags>(crawth).unwrap().0 & 0x300, 0);
    assert!(
        world
            .get::<super::super::jump_movement::JumpMovement>(crawth)
            .is_some()
    );
    world
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs(10));
    super::super::jump_movement::advance_jumps(world);
    app.update();
    let world = app.world();
    let position = world.get::<Position>(crawth).unwrap();
    assert!((position.x - 2369.31).abs() < 0.01);
    assert!((position.y - 912.2291).abs() < 0.01);
    assert!((position.z - 3041.47).abs() < 0.01);
    assert_eq!(world.get::<UnitFlags>(crawth).unwrap().0 & 0x300, 0);
    assert!(world.get::<ThreatTable>(crawth).is_some());
}

#[test]
fn route_driver_gates_match_the_production_intro_unlocks() {
    // The unlock behavior itself is proven above; the route driver must plan those same deaths.
    let gates = crate::test_support::objectives::boss_gate_entries(2526);
    assert_eq!(
        gates.into_iter().collect::<Vec<_>>(),
        vec![(191736, vec![192333]), (196482, vec![197219])]
    );
    assert!(crate::test_support::objectives::boss_gate_entries(2527).is_empty());
}
