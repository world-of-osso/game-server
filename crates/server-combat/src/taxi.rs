//! Flight masters (docs/specs/flight-master.md): known flight points, the flight
//! map, and server-driven flights.
//!
//! Opening a flight master's role (`npc_interaction`) learns its node the first
//! time (AzerothCore `WorldSession::SendLearnNewTaxiNode`, Handlers/TaxiHandler.cpp:127)
//! and sends `TaxiMap`; unlike AzerothCore, which shows the menu only on the next
//! talk, Retail opens the map at once. `ActivateTaxi` follows
//! `Player::ActivateTaxiPathTo` (Player.cpp:10189-10350): refused while in combat
//! or already flying, the destination must be known and routed over known nodes,
//! and the whole route is paid at take-off (Retail; AzerothCore pays per hop). A
//! mounted player is dismounted. The player then flies the route's
//! `TaxiPathNode` points at `PLAYER_FLIGHT_SPEED` (32 yd/s,
//! WaypointMovementGenerator.cpp:430) on the node's taxi mount, with
//! `MovementControl::controlled` set so input and gravity leave it alone, and
//! lands on the last point (`teleport::teleported`).

use std::collections::BTreeSet;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{
    CombatStatus, Mounted, MovementControl, MovementSpeed, Npc, Player, Position, Rotation,
};
use shared::protocol::{
    ActivateTaxi, InteractionChannel, InteractionClosed, NpcRole, TaxiChannel, TaxiError,
    TaxiFailed, TaxiMap, TaxiNodeDiscovered, TaxiNodeInfo, TaxiNodeState,
};

use crate::auth::InWorld;
use crate::faction::faction_for_race;
use crate::gold::GoldUpdateQueue;
use crate::interest::InterestDirtyQueue;
use crate::networking::PlayerLink;
use crate::npc_interaction::ActiveInteraction;
use crate::persistence::taxi_store::{load_known_taxi_nodes, save_known_taxi_nodes};
use crate::persistence::{self, GameDatabase};
use crate::teleport::teleported;
use crate::terrain_height::bevy_to_wow;

mod network;

pub(crate) use network::{TaxiNetwork, TaxiRoute, Team, load_taxi_network};

/// AzerothCore `PLAYER_FLIGHT_SPEED`.
pub(crate) const FLIGHT_SPEED: f32 = 32.0;
/// `networking_movement::DEFAULT_RUN_SPEED`, restored on landing.
const RUN_SPEED: f32 = 7.0;

// DARKFLAME_CART_CONTINUATION_LOG_CORRELATED: WCL WBYDm1N6JTZzCFnd fight3
// actor2 at t=6741925 (542.82,-251.89) and t=6746889 (543.72,-69.77)
// moved ~36.7 yd/s. The landing is that second XY projected to the native
// navmesh (87.4). Heights between TaxiPath9458's endpoint and the landing are
// unsourced; the taxi mover interpolates them linearly.
pub(crate) const DARKFLAME_CART_PATH: u32 = 9458;
pub(crate) const DARKFLAME_CART_MAP: u32 = 2651;
pub(crate) const DARKFLAME_CART_ENTRY: u32 = 209439;
const DARKFLAME_CART_SPEED: f32 = 36.7;
pub(crate) const DARKFLAME_CART_LANDING: Vec3 = Vec3::new(543.72, 87.4, 69.77);

#[derive(Component)]
pub(crate) struct ScriptedTaxiSpeed(f32);

/// A flight in progress: Bevy-space points still to fly through.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct TaxiFlight {
    pub points: Vec<Vec3>,
    pub next: usize,
}

pub struct TaxiPlugin;

impl Plugin for TaxiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TaxiNetwork>()
            .add_systems(Update, (open_flight_maps, process_activate_taxi).chain())
            .add_systems(FixedUpdate, fly_taxis);
    }
}

pub(crate) fn team_for_race(race: u8) -> Option<Team> {
    match faction_for_race(race) {
        1 => Some(Team::Alliance),
        2 => Some(Team::Horde),
        _ => None,
    }
}

fn wow(position: &Position) -> Vec3 {
    let (x, y, z) = bevy_to_wow(position.x, position.y, position.z);
    Vec3::new(x, y, z)
}

/// The flight map of `current`: every node of the continent the team sees.
pub(crate) fn taxi_map(
    network: &TaxiNetwork,
    npc: u64,
    current: u32,
    known: &BTreeSet<u32>,
    team: Team,
) -> TaxiMap {
    let continent = network.nodes[&current].continent;
    let mut nodes: Vec<TaxiNodeInfo> = network
        .map_nodes(continent, team)
        .map(|(id, node)| {
            let route = (id != current)
                .then(|| network.route(current, id, known, team))
                .flatten();
            let state = match (&route, id == current) {
                (_, true) => TaxiNodeState::Current,
                (Some(_), false) => TaxiNodeState::Reachable,
                (None, false) => TaxiNodeState::Unreachable,
            };
            let (cost, route) = route.map_or((0, Vec::new()), |route| {
                (u64::from(route.cost), route.nodes)
            });
            TaxiNodeInfo {
                node: id,
                name: node.name.clone(),
                world_x: node.position.x,
                world_y: node.position.y,
                state,
                cost,
                route,
            }
        })
        .collect();
    nodes.sort_by_key(|node| node.node);
    TaxiMap {
        npc,
        continent,
        nodes,
    }
}

#[derive(SystemParam)]
struct TaxiSenders<'w, 's> {
    maps: Query<'w, 's, &'static mut MessageSender<TaxiMap>>,
    discovered: Query<'w, 's, &'static mut MessageSender<TaxiNodeDiscovered>>,
    failed: Query<'w, 's, &'static mut MessageSender<TaxiFailed>>,
    closed: Query<'w, 's, &'static mut MessageSender<InteractionClosed>>,
}

impl TaxiSenders<'_, '_> {
    fn map(&mut self, connection: Entity, map: TaxiMap) {
        if let Ok(mut sender) = self.maps.get_mut(connection) {
            sender.send::<TaxiChannel>(map);
        }
    }

    fn discovered(&mut self, connection: Entity, node: u32) {
        if let Ok(mut sender) = self.discovered.get_mut(connection) {
            sender.send::<TaxiChannel>(TaxiNodeDiscovered { node });
        }
    }

    fn failed(&mut self, connection: Entity, npc: u64, error: TaxiError) {
        if let Ok(mut sender) = self.failed.get_mut(connection) {
            sender.send::<TaxiChannel>(TaxiFailed { npc, error });
        }
    }

    fn closed(&mut self, connection: Entity, npc: Entity) {
        if let Ok(mut sender) = self.closed.get_mut(connection) {
            sender.send::<InteractionChannel>(InteractionClosed { npc: npc.to_bits() });
        }
    }
}

fn open_flight_master(interaction: Option<&ActiveInteraction>) -> Option<Entity> {
    interaction
        .filter(|interaction| interaction.role == Some(NpcRole::FlightMaster))
        .map(|interaction| interaction.npc)
}

type FlightMasterPositions<'w, 's> =
    Query<'w, 's, (&'static Position, &'static crate::maps::InMap), With<Npc>>;

/// Opening a flight master learns its node and shows the map.
fn open_flight_maps(
    opened: Query<(Entity, &InWorld, &PlayerLink, &ActiveInteraction), Changed<ActiveInteraction>>,
    npcs: FlightMasterPositions,
    players: Query<&Player>,
    network: Res<TaxiNetwork>,
    db: Option<Res<GameDatabase>>,
    mut senders: TaxiSenders,
    mut commands: Commands,
) {
    let Some(db) = db else { return };
    for (connection, in_world, link, interaction) in &opened {
        let Some(npc) = open_flight_master(Some(interaction)) else {
            continue;
        };
        let (Ok((npc_position, npc_map)), Some(team)) = (
            npcs.get(npc),
            players.get(link.0).ok().and_then(|p| team_for_race(p.race)),
        ) else {
            continue;
        };
        let Some(current) = network.nearest_node(npc_map.map_id, wow(npc_position), team) else {
            continue;
        };
        let mut known = load_known_taxi_nodes(&db.db, in_world.character_id);
        if known.insert(current) {
            save_known_taxi_nodes(&db.db, in_world.character_id, &known);
            senders.discovered(connection, current);
            // TC TaxiHandler.cpp:146 credits only a newly learned node.
            let character = in_world.character_id;
            commands.queue(move |world: &mut World| {
                crate::achievement::record_character_event(
                    world,
                    character,
                    crate::achievement::CriteriaEvent::LearnTaxiNode(current),
                );
            });
        }
        let map = taxi_map(&network, npc.to_bits(), current, &known, team);
        senders.map(connection, map);
    }
}

type TaxiPlayers<'w, 's> = Query<
    'w,
    's,
    (
        &'static Player,
        Option<&'static CombatStatus>,
        Has<TaxiFlight>,
    ),
>;

/// Everything `ActivateTaxi` reads and writes.
#[derive(SystemParam)]
struct Takeoff<'w, 's> {
    network: Res<'w, TaxiNetwork>,
    db: Option<Res<'w, GameDatabase>>,
    gold: ResMut<'w, GoldUpdateQueue>,
    npcs: FlightMasterPositions<'w, 's>,
    players: TaxiPlayers<'w, 's>,
}

impl Takeoff<'_, '_> {
    /// `Player::ActivateTaxiPathTo` checks, then payment. Returns the route and
    /// the taxi mount display.
    fn take_off(
        &mut self,
        character_id: u64,
        player: Entity,
        npc: Entity,
        destination: u32,
    ) -> Result<(TaxiRoute, u32), TaxiError> {
        let db = &self.db.as_ref().ok_or(TaxiError::Busy)?.db;
        let (player_data, combat, flying) =
            self.players.get(player).map_err(|_| TaxiError::Busy)?;
        if flying {
            return Err(TaxiError::Busy);
        }
        if combat.is_some_and(|status| status.0) {
            return Err(TaxiError::InCombat);
        }
        let team = team_for_race(player_data.race).ok_or(TaxiError::NoPath)?;
        let (npc_position, npc_map) = self.npcs.get(npc).map_err(|_| TaxiError::NoVendorNearby)?;
        let current = self
            .network
            .nearest_node(npc_map.map_id, wow(npc_position), team)
            .ok_or(TaxiError::NoVendorNearby)?;
        if destination == current {
            return Err(TaxiError::SameNode);
        }
        let known = load_known_taxi_nodes(db, character_id);
        if !known.contains(&destination) {
            return Err(TaxiError::NotVisited);
        }
        let route = self
            .network
            .route(current, destination, &known, team)
            .ok_or(TaxiError::NoPath)?;
        let mount = self
            .network
            .mount_display(current, team)
            .ok_or(TaxiError::NoPath)?;
        let mut inventory = persistence::load_inventory(db, character_id);
        let cost = u64::from(route.cost);
        if inventory.gold < cost {
            return Err(TaxiError::NotEnoughMoney);
        }
        inventory.gold -= cost;
        persistence::save_inventory(db, character_id, &inventory);
        self.gold.record(character_id, inventory.gold);
        Ok((route, mount))
    }
}

type ActivateRequests<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static InWorld,
        &'static PlayerLink,
        Option<&'static ActiveInteraction>,
        &'static mut MessageReceiver<ActivateTaxi>,
    ),
>;

fn process_activate_taxi(
    mut requests: ActivateRequests,
    mut takeoff: Takeoff,
    mut controls: Query<&mut MovementControl>,
    mut senders: TaxiSenders,
    mut commands: Commands,
) {
    for (connection, in_world, link, interaction, mut receiver) in &mut requests {
        let mut took_off = false;
        for request in receiver.receive() {
            let npc = open_flight_master(interaction)
                .filter(|npc| npc.to_bits() == request.npc)
                .ok_or(TaxiError::NoVendorNearby);
            let result = npc.and_then(|npc| {
                if took_off {
                    return Err(TaxiError::Busy);
                }
                let flight =
                    takeoff.take_off(in_world.character_id, link.0, npc, request.destination)?;
                Ok((npc, flight))
            });
            let (npc, (route, mount)) = match result {
                Ok(started) => started,
                Err(error) => {
                    senders.failed(connection, request.npc, error);
                    continue;
                }
            };
            senders.closed(connection, npc);
            commands.entity(connection).remove::<ActiveInteraction>();
            // TC Player.cpp:23289-23307: a paid, started flight.
            let (character, cost) = (in_world.character_id, u64::from(route.cost));
            commands.queue(move |world: &mut World| {
                use crate::achievement::CriteriaEvent;
                crate::achievement::record_character_event(
                    world,
                    character,
                    CriteriaEvent::BuyTaxi,
                );
                if cost > 0 {
                    crate::achievement::record_character_event(
                        world,
                        character,
                        CriteriaEvent::MoneySpentOnTaxis(cost),
                    );
                }
            });
            let points = takeoff.network.flight_points(&route.nodes);
            let rider = link.0;
            commands.queue(move |world: &mut World| {
                crate::hunter_pet::unsummon_temporarily(world, rider)
            });
            commands.entity(link.0).insert((
                TaxiFlight { points, next: 0 },
                Mounted {
                    mount_display_id: mount,
                    ..Default::default()
                },
                MovementSpeed(FLIGHT_SPEED),
            ));
            if let Ok(mut control) = controls.get_mut(link.0) {
                control.controlled = true;
            }
            took_off = true;
        }
    }
}

pub(crate) const RUBY_FERRY_MAP: u32 = 2521;
pub(crate) const RUBY_FERRY_ENTRY: u32 = 194667;
pub(crate) const RUBY_FERRY_PATH: u32 = 11715;
pub(crate) const RUBY_FERRY_SOURCE: Vec3 = Vec3::new(1540.7935, 131.9674, 246.033);

/// Start the lead-approved Ruby ferry approximation. The source action set and
/// vehicle seat are unavailable; see the follower-dungeon spec for the inference.
pub(crate) fn start_ruby_ferry(world: &mut World, player: Entity, npc: Entity) {
    const RADIANT_DRAKE_DISPLAY: u32 = 107204;
    let Some(map) = world.get::<crate::maps::InMap>(player).copied() else {
        return;
    };
    if map.map_id != RUBY_FERRY_MAP || world.get::<crate::maps::InMap>(npc) != Some(&map) {
        return;
    }
    let Some(npc_position) = world.get::<Position>(npc) else {
        return;
    };
    if Vec3::new(npc_position.x, npc_position.y, npc_position.z).distance(RUBY_FERRY_SOURCE) > 1.0 {
        return;
    }
    let encounter = crate::scripted_ai::instance_script::boss_id_for_encounter(world, map, 2609);
    if encounter.and_then(|id| crate::scripted_ai::instance_script::get_boss_state(world, map, id))
        != Some(crate::scripted_ai::instance_script::EncounterState::Done)
    {
        return;
    }
    start_scripted_party_flight(
        world,
        player,
        map,
        RUBY_FERRY_PATH,
        Some(RADIANT_DRAKE_DISPLAY),
        false,
    );
}

/// DARKFLAME_CART_BOARDING_LOG_CORRELATED: the point is TaxiPath9458's first
/// node; WCL WBYDm1N6JTZzCFnd fight3 pull14 shows the party on the same path
/// using Throw Dynamite423374. The exact boarding trigger is unsourced.
pub(crate) fn start_darkflame_cart(world: &mut World, player: Entity, npc: Entity) {
    const CART_DISPLAY: u32 = 118139;
    let Some(map) = world.get::<crate::maps::InMap>(player).copied() else {
        return;
    };
    if map.map_id != DARKFLAME_CART_MAP || world.get::<crate::maps::InMap>(npc) != Some(&map) {
        return;
    }
    let encounter = crate::scripted_ai::instance_script::boss_id_for_encounter(world, map, 2787);
    if encounter.and_then(|id| crate::scripted_ai::instance_script::get_boss_state(world, map, id))
        != Some(crate::scripted_ai::instance_script::EncounterState::Done)
    {
        return;
    }
    let Some(points) = world
        .resource::<TaxiNetwork>()
        .scripted_path(DARKFLAME_CART_PATH)
    else {
        error!("Darkflame cart TaxiPath {DARKFLAME_CART_PATH} is unavailable");
        return;
    };
    let Some(position) = world.get::<Position>(npc) else {
        return;
    };
    let npc_position = Vec3::new(position.x, position.y, position.z);
    if points
        .first()
        .is_none_or(|first| first.distance(npc_position) > 1.0)
    {
        return;
    }
    start_scripted_party_flight(
        world,
        player,
        map,
        DARKFLAME_CART_PATH,
        Some(CART_DISPLAY),
        false,
    );
}

pub(crate) fn start_dungeon_passage_flight(
    world: &mut World,
    player: Entity,
    map: crate::maps::InMap,
    path: u32,
) {
    start_scripted_party_flight(world, player, map, path, None, true);
}

fn start_scripted_party_flight(
    world: &mut World,
    player: Entity,
    map: crate::maps::InMap,
    path: u32,
    display: Option<u32>,
    carry_pets: bool,
) {
    let Some(mut points) = world.resource::<TaxiNetwork>().scripted_path(path) else {
        error!("Scripted party TaxiPath {path} is unavailable");
        return;
    };
    if path == DARKFLAME_CART_PATH {
        points.push(DARKFLAME_CART_LANDING);
    }
    let mut riders = vec![player];
    let followers: Vec<Entity> = world
        .query::<(Entity, &crate::followers::Follower, &crate::maps::InMap)>()
        .iter(world)
        .filter_map(|(entity, follower, current)| {
            (follower.owner == player && *current == map).then_some(entity)
        })
        .collect();
    riders.extend(followers);
    if carry_pets {
        riders.retain(|&rider| crate::spell_cast::targets::is_alive(world, rider));
        let pets: Vec<_> = riders
            .iter()
            .flat_map(|&rider| crate::hunter_pet::active_owned_pets(world, rider))
            .collect();
        riders.extend(pets);
    }
    if riders.iter().any(|&rider| {
        world.get::<TaxiFlight>(rider).is_some()
            || world.get::<Mounted>(rider).is_some()
            || world.get::<Position>(rider).is_none()
            || world.get::<Rotation>(rider).is_none()
    }) {
        error!("Scripted TaxiPath {path} party contains a rider who cannot start flight");
        return;
    }
    for rider in riders {
        if !carry_pets {
            crate::hunter_pet::unsummon_temporarily(world, rider);
        }
        world.entity_mut(rider).insert(TaxiFlight {
            points: points.clone(),
            next: 0,
        });
        if let Some(display) = display {
            world.entity_mut(rider).insert(Mounted {
                mount_display_id: display,
                ..Default::default()
            });
        }
        if path == DARKFLAME_CART_PATH {
            world
                .entity_mut(rider)
                .insert(ScriptedTaxiSpeed(DARKFLAME_CART_SPEED));
        }
        if let Some(mut control) = world.get_mut::<MovementControl>(rider) {
            control.controlled = true;
        }
        if let Some(queue) = world.get_resource::<ReplicationDirtyQueue>() {
            queue.mark(rider);
        }
    }
}

/// Move `position` up to `step` yards along the flight; true once it landed.
pub(crate) fn advance_flight(flight: &mut TaxiFlight, position: &mut Vec3, mut step: f32) -> bool {
    while let Some(&target) = flight.points.get(flight.next) {
        let gap = position.distance(target);
        if gap > step {
            *position += (target - *position) / gap * step;
            return false;
        }
        *position = target;
        step -= gap;
        flight.next += 1;
    }
    true
}

type Flyers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut TaxiFlight,
        &'static mut Position,
        &'static mut Rotation,
        Option<&'static mut MovementControl>,
        Option<&'static ScriptedTaxiSpeed>,
    ),
>;

pub(crate) fn fly_taxis(
    time: Res<Time>,
    mut flyers: Flyers,
    dirty_queue: Option<Res<InterestDirtyQueue>>,
    mut commands: Commands,
) {
    for (entity, mut flight, mut position, mut rotation, control, scripted_speed) in &mut flyers {
        let speed = scripted_speed.map_or(FLIGHT_SPEED, |speed| speed.0);
        let step = speed * time.delta_secs();
        let start = Vec3::new(position.x, position.y, position.z);
        let mut next = start;
        let landed = advance_flight(&mut flight, &mut next, step);
        let heading = next - start;
        if heading.x != 0.0 || heading.z != 0.0 {
            rotation.y = heading.x.atan2(heading.z);
        }
        *position = Position {
            x: next.x,
            y: next.y,
            z: next.z,
        };
        crate::interest::mark_interest_position_changed(dirty_queue.as_deref(), entity);
        if landed {
            if let Some(mut control) = control {
                teleported(dirty_queue.as_deref(), entity, &mut control);
            }
            commands.entity(entity).queue(finish_flight);
            commands.queue(move |world: &mut World| fall_after_flight(world, entity));
            commands.queue(move |world: &mut World| {
                crate::hunter_pet::resummon_temporarily_unsummoned(world, entity)
            });
        }
    }
}

/// A creature that loses flight above the floor falls (`SetCanFly(false)` →
/// `MotionMaster::MoveFall`, `AuraEffect::HandleAuraAllowFlight`); a player's client drops it.
fn fall_after_flight(world: &mut World, rider: Entity) {
    if world.get_entity(rider).is_ok() && world.get::<Player>(rider).is_none() {
        crate::scripted_ai::fall_movement::move_fall(world, rider, 0);
    }
}

/// End a flight where the player is (landing, `Player::FinishTaxiFlight`).
pub(crate) fn finish_flight(mut player: EntityWorldMut) {
    if !player.contains::<TaxiFlight>() {
        return;
    }
    player.remove::<(TaxiFlight, Mounted, ScriptedTaxiSpeed)>();
    if player.contains::<Player>() {
        player.insert(MovementSpeed(RUN_SPEED));
    }
    if let Some(mut control) = player.get_mut::<MovementControl>() {
        control.controlled = false;
    }
}

#[cfg(test)]
#[path = "taxi_tests.rs"]
mod tests;
