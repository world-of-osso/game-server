use super::*;
use shared::protocol::{PvpQueueKindSnapshot, PvpQueueSnapshot};

#[test]
fn queue_for_battleground_sets_queue_state() {
    let mut player_state = PlayerPvpState::default();
    let mut state = PvpServerState::default();

    let message = queue_for_battleground(&mut player_state, &mut state, 10, 30, 1, 5)
        .expect("queue succeeds");

    assert_eq!(message, "queued for Warsong Gulch");
    assert!(matches!(
        player_state.queue,
        Some(PlayerPvpQueue::Battleground {
            battleground_id: 1,
            ..
        })
    ));
}

#[test]
fn queue_for_rated_bracket_blocks_double_queue() {
    let mut player_state = PlayerPvpState::default();
    let mut state = PvpServerState::default();

    queue_for_rated_bracket(
        &mut player_state,
        &mut state,
        Entity::PLACEHOLDER,
        44,
        PvpBracketSnapshot::Arena2v2,
        1,
    )
    .expect("first queue succeeds");

    let error = queue_for_rated_bracket(
        &mut player_state,
        &mut state,
        Entity::PLACEHOLDER,
        44,
        PvpBracketSnapshot::Arena3v3,
        2,
    )
    .expect_err("second queue must fail");

    assert!(error.contains("already queued"));
}

#[test]
fn decide_winner_prefers_higher_rating_then_earlier_queue() {
    let first = RatedQueueEntry {
        player_entity: Entity::PLACEHOLDER,
        character_id: 10,
        bracket: PvpBracketSnapshot::Arena2v2,
        queued_at: 5,
    };
    let second = RatedQueueEntry {
        player_entity: Entity::PLACEHOLDER,
        character_id: 20,
        bracket: PvpBracketSnapshot::Arena2v2,
        queued_at: 4,
    };

    let (winner, loser) = decide_winner(&first, 1550, &second, 1500);
    assert_eq!(winner.character_id, 10);
    assert_eq!(loser.character_id, 20);

    let (winner, loser) = decide_winner(&first, 1500, &second, 1500);
    assert_eq!(winner.character_id, 20);
    assert_eq!(loser.character_id, 10);
}

#[test]
fn build_snapshot_reflects_queue_kind() {
    let state = PlayerPvpState {
        progress: PlayerPvpProgress::default(),
        queue: Some(PlayerPvpQueue::RatedBracket {
            bracket: PvpBracketSnapshot::SoloShuffle,
        }),
    };

    let snapshot = build_pvp_snapshot(&state);

    assert_eq!(snapshot.brackets.len(), 4);
    assert!(matches!(
        snapshot.queue,
        Some(PvpQueueSnapshot {
            kind: PvpQueueKindSnapshot::RatedBracket {
                bracket: PvpBracketSnapshot::SoloShuffle
            }
        })
    ));
}
