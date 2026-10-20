//! Project policy SKIRMISH_REQUEUE_LEAVE_THEN_QUEUE_SAME_MODE: one match-bound request
//! leaves through the ordinary return, then the successful origin ACK queues the played
//! skirmish mode. Retail proves only the skirmish-only button and its immediate disable
//! (PVPMatchResults.lua152–159/490–494); SKIRMISH_REQUEUE_TRANSITION_UNQUALIFIED remains.

use super::*;
use crate::pvp::{PlayerPvpQueue, PlayerPvpState};
use shared::battleground::BgPhase;
use shared::protocol::{ArenaQueueMode, RequeueSkirmish};

const RETURNED: &str = "returned to battleground origin";
const QUEUED: &str = "queued for arena skirmish";

#[test]
#[ignore = "RED: skirmish Queue Again handler not implemented"]
fn skirmish_requeue_2v2_returns_then_queues_same_mode_after_origin_ack() {
    exercise_arena_channel_with_completion(
        ArenaQueueMode::Skirmish2v2,
        false,
        572,
        None,
        Some(requeue_every_player),
    );
}

#[test]
#[ignore = "RED: skirmish Queue Again handler not implemented"]
fn skirmish_requeue_3v3_returns_then_queues_same_mode_after_origin_ack() {
    exercise_arena_channel_with_completion(
        ArenaQueueMode::Skirmish3v3,
        false,
        572,
        None,
        Some(requeue_every_player),
    );
}

#[test]
#[ignore = "RED: skirmish Queue Again handler not implemented"]
fn skirmish_requeue_queue_failure_after_return_stays_at_origin() {
    exercise_arena_channel_with_completion(
        ArenaQueueMode::Skirmish2v2,
        false,
        572,
        None,
        Some(requeue_then_fail_queue),
    );
}

#[test]
#[ignore = "RED: skirmish Queue Again handler not implemented"]
fn skirmish_requeue_refuses_rated_arena() {
    exercise_arena_channel_with_completion(
        ArenaQueueMode::Rated2v2,
        false,
        572,
        None,
        Some(assert_requeue_refused),
    );
}

#[test]
#[ignore = "RED: skirmish Queue Again handler not implemented"]
fn skirmish_requeue_refuses_solo_shuffle() {
    exercise_arena_channel_with_completion(
        ArenaQueueMode::SoloShuffle,
        false,
        572,
        None,
        Some(assert_requeue_refused),
    );
}

#[test]
#[ignore = "RED: skirmish Queue Again handler not implemented"]
fn skirmish_requeue_refuses_ordinary_battleground() {
    flag_match_with_completion(
        shared::battleground::warsong_gulch(),
        "warsonggulch2",
        None,
        Some(assert_requeue_refused),
    );
}

fn send_requeue(h: &mut Harness, who: &Traveler, match_id: u64) {
    if h.app
        .world()
        .get::<MessageSender<RequeueSkirmish>>(who.connection)
        .is_none()
    {
        h.app
            .world_mut()
            .entity_mut(who.connection)
            .insert(pair::<RequeueSkirmish>());
    }
    h.app
        .world_mut()
        .get_mut::<MessageSender<RequeueSkirmish>>(who.connection)
        .unwrap()
        .send::<PvpChannel>(RequeueSkirmish { match_id });
}

fn pvp<'a>(h: &'a Harness, who: &Traveler) -> &'a PlayerPvpState {
    h.app.world().get::<PlayerPvpState>(who.player).unwrap()
}

fn messages(updates: &[PvpStateUpdate]) -> Vec<&str> {
    updates
        .iter()
        .filter_map(|u| u.message.as_deref())
        .collect()
}

fn errors(updates: &[PvpStateUpdate]) -> Vec<&str> {
    updates.iter().filter_map(|u| u.error.as_deref()).collect()
}

fn ack_with_updates(h: &mut Harness, who: &Traveler) -> Vec<PvpStateUpdate> {
    h.app
        .world_mut()
        .get_mut::<MessageSender<WorldPortAck>>(who.connection)
        .unwrap()
        .send::<TransferChannel>(WorldPortAck);
    h.receive_pvp_updates(3)
        .remove(&who.connection)
        .unwrap_or_default()
}

fn played_mode(h: &Harness, who: &Traveler, match_id: u64) -> ArenaQueueMode {
    let snapshot = pvp(h, who).battleground.as_ref().unwrap();
    assert_eq!(snapshot.match_id, match_id);
    assert_eq!(snapshot.phase, BgPhase::Ended);
    assert!(snapshot.accepted);
    snapshot.arena.as_ref().unwrap().mode
}

/// Request from the finished copy: ordinary return, no queue until the origin ACK.
fn request_and_return(h: &mut Harness, players: &[Traveler], match_id: u64) {
    for who in players {
        send_requeue(h, who, match_id);
    }
    // A repeated click in the same frame must not leave or queue twice.
    send_requeue(h, &players[0], match_id);
    let updates = h.receive_pvp_updates(5);
    for (index, who) in players.iter().enumerate() {
        let received = &updates[&who.connection];
        assert_eq!(
            messages(received),
            [RETURNED],
            "ordinary leave: {received:?}"
        );
        assert_eq!(
            errors(received).len(),
            usize::from(index == 0),
            "{received:?}"
        );
        let state = pvp(h, who);
        assert!(
            state.battleground.is_none(),
            "membership released on return"
        );
        assert!(state.queue.is_none(), "no queue before origin ACK");
        let destinations = h.new_worlds(who);
        assert_eq!(destinations.len(), 1, "one return transfer");
        assert_eq!(destinations[0].map_id, STORMWIND.map_id);
        assert_eq!(h.map_of(who), None, "left the copy, awaiting origin ACK");
    }
}

fn requeue_every_player(h: &mut Harness, players: &[Traveler], origins: &[Vec3], match_id: u64) {
    let mode = played_mode(h, &players[0], match_id);
    // A stale identity is refused without leaving the finished copy.
    send_requeue(h, &players[0], match_id + 1000);
    let stale = h
        .receive_pvp_updates(5)
        .remove(&players[0].connection)
        .unwrap();
    assert_eq!(errors(&stale).len(), 1, "stale refusal: {stale:?}");
    assert!(h.new_worlds(&players[0]).is_empty());
    assert_eq!(
        pvp(h, &players[0]).battleground.as_ref().unwrap().match_id,
        match_id
    );
    request_and_return(h, players, match_id);
    let (last, waiting) = players.split_last().unwrap();
    for (who, origin) in waiting.iter().zip(origins) {
        let updates = ack_with_updates(h, who);
        assert_eq!(messages(&updates), [QUEUED], "{updates:?}");
        assert_eq!(h.map_of(who), Some(STORMWIND));
        assert!(close(h.position(who), *origin));
        assert_eq!(pvp(h, who).queue, Some(PlayerPvpQueue::Arena { mode }));
    }
    let updates = ack_with_updates(h, last);
    assert_eq!(messages(&updates)[0], QUEUED, "{updates:?}");
    for (who, origin) in players.iter().zip(origins) {
        assert_eq!(h.map_of(who), Some(STORMWIND));
        assert!(close(h.position(who), *origin));
        let state = pvp(h, who);
        let invitation = state.battleground.as_ref().expect("fresh invitation");
        assert_ne!(invitation.match_id, match_id);
        assert!(!invitation.accepted);
        assert_eq!(invitation.arena.as_ref().unwrap().mode, mode);
        assert!(state.queue.is_none());
    }
}

fn requeue_then_fail_queue(h: &mut Harness, players: &[Traveler], origins: &[Vec3], match_id: u64) {
    played_mode(h, &players[0], match_id);
    let who = &players[0];
    request_and_return(h, &players[..1], match_id);
    // Below the source skirmish floor once back: the ordinary queue refusal applies.
    h.app
        .world_mut()
        .entity_mut(who.player)
        .insert(shared::components::Level(1));
    let updates = ack_with_updates(h, who);
    assert!(messages(&updates).is_empty(), "{updates:?}");
    assert_eq!(
        errors(&updates),
        ["no source-backed arena supports this level"]
    );
    assert_eq!(h.map_of(who), Some(STORMWIND));
    assert!(close(h.position(who), origins[0]));
    let state = pvp(h, who);
    assert!(state.queue.is_none() && state.battleground.is_none());
    // A later unrelated arrival must not replay the consumed request.
    assert!(h.new_worlds(who).is_empty());
}

fn assert_requeue_refused(h: &mut Harness, players: &[Traveler], _: &[Vec3], match_id: u64) {
    let who = &players[0];
    let map = h.map_of(who);
    assert_eq!(
        pvp(h, who).battleground.as_ref().unwrap().phase,
        BgPhase::Ended
    );
    send_requeue(h, who, match_id);
    let updates = h.receive_pvp_updates(5).remove(&who.connection).unwrap();
    assert!(messages(&updates).is_empty(), "{updates:?}");
    assert_eq!(errors(&updates).len(), 1, "explicit refusal: {updates:?}");
    assert!(h.new_worlds(who).is_empty(), "no return transfer");
    assert_eq!(h.map_of(who), map);
    let state = pvp(h, who);
    assert_eq!(state.battleground.as_ref().unwrap().match_id, match_id);
    assert!(state.queue.is_none());
}
