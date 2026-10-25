use super::*;

fn full_arathi() -> EnteredMatch {
    let template = shared::battleground::arathi_basin();
    let mut entered = enter_source_match_queued(
        &template,
        node_rules(template.map_id).unwrap().starts,
        |app| {
            app.insert_resource(crate::pvp::random_bg::tests::source_catalog(&[1018]));
            crate::networking::install_disconnect_fixture(app);
        },
        SourceQueueFixture::FourPartiesOfFiveAt25,
    );
    let later = spawn_backfill_parties(&mut entered, 21, 10, 25);
    queue_and_enter_existing(&mut entered, later, 32, |_| {});
    assert_eq!(entered.players.len(), 30);
    entered
}

fn queue_waiting_five(entered: &mut EnteredMatch) -> Vec<Traveler> {
    let pending = spawn_backfill_parties(entered, 31, 5, 25);
    entered
        .h
        .app
        .world_mut()
        .get_mut::<MessageSender<QueueForBattleground>>(pending[0].connection)
        .unwrap()
        .send::<PvpChannel>(QueueForBattleground {
            battleground_id: 32,
        });
    entered.h.receive_pvp_updates(5);
    assert_still_queued(entered, &pending);
    pending
}

fn decline(entered: &mut EnteredMatch, players: &[Traveler]) {
    for who in players {
        entered
            .h
            .app
            .world_mut()
            .get_mut::<MessageSender<RespondToBattlegroundInvite>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(RespondToBattlegroundInvite {
                match_id: entered.match_id,
                accepted: false,
            });
    }
    entered.h.receive_pvp_updates(5);
}

#[test]
fn bg_backfill_decline_releases_invited_slots_and_refills_without_a_new_queue_request() {
    let template = shared::battleground::arathi_basin();
    let mut entered = enter_source_match_queued(
        &template,
        node_rules(template.map_id).unwrap().starts,
        |app| {
            app.insert_resource(crate::pvp::random_bg::tests::source_catalog(&[1018]));
        },
        SourceQueueFixture::FourPartiesOfFiveAt25,
    );
    let mut invitations = spawn_backfill_parties(&mut entered, 21, 15, 25);
    for leader in invitations.iter().step_by(5) {
        entered
            .h
            .app
            .world_mut()
            .get_mut::<MessageSender<QueueForBattleground>>(leader.connection)
            .unwrap()
            .send::<PvpChannel>(QueueForBattleground {
                battleground_id: 32,
            });
    }
    entered.h.receive_pvp_updates(5);
    let replacement = invitations.split_off(10);
    assert_still_queued(&entered, &replacement);
    for who in &invitations {
        assert_eq!(battleground(&entered.h, who).match_id, entered.match_id);
    }
    let held = invitations.split_off(5);
    decline(&mut entered, &invitations);
    enter_existing_invitations(&mut entered, replacement, 32, |_| {});
    decline(&mut entered, &held);
    assert_eq!(entered.players.len(), 25);
    for who in invitations.iter().chain(&held) {
        assert!(
            entered
                .h
                .app
                .world()
                .get::<crate::pvp::PlayerPvpState>(who.player)
                .unwrap()
                .battleground
                .is_none()
        );
    }
    leave_and_return(&mut entered);
}

fn assert_still_queued(entered: &EnteredMatch, players: &[Traveler]) {
    for who in players {
        let state = entered
            .h
            .app
            .world()
            .get::<crate::pvp::PlayerPvpState>(who.player)
            .unwrap();
        assert!(
            state.battleground.is_none(),
            "full copy does not over-invite"
        );
        assert!(matches!(
            state.queue,
            Some(crate::pvp::PlayerPvpQueue::Battleground {
                battleground_id: 32,
                ..
            })
        ));
    }
}

fn departing_team_five(entered: &EnteredMatch) -> Vec<(usize, Entity, Entity)> {
    entered
        .players
        .iter()
        .zip(&entered.teams)
        .enumerate()
        .filter(|(_, (_, team))| **team == BgTeam::A)
        .take(5)
        .map(|(index, (who, _))| (index, who.player, who.connection))
        .collect()
}

fn remove_departures_from_fixture(
    entered: &mut EnteredMatch,
    departures: &[(usize, Entity, Entity)],
) {
    entered.teams = entered
        .teams
        .iter()
        .enumerate()
        .filter(|(index, _)| !departures.iter().any(|(old, _, _)| old == index))
        .map(|(_, team)| *team)
        .collect();
    entered.players.retain(|who| {
        !departures
            .iter()
            .any(|(_, player, _)| *player == who.player)
    });
}

#[test]
fn bg_backfill_leave_releases_capacity_and_invites_waiting_party_without_a_new_queue_request() {
    let mut entered = full_arathi();
    let pending = queue_waiting_five(&mut entered);
    let departures = departing_team_five(&entered);
    for &(_, _, connection) in &departures {
        entered
            .h
            .app
            .world_mut()
            .get_mut::<MessageSender<LeaveBattleground>>(connection)
            .unwrap()
            .send::<PvpChannel>(LeaveBattleground {
                match_id: entered.match_id,
            });
    }
    entered.h.receive_pvp_updates(5);
    for &(_, player, connection) in &departures {
        let who = Traveler { player, connection };
        assert_eq!(entered.h.new_worlds(&who).len(), 1);
        entered.h.ack(&who);
        assert_eq!(entered.h.map_of(&who), Some(STORMWIND));
    }
    remove_departures_from_fixture(&mut entered, &departures);
    enter_existing_invitations(&mut entered, pending, 32, |_| {});
    assert_eq!(entered.players.len(), 30);
    assert!(
        entered
            .players
            .iter()
            .all(|who| battleground(&entered.h, who).phase == BgPhase::InProgress)
    );
    leave_and_return(&mut entered);
}

#[test]
fn bg_backfill_offline_members_hold_capacity_until_existing_reservation_expiry_event() {
    let mut entered = full_arathi();
    let pending = queue_waiting_five(&mut entered);
    let departures = departing_team_five(&entered);
    use_second_frames(&mut entered.h);
    for &(_, _, connection) in &departures {
        entered
            .h
            .app
            .world_mut()
            .entity_mut(connection)
            .insert(lightyear::prelude::server::ClientOf)
            .remove::<Connected>();
    }
    entered.h.advance(3);
    assert!(
        departures
            .iter()
            .all(|(_, player, _)| entered.h.app.world().get_entity(*player).is_err()),
        "actual disconnect cleanup despawned old members"
    );
    entered.h.advance(280);
    assert_still_queued(&entered, &pending);
    entered.h.advance(20);
    remove_departures_from_fixture(&mut entered, &departures);
    enter_existing_invitations(&mut entered, pending, 32, |_| {});
    assert_eq!(entered.players.len(), 30);
    assert!(
        entered
            .players
            .iter()
            .all(|who| entered.h.map_of(who) == Some(entered.copy))
    );
    leave_and_return(&mut entered);
}
