//! Teleports (TrinityCore `Player::TeleportTo`, Player.cpp:1225-1466) and the far
//! teleport's arrival (`WorldSession::HandleMoveWorldportAck`, MovementHandler.cpp:189-384).
//!
//! A teleport on the player's own map moves it at once. One to another map checks the
//! destination admits the player (`Map::PlayerCannotEnter`), stops its combat and cast,
//! takes it out of its map (no `InMap`, a [`MapTransfer`] instead) and sends `NewWorld`.
//! The player joins the destination copy (`MapManager::CreateMap`) when its client
//! answers `WorldPortAck`, after the copy's own `CannotEnter` check; a refused arrival
//! sends it back where it left from (TrinityCore uses the homebind, which the server
//! has no data for).

use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{MovementControl, Player, Position, Rotation};
use shared::protocol::{
    NewWorld, TransferAbortReason, TransferAborted, TransferChannel, WorldPortAck,
};

use super::instances::InstanceEntrant;
use super::{InMap, InstanceManager, MapStore, WorldLocation};
use crate::auth::{InWorld, SentTiles};
use crate::group::GroupRegistry;
use crate::interest::InterestDirtyQueue;
use crate::networking::PlayerLink;
use crate::persistence::ClientId;

/// TrinityCore `TeleportToOptions` the server uses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TeleportOptions {
    /// `TELE_REVIVE_AT_TELEPORT`: a dead player is resurrected on arrival.
    pub revive: bool,
}

/// The player left its map for `destination` and waits for its client's `WorldPortAck`
/// (`TeleportState::WaitingForWorldPortAck`; `m_teleport_dest` / `m_teleport_options`).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct MapTransfer {
    pub map_id: u32,
    pub destination: WorldLocation,
    pub options: TeleportOptions,
    pub origin: WorldLocation,
}

/// Teleport `player` to `destination`; false when it is refused or already in a
/// transfer.
pub(crate) fn teleport_to(
    world: &mut World,
    player: Entity,
    destination: WorldLocation,
    options: TeleportOptions,
) -> bool {
    let Some(&current) = world.get::<InMap>(player) else {
        return false;
    };
    if current.map_id == destination.map_id {
        near_teleport(world, player, &destination, options);
        return true;
    }
    let entrant = entrant_of(world, player);
    let refused = world.resource::<InstanceManager>().player_cannot_enter(
        world.resource::<MapStore>(),
        destination.map_id,
        entrant,
    );
    if let Some(reason) = refused {
        info!(
            "Teleport of {player} to map {} refused: {reason:?}",
            destination.map_id
        );
        send_to_connection(
            world,
            player,
            TransferAborted {
                map_id: destination.map_id,
                reason,
            },
        );
        return false;
    }
    let origin = current_location(world, player, current.map_id);
    begin_transfer(world, player, destination, options, origin);
    true
}

fn near_teleport(
    world: &mut World,
    player: Entity,
    destination: &WorldLocation,
    options: TeleportOptions,
) {
    if options.revive && !crate::spell_cast::targets::is_alive(world, player) {
        crate::death::resurrect_player(world, player);
    }
    place_player(world, player, destination);
}

/// Move `player` to `destination` on its map: the client adopts the new position on the
/// `MovementControl` epoch change.
fn place_player(world: &mut World, player: Entity, destination: &WorldLocation) {
    let position = destination.bevy_position();
    world.entity_mut(player).insert((
        position,
        facing_rotation(destination.orientation),
        crate::networking::FallTracker::grounded(position.y),
    ));
    let Some(mut control) = world.get::<MovementControl>(player).copied() else {
        return;
    };
    crate::teleport::teleported(
        world.get_resource::<InterestDirtyQueue>(),
        player,
        &mut control,
    );
    world.entity_mut(player).insert(control);
}

/// Player facing as the replicated `Rotation` (same convention as creature spawns).
fn facing_rotation(orientation: f32) -> Rotation {
    Rotation {
        x: 0.0,
        y: orientation + std::f32::consts::FRAC_PI_2,
        z: 0.0,
    }
}

fn current_location(world: &World, player: Entity, map_id: u32) -> WorldLocation {
    let position = *world
        .get::<Position>(player)
        .expect("player has a Position");
    let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
    let orientation = world
        .get::<Rotation>(player)
        .expect("player has a Rotation")
        .y
        - std::f32::consts::FRAC_PI_2;
    WorldLocation {
        map_id,
        position: Vec3::new(x, y, z),
        orientation,
    }
}

/// The far branch of `Player::TeleportTo` once the destination admits the player.
fn begin_transfer(
    world: &mut World,
    player: Entity,
    destination: WorldLocation,
    options: TeleportOptions,
    origin: WorldLocation,
) {
    // SetSelection(Empty); CombatStop(); InterruptNonMeleeSpells; the taxi flight ends.
    if let Some(mut target) = world.get_mut::<crate::networking::CombatTarget>(player) {
        target.0 = None;
    }
    crate::combat::combat_stop(world, player);
    crate::spell_cast::cancel_cast(world, player);
    crate::taxi::finish_flight(world.entity_mut(player));
    // oldmap->RemovePlayerFromMap; the saved position becomes m_teleport_dest.
    world.entity_mut(player).remove::<InMap>().insert((
        destination.bevy_position(),
        MapTransfer {
            map_id: destination.map_id,
            destination,
            options,
            origin,
        },
    ));
    crate::interest::mark_interest_position_changed(
        world.get_resource::<InterestDirtyQueue>(),
        player,
    );
    let Some(map_directory) = world
        .resource::<MapStore>()
        .map(destination.map_id)
        .map(|entry| entry.directory.clone())
    else {
        error!(
            "Transfer to map {} without a Map.db2 entry",
            destination.map_id
        );
        return;
    };
    let position = destination.bevy_position();
    send_to_connection(
        world,
        player,
        NewWorld {
            map_id: destination.map_id,
            map_directory,
            position: [position.x, position.y, position.z],
            facing: destination.orientation,
        },
    );
    info!(
        "{player} leaves map {} for map {} at {:?}",
        origin.map_id, destination.map_id, destination.position
    );
}

/// `CMSG_WORLD_PORT_RESPONSE` from a player in transfer.
pub(crate) fn receive_world_port_acks(world: &mut World) {
    let mut receivers =
        world.query_filtered::<(&PlayerLink, &mut MessageReceiver<WorldPortAck>), With<InWorld>>();
    let players: Vec<Entity> = receivers
        .iter_mut(world)
        .filter_map(|(link, mut receiver)| (receiver.receive().count() > 0).then_some(link.0))
        .collect();
    for player in players {
        complete_transfer(world, player);
    }
}

/// `HandleMoveWorldportAck`: add the player to the destination copy.
fn complete_transfer(world: &mut World, player: Entity) {
    let Some(transfer) = world.get::<MapTransfer>(player).copied() else {
        warn!("{player} sent WorldPortAck without a transfer");
        return;
    };
    world.entity_mut(player).remove::<MapTransfer>();
    let entrant = entrant_of(world, player);
    let now = world.resource::<Time>().elapsed_secs_f64();
    let resolved = world.resource_scope(|world, mut instances: Mut<InstanceManager>| {
        let resolved =
            instances.create_map(world.resource::<MapStore>(), transfer.map_id, entrant, now)?;
        match instances.cannot_enter(resolved.map, entrant) {
            Some(reason) => Err(reason),
            None => Ok(resolved),
        }
    });
    let resolved = match resolved {
        Ok(resolved) => resolved,
        Err(reason) => {
            refuse_arrival(world, player, &transfer, reason);
            return;
        }
    };
    if resolved.created {
        spawn_map_content(world, resolved.map);
    }
    world.entity_mut(player).insert(resolved.map);
    place_player(world, player, &transfer.destination);
    reset_sent_tiles(world, player);
    let corpse_here = world
        .get::<crate::death::PlayerDeathState>(player)
        .and_then(|death| death.corpse.as_ref())
        .is_some_and(|corpse| u32::from(corpse.map_id) == transfer.map_id);
    let dungeon = world
        .resource::<MapStore>()
        .map(transfer.map_id)
        .is_some_and(|entry| entry.is_dungeon());
    // TELE_REVIVE_AT_TELEPORT, and a ghost entering the dungeon its corpse is in.
    if (transfer.options.revive || (dungeon && corpse_here))
        && !crate::spell_cast::targets::is_alive(world, player)
    {
        crate::death::resurrect_player(world, player);
    }
    info!("{player} entered map {:?}", resolved.map);
}

/// The destination refused the arriving player (it filled up meanwhile).
fn refuse_arrival(
    world: &mut World,
    player: Entity,
    transfer: &MapTransfer,
    reason: TransferAbortReason,
) {
    warn!(
        "{player} could not enter map {} ({reason:?}); returning to map {}",
        transfer.map_id, transfer.origin.map_id
    );
    send_to_connection(
        world,
        player,
        TransferAborted {
            map_id: transfer.map_id,
            reason,
        },
    );
    begin_transfer(
        world,
        player,
        transfer.origin,
        TeleportOptions::default(),
        transfer.origin,
    );
}

/// Spawn a newly created copy's creatures and game objects.
pub(crate) fn spawn_map_content(world: &mut World, map: InMap) {
    let mut queue = CommandQueue::default();
    {
        let mut commands = Commands::new(&mut queue, world);
        if let Some(world_data) = world.get_resource::<crate::world_data::WorldData>() {
            crate::zones::spawn_map_creatures(&mut commands, world_data, map, true);
        }
        if let Some(objects) = world.get_resource::<crate::game_objects::GameObjectSpawns>() {
            crate::game_objects::spawn_map_game_objects(&mut commands, objects, map);
        }
    }
    queue.apply(world);
}

/// The destination map streams its own tiles: forget the old map's.
fn reset_sent_tiles(world: &mut World, player: Entity) {
    let Some(connection) = connection_of(world, player) else {
        return;
    };
    if let Some(mut sent) = world.get_mut::<SentTiles>(connection) {
        sent.tiles.clear();
        sent.last_tile = None;
    }
}

pub(crate) fn entrant_of(world: &World, player: Entity) -> InstanceEntrant {
    let character_id = world
        .get::<ClientId>(player)
        .expect("player has a ClientId")
        .0;
    let group = world
        .get_resource::<GroupRegistry>()
        .and_then(|groups| groups.group_id_of(character_id));
    InstanceEntrant {
        character_id,
        group,
    }
}

fn connection_of(world: &mut World, player: Entity) -> Option<Entity> {
    let mut connections = world.query_filtered::<(Entity, &PlayerLink), With<InWorld>>();
    connections
        .iter(world)
        .find_map(|(connection, link)| (link.0 == player).then_some(connection))
}

fn send_to_connection<M: lightyear::prelude::Message + Clone>(
    world: &mut World,
    player: Entity,
    message: M,
) {
    if world.get::<Player>(player).is_none() {
        return;
    }
    let Some(connection) = connection_of(world, player) else {
        return;
    };
    if let Some(mut sender) = world.get_mut::<MessageSender<M>>(connection) {
        sender.send::<TransferChannel>(message);
    }
}
