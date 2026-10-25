use super::*;
use crate::pvp::{form_battleground_invitation, random_bg};
use shared::battleground::{BgPhase, BgTeam};

fn fill_existing_copies(
    state: &mut PvpServerState,
    instances: &mut InstanceManager,
    catalog: &random_bg::Catalog,
    queue: u32,
) -> Result<Vec<Invitations>, String> {
    let mut batches = Vec::new();
    while let Some(batch) = fill_existing_copy(state, instances, catalog, queue)? {
        batches.push(batch);
    }
    Ok(batches)
}

fn form_room(state: &mut PvpServerState, first: u64, level: u8, queue: u32) -> u64 {
    for character in first..first + 16 {
        state
            .battleground_queue
            .join_solo(character, level, queue, 0)
            .unwrap();
    }
    form_battleground_invitation(state, queue, &random_bg::Catalog::default()).unwrap()
}

#[test]
fn bg_backfill_unaccepted_rosters_reserve_capacity_and_older_matches_fill_first() {
    let mut state = PvpServerState::default();
    let older = form_room(&mut state, 1, 25, 1018);
    let newer = form_room(&mut state, 17, 25, 1018);
    for leader in [33, 38, 43, 48] {
        state
            .battleground_queue
            .join_group(leader, (leader..leader + 5).collect(), vec![25; 5], 32, 0)
            .unwrap();
    }
    let mut instances = InstanceManager::default();
    let catalog = random_bg::tests::source_catalog(&[1018]);
    let batches = fill_existing_copies(&mut state, &mut instances, &catalog, 32).unwrap();
    assert_eq!(
        batches
            .iter()
            .map(|batch| batch.match_id)
            .collect::<Vec<_>>(),
        [older, newer]
    );
    assert_eq!(batches[0].characters, (33..43).collect::<Vec<_>>());
    assert_eq!(batches[1].characters, (43..53).collect::<Vec<_>>());
    for id in [older, newer] {
        let room = &state.battleground_matches[&id];
        assert!(
            room.accepted.is_empty(),
            "outstanding invitations consume capacity before porting"
        );
        assert_eq!(
            (
                room.instance.team_a_players.len(),
                room.instance.team_b_players.len()
            ),
            (13, 13)
        );
    }
    state
        .battleground_queue
        .join_group(53, vec![53, 54, 55], vec![25; 3], 32, 0)
        .unwrap();
    assert!(
        fill_existing_copies(&mut state, &mut instances, &catalog, 32)
            .unwrap()
            .is_empty()
    );
    assert!((53..=55).all(|character| state.battleground_queue.is_queued(character)));
}

#[test]
fn bg_backfill_nonfitting_whole_party_stays_queued_while_a_later_smaller_party_fits() {
    let mut state = PvpServerState::default();
    let id = form_room(&mut state, 1, 25, 1018);
    let catalog = random_bg::tests::source_catalog(&[1018]);
    let mut instances = InstanceManager::default();
    for leader in [17, 22] {
        state
            .battleground_queue
            .join_group(leader, (leader..leader + 5).collect(), vec![25; 5], 32, 0)
            .unwrap();
    }
    fill_existing_copies(&mut state, &mut instances, &catalog, 32).unwrap();
    state
        .battleground_queue
        .join_group(27, (27..32).collect(), vec![25; 5], 32, 0)
        .unwrap();
    state.battleground_queue.join_solo(32, 25, 32, 0).unwrap();
    let batches = fill_existing_copies(&mut state, &mut instances, &catalog, 32).unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].characters, [32]);
    let room = &state.battleground_matches[&id];
    assert_eq!(
        (
            room.instance.team_a_players.len(),
            room.instance.team_b_players.len()
        ),
        (14, 13)
    );
    assert!((27..32).all(|character| state.battleground_queue.is_queued(character)));
    assert_eq!(room.original_queues.get(&32), Some(&32));
}

#[test]
fn bg_backfill_excludes_ended_wrong_range_and_unconfigured_random_maps() {
    let mut state = PvpServerState::default();
    let ended = form_room(&mut state, 1, 25, 1018);
    let wrong_range = form_room(&mut state, 17, 35, 1018);
    let other_map = form_room(&mut state, 33, 25, 7);
    let template = shared::battleground::arathi_basin();
    let room = &mut state.battleground_matches.get_mut(&ended).unwrap().instance;
    assert!(room.start());
    room.add_score(BgTeam::A, template.win_score, &template);
    assert_eq!(room.phase, BgPhase::Ended);
    state
        .battleground_queue
        .join_group(49, (49..54).collect(), vec![25; 5], 32, 0)
        .unwrap();
    let catalog = random_bg::tests::source_catalog(&[1018]);
    let mut instances = InstanceManager::default();
    assert!(
        fill_existing_copies(&mut state, &mut instances, &catalog, 32)
            .unwrap()
            .is_empty()
    );
    assert!((49..54).all(|character| state.battleground_queue.is_queued(character)));
    assert_eq!(
        state.battleground_matches[&wrong_range]
            .instance
            .team_a_players
            .len(),
        8
    );
    assert_eq!(
        state.battleground_matches[&other_map]
            .instance
            .team_a_players
            .len(),
        8
    );
}

#[test]
fn bg_backfill_never_fills_rated_or_skirmish_arena_copies() {
    use crate::pvp::arena;
    use shared::protocol::ArenaQueueMode;
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE arena_map(battlemaster_id INTEGER,map_id INTEGER,name TEXT,start_a INTEGER,start_b INTEGER,min_level INTEGER,max_level INTEGER,weight INTEGER,wdt_fdid INTEGER); INSERT INTO arena_map VALUES(8,572,'Ruins of Lordaeron',1258,1259,10,90,1,789984)").unwrap();
    let arenas = arena::Catalog::load(&db).unwrap();
    for mode in [ArenaQueueMode::Rated2v2, ArenaQueueMode::Skirmish2v2] {
        let mut state = PvpServerState::default();
        let id = arena::create_match(
            &mut state,
            &arenas,
            mode,
            [vec![1, 2], vec![3, 4]],
            &[90; 4],
        )
        .unwrap();
        state
            .battleground_queue
            .join_group(5, (5..10).collect(), vec![90; 5], 32, 0)
            .unwrap();
        let mut instances = InstanceManager::default();
        let catalog = random_bg::tests::source_catalog(&[]);
        assert!(
            fill_existing_copies(&mut state, &mut instances, &catalog, 32)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            state.battleground_matches[&id].instance.team_a_players,
            [1, 2]
        );
        assert!((5..10).all(|character| state.battleground_queue.is_queued(character)));
    }
}
