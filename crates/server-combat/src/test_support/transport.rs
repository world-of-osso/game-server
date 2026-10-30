//! Simulator view of transport registrations already used by server interaction systems.
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Action {
    SpellClick,
    Proximity,
    ObjectGossip,
    ObjectScript,
    CollisionDrop,
}

#[derive(Clone, Debug)]
pub struct Registration {
    pub entry: u32,
    pub source_guid: Option<u32>,
    pub source: Vec3,
    pub destination: Vec3,
    pub distance: f32,
    pub radius: f32,
    pub departure: Option<Vec3>,
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
            source_guid: None,
            source: crate::taxi::RUBY_FERRY_SOURCE,
            destination,
            distance,
            radius: crate::npc_interaction::INTERACTION_DISTANCE,
            departure: None,
            action: Action::SpellClick,
        });
    }
    if map == crate::scripted_ai::academy_transport::MAP {
        registrations.extend(crate::scripted_ai::academy_transport::PADS.iter().map(
            |(entry, source, destination, radius)| Registration {
                entry: *entry,
                source_guid: None,
                source: *source,
                destination: *destination,
                distance: source.distance(*destination),
                radius: *radius,
                departure: None,
                action: Action::Proximity,
            },
        ));
    }
    registrations.extend(read_object_registrations(map, db)?);
    Ok(registrations)
}

fn read_object_registrations(
    map: u32,
    db: &rusqlite::Connection,
) -> Result<Vec<Registration>, String> {
    if map != crate::npc_interaction::nalorakk_pyre::MAP
        && map != crate::scripted_ai::voidscar_portal::MAP
    {
        return Ok(Vec::new());
    }
    let objects = crate::game_objects::load_game_object_spawns(db)
        .map_err(|error| format!("map{map} transport objects: {error}"))?;
    let mut registrations = Vec::new();
    for object in objects.0.iter().filter(|object| object.map_id == map) {
        let destination = if map == crate::npc_interaction::nalorakk_pyre::MAP {
            crate::npc_interaction::nalorakk_pyre::destination(object.info.entry).map(|location| {
                let point = location.bevy_position();
                (Vec3::new(point.x, point.y, point.z), Action::ObjectGossip)
            })
        } else if object.script_name == crate::scripted_ai::voidscar_portal::SCRIPT {
            Some((
                crate::scripted_ai::voidscar_portal::LANDING,
                Action::ObjectScript,
            ))
        } else {
            None
        };
        if let Some((destination, action)) = destination {
            let position = crate::game_objects::bevy_position(object.position);
            let source = Vec3::new(position.x, position.y, position.z);
            registrations.push(Registration {
                entry: object.info.entry,
                source_guid: Some(object.guid),
                source,
                destination,
                distance: source.distance(destination),
                radius: crate::game_objects::GAME_OBJECT_INTERACT_DISTANCE,
                departure: None,
                action,
            });
        }
    }
    Ok(registrations)
}

pub fn spawn_objects(
    world: &mut World,
    db: &rusqlite::Connection,
    map: crate::maps::InMap,
    guids: &[u32],
) -> Result<std::collections::BTreeMap<u32, Entity>, String> {
    let objects = crate::game_objects::load_game_object_spawns(db)
        .map_err(|error| format!("load transport objects: {error}"))?;
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, world);
    let actors: std::collections::BTreeMap<_, _> = objects
        .0
        .iter()
        .filter(|object| object.map_id == map.map_id && guids.contains(&object.guid))
        .map(|object| {
            (
                object.guid,
                crate::game_objects::spawn_game_object(&mut commands, object, map),
            )
        })
        .collect();
    drop(commands);
    queue.apply(world);
    for guid in guids {
        if !actors.contains_key(guid) {
            return Err(format!(
                "map{} transport game object{guid} missing",
                map.map_id
            ));
        }
    }
    Ok(actors)
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

pub fn install_drop_system(
    app: &mut App,
    db: &rusqlite::Connection,
    nav_root: &std::path::Path,
) -> Result<(), String> {
    app.insert_resource(crate::navigation::Navigation::new(nav_root));
    if !app.world().contains_resource::<crate::ground::GroundMap>() {
        app.insert_resource(open_ground(db)?);
    }
    app.add_systems(
        FixedUpdate,
        crate::navigation::collision_drop::advance_collision_drops,
    );
    Ok(())
}

fn start_collision_drop(
    world: &mut World,
    owner: Entity,
    edge: &super::graph::TransportEdge,
) -> Result<(), String> {
    let copy = *world
        .get::<crate::maps::InMap>(owner)
        .ok_or("drop owner lacks map")?;
    let departure = edge.departure.ok_or("registered drop lacks departure")?;
    let drop = ::navigation::CollisionDrop {
        after_corner: 0,
        departure: Vec3::from_array(departure),
        landing: Vec3::from_array(edge.landing),
    };
    let mut riders = vec![owner];
    riders.extend(
        world
            .query::<(Entity, &crate::followers::Follower, &crate::maps::InMap)>()
            .iter(world)
            .filter_map(|(unit, follower, map)| {
                (follower.owner == owner && *map == copy).then_some(unit)
            }),
    );
    let pets: Vec<_> = riders
        .iter()
        .flat_map(|unit| crate::hunter_pet::active_owned_pets(world, *unit))
        .collect();
    riders.extend(pets);
    for rider in riders {
        crate::navigation::collision_drop::start_departure(world, rider, copy.map_id, drop.clone());
    }
    Ok(())
}

pub fn in_progress(world: &World, unit: Entity) -> bool {
    world
        .get::<crate::navigation::collision_drop::CollisionDropMovement>(unit)
        .is_some()
        || world.get::<crate::taxi::TaxiFlight>(unit).is_some()
        || world.get::<TaxiLandingPending>(unit).is_some()
        || world
            .get::<crate::scripted_ai::fall_movement::FallMovement>(unit)
            .is_some()
        || world
            .get::<crate::scripted_ai::academy_transport::AcademyPadFlight>(unit)
            .is_some()
        || world
            .get::<crate::scripted_ai::voidscar_portal::PortalFlight>(unit)
            .is_some()
}

pub fn activate(
    world: &mut World,
    owner: Entity,
    npc: Option<Entity>,
    edge: &super::graph::TransportEdge,
) -> Result<(), String> {
    if edge.action == Action::CollisionDrop {
        return start_collision_drop(world, owner, edge);
    }
    let npc = npc.ok_or("registered transport source actor absent")?;
    match edge.action {
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
        Action::ObjectGossip => crate::npc_interaction::nalorakk_pyre::traverse(world, owner, npc)?,
        Action::ObjectScript => {
            crate::scripted_ai::game_object_gossip_hello(world, owner, npc);
            if !in_progress(world, owner) {
                return Err("registered game object script did not start transport".into());
            }
        }
        Action::CollisionDrop => unreachable!(),
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
    let pyres = read_registrations(2825, &db).unwrap();
    assert_eq!(pyres.len(), 2);
    assert!(
        pyres
            .iter()
            .all(|row| row.source_guid.is_some() && row.action == Action::ObjectGossip)
    );
    let portals = read_registrations(2923, &db).unwrap();
    assert_eq!(portals.len(), 1);
    assert_eq!(portals[0].source_guid, Some(81002923));
    assert_eq!(portals[0].action, Action::ObjectScript);
}
