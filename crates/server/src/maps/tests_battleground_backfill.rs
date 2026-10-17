use super::*;

#[path = "tests_battleground_backfill_release.rs"]
mod release;

#[test]
fn bg_backfill_reserves_real_map_ownership_atomically_including_unaccepted_invitations() {
    let store = map_store().expect("source map store");
    let mut instances = InstanceManager::default();
    let map = instances
        .create_battleground_map(store, 2107, &(1..=30).collect::<Vec<_>>(), 30, at(0.0))
        .unwrap();
    assert!(
        instances.reserve_battleground_players(map, &[31]).is_err(),
        "all thirty unaccepted invitations reserve the full copy"
    );
    assert!(
        instances
            .create_map(store, 2107, entrant(31, None), at(1.0))
            .is_err()
    );
    instances.unassign_battleground_player(map, 30);
    instances.reserve_battleground_players(map, &[31]).unwrap();
    let entry = instances
        .create_map(store, 2107, entrant(31, None), at(2.0))
        .unwrap();
    assert_eq!(entry.map, map);
    assert!(!entry.created);
    instances.unassign_battleground_player(map, 29);
    instances.unassign_battleground_player(map, 28);
    assert!(
        instances
            .reserve_battleground_players(map, &[32, 2])
            .is_err(),
        "ownership conflict refuses the entire group"
    );
    assert!(
        instances
            .create_map(store, 2107, entrant(32, None), at(3.0))
            .is_err(),
        "failed group reservation did not leak its first member"
    );
}

fn spawn_backfill_parties(
    entered: &mut EnteredMatch,
    first: u64,
    count: u64,
    level: u8,
) -> Vec<Traveler> {
    let players: Vec<_> = (first..first + count)
        .map(|id| entered.h.spawn(id, STORMWIND, entered.origin))
        .collect();
    for who in &players {
        entered.h.app.world_mut().entity_mut(who.player).insert((
            crate::pvp::PlayerPvpState::default(),
            crate::currency::PlayerCurrencies::default(),
            shared::components::Level(level),
            shared::components::UnitFactionTemplate(crate::faction::faction_template_for_race(1)),
            shared::components::CurrentPvpFaction(2),
        ));
        entered
            .h
            .app
            .world_mut()
            .entity_mut(who.connection)
            .insert((
                pair::<QueueForBattleground>(),
                pair::<RespondToBattlegroundInvite>(),
                pair::<LeaveBattleground>(),
                pair::<PvpStateUpdate>(),
            ));
    }
    form_source_five_player_parties(&mut entered.h, &players, level);
    players
}

fn queue_and_enter_existing(
    entered: &mut EnteredMatch,
    players: Vec<Traveler>,
    queue_id: u32,
    before_acceptance: impl FnOnce(&mut EnteredMatch),
) {
    for leader in players.iter().step_by(5) {
        entered
            .h
            .app
            .world_mut()
            .get_mut::<MessageSender<QueueForBattleground>>(leader.connection)
            .unwrap()
            .send::<PvpChannel>(QueueForBattleground {
                battleground_id: queue_id,
            });
    }
    enter_existing_invitations(entered, players, queue_id, before_acceptance);
}

fn enter_existing_invitations(
    entered: &mut EnteredMatch,
    players: Vec<Traveler>,
    queue_id: u32,
    before_acceptance: impl FnOnce(&mut EnteredMatch),
) {
    let updates = entered.h.receive_pvp_updates(5);
    let invitations: Vec<_> = players.iter().map(|who| {
        let state = entered.h.app.world().get::<crate::pvp::PlayerPvpState>(who.player).unwrap();
        let invitation = state.battleground.clone().unwrap_or_else(|| panic!("backfill invitation missing: {:?}", updates.get(&who.connection)));
        assert_eq!(invitation.match_id, entered.match_id, "later ordinary parties must fill the existing selected copy before forming a new one");
        assert_eq!(invitation.map_id, entered.copy.map_id);
        invitation
    }).collect();
    before_acceptance(entered);
    for who in &players {
        entered
            .h
            .app
            .world_mut()
            .get_mut::<MessageSender<RespondToBattlegroundInvite>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(RespondToBattlegroundInvite {
                match_id: entered.match_id,
                accepted: true,
            });
    }
    let updates = entered.h.receive_pvp_updates(5);
    for who in &players {
        assert!(
            updates
                .get(&who.connection)
                .into_iter()
                .flatten()
                .all(|update| update.error.is_none()),
            "real late-entry response: {:?}",
            updates.get(&who.connection)
        );
        let destinations = entered.h.new_worlds(who);
        assert_eq!(
            destinations.len(),
            1,
            "late roster member owns a real existing-copy reservation"
        );
        assert_eq!(destinations[0].map_id, entered.copy.map_id);
        entered.h.ack(who);
        assert_eq!(entered.h.map_of(who), Some(entered.copy));
        let character = entered.h.app.world().get::<ClientId>(who.player).unwrap().0;
        assert_eq!(
            crate::pvp::random_bg::tests::original_queue(
                entered.h.app.world(),
                entered.match_id,
                character
            ),
            Some(queue_id)
        );
    }
    entered
        .teams
        .extend(invitations.iter().map(|invitation| invitation.team));
    entered.players.extend(players);
    entered.h.receive_pvp_updates(5);
}

#[test]
fn bg_backfill_waiting_arathi_five_v_five_later_parties_enter_same_twenty_player_copy() {
    let template = shared::battleground::arathi_basin();
    let mut entered = enter_source_match_queued(
        &template,
        node_rules(template.map_id).unwrap().starts,
        |app| {
            app.insert_resource(crate::pvp::random_bg::tests::source_catalog(&[1018]));
        },
        SourceQueueFixture::RandomPartiesOfFive,
    );
    assert_eq!(
        battleground(&entered.h, &entered.players[0]).phase,
        BgPhase::Waiting
    );
    let later = spawn_backfill_parties(&mut entered, 11, 10, 25);
    queue_and_enter_existing(&mut entered, later, 32, |_| {});
    assert_eq!(entered.players.len(), 20);
    for who in &entered.players {
        let snapshot = battleground(&entered.h, who);
        assert_eq!(snapshot.match_id, entered.match_id);
        assert_eq!(snapshot.phase, BgPhase::InProgress);
        assert_eq!(snapshot.players.len(), 20);
        assert_eq!(entered.h.map_of(who), Some(entered.copy));
    }
    assert_eq!(
        entered
            .teams
            .iter()
            .filter(|team| **team == BgTeam::A)
            .count(),
        10
    );
    assert_eq!(
        entered
            .teams
            .iter()
            .filter(|team| **team == BgTeam::B)
            .count(),
        10
    );
    leave_and_return(&mut entered);
}

#[test]
fn bg_backfill_running_arathi_preserves_real_banner_score_and_mixed_queue_origins() {
    let template = shared::battleground::arathi_basin();
    let mut entered = enter_source_match_queued(
        &template,
        node_rules(template.map_id).unwrap().starts,
        |app| {
            app.insert_resource(crate::pvp::random_bg::tests::source_catalog(&[1018]));
        },
        SourceQueueFixture::FourPartiesOfFiveAt25,
    );
    assert_eq!(
        battleground(&entered.h, &entered.players[0]).phase,
        BgPhase::InProgress
    );
    let later = spawn_backfill_parties(&mut entered, 21, 10, 25);
    let point = source_battleground_objects(template.map_id).0.remove(0);
    let (object, _) = objects_in(&mut entered.h, entered.copy, point.info.entry)[0];
    use_second_frames(&mut entered.h);
    entered.h.walk_to(&entered.players[0], point.position);
    use_object(&mut entered.h, &entered.players[0], object);
    entered.h.advance(30);
    let before = battleground(&entered.h, &entered.players[0])
        .players
        .clone();
    let assault_stat = node_rules(template.map_id).unwrap().assault_stat;
    assert!(
        before
            .iter()
            .any(|row| row.objective_scores.contains(&(assault_stat, 1)))
    );
    let rules = node_rules(template.map_id).unwrap();
    let node = rules
        .nodes
        .iter()
        .find(|node| node.entry == point.info.entry)
        .unwrap();
    let actor_team = battleground(&entered.h, &entered.players[0]).team;
    let widget = if actor_team == BgTeam::A {
        node.alliance_widget
    } else {
        node.horde_widget
    };
    assert!(
        battleground(&entered.h, &entered.players[0])
            .world_states
            .contains(&(widget, 1))
    );
    queue_and_enter_existing(&mut entered, later, 32, |entered| {
        entered.h.advance(30);
        assert!(
            battleground(&entered.h, &entered.players[0])
                .world_states
                .contains(&(widget, 2)),
            "real source60s banner capture deadline was not restarted by backfill"
        );
    });
    for who in &entered.players[..20] {
        let character = entered.h.app.world().get::<ClientId>(who.player).unwrap().0;
        assert_eq!(
            crate::pvp::random_bg::tests::original_queue(
                entered.h.app.world(),
                entered.match_id,
                character
            ),
            Some(1018)
        );
    }
    for who in &entered.players {
        let snapshot = battleground(&entered.h, who);
        assert_eq!(snapshot.phase, BgPhase::InProgress);
        assert_eq!(snapshot.players.len(), 30);
        for old in &before {
            assert_eq!(
                snapshot
                    .players
                    .iter()
                    .find(|row| row.character_id == old.character_id),
                Some(old)
            );
        }
    }
    leave_and_return(&mut entered);
}
