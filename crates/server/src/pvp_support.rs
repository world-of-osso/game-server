use bevy::prelude::*;
use lightyear::prelude::*;
use shared::battleground::BgQueueError;
use shared::currency::CurrencyWallet;
use shared::protocol::{
    CurrencyStateUpdate, PvpBracketSnapshot, PvpBracketStatsSnapshot, PvpChannel,
    PvpQueueKindSnapshot, PvpQueueSnapshot, PvpSnapshot, PvpStateUpdate,
};

use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::{self, GameDatabase};
use crate::pvp::{
    CONQUEST_CURRENCY_ID, DEFAULT_RATING, HONOR_CURRENCY_ID, LOSER_CONQUEST, LOSER_HONOR,
    PlayerPvpBracketState, PlayerPvpProgress, PlayerPvpQueue, PlayerPvpState, RatedQueueEntry,
    WINNER_CONQUEST, WINNER_HONOR, sync_progress_currencies_from_wallet,
};

pub(super) fn connection_for_character(
    connections: &Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    character_id: u64,
) -> Option<(Entity, Entity)> {
    connections
        .iter()
        .find_map(|(conn_entity, link, in_world)| {
            (in_world.character_id == character_id).then_some((conn_entity, link.0))
        })
}

pub(super) fn send_pvp_update(
    conn_entity: Entity,
    state: &PlayerPvpState,
    message: Option<String>,
    error: Option<String>,
    senders: &mut Query<&mut MessageSender<PvpStateUpdate>>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<PvpChannel>(PvpStateUpdate {
        snapshot: Some(build_pvp_snapshot(state)),
        message,
        error,
    });
}

pub(super) fn send_pvp_error(
    conn_entity: Entity,
    senders: &mut Query<&mut MessageSender<PvpStateUpdate>>,
    error: impl Into<String>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<PvpChannel>(PvpStateUpdate {
        snapshot: None,
        message: None,
        error: Some(error.into()),
    });
}

pub(super) fn build_pvp_snapshot(state: &PlayerPvpState) -> PvpSnapshot {
    PvpSnapshot {
        battleground: None,
        honor: state.progress.honor,
        honor_max: state.progress.honor_max,
        conquest: state.progress.conquest,
        conquest_max: state.progress.conquest_max,
        brackets: state
            .progress
            .brackets
            .iter()
            .cloned()
            .map(|entry| PvpBracketStatsSnapshot {
                bracket: entry.bracket,
                rating: entry.rating,
                season_wins: entry.season_wins,
                season_losses: entry.season_losses,
                weekly_wins: entry.weekly_wins,
                weekly_losses: entry.weekly_losses,
            })
            .collect(),
        queue: state.queue.as_ref().map(build_queue_snapshot),
        battleground: state.battleground.clone(),
    }
}

fn build_queue_snapshot(queue: &PlayerPvpQueue) -> PvpQueueSnapshot {
    PvpQueueSnapshot {
        kind: match queue {
            PlayerPvpQueue::Battleground {
                battleground_id,
                name,
            } => PvpQueueKindSnapshot::Battleground {
                battleground_id: *battleground_id,
                name: name.clone(),
            },
            PlayerPvpQueue::RatedBracket { bracket } => PvpQueueKindSnapshot::RatedBracket {
                bracket: bracket.clone(),
            },
        },
    }
}

pub(super) fn queue_label(queue: &PlayerPvpQueue) -> String {
    match queue {
        PlayerPvpQueue::Battleground { name, .. } => name.clone(),
        PlayerPvpQueue::RatedBracket { bracket } => bracket_label(bracket).into(),
    }
}

pub(super) fn bracket_label(bracket: &PvpBracketSnapshot) -> &'static str {
    match bracket {
        PvpBracketSnapshot::Arena2v2 => "2v2 arena",
        PvpBracketSnapshot::Arena3v3 => "3v3 arena",
        PvpBracketSnapshot::RatedBattleground => "rated battleground",
        PvpBracketSnapshot::SoloShuffle => "solo shuffle",
    }
}

pub(super) fn format_bg_queue_error(error: BgQueueError) -> String {
    match error {
        BgQueueError::AlreadyQueued => "already queued for battleground".into(),
        BgQueueError::UnknownBg => "unknown battleground".into(),
        BgQueueError::OutOfBracket => "player level is outside this battleground bracket".into(),
        BgQueueError::EmptyGroup => "empty battleground group".into(),
        BgQueueError::LevelCountMismatch => "battleground level count mismatch".into(),
        BgQueueError::DuplicatePlayer => "duplicate battleground player".into(),
        BgQueueError::Deserter => "player has deserter".into(),
    }
}

pub(super) struct RatedParticipant<'a> {
    pub entry: &'a RatedQueueEntry,
    pub name: String,
}

pub(super) enum RatedMatchOutcome {
    Winner,
    Loser,
}

pub(super) struct RatedReward {
    honor: u32,
    conquest: u32,
    new_rating: u32,
    message: String,
    outcome: RatedMatchOutcome,
}

pub(super) fn build_rated_reward(
    outcome: RatedMatchOutcome,
    participant: &RatedParticipant<'_>,
    opponent: &RatedParticipant<'_>,
    old_rating: u32,
    new_rating: u32,
) -> RatedReward {
    let bracket = bracket_label(&participant.entry.bracket);
    match outcome {
        RatedMatchOutcome::Winner => RatedReward {
            honor: WINNER_HONOR,
            conquest: WINNER_CONQUEST,
            new_rating,
            message: format!(
                "{} defeated {} in {} ({} -> {})",
                participant.name, opponent.name, bracket, old_rating, new_rating
            ),
            outcome,
        },
        RatedMatchOutcome::Loser => RatedReward {
            honor: LOSER_HONOR,
            conquest: LOSER_CONQUEST,
            new_rating,
            message: format!(
                "{} lost to {} in {} ({} -> {})",
                participant.name, opponent.name, bracket, old_rating, new_rating
            ),
            outcome,
        },
    }
}

pub(super) fn apply_single_rated_result(
    reward: RatedReward,
    entry: &RatedQueueEntry,
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
    let Ok((_player, mut state, mut currencies)) = players.get_mut(entry.player_entity) else {
        return;
    };

    update_rated_bracket(
        state.progress.bracket_mut(entry.bracket.clone()),
        reward.new_rating,
        reward.outcome,
    );
    apply_rated_currency_rewards(
        &mut state.progress,
        &mut currencies.0,
        reward.honor,
        reward.conquest,
    );
    state.queue = None;
    persist_rated_result(db, entry.character_id, &state.progress, &currencies.0);
    send_rated_result_notifications(
        connections,
        entry.character_id,
        &state,
        &currencies.0,
        reward.message,
        senders,
        currency_senders,
    );
}

fn persist_rated_result(
    db: &GameDatabase,
    character_id: u64,
    progress: &PlayerPvpProgress,
    wallet: &CurrencyWallet,
) {
    persistence::save_pvp(&db.db, character_id, progress);
    persistence::save_currencies(&db.db, character_id, wallet);
}

fn send_rated_result_notifications(
    connections: &Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    character_id: u64,
    state: &PlayerPvpState,
    wallet: &CurrencyWallet,
    message: String,
    senders: &mut Query<&mut MessageSender<PvpStateUpdate>>,
    currency_senders: &mut Query<&mut MessageSender<CurrencyStateUpdate>>,
) {
    send_rated_result_update(connections, character_id, state, senders, message);
    send_rated_currency_update(connections, character_id, wallet, currency_senders);
}

fn update_rated_bracket(
    bracket: &mut PlayerPvpBracketState,
    new_rating: u32,
    outcome: RatedMatchOutcome,
) {
    bracket.rating = new_rating;
    match outcome {
        RatedMatchOutcome::Winner => {
            bracket.season_wins += 1;
            bracket.weekly_wins += 1;
        }
        RatedMatchOutcome::Loser => {
            bracket.season_losses += 1;
            bracket.weekly_losses += 1;
        }
    }
}

pub(super) fn apply_rated_currency_rewards(
    progress: &mut PlayerPvpProgress,
    wallet: &mut CurrencyWallet,
    honor: u32,
    conquest: u32,
) {
    let _ = crate::currency::earn_currency(wallet, HONOR_CURRENCY_ID, honor);
    let _ = crate::currency::earn_currency(wallet, CONQUEST_CURRENCY_ID, conquest);
    sync_progress_currencies_from_wallet(progress, wallet);
}

fn send_rated_result_update(
    connections: &Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    character_id: u64,
    state: &PlayerPvpState,
    senders: &mut Query<&mut MessageSender<PvpStateUpdate>>,
    message: String,
) {
    let Some((conn_entity, _player_entity)) = connection_for_character(connections, character_id)
    else {
        return;
    };
    send_pvp_update(conn_entity, state, Some(message), None, senders);
}

fn send_rated_currency_update(
    connections: &Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    character_id: u64,
    wallet: &CurrencyWallet,
    senders: &mut Query<&mut MessageSender<CurrencyStateUpdate>>,
) {
    let Some((conn_entity, _player_entity)) = connection_for_character(connections, character_id)
    else {
        return;
    };
    crate::currency::send_currency_update(conn_entity, wallet, None, None, senders);
}

pub(super) fn default_brackets() -> Vec<PlayerPvpBracketState> {
    [
        PvpBracketSnapshot::Arena2v2,
        PvpBracketSnapshot::Arena3v3,
        PvpBracketSnapshot::RatedBattleground,
        PvpBracketSnapshot::SoloShuffle,
    ]
    .into_iter()
    .map(|bracket| PlayerPvpBracketState {
        bracket,
        rating: DEFAULT_RATING,
        season_wins: 0,
        season_losses: 0,
        weekly_wins: 0,
        weekly_losses: 0,
    })
    .collect()
}

pub(super) fn current_seconds(time: &Time) -> u64 {
    time.elapsed_secs_f64().floor() as u64
}
