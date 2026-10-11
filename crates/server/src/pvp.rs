use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use shared::arena::calculate_rating;
use shared::battleground::{BgInstance, BgQueue, template_by_id};
use shared::components::Level;
use shared::currency::CurrencyWallet;
use shared::protocol::{
    CurrencyStateUpdate, DequeueFromPvp, PvpBracketSnapshot, PvpStateUpdate, QueryPvpStatus,
    QueueForBattleground, QueueForRatedPvp,
};
use std::collections::{BTreeMap, BTreeSet};

use self::pvp_support::{
    RatedParticipant, apply_single_rated_result, bracket_label, build_rated_reward,
    connection_for_character, current_seconds, default_brackets, format_bg_queue_error,
    queue_label, send_pvp_error, send_pvp_update,
};
use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::GameDatabase;

#[path = "pvp_support.rs"]
mod pvp_support;

#[path = "pvp_entry.rs"]
mod pvp_entry;
#[path = "pvp_invites.rs"]
mod pvp_invites;
use pvp_invites::process_battleground_invitation_responses;

#[path = "pvp_leave.rs"]
mod pvp_leave;
use pvp_leave::process_battleground_leave_requests;

const DEFAULT_RATING: u32 = 1500;
const HONOR_CAP: u32 = 15_000;
const CONQUEST_CAP: u32 = 1_800;
const WINNER_HONOR: u32 = 150;
const LOSER_HONOR: u32 = 75;
const WINNER_CONQUEST: u32 = 100;
const LOSER_CONQUEST: u32 = 25;
const HONOR_CURRENCY_ID: u32 = 1;
const CONQUEST_CURRENCY_ID: u32 = 2;

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

impl PlayerPvpProgress {
    fn bracket_mut(&mut self, bracket: PvpBracketSnapshot) -> &mut PlayerPvpBracketState {
        let index = self
            .brackets
            .iter()
            .position(|entry| entry.bracket == bracket)
            .expect("pvp bracket missing");
        &mut self.brackets[index]
    }

    fn bracket_rating(&self, bracket: &PvpBracketSnapshot) -> u32 {
        self.brackets
            .iter()
            .find(|entry| &entry.bracket == bracket)
            .map(|entry| entry.rating)
            .unwrap_or(DEFAULT_RATING)
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
    pub battleground: Option<shared::protocol::BattlegroundSnapshot>,
}

impl PlayerPvpState {
    pub fn from_progress(progress: PlayerPvpProgress) -> Self {
        Self {
            progress,
            queue: None,
            battleground: None,
        }
    }
}

pub(crate) fn sync_progress_currencies_from_wallet(
    progress: &mut PlayerPvpProgress,
    wallet: &CurrencyWallet,
) -> bool {
    let honor = wallet.get(HONOR_CURRENCY_ID).min(progress.honor_max);
    let conquest = wallet.get(CONQUEST_CURRENCY_ID).min(progress.conquest_max);
    let changed = progress.honor != honor || progress.conquest != conquest;
    progress.honor = honor;
    progress.conquest = conquest;
    changed
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RatedQueueEntry {
    player_entity: Entity,
    character_id: u64,
    bracket: PvpBracketSnapshot,
    queued_at: u64,
}

#[derive(Debug)]
struct InvitedBattleground {
    instance: BgInstance,
    accepted: BTreeSet<u64>,
    map: Option<crate::maps::InMap>,
    origins: BTreeMap<u64, crate::maps::WorldLocation>,
}

#[derive(Resource, Default)]
struct PvpServerState {
    battleground_queue: BgQueue,
    rated_queues: Vec<RatedQueueEntry>,
    battleground_matches: BTreeMap<u64, InvitedBattleground>,
    next_battleground_match: u64,
}

fn form_battleground_invitation(state: &mut PvpServerState, battleground_id: u32) -> Option<u64> {
    let template = template_by_id(battleground_id)?;
    let matched = state.battleground_queue.try_match(battleground_id)?;
    state.next_battleground_match += 1;
    let match_id = state.next_battleground_match;
    state.battleground_matches.insert(
        match_id,
        InvitedBattleground {
            instance: BgInstance::from_match(&matched, &template),
            accepted: BTreeSet::new(),
            map: None,
            origins: BTreeMap::new(),
        },
    );
    Some(match_id)
}

fn respond_to_battleground_invitation(
    state: &mut PvpServerState,
    match_id: u64,
    character_id: u64,
    accepted: bool,
) -> Result<(), String> {
    let invitation = state
        .battleground_matches
        .get_mut(&match_id)
        .ok_or("battleground invitation is no longer available")?;
    let instance = &mut invitation.instance;
    let is_member = instance.team_a_players.contains(&character_id)
        || instance.team_b_players.contains(&character_id);
    if !is_member || instance.phase != shared::battleground::BgPhase::Waiting {
        return Err("player has no invitation to this battleground".into());
    }
    if invitation.accepted.contains(&character_id) {
        return Err("battleground invitation already accepted".into());
    }
    if accepted {
        invitation.accepted.insert(character_id);
    } else {
        instance.team_a_players.retain(|id| *id != character_id);
        instance.team_b_players.retain(|id| *id != character_id);
    }
    if instance.team_a_players.is_empty() && instance.team_b_players.is_empty() {
        state.battleground_matches.remove(&match_id);
    }
    Ok(())
}

#[derive(bevy::ecs::system::SystemParam)]
struct RatedQueueContext<'w, 's> {
    connections:
        Query<'w, 's, (Entity, &'static PlayerLink, &'static InWorld), With<Authenticated>>,
    state: ResMut<'w, PvpServerState>,
    db: Option<Res<'w, GameDatabase>>,
    time: Res<'w, Time>,
    senders: Query<'w, 's, &'static mut MessageSender<PvpStateUpdate>>,
    currency_senders: Query<'w, 's, &'static mut MessageSender<CurrencyStateUpdate>>,
}

pub struct PvpPlugin;

impl Plugin for PvpPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PvpServerState>();
        app.add_systems(Update, send_initial_pvp_snapshot);
        app.add_systems(Update, process_pvp_queries);
        app.add_systems(Update, process_battleground_queue_requests);
        app.add_systems(
            Update,
            process_battleground_invitation_responses.after(process_battleground_queue_requests),
        );
        app.add_systems(
            Update,
            process_battleground_leave_requests.after(process_battleground_invitation_responses),
        );
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
    let Some(db) = context.db.as_ref() else {
        return;
    };
    let state = context.state.into_inner();
    let now = current_seconds(&context.time);
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
                    link.0,
                    in_world.character_id,
                    request.bracket.clone(),
                    now,
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
                    resolve_rated_matches(
                        state,
                        &mut players,
                        &context.connections,
                        db,
                        &mut context.senders,
                        &mut context.currency_senders,
                    );
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
    if player_state.battleground.is_some() {
        return Err("respond to the current battleground invitation first".into());
    }
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
    player_entity: Entity,
    character_id: u64,
    bracket: PvpBracketSnapshot,
    now: u64,
) -> Result<String, String> {
    if player_state.battleground.is_some() {
        return Err("respond to the current battleground invitation first".into());
    }
    if let Some(queue) = &player_state.queue {
        return Err(format!("already queued for {}", queue_label(queue)));
    }
    if state
        .rated_queues
        .iter()
        .any(|entry| entry.character_id == character_id)
    {
        return Err("already queued for rated pvp".into());
    }
    player_state.queue = Some(PlayerPvpQueue::RatedBracket {
        bracket: bracket.clone(),
    });
    state.rated_queues.push(RatedQueueEntry {
        player_entity,
        character_id,
        bracket: bracket.clone(),
        queued_at: now,
    });
    Ok(format!("queued for {}", bracket_label(&bracket)))
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
            state
                .rated_queues
                .retain(|entry| entry.character_id != character_id);
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
    while let Some(match_id) = form_battleground_invitation(state, battleground_id) {
        let invitation = &state.battleground_matches[&match_id];
        let template = template_by_id(invitation.instance.bg_id).expect("matched template exists");
        let teams = invitation
            .instance
            .team_a_players
            .iter()
            .map(|id| (*id, shared::battleground::BgTeam::A))
            .chain(
                invitation
                    .instance
                    .team_b_players
                    .iter()
                    .map(|id| (*id, shared::battleground::BgTeam::B)),
            );
        for (character_id, team) in teams {
            let Some((conn_entity, player_entity)) =
                connection_for_character(connections, character_id)
            else {
                continue;
            };
            let Ok((_level, _player, mut player_state)) = players.get_mut(player_entity) else {
                continue;
            };
            player_state.queue = None;
            player_state.battleground = Some(shared::protocol::BattlegroundSnapshot {
                match_id,
                battleground_id: template.id,
                name: template.name.clone(),
                map_id: template.map_id,
                team,
                phase: invitation.instance.phase,
                accepted: false,
                invitation_time_left_secs: None,
            });
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

fn resolve_rated_matches(
    state: &mut PvpServerState,
    players: &mut Query<(
        &shared::components::Player,
        &mut PlayerPvpState,
        &mut crate::currency::PlayerCurrencies,
    )>,
    connections: &Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    db: &GameDatabase,
    senders: &mut Query<&mut MessageSender<PvpStateUpdate>>,
    currency_senders: &mut Query<&mut MessageSender<CurrencyStateUpdate>>,
) {
    while let Some((first, second)) = take_next_rated_pair(&mut state.rated_queues) {
        let Ok((first_player, first_state, _first_currencies)) = players.get(first.player_entity)
        else {
            continue;
        };
        let Ok((second_player, second_state, _second_currencies)) =
            players.get(second.player_entity)
        else {
            continue;
        };
        let first_rating = first_state.progress.bracket_rating(&first.bracket);
        let second_rating = second_state.progress.bracket_rating(&second.bracket);
        let (winner, loser) = decide_winner(&first, first_rating, &second, second_rating);
        let winner_name = if winner.character_id == first.character_id {
            first_player.name.clone()
        } else {
            second_player.name.clone()
        };
        let loser_name = if loser.character_id == first.character_id {
            first_player.name.clone()
        } else {
            second_player.name.clone()
        };
        apply_rated_result(
            RatedParticipant {
                entry: winner,
                name: winner_name,
            },
            RatedParticipant {
                entry: loser,
                name: loser_name,
            },
            players,
            connections,
            db,
            senders,
            currency_senders,
        );
    }
}

fn take_next_rated_pair(
    queue: &mut Vec<RatedQueueEntry>,
) -> Option<(RatedQueueEntry, RatedQueueEntry)> {
    let first = queue.first()?.clone();
    let second_index = queue
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, entry)| (entry.bracket == first.bracket).then_some(index))?;
    let second = queue.remove(second_index);
    let first = queue.remove(0);
    Some((first, second))
}

fn decide_winner<'a>(
    first: &'a RatedQueueEntry,
    first_rating: u32,
    second: &'a RatedQueueEntry,
    second_rating: u32,
) -> (&'a RatedQueueEntry, &'a RatedQueueEntry) {
    if first_rating > second_rating {
        return (first, second);
    }
    if second_rating > first_rating {
        return (second, first);
    }
    if first.queued_at < second.queued_at {
        return (first, second);
    }
    if second.queued_at < first.queued_at {
        return (second, first);
    }
    if first.character_id <= second.character_id {
        (first, second)
    } else {
        (second, first)
    }
}

fn apply_rated_result(
    winner: RatedParticipant<'_>,
    loser: RatedParticipant<'_>,
    players: &mut Query<(
        &shared::components::Player,
        &mut PlayerPvpState,
        &mut crate::currency::PlayerCurrencies,
    )>,
    connections: &Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    db: &GameDatabase,
    senders: &mut Query<&mut MessageSender<PvpStateUpdate>>,
    currency_senders: &mut Query<&mut MessageSender<CurrencyStateUpdate>>,
) {
    let winner_old = players
        .get(winner.entry.player_entity)
        .map(|(_, state, _currencies)| state.progress.bracket_rating(&winner.entry.bracket))
        .unwrap_or(DEFAULT_RATING);
    let loser_old = players
        .get(loser.entry.player_entity)
        .map(|(_, state, _currencies)| state.progress.bracket_rating(&loser.entry.bracket))
        .unwrap_or(DEFAULT_RATING);
    let (winner_adj, loser_adj) = calculate_rating(winner_old, loser_old);
    apply_single_rated_result(
        build_rated_reward(
            pvp_support::RatedMatchOutcome::Winner,
            &winner,
            &loser,
            winner_adj.old_rating,
            winner_adj.new_rating,
        ),
        winner.entry,
        players,
        connections,
        db,
        senders,
        currency_senders,
    );
    apply_single_rated_result(
        build_rated_reward(
            pvp_support::RatedMatchOutcome::Loser,
            &loser,
            &winner,
            loser_adj.old_rating,
            loser_adj.new_rating,
        ),
        loser.entry,
        players,
        connections,
        db,
        senders,
        currency_senders,
    );
}

#[cfg(test)]
#[path = "pvp_tests.rs"]
mod tests;
