use super::*;
use shared::battleground::BgPhase;
use shared::components::CombatStatus;

#[test]
fn pvp_leave_combat_concrete_refuses_until_combat_stops() {
    let (mut h, players, _database) = pending_battleground(1018);
    enter_all(&mut h, &players);
    assert_active_combat_leave_refused(&mut h, &players);
}

#[test]
fn pvp_leave_combat_random_refuses_until_combat_stops() {
    let (mut h, players, _database) = pending_battleground(32);
    enter_all(&mut h, &players);
    assert_active_combat_leave_refused(&mut h, &players);
}

#[test]
fn pvp_leave_combat_arena_refuses_until_combat_stops() {
    let (mut h, players, _database) = pending_arena(25);
    enter_all(&mut h, &players);
    use_second_frames(&mut h);
    h.advance(65);
    assert_active_combat_leave_refused(&mut h, &players);
}

#[test]
fn pvp_leave_combat_ended_match_allows_return_despite_combat() {
    let template = shared::battleground::arathi_basin();
    let mut entered = enter_source_match(&template, node_rules(template.map_id).unwrap().starts);
    let held = &entered.players[0];
    let h = &mut entered.h;
    // Produce an actual source-node victory through the ordinary interaction harness.
    for point in source_battleground_objects(entered.copy.map_id).0 {
        let (object, at) = objects_in(h, entered.copy, point.info.entry)[0];
        h.walk_to(held, at);
        use_object(h, held, object);
    }
    use_second_frames(h);
    h.advance(180);
    assert_eq!(battleground(h, held).phase, BgPhase::Ended);
    let opponent = find_opponent(h, &entered.players);
    land_nonlethal_hit(h, held, opponent);
    assert_return_to_origin(h, held, entered.match_id);
    assert!(
        entered.players[1..]
            .iter()
            .all(|who| h.map_of(who) == Some(entered.copy))
    );
}

fn enter_all(h: &mut Harness, players: &[Traveler]) {
    for who in players {
        h.app
            .world_mut()
            .entity_mut(who.connection)
            .insert(pair::<LeaveBattleground>());
        let invitation = battleground(h, who).clone();
        let updates = request_port(h, who, invitation.match_id);
        assert!(
            updates.iter().all(|update| update.error.is_none()),
            "entry: {updates:?}"
        );
        let destinations = h.new_worlds(who);
        assert_eq!(destinations.len(), 1);
        assert_eq!(destinations[0].map_id, invitation.map_id);
        h.ack(who);
    }
    let copy = h.map_of(&players[0]).unwrap();
    assert!(players.iter().all(|who| h.map_of(who) == Some(copy)));
}

fn find_opponent<'a>(h: &Harness, players: &'a [Traveler]) -> &'a Traveler {
    let team = battleground(h, &players[0]).team;
    players
        .iter()
        .find(|who| battleground(h, who).team != team)
        .expect("assigned opposing team")
}

fn land_nonlethal_hit(h: &mut Harness, held: &Traveler, opponent: &Traveler) {
    assert_ne!(battleground(h, held).team, battleground(h, opponent).team);
    let before = h.app.world().get::<Health>(held.player).unwrap().current;
    crate::combat::deal_direct_damage(h.app.world_mut(), opponent.player, held.player, 10.0);
    let after = h.app.world().get::<Health>(held.player).unwrap().current;
    assert!(after > 0.0 && after < before, "real nonlethal damage");
    assert_eq!(
        h.app.world().get::<CombatStatus>(held.player),
        Some(&CombatStatus(true))
    );
    assert_eq!(
        h.app.world().get::<CombatStatus>(opponent.player),
        Some(&CombatStatus(true))
    );
}

fn request_leave(h: &mut Harness, who: &Traveler, match_id: u64) -> Vec<PvpStateUpdate> {
    h.app
        .world_mut()
        .get_mut::<MessageSender<LeaveBattleground>>(who.connection)
        .unwrap()
        .send::<PvpChannel>(LeaveBattleground { match_id });
    h.receive_pvp_updates(5)
        .remove(&who.connection)
        .expect("normal leave response")
}

fn assert_active_combat_leave_refused(h: &mut Harness, players: &[Traveler]) {
    let held = &players[0];
    let opponent = find_opponent(h, players);
    let invitation = battleground(h, held).clone();
    assert_eq!(invitation.phase, BgPhase::InProgress);
    let copy = h.map_of(held).unwrap();
    let character = h.app.world().get::<ClientId>(held.player).unwrap().0;
    land_nonlethal_hit(h, held, opponent);
    let updates = request_leave(h, held, invitation.match_id);
    assert!(
        h.new_worlds(held).is_empty(),
        "combat must refuse battlefield leave: {updates:?}"
    );
    assert_eq!(h.map_of(held), Some(copy));
    let retained = battleground(h, held);
    assert_eq!(retained.match_id, invitation.match_id);
    assert!(retained.accepted);
    assert_eq!(retained.phase, BgPhase::InProgress);
    assert_eq!(
        h.app.world().resource::<InstanceManager>().cannot_enter(
            copy,
            entrant(character, None),
            SUNDAY_NOON
        ),
        None,
        "refusal retains actual copy ownership"
    );
    assert!(players.iter().all(|who| h.map_of(who) == Some(copy)));
    crate::combat::combat_stop(h.app.world_mut(), held.player);
    assert_eq!(
        h.app.world().get::<CombatStatus>(held.player),
        Some(&CombatStatus(false))
    );
    assert_return_to_origin(h, held, invitation.match_id);
    assert!(players[1..].iter().all(|who| h.map_of(who) == Some(copy)));
}

fn assert_return_to_origin(h: &mut Harness, who: &Traveler, match_id: u64) {
    let updates = request_leave(h, who, match_id);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "allowed leave: {updates:?}"
    );
    let destinations = h.new_worlds(who);
    assert_eq!(destinations.len(), 1);
    assert_eq!(destinations[0].map_id, STORMWIND.map_id);
    h.ack(who);
    assert_eq!(h.map_of(who), Some(STORMWIND));
    assert!(
        h.app
            .world()
            .get::<crate::pvp::PlayerPvpState>(who.player)
            .unwrap()
            .battleground
            .is_none()
    );
}
