//! Queue Again exists only for a finished skirmish: invitations and live copies refuse it.

use super::*;
use shared::battleground::BgPhase;
use shared::protocol::RequeueSkirmish;

#[test]
fn skirmish_requeue_refuses_unaccepted_invitation() {
    let (mut h, players, _database) = pending_arena(25);
    let who = &players[0];
    assert!(!battleground(&h, who).accepted);
    assert_refused(&mut h, who, BgPhase::Waiting);
}

#[test]
fn skirmish_requeue_refuses_waiting_copy() {
    let (mut h, players, _database) = pending_arena(25);
    enter(&mut h, &players[..1]);
    assert_refused(&mut h, &players[0], BgPhase::Waiting);
}

#[test]
fn skirmish_requeue_refuses_in_progress_copy() {
    let (mut h, players, _database) = pending_arena(25);
    enter(&mut h, &players);
    use_second_frames(&mut h);
    h.advance(62);
    assert_refused(&mut h, &players[0], BgPhase::InProgress);
}

fn enter(h: &mut Harness, players: &[Traveler]) {
    for who in players {
        let invitation = battleground(h, who).clone();
        let updates = request_port(h, who, invitation.match_id);
        assert!(
            updates.iter().all(|update| update.error.is_none()),
            "{updates:?}"
        );
        assert_eq!(h.new_worlds(who).len(), 1);
        h.ack(who);
        assert_eq!(h.map_of(who).unwrap().map_id, invitation.map_id);
    }
}

fn assert_refused(h: &mut Harness, who: &Traveler, phase: BgPhase) {
    let before = battleground(h, who).clone();
    assert_eq!(before.phase, phase);
    let map = h.map_of(who);
    h.app
        .world_mut()
        .entity_mut(who.connection)
        .insert(pair::<RequeueSkirmish>());
    h.app
        .world_mut()
        .get_mut::<MessageSender<RequeueSkirmish>>(who.connection)
        .unwrap()
        .send::<PvpChannel>(RequeueSkirmish {
            match_id: before.match_id,
        });
    let updates = h.receive_pvp_updates(5).remove(&who.connection).unwrap();
    assert!(updates.iter().all(|u| u.message.is_none()), "{updates:?}");
    assert_eq!(
        updates.iter().filter(|u| u.error.is_some()).count(),
        1,
        "explicit refusal: {updates:?}"
    );
    assert!(h.new_worlds(who).is_empty(), "no return transfer");
    assert_eq!(h.map_of(who), map);
    assert_eq!(battleground(h, who).match_id, before.match_id);
    let state = h
        .app
        .world()
        .get::<crate::pvp::PlayerPvpState>(who.player)
        .unwrap();
    assert!(state.queue.is_none());
}
