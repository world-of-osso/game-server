//! Simulator view of transport registrations already used by server interaction systems.
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Action {
    SpellClick,
    Proximity,
}

#[derive(Clone, Debug)]
pub struct Registration {
    pub entry: u32,
    pub source: Vec3,
    pub destination: Vec3,
    pub distance: f32,
    pub action: Action,
}

/// Read registrations from the same constants and taxi network as live gameplay.
pub fn read_registrations(
    map: u32,
    db: &rusqlite::Connection,
) -> Result<Vec<Registration>, String> {
    let mut registrations = Vec::new();
    if map == crate::taxi::RUBY_FERRY_MAP {
        let network =
            crate::taxi::load_taxi_network(db).map_err(|error| format!("taxi network: {error}"))?;
        let points = network
            .scripted_path(crate::taxi::RUBY_FERRY_PATH)
            .ok_or_else(|| {
                format!(
                    "registered taxi path{} unavailable",
                    crate::taxi::RUBY_FERRY_PATH
                )
            })?;
        let endpoint = *points.last().ok_or("registered taxi path has no points")?;
        let destination = landing_floor(db, map, endpoint)?;
        let distance = points
            .windows(2)
            .map(|pair| pair[0].distance(pair[1]))
            .sum();
        registrations.push(Registration {
            entry: crate::taxi::RUBY_FERRY_ENTRY,
            source: crate::taxi::RUBY_FERRY_SOURCE,
            destination,
            distance,
            action: Action::SpellClick,
        });
    }
    if map == crate::scripted_ai::academy_transport::MAP {
        registrations.extend(crate::scripted_ai::academy_transport::PADS.iter().map(
            |(entry, source, destination)| Registration {
                entry: *entry,
                source: *source,
                destination: *destination,
                distance: source.distance(*destination),
                action: Action::Proximity,
            },
        ));
    }
    Ok(registrations)
}

fn open_ground(db: &rusqlite::Connection) -> Result<crate::ground::GroundMap, String> {
    let maps = crate::maps::store::load_map_store(db)
        .map_err(|error| format!("transport maps: {error}"))?;
    let wdts = maps
        .maps
        .values()
        .map(|entry| (entry.id, entry.wdt_file_data_id))
        .collect();
    Ok(crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        wdts,
    ))
}

fn landing_floor(db: &rusqlite::Connection, map: u32, endpoint: Vec3) -> Result<Vec3, String> {
    let mut ground = open_ground(db)?;
    match ground.creature_ground_at(map, endpoint) {
        crate::ground::GroundProbe::Supported(height) if height <= endpoint.y => {
            Ok(Vec3::new(endpoint.x, height, endpoint.z))
        }
        other => Err(format!(
            "map{map} taxi endpoint {endpoint:?} has no native landing floor: {other:?}"
        )),
    }
}

#[derive(Component)]
struct TaxiLandingPending;

fn start_player_landing(world: &mut World) {
    let owners: Vec<_> = world
        .query_filtered::<Entity, (With<TaxiLandingPending>, Without<crate::taxi::TaxiFlight>)>()
        .iter(world)
        .collect();
    for owner in owners {
        world.entity_mut(owner).remove::<TaxiLandingPending>();
        crate::scripted_ai::fall_movement::move_fall(world, owner, 0);
    }
}

pub fn install_taxi_system(app: &mut App, db: &rusqlite::Connection) -> Result<(), String> {
    let network =
        crate::taxi::load_taxi_network(db).map_err(|error| format!("taxi network: {error}"))?;
    app.insert_resource(network);
    app.insert_resource(open_ground(db)?);
    app.add_systems(
        FixedUpdate,
        (crate::taxi::fly_taxis, start_player_landing).chain(),
    );
    Ok(())
}

pub fn interaction_distance() -> f32 {
    crate::npc_interaction::INTERACTION_DISTANCE
}

pub fn in_progress(world: &World, unit: Entity) -> bool {
    world.get::<crate::taxi::TaxiFlight>(unit).is_some()
        || world.get::<TaxiLandingPending>(unit).is_some()
        || world
            .get::<crate::scripted_ai::fall_movement::FallMovement>(unit)
            .is_some()
        || world
            .get::<crate::scripted_ai::academy_transport::AcademyPadFlight>(unit)
            .is_some()
}

pub fn activate(
    world: &mut World,
    owner: Entity,
    npc: Entity,
    action: Action,
) -> Result<(), String> {
    match action {
        Action::SpellClick => {
            let entry = world
                .get::<shared::components::Npc>(npc)
                .ok_or("transport source is not an NPC")?
                .template_id;
            crate::npc_interaction::spellclick::cast_spellclick(world, owner, npc, entry);
            if world.get::<crate::taxi::TaxiFlight>(owner).is_none() {
                return Err(format!(
                    "transport spellclick npc{entry} did not start taxi flight"
                ));
            }
            world.entity_mut(owner).insert(TaxiLandingPending);
        }
        Action::Proximity => {}
    }
    Ok(())
}

#[test]
fn headless_taxi_landing_uses_native_fall_to_the_ground() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(crate::ground::GroundMap::flat_arena(10.0));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(50),
    ));
    app.insert_resource(Time::<Fixed>::from_hz(20.0));
    app.add_systems(
        FixedPreUpdate,
        crate::scripted_ai::fall_movement::release_finished_falls,
    );
    app.add_systems(
        FixedUpdate,
        (
            start_player_landing,
            crate::scripted_ai::fall_movement::advance_falls,
        )
            .chain(),
    );
    let owner = app
        .world_mut()
        .spawn((
            TaxiLandingPending,
            crate::maps::InMap {
                map_id: 2521,
                instance_id: 1,
            },
            shared::components::Position {
                x: 0.0,
                y: 20.0,
                z: 0.0,
            },
        ))
        .id();
    for _ in 0..100 {
        app.update();
    }
    let position = app
        .world()
        .get::<shared::components::Position>(owner)
        .unwrap();
    assert!((position.y - 10.0).abs() < 0.1);
    assert!(!in_progress(app.world(), owner));
}

#[test]
fn ruby_registration_uses_native_taxi_points_and_academy_uses_pad_registry() {
    let path = crate::test_data::world_db_path();
    let db =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let ruby = read_registrations(2521, &db).unwrap();
    assert_eq!(ruby.len(), 1);
    assert_eq!(ruby[0].entry, 194667);
    assert_eq!(ruby[0].action, Action::SpellClick);
    assert!(ruby[0].destination.y > ruby[0].source.y);
    let academy = read_registrations(2526, &db).unwrap();
    assert_eq!(academy.len(), 5);
    assert!(academy.iter().all(|row| row.action == Action::Proximity));
}
