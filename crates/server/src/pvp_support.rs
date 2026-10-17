use bevy::prelude::*;
use lightyear::prelude::*;
use shared::battleground::BgQueueError;
use shared::protocol::{
    PvpBracketSnapshot, PvpBracketStatsSnapshot, PvpChannel, PvpQueueKindSnapshot,
    PvpQueueSnapshot, PvpSnapshot, PvpStateUpdate,
};

use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::pvp::{PlayerPvpBracketState, PlayerPvpQueue, PlayerPvpState};

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
        lifetime_honor_xp: state.progress.lifetime_honor_xp,
        honor_level: state.progress.honor_level,
        honor: state.progress.honor,
        honor_max: state.progress.honor_max,
        conquest: state.progress.conquest,
        // Old stored caps/ratings came from the retired simulation, not a source policy.
        conquest_max: None,
        brackets: state
            .progress
            .brackets
            .iter()
            .cloned()
            .map(|entry| PvpBracketStatsSnapshot {
                bracket: entry.bracket,
                rating: entry.matchmaker_rating.and(entry.rating),
                season_wins: entry.season_wins,
                season_losses: entry.season_losses,
                weekly_wins: entry.weekly_wins,
                weekly_losses: entry.weekly_losses,
            })
            .collect(),
        queue: state.queue.as_ref().map(build_queue_snapshot),
        battleground: state.battleground.clone(),
        war_mode: state.war_mode.clone(),
        talents: state.talents.clone(),
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
            PlayerPvpQueue::Arena { mode } => PvpQueueKindSnapshot::Arena { mode: *mode },
            PlayerPvpQueue::RatedBracket { bracket } => PvpQueueKindSnapshot::RatedBracket {
                bracket: bracket.clone(),
            },
        },
    }
}

pub(super) fn queue_label(queue: &PlayerPvpQueue) -> String {
    match queue {
        PlayerPvpQueue::Battleground { name, .. } => name.clone(),
        PlayerPvpQueue::Arena { mode } => format!("{mode:?}"),
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
        rating: None,
        matchmaker_rating: None,
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
