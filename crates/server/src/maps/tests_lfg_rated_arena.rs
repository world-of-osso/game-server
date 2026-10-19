use super::*;
use shared::protocol::DequeueFromPvp;

#[test]
#[ignore = "RED: ordinary LFG currently admits actual rated arena queue members"]
fn lfg_rated_2v2_member_refuses_until_native_pvp_dequeue_clears_team() {
    assert_rated_queue_refuses_lfg(2);
}

#[test]
#[ignore = "RED: ordinary LFG currently admits actual rated arena queue members"]
fn lfg_rated_3v3_member_refuses_until_native_pvp_dequeue_clears_team() {
    assert_rated_queue_refuses_lfg(3);
}

fn queued_rated_party(size: usize) -> (Harness, Vec<Traveler>, tempfile::NamedTempFile) {
    let (mut h, players, database) = rated_harness(size, size as u64);
    let updates = request_rated(&mut h, &players[0], size);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "actual rated team admission: {updates:?}"
    );
    assert_rated_queued(&h, &players, size);
    (h, players, database)
}

fn receive_lfg_responses(h: &mut Harness, who: &Traveler) -> Vec<LfgStateUpdate> {
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

fn assert_rated_queue_refuses_lfg(size: usize) {
    let (mut h, players, _database) = queued_rated_party(size);
    let active = players.last().unwrap();
    let pvp_before: Vec<_> = players
        .iter()
        .map(|who| {
            h.app
                .world()
                .get::<crate::pvp::PlayerPvpState>(who.player)
                .unwrap()
                .clone()
        })
        .collect();
    let lfg_before: Vec<_> = players
        .iter()
        .map(|who| {
            h.app
                .world()
                .get::<crate::lfg::PlayerLfgState>(who.player)
                .unwrap()
                .clone()
        })
        .collect();
    queue_lfg(&mut h, active, GroupRoleSnapshot::Damage);
    let updates = receive_lfg_responses(&mut h, active);
    assert!(
        updates.iter().any(|update| update
            .error
            .as_deref()
            .is_some_and(|error| error.contains("battlefield"))),
        "actual rated arena queue must refuse ordinary LFG: {updates:?}"
    );
    for (who, before) in players.iter().zip(pvp_before) {
        assert_eq!(
            h.app.world().get::<crate::pvp::PlayerPvpState>(who.player),
            Some(&before)
        );
    }
    for (who, before) in players.iter().zip(lfg_before) {
        assert_eq!(
            h.app.world().get::<crate::lfg::PlayerLfgState>(who.player),
            Some(&before)
        );
    }
    let refused = query_lfg(&mut h, active);
    assert!(!refused.queued && refused.role_check.is_none() && refused.queue_size == 0);
    h.app
        .world_mut()
        .entity_mut(active.connection)
        .insert(pair::<DequeueFromPvp>());
    h.app
        .world_mut()
        .get_mut::<MessageSender<DequeueFromPvp>>(active.connection)
        .unwrap()
        .send::<PvpChannel>(DequeueFromPvp);
    let updates = h.receive_pvp_updates(5);
    assert!(
        updates
            .values()
            .flatten()
            .all(|update| update.error.is_none()),
        "native rated team dequeue: {updates:?}"
    );
    assert_not_queued(&h, &players);
    queue_lfg(&mut h, active, GroupRoleSnapshot::Damage);
    h.advance(5);
    let allowed = query_lfg(&mut h, active);
    assert!(allowed.queued && allowed.role_check.is_none());
    assert_eq!(allowed.dungeon_ids, vec![DEADMINES]);
}

#[test]
fn lfg_rated_2v2_follower_mixed_selection_keeps_existing_route() {
    let (mut h, players, _database) = queued_rated_party(2);
    let active = &players[1];
    h.app
        .world_mut()
        .resource_mut::<crate::followers::FollowerCatalog>()
        .follower_ids
        .insert(999);
    h.app
        .world_mut()
        .get_mut::<MessageSender<QueueForLfg>>(active.connection)
        .unwrap()
        .send::<LfgChannel>(QueueForLfg {
            role: GroupRoleSnapshot::Damage,
            dungeon_ids: vec![999, DEADMINES],
        });
    let updates = receive_lfg_responses(&mut h, active);
    assert!(
        updates
            .iter()
            .any(|update| update.error.as_deref() == Some("select one follower dungeon")),
        "unchanged follower dispatch: {updates:?}"
    );
    assert_rated_queued(&h, &players, 2);
    assert!(!query_lfg(&mut h, active).queued);
}

#[test]
fn lfg_rated_idle_player_still_joins_ordinary_lfg() {
    let (mut h, players, _database) = rated_harness(2, 2);
    let active = &players[1];
    assert_not_queued(&h, &players);
    queue_lfg(&mut h, active, GroupRoleSnapshot::Damage);
    h.advance(5);
    assert!(query_lfg(&mut h, active).queued);
    assert_not_queued(&h, &players);
}
