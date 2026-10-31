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

#[derive(Component)]
pub(crate) struct AwaitingBoardAck;

#[derive(Component)]
struct Disembarking;

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
        for event in source.route.initial_events() {
            warn!(
                "Transport {} in {map:?} has unhandled route event {} at {}ms",
                source.spawn_id, event.id, event.elapsed_ms
            );
        }
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
                acceleration: source.acceleration,
                nodes: source.nodes,
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
    world
        .entity_mut(player)
        .remove::<(TransportPassenger, AwaitingBoardAck, Disembarking)>();
}

pub(crate) fn advance_ships(world: &mut World) {
    let delta_ms = world.resource::<Time<Fixed>>().delta().as_millis() as u64;
    let mut ships = world.query::<(Entity, &ShipRoute, &TransportState)>();
    let updates: Vec<_> = ships
        .iter(world)
        .map(|(entity, route, state)| {
            let elapsed = state.elapsed_ms + delta_ms;
            (
                entity,
                state.spawn_id,
                elapsed,
                route.0.pose_at(elapsed),
                route.0.events_between(state.elapsed_ms, elapsed),
            )
        })
        .collect();
    for (entity, spawn_id, elapsed, pose, events) in updates {
        for event in events {
            warn!(
                "Transport {spawn_id} has unhandled route event {} at {}ms",
                event.id, event.elapsed_ms
            );
        }
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

pub(crate) fn sync_deck_passengers(world: &mut World) {
    let mut actors = world.query_filtered::<(
        Entity,
        &InMap,
        &Position,
        &Rotation,
        Option<&TransportPassenger>,
        Option<&shared::components::Player>,
        Option<&Disembarking>,
    ), Or<(
        With<shared::components::Player>,
        With<crate::followers::Follower>,
    )>>();
    let updates: Vec<_> = actors
        .iter(world)
        .map(
            |(entity, map, position, rotation, passenger, player, disembarking)| {
                let feet = Vec3::new(position.x, position.y, position.z);
                let ship = world
                    .get_resource::<crate::ground::GroundMap>()
                    .and_then(|ground| ground.transport_at(*map, feet));
                (
                    entity,
                    *map,
                    *position,
                    *rotation,
                    passenger.copied(),
                    player.is_some(),
                    disembarking.is_some(),
                    ship,
                )
            },
        )
        .collect();
    for (actor, map, position, rotation, passenger, player, disembarking, ship) in updates {
        if disembarking {
            if ship.is_none() {
                world.entity_mut(actor).remove::<Disembarking>();
            }
            continue;
        }
        if passenger.is_none() {
            let Some(ship) = ship else { continue };
            if world.get::<ShipRoute>(ship).is_none() || world.get::<InMap>(ship) != Some(&map) {
                continue;
            }
            let local = source_pose(world, ship).world_to_local(TransportPose {
                position: [position.x, -position.z, position.y],
                orientation: rotation.y - std::f32::consts::FRAC_PI_2,
            });
            world.entity_mut(actor).insert(TransportPassenger {
                transport: ship.to_bits(),
                position: local.position,
                orientation: local.orientation,
            });
            if player {
                world.entity_mut(actor).insert(AwaitingBoardAck);
            }
            mark_replication(world, actor);
        } else if !player && ship.map(Entity::to_bits) != passenger.map(|frame| frame.transport) {
            world.entity_mut(actor).remove::<TransportPassenger>();
            mark_replication(world, actor);
        }
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
    world.entity_mut(player).remove::<AwaitingBoardAck>();
    mark_replication(world, player);
    let Some(bg) = world
        .get::<crate::pvp::PlayerPvpState>(player)
        .and_then(|state| state.battleground.as_ref())
    else {
        world.entity_mut(player).insert(Disembarking);
        return;
    };
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
    world.entity_mut(player).insert(Disembarking);
    // TC transport_seething_shore::OnRemovePassenger, both triggered on the player.
    crate::spell_cast::cast_triggered(world, player, 250917, Some(player), 0);
    crate::spell_cast::cast_triggered(world, player, 262359, Some(player), 0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::world::CommandQueue;
    use std::time::Duration;

    #[test]
    fn deck_contact_boards_party_and_carries_only_the_same_map_copy() {
        use shared::components::{CharacterAppearance, Player};
        use shared::group::GroupRole;
        use std::{collections::HashMap, path::Path, sync::Arc};

        let mut world = World::new();
        world.insert_resource(crate::transport_catalog::fixture_catalog());
        world.insert_resource(Time::<Fixed>::default());
        let map = InMap {
            map_id: 2662,
            instance_id: 1,
        };
        let other = InMap {
            map_id: 2662,
            instance_id: 2,
        };
        spawn_ships(&mut world, map);
        let ship = world
            .query::<(Entity, &TransportState)>()
            .iter(&world)
            .find(|(_, state)| state.spawn_id == 409273)
            .unwrap()
            .0;
        let ship_position = *world.get::<Position>(ship).unwrap();
        let at = Vec3::new(ship_position.x, ship_position.y, ship_position.z);
        let hull = Arc::new(shared::los::LosTile::build(
            0,
            0,
            vec![
                at + Vec3::new(-4.0, 0.0, -4.0),
                at + Vec3::new(4.0, 0.0, -4.0),
                at + Vec3::new(0.0, 0.0, 4.0),
            ],
            vec![[0, 1, 2]],
        ));
        let mut ground = crate::ground::GroundMap::new(Path::new("."), HashMap::new());
        ground.set_transport_hulls(HashMap::from([(map, vec![(ship, hull)])]));
        world.insert_resource(ground);
        let player = world
            .spawn((
                Player {
                    name: "Rider".into(),
                    race: 1,
                    class: 1,
                    appearance: CharacterAppearance::default(),
                },
                map,
                ship_position,
                Rotation {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            ))
            .id();
        let follower = world
            .spawn((
                crate::followers::Follower {
                    owner: player,
                    profile: crate::followers::Profile {
                        entry: 1,
                        name: "Follower".into(),
                        class_id: 1,
                        spec_id: 1,
                        role: GroupRole::Dps,
                        attack_spell: 0,
                        cooldown_spell: 0,
                        heal_spell: 0,
                        taunt_spell: 0,
                        form_spell: 0,
                        melee: true,
                    },
                },
                map,
                ship_position,
                Rotation {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            ))
            .id();
        let other_player = world
            .spawn((
                Player {
                    name: "Other".into(),
                    race: 1,
                    class: 1,
                    appearance: CharacterAppearance::default(),
                },
                other,
                ship_position,
                Rotation {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            ))
            .id();
        sync_deck_passengers(&mut world);
        assert_eq!(
            world.get::<TransportPassenger>(player).unwrap().transport,
            ship.to_bits()
        );
        assert_eq!(
            world.get::<TransportPassenger>(follower).unwrap().transport,
            ship.to_bits()
        );
        assert!(world.get::<AwaitingBoardAck>(player).is_some());
        assert!(world.get::<TransportPassenger>(other_player).is_none());
        world
            .resource_mut::<Time<Fixed>>()
            .advance_by(Duration::from_secs(6));
        advance_ships(&mut world);
        assert_ne!(*world.get::<Position>(player).unwrap(), ship_position);
        assert_eq!(
            *world.get::<Position>(player).unwrap(),
            *world.get::<Position>(follower).unwrap()
        );
        disembark(&mut world, player);
        assert!(world.get::<TransportPassenger>(player).is_none());
        assert!(world.get::<Disembarking>(player).is_some());
        let destination = WorldLocation {
            map_id: 2662,
            position: Vec3::new(1500.0, -200.0, 30.0),
            orientation: 0.0,
        };
        crate::maps::transfer::teleport_creature_with_companions(
            &mut world,
            follower,
            &destination,
        );
        assert!(world.get::<TransportPassenger>(follower).is_none());
        let landed = *world.get::<Position>(follower).unwrap();
        advance_ships(&mut world);
        assert_eq!(*world.get::<Position>(follower).unwrap(), landed);
    }

    #[test]
    fn dungeon_copies_spawn_and_advance_independent_ships() {
        let mut world = World::new();
        world.insert_resource(crate::maps::MapStore::default());
        world.insert_resource(crate::transport_catalog::fixture_catalog());
        world.insert_resource(Time::<Fixed>::default());
        let copies = [
            InMap {
                map_id: 2662,
                instance_id: 1,
            },
            InMap {
                map_id: 2662,
                instance_id: 2,
            },
        ];
        for map in copies {
            let mut queue = CommandQueue::default();
            let mut commands = Commands::new(&mut queue, &world);
            crate::maps::transfer::queue_map_content(
                &mut commands,
                None,
                None,
                crate::zones::MapContent {
                    map,
                    difficulty_id: 1,
                    dead_bosses: vec![],
                },
                None,
            );
            queue.apply(&mut world);
        }
        let mut ships = world.query::<(Entity, &InMap, &TransportState, &Position)>();
        let initial: Vec<_> = ships
            .iter(&world)
            .map(|(entity, map, state, position)| (entity, *map, state.spawn_id, *position))
            .collect();
        assert_eq!(initial.len(), 2);
        assert!(copies.iter().all(|copy| {
            initial
                .iter()
                .any(|(_, map, id, _)| map == copy && *id == 409273)
        }));
        world
            .resource_mut::<Time<Fixed>>()
            .advance_by(Duration::from_secs(6));
        advance_ships(&mut world);
        for (entity, _, _, initial_position) in initial {
            let state = world.get::<TransportState>(entity).unwrap();
            assert_eq!(state.elapsed_ms, 6000);
            assert_ne!(*world.get::<Position>(entity).unwrap(), initial_position);
            assert_eq!(state.path_id, 9456);
        }
    }
}

fn mark_replication(world: &World, entity: Entity) {
    if let Some(queue) = world.get_resource::<crate::interest::InterestDirtyQueue>() {
        queue.mark(entity);
    }
    if let Some(queue) = world.get_resource::<ReplicationDirtyQueue>() {
        queue.mark(entity);
    }
}
