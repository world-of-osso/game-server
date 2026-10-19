use super::*;
use shared::protocol::{
    DequeueFromLfg, GroupRoleSnapshot, LfgChannel, LfgSnapshot, LfgStateUpdate, QueryLfgStatus,
    QueueForLfg, RespondToLfgRoleCheck,
};

const DEADMINES: u32 = 100;

#[test]
#[ignore = "RED: ordinary BG admission ignores actual LFG queue activity"]
fn bg_lfg_solo_concrete_refuses_queue_and_allows_native_dequeue() {
    assert_queued_lfg_refuses_bg(1018, 1);
}

#[test]
#[ignore = "RED: ordinary BG admission ignores actual LFG queue activity"]
fn bg_lfg_solo_random_refuses_queue_and_allows_native_dequeue() {
    assert_queued_lfg_refuses_bg(32, 1);
}

#[test]
#[ignore = "RED: ordinary BG party admission ignores a member queued for LFG"]
fn bg_lfg_party_concrete_refuses_atomically_until_member_dequeues() {
    assert_queued_lfg_refuses_bg(1018, 5);
}

#[test]
#[ignore = "RED: ordinary BG party admission ignores a member queued for LFG"]
fn bg_lfg_party_random_refuses_atomically_until_member_dequeues() {
    assert_queued_lfg_refuses_bg(32, 5);
}

#[test]
#[ignore = "RED: ordinary BG admission ignores an active LFG role check"]
fn bg_lfg_role_check_solo_refuses_then_allows_cancelled_selection_history() {
    assert_role_check_refuses_bg(1018, false);
}

#[test]
#[ignore = "RED: ordinary BG party admission ignores an active LFG role check"]
fn bg_lfg_role_check_party_refuses_then_allows_cancelled_selection_history() {
    assert_role_check_refuses_bg(32, true);
}

fn lfg_harness(count: u64) -> (Harness, Vec<Traveler>, tempfile::NamedTempFile) {
    let database_file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(database_file.path()).unwrap();
    crate::persistence::create_tables(&db);
    let mut h = harness_with(move |app| {
        app.insert_resource(crate::persistence::GameDatabase::new(db));
        app.init_resource::<crate::followers::FollowerCatalog>();
        app.insert_resource(crate::pvp::random_bg::tests::source_catalog(&[1018]));
        app.add_plugins((
            crate::group::GroupPlugin,
            crate::lfg::LfgPlugin,
            crate::pvp::PvpPlugin,
        ));
    })
    .expect("actual map and normal LFG/PvP channels");
    let players: Vec<_> = (1..=count)
        .map(|id| h.spawn_at_level(id, 25, STORMWIND, STOCKADE_ENTRANCE_TRIGGER))
        .collect();
    for who in &players {
        h.app.world_mut().entity_mut(who.player).insert((
            crate::pvp::PlayerPvpState::default(),
            crate::lfg::PlayerLfgState::default(),
        ));
        h.app.world_mut().entity_mut(who.connection).insert((
            pair::<QueueForBattleground>(),
            pair::<PvpStateUpdate>(),
            pair::<QueueForLfg>(),
            pair::<DequeueFromLfg>(),
            pair::<RespondToLfgRoleCheck>(),
            pair::<QueryLfgStatus>(),
            pair::<LfgStateUpdate>(),
        ));
    }
    h.receive_pvp_updates(5);
    (h, players, database_file)
}

fn queue_lfg(h: &mut Harness, who: &Traveler, role: GroupRoleSnapshot) {
    h.app
        .world_mut()
        .get_mut::<MessageSender<QueueForLfg>>(who.connection)
        .unwrap()
        .send::<LfgChannel>(QueueForLfg {
            role,
            dungeon_ids: vec![DEADMINES],
        });
}

fn query_lfg(h: &mut Harness, who: &Traveler) -> LfgSnapshot {
    h.app
        .world_mut()
        .get_mut::<MessageReceiver<LfgStateUpdate>>(who.connection)
        .unwrap()
        .receive()
        .for_each(drop);
    h.app
        .world_mut()
        .get_mut::<MessageSender<QueryLfgStatus>>(who.connection)
        .unwrap()
        .send::<LfgChannel>(QueryLfgStatus);
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
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "normal LFG response: {updates:?}"
    );
    updates
        .into_iter()
        .filter_map(|update| update.snapshot)
        .last()
        .expect("authoritative LFG snapshot")
}

fn assert_refused_without_lfg_mutation(
    h: &mut Harness,
    players: &[Traveler],
    active: &Traveler,
    queue_id: u32,
) {
    let before = h
        .app
        .world()
        .get::<crate::lfg::PlayerLfgState>(active.player)
        .unwrap()
        .clone();
    let updates = request_queue(h, &players[0], queue_id);
    assert!(
        updates.iter().any(|update| update
            .error
            .as_deref()
            .is_some_and(|error| error.contains("dungeon finder"))),
        "actual LFG activity must refuse ordinary BG admission: {updates:?}"
    );
    assert_not_queued(h, players);
    assert_eq!(
        h.app
            .world()
            .get::<crate::lfg::PlayerLfgState>(active.player),
        Some(&before)
    );
}

fn assert_bg_join_allowed(h: &mut Harness, players: &[Traveler], queue_id: u32) {
    let updates = request_queue(h, &players[0], queue_id);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "eligible BG join: {updates:?}"
    );
    assert_queued(h, players, queue_id);
}

fn assert_queued_lfg_refuses_bg(queue_id: u32, count: u64) {
    let (mut h, players, _database) = lfg_harness(count);
    if count == 5 {
        form_source_five_player_parties(&mut h, &players, 25);
    }
    let active = players.last().unwrap();
    queue_lfg(&mut h, active, GroupRoleSnapshot::Damage);
    h.advance(5);
    let before = query_lfg(&mut h, active);
    assert!(before.queued && before.role_check.is_none());
    assert_eq!(before.dungeon_ids, vec![DEADMINES]);
    assert_refused_without_lfg_mutation(&mut h, &players, active, queue_id);
    let retained = query_lfg(&mut h, active);
    assert!(retained.queued && retained.role_check.is_none());
    assert_eq!(retained.queue_size, before.queue_size);
    h.app
        .world_mut()
        .get_mut::<MessageSender<DequeueFromLfg>>(active.connection)
        .unwrap()
        .send::<LfgChannel>(DequeueFromLfg);
    h.advance(5);
    let idle = query_lfg(&mut h, active);
    assert!(!idle.queued && idle.role_check.is_none() && idle.dungeon_ids.is_empty());
    assert_bg_join_allowed(&mut h, &players, queue_id);
}

fn assert_role_check_refuses_bg(queue_id: u32, party: bool) {
    let (mut h, players, _database) = lfg_harness(5);
    if party {
        form_source_five_player_parties(&mut h, &players, 25);
    }
    h.app
        .world_mut()
        .get_mut::<Player>(players[1].player)
        .unwrap()
        .class = 5;
    for (who, role) in players.iter().zip([
        GroupRoleSnapshot::Tank,
        GroupRoleSnapshot::Healer,
        GroupRoleSnapshot::Damage,
        GroupRoleSnapshot::Damage,
        GroupRoleSnapshot::Damage,
    ]) {
        queue_lfg(&mut h, who, role);
    }
    h.advance(5);
    let leader = &players[0];
    let checking = query_lfg(&mut h, leader);
    assert!(
        !checking.queued,
        "matchmaking consumed the real LFG queue entries"
    );
    assert_eq!(checking.role_check.as_ref().unwrap().total_count, 5);
    assert_refused_without_lfg_mutation(&mut h, &players, leader, queue_id);
    let retained = query_lfg(&mut h, leader);
    assert_eq!(retained.role_check, checking.role_check);
    h.app
        .world_mut()
        .get_mut::<MessageSender<RespondToLfgRoleCheck>>(leader.connection)
        .unwrap()
        .send::<LfgChannel>(RespondToLfgRoleCheck { accepted: false });
    h.advance(5);
    let cancelled = query_lfg(&mut h, leader);
    assert!(!cancelled.queued && cancelled.role_check.is_none());
    assert_eq!(
        cancelled.dungeon_ids,
        vec![DEADMINES],
        "selection history is not activity"
    );
    assert!(cancelled.selected_role.is_some());
    let joining = if party {
        players.as_slice()
    } else {
        &players[..1]
    };
    assert_bg_join_allowed(&mut h, joining, queue_id);
}
