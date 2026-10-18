use super::*;
use shared::components::{Level, UnitLevel};

#[test]
fn bg_bracket_port_concrete_refuses_xp_level_up_beyond_retained_maximum() {
    assert_overlevel_port_refused(1018);
}

#[test]
fn bg_bracket_port_random_refuses_xp_level_up_beyond_retained_maximum() {
    assert_overlevel_port_refused(32);
}

#[test]
fn bg_bracket_port_within_bracket_xp_level_up_still_enters() {
    let (mut h, players, _database) = pending_battleground_at_level(1018, 28);
    let held = &players[0];
    let invitation = battleground(&h, held).clone();
    let copy = enter_supports(&mut h, &players[1..], invitation.match_id);
    award_one_level(&mut h, held, 28);
    assert_port_enters(&mut h, held, invitation.match_id, copy.map_id);
    h.ack(held);
    assert_eq!(h.map_of(held), Some(copy));
}

#[test]
fn bg_bracket_port_arena_xp_level_up_still_enters() {
    let (mut h, players, _database) = pending_arena(29);
    let held = &players[0];
    let invitation = battleground(&h, held).clone();
    assert!(invitation.arena.is_some());
    award_one_level(&mut h, held, 29);
    assert_port_enters(&mut h, held, invitation.match_id, invitation.map_id);
}

fn award_one_level(h: &mut Harness, who: &Traveler, previous_level: u8) {
    use crate::experience::{XpAward, XpAwards, xp_to_next_level};
    let character_id = h.app.world().get::<ClientId>(who.player).unwrap().0;
    assert_eq!(
        h.app.world().get::<Level>(who.player).unwrap().0,
        previous_level
    );
    h.app
        .world_mut()
        .entity_mut(who.player)
        .insert(UnitLevel(previous_level));
    h.app
        .world_mut()
        .resource_mut::<XpAwards>()
        .0
        .push(XpAward::quest(
            character_id,
            xp_to_next_level(previous_level),
        ));
    h.advance(1);
    assert_eq!(
        h.app.world().get::<Level>(who.player).unwrap().0,
        previous_level + 1
    );
    assert_eq!(
        h.app.world().get::<UnitLevel>(who.player).unwrap().0,
        previous_level + 1
    );
    let db = &h
        .app
        .world()
        .resource::<crate::persistence::GameDatabase>()
        .db;
    assert_eq!(crate::persistence_progression::load_xp(db, character_id), 0);
}

fn assert_port_enters(h: &mut Harness, who: &Traveler, match_id: u64, map_id: u32) {
    let updates = request_port(h, who, match_id);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "allowed port: {updates:?}"
    );
    assert!(battleground(h, who).accepted);
    let destinations = h.new_worlds(who);
    assert_eq!(destinations.len(), 1);
    assert_eq!(destinations[0].map_id, map_id);
}

fn assert_overlevel_port_refused(queue_id: u32) {
    let (mut h, players, _database) = pending_battleground_at_level(queue_id, 29);
    let held = &players[0];
    let character = h.app.world().get::<ClientId>(held.player).unwrap().0;
    let invitation = battleground(&h, held).clone();
    let bracket = shared::battleground::source_level_bracket(invitation.map_id, 29).unwrap();
    assert_eq!((bracket.min_level, bracket.max_level), (20, 29));
    let supports = &players[1..];
    let copy = enter_supports(&mut h, supports, invitation.match_id);
    assert_eq!(
        h.app.world().resource::<InstanceManager>().cannot_enter(
            copy,
            entrant(character, None),
            SUNDAY_NOON
        ),
        None,
        "pending invitation reserves the actual copy"
    );
    assert!(h.new_worlds(held).is_empty());
    award_one_level(&mut h, held, 29);
    assert_eq!(battleground(&h, held).match_id, invitation.match_id);
    assert!(!battleground(&h, held).accepted);
    let updates = request_port(&mut h, held, invitation.match_id);
    assert!(
        h.new_worlds(held).is_empty(),
        "level30 must not port into retained20-29: {updates:?}"
    );
    assert_eq!(h.map_of(held), Some(STORMWIND));
    assert_not_queued(&h, std::slice::from_ref(held));
    assert_eq!(
        h.app.world().resource::<InstanceManager>().cannot_enter(
            copy,
            entrant(character, None),
            SUNDAY_NOON
        ),
        Some(TransferAbortReason::MapNotAllowed),
        "normal decline releases reservation"
    );
    assert!(supports.iter().all(|who| h.map_of(who) == Some(copy)));
    assert!(
        supports
            .iter()
            .all(|who| battleground(&h, who).phase == shared::battleground::BgPhase::InProgress)
    );
}
