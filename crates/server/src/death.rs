use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use shared::components::{Health, Level, Mana, Player, Position};
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
use crate::interest::{InterestDirtyQueue, mark_interest_position_changed};
use crate::networking::PlayerLink;
use crate::world_data::WorldData;

const WORLD_MAP_ID: u16 = 0;
const GHOST_RESURRECT_HEALTH_PCT: f32 = 0.5;
const GHOST_RESURRECT_MANA_PCT: f32 = 0.5;

#[path = "death_support.rs"]
mod support;
#[path = "death/transitions.rs"]
mod transitions;

#[cfg(test)]
use self::support::build_snapshot;
use self::support::{current_seconds, send_error, send_update};
use transitions::process_player_death_transitions;
pub(crate) use transitions::{PlayerDeathTransition, apply_health_damage};

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
struct DeathServerState {
    graveyards: GraveyardRegistry,
}

pub struct DeathPlugin;

/// Per-App death-transition work counters, so parallel tests don't share them.
#[cfg(test)]
#[derive(Resource, Debug, Default, PartialEq)]
struct DeathTransitionCounters {
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
        app.add_systems(
            Update,
            (restore_persisted_ghosts, send_initial_death_snapshot).chain(),
        );
        app.add_systems(Update, process_death_queries);
        app.add_systems(Update, process_release_spirit_requests);
        app.add_systems(Update, process_resurrect_at_corpse_requests);
        app.add_systems(Update, process_spirit_healer_requests);
        app.add_systems(Update, process_stuck_escape_requests);
        app.add_systems(
            FixedUpdate,
            process_player_death_transitions
                .after(crate::networking::apply_terrain_gravity)
                .after(crate::combat::auto_attack_system),
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
fn restore_persisted_ghosts(
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
        if !NpcFlags::from_azerothcore(template.npc_flags).contains(NpcFlags::SPIRIT_HEALER) {
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

fn send_initial_death_snapshot(
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
    mut players: Query<(&mut Position, &mut PlayerDeathState, &Player)>,
    state: Res<DeathServerState>,
    dirty_queue: Option<Res<InterestDirtyQueue>>,
    mut senders: Query<&mut MessageSender<DeathStateUpdate>>,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let Ok((mut position, mut death_state, player)) = players.get_mut(link.0) else {
                send_error(
                    conn_entity,
                    &mut senders,
                    "death state unavailable for player",
                );
                continue;
            };
            let result = release_spirit_for_player(
                &mut position,
                &mut death_state,
                player.race,
                &state.graveyards,
            );
            if result.is_ok() {
                mark_interest_position_changed(dirty_queue.as_deref(), link.0);
            }
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
    )>,
    dirty_queue: Option<Res<InterestDirtyQueue>>,
    mut senders: Query<&mut MessageSender<DeathStateUpdate>>,
) {
    for (conn_entity, link, player, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let Ok((mut position, mut health, mana, mut death_state)) = players.get_mut(link.0)
            else {
                send_error(
                    conn_entity,
                    &mut senders,
                    "death state unavailable for player",
                );
                continue;
            };
            let result =
                resurrect_player_at_corpse(&mut position, &mut health, mana, &mut death_state);
            if result.is_ok() {
                mark_interest_position_changed(dirty_queue.as_deref(), link.0);
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
) {
    let now = current_seconds(&time);
    for (conn_entity, link, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let Ok((mut position, mut health, mana, level, durability, mut death_state)) =
                players.get_mut(link.0)
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
                Ok(message) => {
                    mark_interest_position_changed(dirty_queue.as_deref(), link.0);
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
    mut players: Query<(&mut Position, Option<&PlayerDeathState>, &Player)>,
    state: Res<DeathServerState>,
    dirty_queue: Option<Res<InterestDirtyQueue>>,
    mut senders: Query<&mut MessageSender<DeathStateUpdate>>,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let Ok((mut position, death_state, player)) = players.get_mut(link.0) else {
                send_error(
                    conn_entity,
                    &mut senders,
                    "death state unavailable for player",
                );
                continue;
            };
            match use_stuck_escape(&mut position, death_state, player.race, &state.graveyards) {
                Ok(message) => {
                    mark_interest_position_changed(dirty_queue.as_deref(), link.0);
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
    position: &Position,
    player_race: u8,
    graveyards: &GraveyardRegistry,
    now: u64,
) -> PlayerDeathState {
    let result = on_death(
        client_id,
        WORLD_MAP_ID,
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
        WORLD_MAP_ID,
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
        WORLD_MAP_ID,
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
) -> Result<String, String> {
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
    Ok("accepted spirit healer resurrection".into())
}

fn complete_resurrection(
    health: &mut Health,
    mut mana: Option<Mut<Mana>>,
    death_state: &mut PlayerDeathState,
) {
    death_state.state = DeathState::Alive;
    death_state.corpse = None;
    death_state.graveyard = None;
    health.current = (health.max * GHOST_RESURRECT_HEALTH_PCT).max(1.0);
    if let Some(mana) = mana.as_mut() {
        mana.current = mana.max * GHOST_RESURRECT_MANA_PCT;
    }
}

#[cfg(test)]
#[path = "death_tests.rs"]
mod tests;
