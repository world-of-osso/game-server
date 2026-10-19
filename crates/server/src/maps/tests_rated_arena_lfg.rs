use super::*;
use shared::protocol::{PvpBracketSnapshot, QueueForRatedPvp};

#[test]
#[ignore = "RED: rated arena team currently admits active LFG members"]
fn rated_lfg_2v2_refuses_queued_member_until_native_dequeue() {
    assert_rated_lfg_refusal(2, false);
}

#[test]
#[ignore = "RED: rated arena team currently admits active LFG members"]
fn rated_lfg_3v3_refuses_queued_member_until_native_dequeue() {
    assert_rated_lfg_refusal(3, false);
}

#[test]
#[ignore = "RED: rated arena team currently admits active LFG members"]
fn rated_lfg_2v2_refuses_role_check_member_until_native_decline() {
    assert_rated_lfg_refusal(2, true);
}

#[test]
#[ignore = "RED: rated arena team currently admits active LFG members"]
fn rated_lfg_3v3_refuses_role_check_member_until_native_decline() {
    assert_rated_lfg_refusal(3, true);
}

fn rated_harness(
    party_size: usize,
    count: u64,
) -> (Harness, Vec<Traveler>, tempfile::NamedTempFile) {
    let (mut h, players, database) = lfg_harness(count);
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let catalog = crate::pvp::arena::Catalog::load(&db).unwrap();
    let maximum: u8 = db
        .query_row("SELECT MAX(max_level) FROM arena_map", [], |row| row.get(0))
        .unwrap();
    h.app.insert_resource(catalog);
    for who in &players {
        h.app
            .world_mut()
            .entity_mut(who.player)
            .insert(shared::components::Level(maximum));
        h.app
            .world_mut()
            .entity_mut(who.connection)
            .insert(pair::<QueueForRatedPvp>());
    }
    form_source_parties(&mut h, &players[..party_size], maximum, party_size);
    (h, players, database)
}

fn request_rated(h: &mut Harness, who: &Traveler, size: usize) -> Vec<PvpStateUpdate> {
    let bracket = if size == 2 {
        PvpBracketSnapshot::Arena2v2
    } else {
        PvpBracketSnapshot::Arena3v3
    };
    h.app
        .world_mut()
        .get_mut::<MessageSender<QueueForRatedPvp>>(who.connection)
        .unwrap()
        .send::<PvpChannel>(QueueForRatedPvp { bracket });
    h.receive_pvp_updates(5).remove(&who.connection).unwrap()
}

fn assert_rated_queued(h: &Harness, party: &[Traveler], size: usize) {
    let bracket = if size == 2 {
        PvpBracketSnapshot::Arena2v2
    } else {
        PvpBracketSnapshot::Arena3v3
    };
    for who in party {
        let pvp = h
            .app
            .world()
            .get::<crate::pvp::PlayerPvpState>(who.player)
            .unwrap();
        assert_eq!(
            pvp.queue,
            Some(crate::pvp::PlayerPvpQueue::RatedBracket {
                bracket: bracket.clone()
            })
        );
        assert!(pvp.battleground.is_none());
    }
}

fn queue_role_check(h: &mut Harness, players: &[Traveler]) {
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
        queue_lfg(h, who, role);
    }
    h.advance(5);
}

fn assert_rated_lfg_refusal(size: usize, role_check: bool) {
    let count = if role_check { 5 } else { size as u64 };
    let (mut h, players, _database) = rated_harness(size, count);
    let party = &players[..size];
    let active = &players[size - 1];
    if role_check {
        queue_role_check(&mut h, &players);
    } else {
        queue_lfg(&mut h, active, GroupRoleSnapshot::Damage);
        h.advance(5);
    }
    let snapshot = query_lfg(&mut h, active);
    assert_eq!(snapshot.queued, !role_check);
    assert_eq!(snapshot.role_check.is_some(), role_check);
    if role_check {
        assert_eq!(snapshot.role_check.as_ref().unwrap().total_count, 5);
    }
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
    let pvp_before: Vec<_> = party
        .iter()
        .map(|who| {
            h.app
                .world()
                .get::<crate::pvp::PlayerPvpState>(who.player)
                .unwrap()
                .clone()
        })
        .collect();
    let updates = request_rated(&mut h, &party[0], size);
    assert!(
        updates.iter().any(|update| update
            .error
            .as_deref()
            .is_some_and(|error| error.contains("dungeon finder"))),
        "member LFG activity must refuse whole rated roster: {updates:?}"
    );
    assert_not_queued(&h, party);
    for (who, before) in party.iter().zip(pvp_before) {
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
    let retained = query_lfg(&mut h, active);
    assert_eq!(retained.queued, snapshot.queued);
    assert_eq!(retained.queue_size, snapshot.queue_size);
    assert_eq!(retained.role_check, snapshot.role_check);
    if role_check {
        h.app
            .world_mut()
            .get_mut::<MessageSender<RespondToLfgRoleCheck>>(active.connection)
            .unwrap()
            .send::<LfgChannel>(RespondToLfgRoleCheck { accepted: false });
    } else {
        h.app
            .world_mut()
            .get_mut::<MessageSender<DequeueFromLfg>>(active.connection)
            .unwrap()
            .send::<LfgChannel>(DequeueFromLfg);
    }
    h.advance(5);
    let cancelled = query_lfg(&mut h, active);
    assert!(!cancelled.queued && cancelled.role_check.is_none());
    if role_check {
        assert_eq!(cancelled.dungeon_ids, vec![DEADMINES]);
        assert!(cancelled.selected_role.is_some());
    }
    let allowed = request_rated(&mut h, &party[0], size);
    assert!(
        allowed.iter().all(|update| update.error.is_none()),
        "native LFG release permits rated roster: {allowed:?}"
    );
    assert_rated_queued(&h, party, size);
}

#[test]
fn rated_lfg_idle_2v2_party_admits_normally() {
    let (mut h, players, _database) = rated_harness(2, 2);
    let updates = request_rated(&mut h, &players[0], 2);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "idle rated queue: {updates:?}"
    );
    assert_rated_queued(&h, &players, 2);
}

#[test]
fn rated_lfg_idle_3v3_party_admits_normally() {
    let (mut h, players, _database) = rated_harness(3, 3);
    let updates = request_rated(&mut h, &players[0], 3);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "idle rated queue: {updates:?}"
    );
    assert_rated_queued(&h, &players, 3);
}

#[test]
fn rated_lfg_nonleader_still_cannot_enqueue_idle_party() {
    let (mut h, players, _database) = rated_harness(2, 2);
    let updates = request_rated(&mut h, &players[1], 2);
    assert!(
        updates.iter().any(|update| update
            .error
            .as_deref()
            .is_some_and(|error| error.contains("leader"))),
        "nonleader refusal: {updates:?}"
    );
    assert_not_queued(&h, &players);
}

#[test]
fn rated_lfg_wrong_size_still_cannot_enqueue_idle_party() {
    let (mut h, players, _database) = rated_harness(2, 2);
    let updates = request_rated(&mut h, &players[0], 3);
    assert!(
        updates.iter().any(|update| update
            .error
            .as_deref()
            .is_some_and(|error| error.contains("exactly 3"))),
        "party size refusal: {updates:?}"
    );
    assert_not_queued(&h, &players);
}
