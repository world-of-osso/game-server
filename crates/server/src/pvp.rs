use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use shared::battleground::{BgQueue, template_by_id};
use shared::components::Level;
use shared::currency::{CONQUEST_CURRENCY_ID, CurrencyWallet, HONOR_CURRENCY_ID};
use shared::protocol::{
    DequeueFromPvp, PvpBracketSnapshot, PvpStateUpdate, QueryPvpStatus, QueueForBattleground,
    QueueForRatedPvp,
};

use self::pvp_support::{
    bracket_label, connection_for_character, current_seconds, default_brackets,
    format_bg_queue_error, queue_label, send_pvp_error, send_pvp_update,
};
use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;

#[path = "pvp_support.rs"]
mod pvp_support;

const DEFAULT_RATING: u32 = 1500;
const HONOR_CAP: u32 = 15_000;
// CurrencyTypes1602 has no static cap; WorldState31270 is unresolved.
const CONQUEST_CAP: u32 = 0;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerPvpBracketState {
    pub bracket: PvpBracketSnapshot,
    pub rating: u32,
    pub season_wins: u32,
    pub season_losses: u32,
    pub weekly_wins: u32,
    pub weekly_losses: u32,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerPvpProgress {
    pub honor: u32,
    pub honor_max: u32,
    pub conquest: u32,
    pub conquest_max: u32,
    pub brackets: Vec<PlayerPvpBracketState>,
}

impl Default for PlayerPvpProgress {
    fn default() -> Self {
        Self {
            honor: 0,
            honor_max: HONOR_CAP,
            conquest: 0,
            conquest_max: CONQUEST_CAP,
            brackets: default_brackets(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerPvpQueue {
    Battleground { battleground_id: u32, name: String },
    RatedBracket { bracket: PvpBracketSnapshot },
}

#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerPvpState {
    pub progress: PlayerPvpProgress,
    pub queue: Option<PlayerPvpQueue>,
}

impl PlayerPvpState {
    pub fn from_progress(progress: PlayerPvpProgress) -> Self {
        Self {
            progress,
            queue: None,
        }
    }
}

pub(crate) fn sync_progress_currencies_from_wallet(
    progress: &mut PlayerPvpProgress,
    wallet: &CurrencyWallet,
) -> bool {
    let honor = wallet.get(HONOR_CURRENCY_ID);
    let conquest = wallet.get(CONQUEST_CURRENCY_ID);
    let changed = progress.honor != honor || progress.conquest != conquest;
    progress.honor = honor;
    progress.conquest = conquest;
    changed
}

#[derive(Resource, Default)]
struct PvpServerState {
    battleground_queue: BgQueue,
    rated_queues: Vec<u64>,
}

#[derive(bevy::ecs::system::SystemParam)]
struct RatedQueueContext<'w, 's> {
    state: ResMut<'w, PvpServerState>,
    senders: Query<'w, 's, &'static mut MessageSender<PvpStateUpdate>>,
}

pub struct PvpPlugin;

impl Plugin for PvpPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PvpServerState>();
        app.add_systems(Update, send_initial_pvp_snapshot);
        app.add_systems(Update, process_pvp_queries);
        app.add_systems(Update, process_battleground_queue_requests);
        app.add_systems(Update, process_rated_queue_requests);
        app.add_systems(Update, process_pvp_dequeue_requests);
    }
}

type NewInWorldLink<'w, 's> =
    Query<'w, 's, (Entity, &'static PlayerLink), (With<Authenticated>, Added<InWorld>)>;

fn send_initial_pvp_snapshot(
    connections: NewInWorldLink,
    players: Query<&PlayerPvpState>,
    mut senders: Query<&mut MessageSender<PvpStateUpdate>>,
) {
    for (conn_entity, link) in &connections {
        let Ok(state) = players.get(link.0) else {
            send_pvp_error(conn_entity, &mut senders, "pvp unavailable for player");
            continue;
        };
        send_pvp_update(
            conn_entity,
            state,
            Some("pvp state loaded".into()),
            None,
            &mut senders,
        );
    }
}

fn process_pvp_queries(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<QueryPvpStatus>),
        With<Authenticated>,
    >,
    players: Query<&PlayerPvpState>,
    mut senders: Query<&mut MessageSender<PvpStateUpdate>>,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        let requests: Vec<_> = receiver.receive().collect();
        if requests.is_empty() {
            continue;
        }
        let Ok(state) = players.get(link.0) else {
            send_pvp_error(conn_entity, &mut senders, "pvp unavailable for player");
            continue;
        };
        send_pvp_update(
            conn_entity,
            state,
            Some("pvp status refreshed".into()),
            None,
            &mut senders,
        );
    }
}

fn process_battleground_queue_requests(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &InWorld,
            &mut MessageReceiver<QueueForBattleground>,
        ),
        With<Authenticated>,
    >,
    mut players: Query<(&Level, &shared::components::Player, &mut PlayerPvpState)>,
    connections: Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    state: ResMut<PvpServerState>,
    time: Res<Time>,
    mut senders: Query<&mut MessageSender<PvpStateUpdate>>,
) {
    let state = state.into_inner();
    let now = current_seconds(&time);
    for (conn_entity, link, in_world, mut receiver) in &mut receivers {
        for request in receiver.receive() {
            let result = {
                let Ok((level, _player, mut player_state)) = players.get_mut(link.0) else {
                    send_pvp_error(conn_entity, &mut senders, "pvp unavailable for player");
                    continue;
                };
                queue_for_battleground(
                    &mut player_state,
                    state,
                    in_world.character_id,
                    level.0,
                    request.battleground_id,
                    now,
                )
            };

            match result {
                Ok(message) => {
                    if let Ok((_level, _player, player_state)) = players.get(link.0) {
                        send_pvp_update(
                            conn_entity,
                            player_state,
                            Some(message),
                            None,
                            &mut senders,
                        );
                    }
                    resolve_battleground_matches(
                        request.battleground_id,
                        state,
                        &mut players,
                        &connections,
                        &mut senders,
                    );
                }
                Err(error) => {
                    if let Ok((_level, _player, player_state)) = players.get(link.0) {
                        send_pvp_update(conn_entity, player_state, None, Some(error), &mut senders);
                    } else {
                        send_pvp_error(conn_entity, &mut senders, "pvp unavailable for player");
                    }
                }
            }
        }
    }
}

fn process_rated_queue_requests(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &InWorld,
            &mut MessageReceiver<QueueForRatedPvp>,
        ),
        With<Authenticated>,
    >,
    mut players: Query<(
        &shared::components::Player,
        &mut PlayerPvpState,
        &mut crate::currency::PlayerCurrencies,
    )>,
    mut context: RatedQueueContext,
) {
    let state = context.state.into_inner();
    for (conn_entity, link, in_world, mut receiver) in &mut receivers {
        for request in receiver.receive() {
            let result = {
                let Ok((_player, mut player_state, _currencies)) = players.get_mut(link.0) else {
                    send_pvp_error(
                        conn_entity,
                        &mut context.senders,
                        "pvp unavailable for player",
                    );
                    continue;
                };
                queue_for_rated_bracket(
                    &mut player_state,
                    state,
                    in_world.character_id,
                    request.bracket.clone(),
                )
            };

            match result {
                Ok(message) => {
                    if let Ok((_player, player_state, _currencies)) = players.get(link.0) {
                        send_pvp_update(
                            conn_entity,
                            player_state,
                            Some(message),
                            None,
                            &mut context.senders,
                        );
                    }
                }
                Err(error) => {
                    if let Ok((_player, player_state, _currencies)) = players.get(link.0) {
                        send_pvp_update(
                            conn_entity,
                            player_state,
                            None,
                            Some(error),
                            &mut context.senders,
                        );
                    } else {
                        send_pvp_error(
                            conn_entity,
                            &mut context.senders,
                            "pvp unavailable for player",
                        );
                    }
                }
            }
        }
    }
}

fn process_pvp_dequeue_requests(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &InWorld,
            &mut MessageReceiver<DequeueFromPvp>,
        ),
        With<Authenticated>,
    >,
    mut players: Query<&mut PlayerPvpState>,
    state: ResMut<PvpServerState>,
    mut senders: Query<&mut MessageSender<PvpStateUpdate>>,
) {
    let state = state.into_inner();
    for (conn_entity, link, in_world, mut receiver) in &mut receivers {
        for _request in receiver.receive() {
            let Ok(mut player_state) = players.get_mut(link.0) else {
                send_pvp_error(conn_entity, &mut senders, "pvp unavailable for player");
                continue;
            };
            match dequeue_player(&mut player_state, state, in_world.character_id) {
                Ok(message) => send_pvp_update(
                    conn_entity,
                    &player_state,
                    Some(message),
                    None,
                    &mut senders,
                ),
                Err(error) => {
                    send_pvp_update(conn_entity, &player_state, None, Some(error), &mut senders)
                }
            }
        }
    }
}

fn queue_for_battleground(
    player_state: &mut PlayerPvpState,
    state: &mut PvpServerState,
    character_id: u64,
    level: u8,
    battleground_id: u32,
    now: u64,
) -> Result<String, String> {
    if let Some(queue) = &player_state.queue {
        return Err(format!("already queued for {}", queue_label(queue)));
    }
    let Some(template) = template_by_id(battleground_id) else {
        return Err(format!("unknown battleground id {}", battleground_id));
    };
    state
        .battleground_queue
        .join_solo(character_id, level, battleground_id, now)
        .map_err(format_bg_queue_error)?;
    player_state.queue = Some(PlayerPvpQueue::Battleground {
        battleground_id,
        name: template.name.clone(),
    });
    Ok(format!("queued for {}", template.name))
}

fn queue_for_rated_bracket(
    player_state: &mut PlayerPvpState,
    state: &mut PvpServerState,
    character_id: u64,
    bracket: PvpBracketSnapshot,
) -> Result<String, String> {
    if let Some(queue) = &player_state.queue {
        return Err(format!("already queued for {}", queue_label(queue)));
    }
    if state
        .rated_queues
        .iter()
        .any(|entry| *entry == character_id)
    {
        return Err("already queued for rated pvp".into());
    }
    player_state.queue = Some(PlayerPvpQueue::RatedBracket {
        bracket: bracket.clone(),
    });
    state.rated_queues.push(character_id);
    Ok(format!(
        "queued for {}; match lifecycle and authored reward/rating policy unavailable",
        bracket_label(&bracket)
    ))
}

fn dequeue_player(
    player_state: &mut PlayerPvpState,
    state: &mut PvpServerState,
    character_id: u64,
) -> Result<String, String> {
    let Some(queue) = player_state.queue.take() else {
        return Err("not queued for pvp".into());
    };
    match &queue {
        PlayerPvpQueue::Battleground { .. } => {
            state.battleground_queue.leave(character_id);
        }
        PlayerPvpQueue::RatedBracket { .. } => {
            state.rated_queues.retain(|entry| *entry != character_id);
        }
    }
    Ok(format!("left {}", queue_label(&queue)))
}

fn resolve_battleground_matches(
    battleground_id: u32,
    state: &mut PvpServerState,
    players: &mut Query<(&Level, &shared::components::Player, &mut PlayerPvpState)>,
    connections: &Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    senders: &mut Query<&mut MessageSender<PvpStateUpdate>>,
) {
    while let Some(bg_match) = state.battleground_queue.try_match(battleground_id) {
        let Some(template) = template_by_id(bg_match.bg_id) else {
            continue;
        };
        let team_a = bg_match.team_a.into_iter().collect::<Vec<_>>();
        let team_b = bg_match.team_b.into_iter().collect::<Vec<_>>();
        for character_id in team_a.iter().chain(team_b.iter()) {
            let Some((conn_entity, player_entity)) =
                connection_for_character(connections, *character_id)
            else {
                continue;
            };
            let Ok((_level, _player, mut player_state)) = players.get_mut(player_entity) else {
                continue;
            };
            player_state.queue = None;
            send_pvp_update(
                conn_entity,
                &player_state,
                Some(format!("battleground ready: {}", template.name)),
                None,
                senders,
            );
        }
    }
}

#[cfg(test)]
#[path = "pvp_tests.rs"]
mod tests;
