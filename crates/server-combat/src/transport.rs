//! Per-copy source airships and ship-relative entry/carry. No ground-start substitution.
use crate::{
    maps::{InMap, WorldLocation},
    transport_catalog::TransportCatalog,
};
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::{
    components::{Position, Rotation, TransportPassenger, TransportState},
    protocol::{GameObjectCollision, GameObjectVisible},
    transport_route::{TransportPose, TransportRoute},
};

#[derive(Component)]
pub(crate) struct ShipRoute(TransportRoute);

pub(crate) type ShipQuery<'w, 's> =
    Query<'w, 's, (&'static TransportState, &'static ShipRoute, &'static InMap)>;

pub(crate) fn movement_frame(
    ships: &ShipQuery<'_, '_>,
    passenger: TransportPassenger,
) -> Option<(u64, InMap, TransportPose)> {
    let (state, route, map) = ships.get(Entity::from_bits(passenger.transport)).ok()?;
    Some((passenger.transport, *map, route.0.pose_at(state.elapsed_ms)))
}

#[derive(Component)]
struct PendingTransportEntry(TransportPassenger);

pub(crate) fn validate_entry(world: &World, spawn_id: u32, map_id: u32) -> Result<(), String> {
    let source = world
        .get_resource::<TransportCatalog>()
        .and_then(|catalog| catalog.0.get(&spawn_id))
        .ok_or_else(|| format!("transport {spawn_id} source catalog unavailable"))?;
    if source.map_id != map_id {
        return Err(format!(
            "transport {spawn_id} path is on map {}, not {map_id}",
            source.map_id
        ));
    }
    Ok(())
}

pub(crate) fn spawn_ships(world: &mut World, map: InMap) {
    let sources: Vec<_> = world
        .resource::<TransportCatalog>()
        .0
        .values()
        .filter(|source| source.map_id == map.map_id)
        .cloned()
        .collect();
    for source in sources {
        let pose = source.route.pose_at(0);
        let (position, rotation) = engine_pose(pose);
        world.spawn((
            position,
            rotation,
            map,
            source.info,
            GameObjectCollision(true),
            GameObjectVisible(true),
            TransportState {
                spawn_id: source.spawn_id,
                path_id: source.path_id,
                interior_map: source.interior_map,
                knots: source.knots,
                speed: source.speed,
                elapsed_ms: 0,
            },
            ShipRoute(source.route),
            Replicate::to_clients(NetworkTarget::All),
            crate::interest::NetworkVisibility::default(),
        ));
    }
}

fn engine_pose(pose: TransportPose) -> (Position, Rotation) {
    let [x, y, z] = pose.position;
    (
        Position { x, y: z, z: -y },
        Rotation {
            x: 0.0,
            y: pose.orientation + std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
    )
}

fn source_pose(world: &World, entity: Entity) -> TransportPose {
    let route = &world
        .get::<ShipRoute>(entity)
        .expect("authored ship route")
        .0;
    let state = world
        .get::<TransportState>(entity)
        .expect("authored ship state");
    route.pose_at(state.elapsed_ms)
}

pub(crate) fn prepare_entry(
    world: &mut World,
    player: Entity,
    map: InMap,
    local: WorldLocation,
    spawn_id: u32,
) -> Result<WorldLocation, String> {
    let mut ships = world.query::<(Entity, &TransportState, &InMap)>();
    let entity = ships
        .iter(world)
        .find(|(_, state, copy)| state.spawn_id == spawn_id && **copy == map)
        .map(|(entity, _, _)| entity)
        .ok_or_else(|| format!("transport {spawn_id} missing from {map:?}"))?;
    let local_pose = TransportPose {
        position: local.position.to_array(),
        orientation: local.orientation,
    };
    let destination = source_pose(world, entity).local_to_world(local_pose);
    world
        .entity_mut(player)
        .insert(PendingTransportEntry(TransportPassenger {
            transport: entity.to_bits(),
            position: local_pose.position,
            orientation: local_pose.orientation,
        }));
    Ok(WorldLocation {
        map_id: map.map_id,
        position: Vec3::from_array(destination.position),
        orientation: destination.orientation,
    })
}

pub(crate) fn cancel_entry(world: &mut World, player: Entity) {
    world.entity_mut(player).remove::<PendingTransportEntry>();
}

pub(crate) fn finish_entry(world: &mut World, player: Entity, map: InMap) {
    let Some(PendingTransportEntry(passenger)) =
        world.entity_mut(player).take::<PendingTransportEntry>()
    else {
        return;
    };
    let ship = Entity::from_bits(passenger.transport);
    assert_eq!(
        world.get::<InMap>(ship),
        Some(&map),
        "accepted transport still belongs to arrival copy"
    );
    let pose = source_pose(world, ship).local_to_world(TransportPose {
        position: passenger.position,
        orientation: passenger.orientation,
    });
    let (position, rotation) = engine_pose(pose);
    world.entity_mut(player).insert((
        position,
        rotation,
        passenger,
        crate::networking::FallTracker::grounded(position.y),
    ));
    mark_replication(world, player);
}

pub(crate) fn clear_passenger(world: &mut World, player: Entity) {
    world.entity_mut(player).remove::<TransportPassenger>();
}

pub(crate) fn advance_ships(world: &mut World) {
    let delta_ms = world.resource::<Time<Fixed>>().delta().as_millis() as u64;
    let mut ships = world.query::<(Entity, &ShipRoute, &TransportState)>();
    let updates: Vec<_> = ships
        .iter(world)
        .map(|(entity, route, state)| {
            let elapsed = state.elapsed_ms + delta_ms;
            (entity, elapsed, route.0.pose_at(elapsed))
        })
        .collect();
    for (entity, elapsed, pose) in updates {
        world.get_mut::<TransportState>(entity).unwrap().elapsed_ms = elapsed;
        let (position, rotation) = engine_pose(pose);
        world.entity_mut(entity).insert((position, rotation));
        mark_replication(world, entity);
    }
    carry_passengers(world);
}

fn carry_passengers(world: &mut World) {
    let mut passengers = world.query::<(Entity, &TransportPassenger, &InMap)>();
    let updates: Vec<_> = passengers
        .iter(world)
        .map(|(player, passenger, map)| (player, *passenger, *map))
        .collect();
    for (player, passenger, map) in updates {
        let ship = Entity::from_bits(passenger.transport);
        assert_eq!(
            world.get::<InMap>(ship),
            Some(&map),
            "attached ship belongs to passenger copy"
        );
        let pose = source_pose(world, ship).local_to_world(TransportPose {
            position: passenger.position,
            orientation: passenger.orientation,
        });
        let (position, rotation) = engine_pose(pose);
        world.entity_mut(player).insert((position, rotation));
        mark_replication(world, player);
    }
}

pub(crate) fn store_passenger_offsets(world: &mut World) {
    let mut players = world.query::<(Entity, &TransportPassenger, &Position, &Rotation)>();
    let updates: Vec<_> = players
        .iter(world)
        .map(|(entity, passenger, position, rotation)| {
            let ship = Entity::from_bits(passenger.transport);
            let world_pose = TransportPose {
                position: [position.x, -position.z, position.y],
                orientation: rotation.y - std::f32::consts::FRAC_PI_2,
            };
            let local = source_pose(world, ship).world_to_local(world_pose);
            (
                entity,
                TransportPassenger {
                    transport: passenger.transport,
                    position: local.position,
                    orientation: local.orientation,
                },
            )
        })
        .collect();
    for (entity, local) in updates {
        if world.get::<TransportPassenger>(entity) != Some(&local) {
            world.entity_mut(entity).insert(local);
            mark_replication(world, entity);
        }
    }
}

pub(crate) fn disembark(world: &mut World, player: Entity) {
    if world
        .entity_mut(player)
        .take::<TransportPassenger>()
        .is_none()
    {
        return;
    }
    let bg = world
        .get::<crate::pvp::PlayerPvpState>(player)
        .and_then(|state| state.battleground.as_ref())
        .expect("Seething passenger match");
    if bg.phase != shared::battleground::BgPhase::InProgress {
        let team = bg.team;
        let start = shared::battleground::SEETHING_STARTS[if team == shared::battleground::BgTeam::A
        {
            0
        } else {
            1
        }];
        let map = *world.get::<InMap>(player).expect("disembarking player map");
        let store = world.resource::<crate::maps::MapStore>();
        let local = store.safe_locs[&start];
        let spawn_id = store.safe_loc_transports[&start];
        let destination =
            prepare_entry(world, player, map, local, spawn_id).expect("validated ship start");
        assert!(crate::maps::teleport_to(
            world,
            player,
            destination,
            crate::maps::TeleportOptions::default()
        ));
        finish_entry(world, player, map);
        return;
    }
    // TC transport_seething_shore::OnRemovePassenger, both triggered on the player.
    crate::spell_cast::cast_triggered(world, player, 250917, Some(player), 0);
    crate::spell_cast::cast_triggered(world, player, 262359, Some(player), 0);
}

fn mark_replication(world: &World, entity: Entity) {
    if let Some(queue) = world.get_resource::<crate::interest::InterestDirtyQueue>() {
        queue.mark(entity);
    }
    if let Some(queue) = world.get_resource::<ReplicationDirtyQueue>() {
        queue.mark(entity);
    }
}
