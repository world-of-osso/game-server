use super::*;
use crate::pvp::pvp_support::{apply_rated_currency_rewards, build_pvp_snapshot};
use shared::currency::{CurrencyWallet, currency_by_id};
use shared::protocol::{PvpQueueKindSnapshot, PvpQueueSnapshot};

#[test]
fn queue_for_battleground_sets_queue_state() {
    let mut player_state = PlayerPvpState::default();
    let mut state = PvpServerState::default();

    let message = queue_for_battleground(&mut player_state, &mut state, 10, 90, 1014, 5)
        .expect("queue succeeds");

    assert_eq!(message, "queued for Warsong Gulch");
    assert!(matches!(
        player_state.queue,
        Some(PlayerPvpQueue::Battleground {
            battleground_id: 1014,
            ..
        })
    ));
}

/// Map.csv (12.1.0.69933), map 529: "Earn 1500 resources".
/// Exercises multiple players through the server queue and shared end condition;
/// it does not claim transport, invitations, or an instantiated map are wired.
#[test]
fn arathi_multiple_players_finish_at_authored_resource_limit() {
    use shared::battleground::{BgInstance, BgOutcome, BgPhase, BgTeam, arathi_basin};

    let template = arathi_basin();
    let mut state = PvpServerState::default();
    let player_count = u64::from(template.min_players_per_team) * 2;
    let mut players = vec![PlayerPvpState::default(); player_count as usize];
    for (index, player) in players.iter_mut().enumerate() {
        let character_id = index as u64 + 1;
        queue_for_battleground(player, &mut state, character_id, 30, template.id, 0)
            .expect("eligible player queues");
    }

    let matched = state.battleground_queue.try_match(template.id).unwrap();
    let mut participants: Vec<_> = matched
        .team_a
        .iter()
        .chain(&matched.team_b)
        .copied()
        .collect();
    participants.sort_unstable();
    assert_eq!(participants, (1..=player_count).collect::<Vec<_>>());
    assert!(state.battleground_queue.is_empty());

    let mut instance = BgInstance::from_match(&matched, &template);
    assert!(instance.start());
    instance.add_score(BgTeam::A, 1490, &template);
    assert_eq!(instance.phase, BgPhase::InProgress);
    instance.add_score(BgTeam::A, 10, &template);
    assert_eq!(instance.phase, BgPhase::Ended);
    assert_eq!(instance.outcome, Some(BgOutcome::Winner(BgTeam::A)));
    instance.add_score(BgTeam::B, 1500, &template);
    assert_eq!(instance.score_b.score, 0);
    assert_eq!(instance.outcome, Some(BgOutcome::Winner(BgTeam::A)));
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

#[test]
fn sync_progress_currencies_from_wallet_uses_wallet_amounts() {
    let mut progress = PlayerPvpProgress::default();
    let mut wallet = CurrencyWallet::default();
    wallet.add(1, 275, currency_by_id(1).unwrap()).unwrap();
    wallet.add(2, 120, currency_by_id(2).unwrap()).unwrap();

    let changed = sync_progress_currencies_from_wallet(&mut progress, &wallet);

    assert!(changed);
    assert_eq!(progress.honor, 275);
    assert_eq!(progress.conquest, 120);
}

#[test]
fn apply_rated_currency_rewards_updates_wallet_and_progress() {
    let mut progress = PlayerPvpProgress::default();
    let mut wallet = CurrencyWallet::default();

    apply_rated_currency_rewards(&mut progress, &mut wallet, 150, 100);

    assert_eq!(wallet.get(1), 150);
    assert_eq!(wallet.get(2), 100);
    assert_eq!(progress.honor, 150);
    assert_eq!(progress.conquest, 100);
}
