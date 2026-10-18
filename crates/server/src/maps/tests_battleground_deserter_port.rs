use super::*;

#[test]
fn bg_deserter_port_concrete_refuses_late_aura_and_releases_reservation() {
    assert_late_deserter_port_refusal(1018);
}

#[test]
fn bg_deserter_port_random_refuses_late_aura_and_releases_reservation() {
    assert_late_deserter_port_refusal(32);
}

#[test]
fn bg_deserter_port_arena_accepts_with_late_deserter_unchanged() {
    use shared::protocol::{ArenaQueueMode, QueueForArena};
    let (mut h, players, _database) = deserter_harness(4);
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let arenas = crate::pvp::arena::Catalog::load(&db).unwrap();
    h.app.insert_resource(arenas);
    h.app
        .insert_resource(crate::game_objects::GameObjectSpawns::default());
    for who in &players {
        h.app.world_mut().entity_mut(who.connection).insert((
            pair::<QueueForArena>(),
            pair::<RespondToBattlegroundInvite>(),
        ));
        h.app
            .world_mut()
            .get_mut::<MessageSender<QueueForArena>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(QueueForArena {
                mode: ArenaQueueMode::Skirmish2v2,
            });
    }
    h.receive_pvp_updates(5);
    let held = &players[0];
    let invitation = battleground(&h, held).clone();
    assert!(invitation.arena.is_some() && !invitation.accepted);
    apply_sourced_aura(&mut h, held, DESERTER_SPELL_ID);
    let updates = request_port(&mut h, held, invitation.match_id);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "arena control: {updates:?}"
    );
    assert!(battleground(&h, held).accepted);
    let destinations = h.new_worlds(held);
    assert_eq!(destinations.len(), 1);
    assert_eq!(destinations[0].map_id, invitation.map_id);
}

fn pending_battleground(queue_id: u32) -> (Harness, Vec<Traveler>, tempfile::NamedTempFile) {
    let (mut h, players, database) = deserter_harness(20);
    h.app.insert_resource(source_battleground_objects(2107));
    h.app
        .insert_resource(crate::faction::FactionTemplates::default());
    for who in &players {
        h.app.world_mut().entity_mut(who.player).insert((
            shared::components::UnitFactionTemplate(crate::faction::faction_template_for_race(1)),
            shared::components::CurrentPvpFaction(2),
        ));
        h.app
            .world_mut()
            .entity_mut(who.connection)
            .insert(pair::<RespondToBattlegroundInvite>());
        h.app
            .world_mut()
            .get_mut::<MessageSender<QueueForBattleground>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(QueueForBattleground {
                battleground_id: queue_id,
            });
    }
    h.receive_pvp_updates(5);
    let match_id = battleground(&h, &players[0]).match_id;
    for who in &players {
        let invitation = battleground(&h, who);
        assert_eq!(invitation.match_id, match_id);
        assert_eq!(invitation.map_id, 2107);
        assert!(!invitation.accepted);
    }
    (h, players, database)
}

fn enter_supports(h: &mut Harness, supports: &[Traveler], match_id: u64) -> InMap {
    for who in supports {
        h.app
            .world_mut()
            .get_mut::<MessageSender<RespondToBattlegroundInvite>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(RespondToBattlegroundInvite {
                match_id,
                accepted: true,
            });
    }
    let updates = h.receive_pvp_updates(5);
    for who in supports {
        assert!(
            updates[&who.connection]
                .iter()
                .all(|update| update.error.is_none())
        );
        let destinations = h.new_worlds(who);
        assert_eq!(destinations.len(), 1, "ordinary support transfer");
        assert_eq!(destinations[0].map_id, 2107);
        h.ack(who);
    }
    let copy = h.map_of(&supports[0]).unwrap();
    assert!(supports.iter().all(|who| h.map_of(who) == Some(copy)));
    assert!(
        supports
            .iter()
            .all(|who| battleground(h, who).phase == shared::battleground::BgPhase::InProgress)
    );
    copy
}

fn request_port(h: &mut Harness, who: &Traveler, match_id: u64) -> Vec<PvpStateUpdate> {
    h.app
        .world_mut()
        .get_mut::<MessageSender<RespondToBattlegroundInvite>>(who.connection)
        .unwrap()
        .send::<PvpChannel>(RespondToBattlegroundInvite {
            match_id,
            accepted: true,
        });
    h.receive_pvp_updates(5)
        .remove(&who.connection)
        .expect("normal invitation response")
}

fn assert_late_deserter_port_refusal(queue_id: u32) {
    let (mut h, players, _database) = pending_battleground(queue_id);
    let held = &players[0];
    let character = h.app.world().get::<ClientId>(held.player).unwrap().0;
    let invitation = battleground(&h, held).clone();
    let copy = enter_supports(&mut h, &players[1..], invitation.match_id);
    assert!(
        h.new_worlds(held).is_empty(),
        "held player was not accepted"
    );
    assert_eq!(h.map_of(held), Some(STORMWIND));
    assert_eq!(
        h.app.world().resource::<InstanceManager>().cannot_enter(
            copy,
            entrant(character, None),
            SUNDAY_NOON
        ),
        None,
        "the unaccepted invitation owns a real copy reservation"
    );
    apply_sourced_aura(&mut h, held, DESERTER_SPELL_ID);
    let updates = request_port(&mut h, held, invitation.match_id);
    assert!(
        updates.iter().any(|update| update
            .error
            .as_deref()
            .is_some_and(|error| error.contains("deserter"))),
        "post-invite Deserter must refuse the actual port: {updates:?}"
    );
    assert!(
        h.new_worlds(held).is_empty(),
        "refused Deserter must never receive NewWorld"
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
        "normal decline must release the real copy reservation"
    );
    assert!(players[1..].iter().all(|who| h.map_of(who) == Some(copy)));
    clear_deserter(&mut h, held);
    let updates = request_queue(&mut h, held, queue_id);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "clear-aura requeue: {updates:?}"
    );
    assert_eq!(battleground(&h, held).match_id, invitation.match_id);
    let updates = request_port(&mut h, held, invitation.match_id);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "clear-aura accept: {updates:?}"
    );
    let destinations = h.new_worlds(held);
    assert_eq!(destinations.len(), 1);
    assert_eq!(destinations[0].map_id, copy.map_id);
    h.ack(held);
    assert_eq!(h.map_of(held), Some(copy));
    assert!(battleground(&h, held).accepted);
}
