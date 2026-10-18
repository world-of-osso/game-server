use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use shared::components::{Health, Level, Mana, MovementControl, Player, Position};
use shared::death::{
    Corpse, DeathState, Graveyard, GraveyardRegistry, SpiritHealerContext, accept_spirit_healer,
    in_corpse_range, on_death, resurrect_at_corpse,
};
use shared::protocol::{
    AcceptSpiritHealerResurrection, DeathChannel, DeathPositionSnapshot, DeathSnapshot,
    DeathStateSnapshot, DeathStateUpdate, NpcFlags, QueryDeathStatus, ReleaseSpirit,
    ResurrectAtCorpse, UseStuckEscape,
};

use crate::auth::{Authenticated, InWorld};
use crate::durability::EquippedDurability;
use crate::faction::faction_for_race;
use crate::interest::InterestDirtyQueue;
use crate::maps::{InMap, MapStore};
use crate::networking::PlayerLink;
use crate::teleport::teleported;
use crate::world_data::WorldData;

const GHOST_RESURRECT_HEALTH_PCT: f32 = 0.5;
const GHOST_RESURRECT_MANA_PCT: f32 = 0.5;

#[path = "death/resurrection.rs"]
pub(crate) mod resurrection;
#[path = "death_support.rs"]
mod support;
#[path = "death/transitions.rs"]
mod transitions;

use self::support::{
    build_snapshot, current_seconds, send_error, send_update, sync_replicated_life_state,
};
pub(crate) use transitions::process_player_death_transitions;
pub(crate) use transitions::{
    PlayerDeathTransition, apply_health_damage, deal_environmental_damage,
};

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerDeathState {
    pub state: DeathState,
    pub corpse: Option<Corpse>,
    pub graveyard: Option<Graveyard>,
}

impl Default for PlayerDeathState {
    fn default() -> Self {
        Self {
            state: DeathState::Alive,
            corpse: None,
            graveyard: None,
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct DeathServerState {
    graveyards: GraveyardRegistry,
}

pub struct DeathPlugin;

/// Per-App death-transition work counters, so parallel tests don't share them.
#[cfg(test)]
#[derive(Resource, Debug, Default, PartialEq)]
pub(crate) struct DeathTransitionCounters {
    players_visited: usize,
    replication_marks: usize,
}

impl Plugin for DeathPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlayerDeathTransition>();
        app.init_resource::<DeathServerState>();
        #[cfg(test)]
        app.init_resource::<DeathTransitionCounters>();
        app.add_systems(PostStartup, load_graveyards_from_world_data);
        // SelectCharacter spawns during Update. Restore legacy corpses and send
        // the initial snapshot only after all login commands have been applied.
        app.add_systems(
            PostUpdate,
            (
                restore_persisted_ghosts,
                sync_replicated_life_state,
                send_initial_death_snapshot,
            )
                .chain(),
        );
        app.add_systems(Update, resurrection::receive_responses);
        app.add_systems(Update, process_death_queries);
        app.add_systems(Update, process_release_spirit_requests);
        app.add_systems(Update, process_resurrect_at_corpse_requests);
        app.add_systems(Update, process_spirit_healer_requests);
        app.add_systems(Update, process_stuck_escape_requests);
        app.add_systems(
            FixedUpdate,
            (process_player_death_transitions, sync_replicated_life_state)
                .chain()
                .after(crate::networking::apply_terrain_gravity)
                .after(crate::combat::auto_attack_system)
                .before(crate::interest::InterestUpdate),
        );
    }
}

/// The corpse of a character that logged out as a ghost, on its new player
/// entity until `restore_persisted_ghosts` rebuilds the death state.
#[derive(Component, Debug)]
pub(crate) struct PersistedGhost(pub Corpse);

/// Player::LoadFromDB: a character saved with `PLAYER_FLAGS_GHOST` loads
/// still dead, as a ghost, and `Player::LoadCorpse` puts its corpse back
/// where it died. The graveyard is the corpse's, as at release.
pub(crate) fn restore_persisted_ghosts(
    ghosts: Query<(Entity, &PersistedGhost, &Player)>,
    state: Res<DeathServerState>,
    mut commands: Commands,
) {
    for (entity, ghost, player) in &ghosts {
        let mut death_state =
            build_player_death_state(ghost.0.clone(), player.race, &state.graveyards);
        death_state.state = DeathState::Ghost;
        commands
            .entity(entity)
            .remove::<PersistedGhost>()
            .insert(death_state);
    }
}

fn load_graveyards_from_world_data(
    world_data: Option<Res<WorldData>>,
    mut state: ResMut<DeathServerState>,
) {
    let Some(world_data) = world_data else { return };
    state.graveyards = build_graveyard_registry(&world_data);
}

fn build_graveyard_registry(world_data: &WorldData) -> GraveyardRegistry {
    let mut graveyards = GraveyardRegistry::new();
    for spawn in &world_data.spawns {
        let Some(template) = world_data.templates.get(&spawn.template_entry) else {
            continue;
        };
        if !NpcFlags(template.npc_flags).contains(NpcFlags::SPIRIT_HEALER) {
            continue;
        }
        let (x, y, z) = crate::terrain_height::wow_to_bevy(
            spawn.position_x,
            spawn.position_y,
            spawn.position_z,
        );
        graveyards.add(Graveyard {
            id: spawn.guid,
            zone_id: spawn.zone_id as u32,
            faction: 0,
            map_id: spawn.map,
            x,
            y,
            z,
        });
    }
    graveyards
}

fn lookup_graveyard(
    graveyards: &GraveyardRegistry,
    map_id: u16,
    x: f32,
    y: f32,
    z: f32,
    player_race: u8,
) -> Option<Graveyard> {
    graveyards
        .nearest(map_id, x, y, z, faction_for_race(player_race))
        .cloned()
}

fn lookup_corpse_graveyard(
    corpse: &Corpse,
    player_race: u8,
    graveyards: &GraveyardRegistry,
) -> Option<Graveyard> {
    lookup_graveyard(
        graveyards,
        corpse.map_id,
        corpse.x,
        corpse.y,
        corpse.z,
        player_race,
    )
}

type NewInWorldLink<'w, 's> =
    Query<'w, 's, (Entity, &'static PlayerLink), (With<Authenticated>, Added<InWorld>)>;

pub(crate) fn send_initial_death_snapshot(
    connections: NewInWorldLink,
    players: Query<(&Position, Option<&PlayerDeathState>)>,
    mut senders: Query<&mut MessageSender<DeathStateUpdate>>,
) {
    for (conn_entity, link) in &connections {
        let Ok((position, death_state)) = players.get(link.0) else {
            send_error(
                conn_entity,
                &mut senders,
                "death state unavailable for player",
            );
            continue;
        };
        send_update(
            conn_entity,
            position,
            death_state.cloned().unwrap_or_default(),
            Some("death state loaded".into()),
            None,
            &mut senders,
        );
    }
}

fn process_death_queries(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<QueryDeathStatus>),
        With<Authenticated>,
    >,
    players: Query<(&Position, Option<&PlayerDeathState>)>,
    mut senders: Query<&mut MessageSender<DeathStateUpdate>>,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        let requests: Vec<_> = receiver.receive().collect();
        if requests.is_empty() {
            continue;
        }
        let Ok((position, death_state)) = players.get(link.0) else {
            send_error(
                conn_entity,
                &mut senders,
                "death state unavailable for player",
            );
            continue;
        };
        send_update(
            conn_entity,
            position,
            death_state.cloned().unwrap_or_default(),
            Some("death state refreshed".into()),
            None,
            &mut senders,
        );
    }
}

fn process_release_spirit_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<ReleaseSpirit>),
        With<Authenticated>,
    >,
    mut players: Query<(
        &mut Position,
        &mut PlayerDeathState,
        &Player,
        &mut MovementControl,
        &InMap,
    )>,
    state: Res<DeathServerState>,
    maps: Option<Res<MapStore>>,
    dirty_queue: Option<Res<InterestDirtyQueue>>,
    mut senders: Query<&mut MessageSender<DeathStateUpdate>>,
    mut commands: Commands,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let Ok((mut position, mut death_state, player, mut control, map)) =
                players.get_mut(link.0)
            else {
                send_error(
                    conn_entity,
                    &mut senders,
                    "death state unavailable for player",
                );
                continue;
            };
            let dungeon = maps
                .as_deref()
                .and_then(|maps| maps.map(map.map_id))
                .filter(|entry| entry.is_dungeon());
            let result = match (dungeon, maps.as_deref()) {
                (Some(entry), Some(maps)) => release_spirit_in_dungeon(
                    link.0,
                    &position,
                    &mut death_state,
                    entry,
                    maps,
                    &mut commands,
                ),
                _ => release_spirit_for_player(
                    &mut position,
                    &mut death_state,
                    player.race,
                    &state.graveyards,
                )
                .inspect(|_| {
                    teleported(dirty_queue.as_deref(), link.0, &mut control);
                    let player = link.0;
                    commands.queue(move |world: &mut World| {
                        crate::spell_cast::cancel_cast(world, player);
                    });
                }),
            };
            send_release_spirit_result(
                conn_entity,
                &position,
                death_state.clone(),
                result,
                &mut senders,
            );
        }
    }
}

/// `Player::RepopAtGraveyard` in a dungeon (Player.cpp:4720-4764): the corpse turns to
/// bones and the player is teleported to the closest graveyard of the dungeon's zone
/// (`ObjectMgr::GetClosestGraveyard`) and resurrected there (`TELE_REVIVE_AT_TELEPORT`).
/// A WMO-only dungeon's zone is its `Map.db2` AreaTableID (`TerrainInfo::GetAreaId`).
pub(crate) fn release_spirit_in_dungeon(
    player: Entity,
    position: &Position,
    death_state: &mut PlayerDeathState,
    map: &crate::maps::store::MapEntry,
    maps: &MapStore,
    commands: &mut Commands,
) -> Result<String, String> {
    if death_state.state != DeathState::Dead {
        return Err("unable to release spirit here".to_string());
    }
    let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
    let location = crate::maps::WorldLocation {
        map_id: map.id,
        position: Vec3::new(x, y, z),
        orientation: 0.0,
    };
    let grave =
        crate::maps::graveyard::closest_graveyard_in_zone(maps, &location, map.area_table_id)
            .ok_or_else(|| "unable to release spirit here".to_string())?;
    death_state.state = DeathState::Ghost;
    death_state.corpse = None;
    death_state.graveyard = None;
    commands.queue(move |world: &mut World| {
        // BuildPlayerRepop sets DEAD before the graveyard teleport/revival.
        crate::spell_cast::cancel_cast(world, player);
        crate::maps::teleport_to(
            world,
            player,
            grave,
            crate::maps::TeleportOptions { revive: true },
        );
    });
    Ok("released spirit".into())
}

fn send_release_spirit_result(
    conn_entity: Entity,
    position: &Position,
    death_state: PlayerDeathState,
    result: Result<String, String>,
    senders: &mut Query<&mut MessageSender<DeathStateUpdate>>,
) {
    match result {
        Ok(message) => {
            send_update(
                conn_entity,
                position,
                death_state,
                Some(message),
                None,
                senders,
            );
        }
        Err(error) => {
            send_update(
                conn_entity,
                position,
                death_state,
                None,
                Some(error),
                senders,
            );
        }
    }
}

type ResurrectReceiver<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static PlayerLink,
        Option<&'static Player>,
        &'static mut MessageReceiver<ResurrectAtCorpse>,
    ),
    With<Authenticated>,
>;

fn process_resurrect_at_corpse_requests(
    mut receivers: ResurrectReceiver,
    mut players: Query<(
        &mut Position,
        &mut Health,
        Option<&mut Mana>,
        &mut PlayerDeathState,
        &mut MovementControl,
        &InMap,
    )>,
    dirty_queue: Option<Res<InterestDirtyQueue>>,
    mut senders: Query<&mut MessageSender<DeathStateUpdate>>,
) {
    for (conn_entity, link, player, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let Ok((mut position, mut health, mana, mut death_state, mut control, map)) =
                players.get_mut(link.0)
            else {
                send_error(
                    conn_entity,
                    &mut senders,
                    "death state unavailable for player",
                );
                continue;
            };
            let result = resurrect_player_at_corpse(
                &mut position,
                &mut health,
                mana,
                &mut death_state,
                map.map_id,
            );
            if result.is_ok() {
                teleported(dirty_queue.as_deref(), link.0, &mut control);
            }
            send_resurrect_response(
                conn_entity,
                &position,
                player,
                result,
                &death_state,
                &mut senders,
            );
        }
    }
}

fn send_resurrect_response(
    conn_entity: Entity,
    position: &Position,
    player: Option<&Player>,
    result: Result<String, String>,
    death_state: &PlayerDeathState,
    senders: &mut Query<&mut MessageSender<DeathStateUpdate>>,
) {
    match result {
        Ok(message) => {
            let name = player.map(|p| p.name.as_str()).unwrap_or("player");
            send_update(
                conn_entity,
                position,
                death_state.clone(),
                Some(format!("{name} {message}")),
                None,
                senders,
            );
        }
        Err(error) => {
            send_update(
                conn_entity,
                position,
                death_state.clone(),
                None,
                Some(error),
                senders,
            );
        }
    }
}

/// Resurrection Sickness: `Player::ResurrectPlayer(0.5f, true)` casts it on the player
/// after a spirit healer resurrection at or above the sickness level.
const RESURRECTION_SICKNESS_SPELL: u32 = 15007;

type SpiritHealerPlayerData<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Position,
        &'static mut Health,
        Option<&'static mut Mana>,
        Option<&'static Level>,
        Option<&'static mut EquippedDurability>,
        &'static mut PlayerDeathState,
        &'static mut MovementControl,
    ),
>;

fn process_spirit_healer_requests(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &mut MessageReceiver<AcceptSpiritHealerResurrection>,
        ),
        With<Authenticated>,
    >,
    mut players: SpiritHealerPlayerData,
    time: Res<Time>,
    dirty_queue: Option<Res<InterestDirtyQueue>>,
    mut senders: Query<&mut MessageSender<DeathStateUpdate>>,
    mut commands: Commands,
) {
    let now = current_seconds(&time);
    for (conn_entity, link, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let Ok((
                mut position,
                mut health,
                mana,
                level,
                durability,
                mut death_state,
                mut control,
            )) = players.get_mut(link.0)
            else {
                send_error(
                    conn_entity,
                    &mut senders,
                    "death state unavailable for player",
                );
                continue;
            };
            match resurrect_at_spirit_healer(
                &mut position,
                &mut health,
                mana,
                durability,
                &mut death_state,
                level.map(|level| level.0).unwrap_or_default(),
                now,
            ) {
                Ok((message, sickness)) => {
                    if sickness {
                        let player = link.0;
                        commands.queue(move |world: &mut World| {
                            crate::spell_cast::cast_triggered(
                                world,
                                player,
                                RESURRECTION_SICKNESS_SPELL,
                                Some(player),
                                0,
                            );
                        });
                    }
                    teleported(dirty_queue.as_deref(), link.0, &mut control);
                    send_update(
                        conn_entity,
                        &position,
                        death_state.clone(),
                        Some(message),
                        None,
                        &mut senders,
                    );
                }
                Err(error) => {
                    send_update(
                        conn_entity,
                        &position,
                        death_state.clone(),
                        None,
                        Some(error),
                        &mut senders,
                    );
                }
            }
        }
    }
}

fn process_stuck_escape_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<UseStuckEscape>),
        With<Authenticated>,
    >,
    mut players: Query<(
        &mut Position,
        Option<&PlayerDeathState>,
        &Player,
        &mut MovementControl,
        &InMap,
    )>,
    state: Res<DeathServerState>,
    dirty_queue: Option<Res<InterestDirtyQueue>>,
    mut senders: Query<&mut MessageSender<DeathStateUpdate>>,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let Ok((mut position, death_state, player, mut control, map)) = players.get_mut(link.0)
            else {
                send_error(
                    conn_entity,
                    &mut senders,
                    "death state unavailable for player",
                );
                continue;
            };
            match use_stuck_escape(
                &mut position,
                death_state,
                player.race,
                map.map_id,
                &state.graveyards,
            ) {
                Ok(message) => {
                    teleported(dirty_queue.as_deref(), link.0, &mut control);
                    send_update(
                        conn_entity,
                        &position,
                        death_state.cloned().unwrap_or_default(),
                        Some(message),
                        None,
                        &mut senders,
                    );
                }
                Err(error) => {
                    send_update(
                        conn_entity,
                        &position,
                        death_state.cloned().unwrap_or_default(),
                        None,
                        Some(error),
                        &mut senders,
                    );
                }
            }
        }
    }
}

fn build_detected_player_death_state(
    client_id: u64,
    map_id: u32,
    position: &Position,
    player_race: u8,
    graveyards: &GraveyardRegistry,
    now: u64,
) -> PlayerDeathState {
    let result = on_death(
        client_id,
        map_id as u16,
        position.x,
        position.y,
        position.z,
        now,
    );
    build_player_death_state(result.corpse, player_race, graveyards)
}

fn build_player_death_state(
    corpse: Corpse,
    player_race: u8,
    graveyards: &GraveyardRegistry,
) -> PlayerDeathState {
    PlayerDeathState {
        state: DeathState::Dead,
        graveyard: lookup_corpse_graveyard(&corpse, player_race, graveyards),
        corpse: Some(corpse),
    }
}

fn connection_for_player(
    player_entity: Entity,
    connections: &Query<(Entity, &PlayerLink), With<Authenticated>>,
) -> Option<Entity> {
    connections
        .iter()
        .find_map(|(conn_entity, link)| (link.0 == player_entity).then_some(conn_entity))
}

fn release_spirit_for_player(
    position: &mut Position,
    death_state: &mut PlayerDeathState,
    player_race: u8,
    graveyards: &GraveyardRegistry,
) -> Result<String, String> {
    if death_state.state != DeathState::Dead {
        return Err("unable to release spirit here".to_string());
    }
    let corpse = death_state
        .corpse
        .as_ref()
        .ok_or_else(|| "corpse is unavailable".to_string())?;
    let graveyard = death_state
        .graveyard
        .clone()
        .or_else(|| lookup_corpse_graveyard(corpse, player_race, graveyards))
        .ok_or_else(|| "unable to release spirit here".to_string())?;
    death_state.state = DeathState::Ghost;
    death_state.graveyard = Some(graveyard.clone());
    position.x = graveyard.x;
    position.y = graveyard.y;
    position.z = graveyard.z;
    Ok("released spirit".into())
}

fn use_stuck_escape(
    position: &mut Position,
    death_state: Option<&PlayerDeathState>,
    player_race: u8,
    map_id: u32,
    graveyards: &GraveyardRegistry,
) -> Result<String, String> {
    if death_state
        .as_ref()
        .is_some_and(|state| state.state != DeathState::Alive)
    {
        return Err("stuck escape is unavailable while dead".into());
    }
    let graveyard = lookup_graveyard(
        graveyards,
        map_id as u16,
        position.x,
        position.y,
        position.z,
        player_race,
    )
    .ok_or_else(|| "unable to find a safe graveyard".to_string())?;
    position.x = graveyard.x;
    position.y = graveyard.y;
    position.z = graveyard.z;
    Ok("stuck escape used".into())
}

fn resurrect_player_at_corpse(
    position: &mut Position,
    health: &mut Health,
    mana: Option<Mut<Mana>>,
    death_state: &mut PlayerDeathState,
    map_id: u32,
) -> Result<String, String> {
    let corpse = death_state
        .corpse
        .as_ref()
        .ok_or_else(|| "corpse is unavailable".to_string())?;
    let result = resurrect_at_corpse(
        death_state.state,
        position.x,
        position.y,
        position.z,
        map_id as u16,
        corpse,
    )
    .map_err(|_| "you are not close enough to your corpse".to_string())?;
    position.x = result.x;
    position.y = result.y;
    position.z = result.z;
    complete_resurrection(health, mana, death_state);
    Ok("resurrected at corpse".into())
}

fn resurrect_at_spirit_healer(
    position: &mut Position,
    health: &mut Health,
    mana: Option<Mut<Mana>>,
    durability: Option<Mut<EquippedDurability>>,
    death_state: &mut PlayerDeathState,
    level: u8,
    now: u64,
) -> Result<(String, bool), String> {
    let graveyard = death_state
        .graveyard
        .as_ref()
        .ok_or_else(|| "spirit healer is unavailable".to_string())?;
    let result = accept_spirit_healer(&SpiritHealerContext {
        state: death_state.state,
        player_level: level,
        player_pos: (position.x, position.y, position.z),
        healer_pos: (graveyard.x, graveyard.y, graveyard.z),
        healer_map: graveyard.map_id,
        now,
    })
    .map_err(|_| "you must be near a spirit healer".to_string())?;
    position.x = result.x;
    position.y = result.y;
    position.z = result.z;
    if let Some(mut durability) = durability {
        crate::durability::apply_spirit_healer_loss(&mut durability);
    }
    complete_resurrection(health, mana, death_state);
    Ok((
        "accepted spirit healer resurrection".into(),
        result.sickness.is_some(),
    ))
}

fn complete_resurrection(
    health: &mut Health,
    mana: Option<Mut<Mana>>,
    death_state: &mut PlayerDeathState,
) {
    restore_life(
        health,
        mana,
        death_state,
        (GHOST_RESURRECT_HEALTH_PCT, GHOST_RESURRECT_MANA_PCT),
    );
}
fn restore_life(
    health: &mut Health,
    mut mana: Option<Mut<Mana>>,
    death_state: &mut PlayerDeathState,
    (health_fraction, mana_fraction): (f32, f32),
) {
    death_state.state = DeathState::Alive;
    death_state.corpse = None;
    death_state.graveyard = None;
    health.current = (health.max * health_fraction).max(1.0);
    if let Some(mana) = mana.as_mut() {
        mana.current = mana.max * mana_fraction;
    }
}

/// `Player::ResurrectPlayer(0.5f)` where the player stands (a teleport's
/// `TELE_REVIVE_AT_TELEPORT`, a ghost entering the dungeon holding its corpse).
pub(crate) fn resurrect_player(world: &mut World, player: Entity) {
    resurrect_at_rates(
        world,
        player,
        (GHOST_RESURRECT_HEALTH_PCT, GHOST_RESURRECT_MANA_PCT),
    );
}
/// TC Battleground.cpp:689/848: ResurrectPlayer(1.0f) on completion/exit.
pub(crate) fn resurrect_player_fully(world: &mut World, player: Entity) {
    resurrect_at_rates(world, player, (1.0, 1.0));
}
fn resurrect_at_rates(world: &mut World, player: Entity, rates: (f32, f32)) {
    let mut entity = world.entity_mut(player);
    let Some(mut death_state) = entity.get::<PlayerDeathState>().cloned() else {
        return;
    };
    let Some(mut health) = entity.get::<Health>().copied() else {
        return;
    };
    restore_life(
        &mut health,
        entity.get_mut::<Mana>(),
        &mut death_state,
        rates,
    );
    entity.insert((health, death_state.clone()));
    if let Some(queue) = world.get_resource::<ReplicationDirtyQueue>() {
        queue.mark(player);
    }
    let position = *world
        .get::<Position>(player)
        .expect("player has a Position");
    let mut connections = world.query_filtered::<(Entity, &PlayerLink), With<Authenticated>>();
    let Some(connection) = connections
        .iter(world)
        .find_map(|(connection, link)| (link.0 == player).then_some(connection))
    else {
        return;
    };
    if let Some(mut sender) = world.get_mut::<MessageSender<DeathStateUpdate>>(connection) {
        sender.send::<DeathChannel>(DeathStateUpdate {
            snapshot: Some(build_snapshot(&position, &death_state)),
            message: Some("resurrected".into()),
            error: None,
        });
    }
}

#[cfg(test)]
#[path = "death_tests.rs"]
mod tests;
