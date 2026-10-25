use super::*;
use shared::protocol::{ArenaQueueMode, DequeueFromPvp, LeaveBattleground, QueueForArena};

#[derive(Clone, Copy)]
enum Activity {
    Queued,
    Invited,
    Entered,
}

#[test]
fn lfg_pvp_concrete_queued_refuses_until_native_dequeue() {
    assert_pvp_refuses_lfg(Some(1018), Activity::Queued);
}

#[test]
fn lfg_pvp_random_queued_refuses_until_native_dequeue() {
    assert_pvp_refuses_lfg(Some(32), Activity::Queued);
}

#[test]
fn lfg_pvp_arena_queued_refuses_until_native_dequeue() {
    assert_pvp_refuses_lfg(None, Activity::Queued);
}

#[test]
fn lfg_pvp_concrete_invited_refuses_until_native_decline() {
    assert_pvp_refuses_lfg(Some(1018), Activity::Invited);
}

#[test]
fn lfg_pvp_random_invited_refuses_until_native_decline() {
    assert_pvp_refuses_lfg(Some(32), Activity::Invited);
}

#[test]
fn lfg_pvp_arena_invited_refuses_until_native_decline() {
    assert_pvp_refuses_lfg(None, Activity::Invited);
}

#[test]
fn lfg_pvp_concrete_entered_refuses_until_native_leave() {
    assert_pvp_refuses_lfg(Some(1018), Activity::Entered);
}

#[test]
fn lfg_pvp_random_entered_refuses_until_native_leave() {
    assert_pvp_refuses_lfg(Some(32), Activity::Entered);
}

#[test]
fn lfg_pvp_arena_entered_refuses_until_native_leave() {
    assert_pvp_refuses_lfg(None, Activity::Entered);
}

fn prepare_pvp(h: &mut Harness, players: &[Traveler], queue_id: Option<u32>) {
    if queue_id.is_none() {
        let db = rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        h.app
            .insert_resource(crate::pvp::arena::Catalog::load(&db).unwrap());
        h.app
            .insert_resource(crate::game_objects::GameObjectSpawns::default());
    } else {
        h.app.insert_resource(source_battleground_objects(2107));
    }
    h.app.init_resource::<crate::faction::FactionTemplates>();
    for who in players {
        h.app.world_mut().entity_mut(who.player).insert((
            shared::components::UnitFactionTemplate(crate::faction::faction_template_for_race(1)),
            shared::components::CurrentPvpFaction(2),
        ));
        h.app.world_mut().entity_mut(who.connection).insert((
            pair::<QueueForArena>(),
            pair::<DequeueFromPvp>(),
            pair::<RespondToBattlegroundInvite>(),
            pair::<LeaveBattleground>(),
        ));
    }
}

fn send_pvp_queue(h: &mut Harness, who: &Traveler, queue_id: Option<u32>) {
    if let Some(battleground_id) = queue_id {
        h.app
            .world_mut()
            .get_mut::<MessageSender<QueueForBattleground>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(QueueForBattleground { battleground_id });
    } else {
        h.app
            .world_mut()
            .get_mut::<MessageSender<QueueForArena>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(QueueForArena {
                mode: ArenaQueueMode::Skirmish2v2,
            });
    }
}

fn receive_lfg(h: &mut Harness, who: &Traveler) -> Vec<LfgStateUpdate> {
    let mut updates = Vec::new();
    for _ in 0..5 {
        h.advance(1);
        updates.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<LfgStateUpdate>>(who.connection)
                .unwrap()
                .receive(),
        );
    }
    updates
}

fn assert_pvp_refuses_lfg(queue_id: Option<u32>, activity: Activity) {
    let count = if matches!(activity, Activity::Queued) {
        1
    } else if queue_id.is_some() {
        20
    } else {
        4
    };
    let (mut h, players, _database) = lfg_harness(count);
    prepare_pvp(&mut h, &players, queue_id);
    for who in &players {
        send_pvp_queue(&mut h, who, queue_id);
    }
    let updates = h.receive_pvp_updates(5);
    assert!(
        updates
            .values()
            .flatten()
            .all(|update| update.error.is_none()),
        "PvP admission: {updates:?}"
    );
    let held = &players[0];
    if matches!(activity, Activity::Entered) {
        let invitation = battleground(&h, held).clone();
        h.app
            .world_mut()
            .get_mut::<MessageSender<RespondToBattlegroundInvite>>(held.connection)
            .unwrap()
            .send::<PvpChannel>(RespondToBattlegroundInvite {
                match_id: invitation.match_id,
                accepted: true,
            });
        let updates = h.receive_pvp_updates(5);
        assert!(
            updates
                .values()
                .flatten()
                .all(|update| update.error.is_none()),
            "entry: {updates:?}"
        );
        let destinations = h.new_worlds(held);
        assert_eq!(destinations.len(), 1);
        assert_eq!(destinations[0].map_id, invitation.map_id);
        h.ack(held);
        assert_eq!(h.map_of(held).unwrap().map_id, invitation.map_id);
    }
    let pvp = h
        .app
        .world()
        .get::<crate::pvp::PlayerPvpState>(held.player)
        .unwrap()
        .clone();
    match activity {
        Activity::Queued => assert!(pvp.queue.is_some() && pvp.battleground.is_none()),
        Activity::Invited => assert!(!pvp.battleground.as_ref().unwrap().accepted),
        Activity::Entered => {
            assert!(pvp.queue.is_none() && pvp.battleground.as_ref().unwrap().accepted)
        }
    }
    let lfg = h
        .app
        .world()
        .get::<crate::lfg::PlayerLfgState>(held.player)
        .unwrap()
        .clone();
    queue_lfg(&mut h, held, GroupRoleSnapshot::Damage);
    let updates = receive_lfg(&mut h, held);
    assert!(
        updates.iter().any(|update| update
            .error
            .as_deref()
            .is_some_and(|error| error.contains("battlefield"))),
        "active PvP must refuse ordinary LFG: {updates:?}"
    );
    assert_eq!(
        h.app.world().get::<crate::pvp::PlayerPvpState>(held.player),
        Some(&pvp)
    );
    assert_eq!(
        h.app.world().get::<crate::lfg::PlayerLfgState>(held.player),
        Some(&lfg)
    );
    let refused = query_lfg(&mut h, held);
    assert!(!refused.queued && refused.role_check.is_none() && refused.queue_size == 0);
    release_pvp(
        &mut h,
        held,
        activity,
        pvp.battleground.as_ref().map(|bg| bg.match_id),
    );
    let released = h
        .app
        .world()
        .get::<crate::pvp::PlayerPvpState>(held.player)
        .unwrap();
    assert!(released.queue.is_none() && released.battleground.is_none());
    queue_lfg(&mut h, held, GroupRoleSnapshot::Damage);
    h.advance(5);
    let allowed = query_lfg(&mut h, held);
    assert!(allowed.queued && allowed.role_check.is_none());
    assert_eq!(allowed.dungeon_ids, vec![DEADMINES]);
}

fn release_pvp(h: &mut Harness, who: &Traveler, activity: Activity, match_id: Option<u64>) {
    match activity {
        Activity::Queued => h
            .app
            .world_mut()
            .get_mut::<MessageSender<DequeueFromPvp>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(DequeueFromPvp),
        Activity::Invited => h
            .app
            .world_mut()
            .get_mut::<MessageSender<RespondToBattlegroundInvite>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(RespondToBattlegroundInvite {
                match_id: match_id.unwrap(),
                accepted: false,
            }),
        Activity::Entered => h
            .app
            .world_mut()
            .get_mut::<MessageSender<LeaveBattleground>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(LeaveBattleground {
                match_id: match_id.unwrap(),
            }),
    }
    let updates = h.receive_pvp_updates(5);
    assert!(
        updates
            .values()
            .flatten()
            .all(|update| update.error.is_none()),
        "native release: {updates:?}"
    );
    if matches!(activity, Activity::Entered) {
        let destinations = h.new_worlds(who);
        assert_eq!(destinations.len(), 1);
        assert_eq!(destinations[0].map_id, STORMWIND.map_id);
        h.ack(who);
        assert_eq!(h.map_of(who), Some(STORMWIND));
    }
}

#[test]
fn lfg_pvp_follower_selection_validation_still_precedes_ordinary_join() {
    let (mut h, players, _database) = lfg_harness(1);
    let held = &players[0];
    prepare_pvp(&mut h, &players, Some(1018));
    send_pvp_queue(&mut h, held, Some(1018));
    h.receive_pvp_updates(5);
    assert!(
        h.app
            .world()
            .get::<crate::pvp::PlayerPvpState>(held.player)
            .unwrap()
            .queue
            .is_some()
    );
    h.app
        .world_mut()
        .resource_mut::<crate::followers::FollowerCatalog>()
        .follower_ids
        .insert(999);
    h.app
        .world_mut()
        .get_mut::<MessageSender<QueueForLfg>>(held.connection)
        .unwrap()
        .send::<LfgChannel>(QueueForLfg {
            role: GroupRoleSnapshot::Damage,
            dungeon_ids: vec![999, DEADMINES],
        });
    let updates = receive_lfg(&mut h, held);
    assert!(
        updates
            .iter()
            .any(|update| update.error.as_deref() == Some("select one follower dungeon")),
        "unchanged follower route: {updates:?}"
    );
    assert!(!query_lfg(&mut h, held).queued);
}
