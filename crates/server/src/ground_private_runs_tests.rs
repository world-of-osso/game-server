//! Private-run destination data must match the Retail ground used by authority.
use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use bevy::ecs::system::RunSystemOnce;
use serde::Deserialize;
use shared::components::{Health, MovementControl, Player, Position, VerticalVelocity};

use super::*;
use crate::networking::FallTracker;

#[derive(Deserialize)]
struct PrivatePositions {
    positions: BTreeMap<String, Destination>,
}

#[derive(Deserialize)]
struct Destination {
    map: u32,
    x: f32,
    y: f32,
    z: f32,
}

fn retail_ground() -> GroundMap {
    let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data");
    GroundMap::new(&data, HashMap::from([(0, 775_971)]))
}

#[test]
fn private_live_destinations_remain_supported_through_server_gravity() {
    let fixture: PrivatePositions =
        serde_json::from_str(include_str!("../../../scripts/private-live-positions.json")).unwrap();
    assert_eq!(fixture.positions.len(), 2);
    for (name, destination) in fixture.positions {
        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins)
            .add_message::<crate::death::PlayerDeathTransition>()
            .insert_resource(retail_ground());
        app.update();
        // Diagnose against the parsed surface, not legacy NPC/start-location Z.
        let column = Vec3::new(destination.x, 100.0, -destination.y);
        let GroundProbe::Supported(floor) = app
            .world_mut()
            .resource_mut::<GroundMap>()
            .ground_at(destination.map, column)
        else {
            panic!("{name}: real Retail ground absent");
        };
        let player = app
            .world_mut()
            .spawn((
                Position {
                    x: destination.x,
                    y: destination.z,
                    z: -destination.y,
                },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                Player {
                    name: "Terrainproof".into(),
                    race: 1,
                    class: 2,
                    appearance: Default::default(),
                },
                MovementControl::default(),
                // A sentinel velocity proves gravity actually processed this entity.
                VerticalVelocity(-10.0),
                FallTracker::grounded(destination.z),
                crate::maps::InMap::continent(destination.map),
            ))
            .id();
        // Same authority system as a live login, including fall tracking and void death.
        for tick in 0..200 {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(Duration::from_millis(50));
            app.world_mut()
                .run_system_once(crate::networking::apply_terrain_gravity)
                .unwrap();
            let position = app.world().get::<Position>(player).unwrap();
            assert!(
                (position.y - floor).abs() < 0.001,
                "{name} tick {tick}: supplied Z={}, Retail floor={floor}, authority Y={}",
                destination.z,
                position.y
            );
            assert_eq!(app.world().get::<Health>(player).unwrap().current, 100.0);
            assert_eq!(
                app.world().get::<VerticalVelocity>(player).unwrap().0,
                0.0,
                "{name}: gravity did not resolve the sentinel downward velocity"
            );
        }
    }
}

#[test]
fn stale_private_destinations_are_not_repaired_by_collision_clamps() {
    let mut ground = retail_ground();
    for feet in [
        Vec3::new(-8985.0, 76.9624, 179.0),
        Vec3::new(1407.0, 54.0, -1090.0),
    ] {
        let surface = ground.ground_at(0, feet.with_y(100.0));
        println!("destination={feet:?}, Retail surface={surface:?}");
        assert_eq!(ground.ground_at(0, feet), GroundProbe::Unsupported);
    }
}
