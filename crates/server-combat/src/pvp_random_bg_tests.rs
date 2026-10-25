use super::*;

/// Read-only configured source rows, optionally restricted to concrete IDs in a test database.
pub(crate) fn source_catalog(only_ids: &[u32]) -> Catalog {
    let source = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    if only_ids.is_empty() {
        return Catalog::load(&source).unwrap();
    }
    let fixture = Connection::open_in_memory().unwrap();
    fixture
        .execute_batch(
            "CREATE TABLE tdb_battleground_template(ID INTEGER PRIMARY KEY,Weight INTEGER)",
        )
        .unwrap();
    for &id in only_ids {
        let weight: u32 = source
            .query_row(
                "SELECT Weight FROM tdb_battleground_template WHERE ID=?1",
                [id],
                |row| row.get(0),
            )
            .unwrap();
        fixture
            .execute(
                "INSERT INTO tdb_battleground_template VALUES(?1,?2)",
                [id, weight],
            )
            .unwrap();
    }
    Catalog::load(&fixture).unwrap()
}

#[test]
fn random_queue_level25_draws_only_eligible_configured_maps_without_deepwind_or_deephaul() {
    let catalog = source_catalog(&[]);
    let maps = catalog.eligible(&[20, 25, 29]);
    let total: u64 = maps.iter().map(|map| u64::from(map.weight)).sum();
    assert!(total > 0);
    let selected: std::collections::BTreeSet<_> = (0..total)
        .map(|ticket| {
            let selection = select_ticket(&maps, ticket).unwrap();
            assert_eq!(selection.bracket, LevelBracket::new(20, 29));
            selection.template.map_id
        })
        .collect();
    assert_eq!(
        selected,
        std::collections::BTreeSet::from([566, 761, 2106, 2107])
    );
    assert!(select_ticket(&maps, total).is_none());
}

#[test]
fn random_queue_level80_can_draw_deephaul_and_never_missing_deepwind() {
    let catalog = source_catalog(&[]);
    let maps = catalog.eligible(&[80; 10]);
    let total: u64 = maps.iter().map(|map| u64::from(map.weight)).sum();
    let selected: std::collections::BTreeSet<_> = (0..total)
        .map(|ticket| {
            let selection = select_ticket(&maps, ticket).unwrap();
            assert_eq!(selection.bracket, LevelBracket::new(80, 80));
            selection.template.map_id
        })
        .collect();
    assert_eq!(
        selected,
        std::collections::BTreeSet::from([566, 726, 727, 761, 998, 1803, 2106, 2107, 2656])
    );
}

#[test]
fn random_queue_ineligible_only_range_is_explicitly_refused_and_not_consumed() {
    let catalog = source_catalog(&[1110]);
    let error = catalog.validate_levels(&[25; 5]).unwrap_err();
    assert!(error.contains("no configured Random Battleground map"));
    let mut queue = BgQueue::default();
    for leader in [1, 6] {
        queue
            .join_group(leader, (leader..leader + 5).collect(), vec![25; 5], 32, 0)
            .unwrap();
    }
    assert!(draw_match(&mut queue, &catalog).is_none());
    assert!((1..=10).all(|player| queue.is_queued(player)));
    assert!(
        catalog.validate_levels(&[80, 81]).is_err(),
        "a party cannot span selected source brackets"
    );
    catalog.validate_levels(&[80; 5]).unwrap();
}

#[test]
fn random_queue_selection_obeys_configured_weights_not_an_assumed_uniform_draw() {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE tdb_battleground_template(ID INTEGER PRIMARY KEY,Weight INTEGER); INSERT INTO tdb_battleground_template VALUES(7,3),(1014,1),(1110,0)").unwrap();
    let catalog = Catalog::load(&db).unwrap();
    let maps = catalog.eligible(&[25]);
    let selected: Vec<_> = (0..4)
        .map(|ticket| select_ticket(&maps, ticket).unwrap().template.map_id)
        .collect();
    assert_eq!(selected, [566, 566, 566, 2106]);
    assert!(select_ticket(&maps, 4).is_none());
}

pub(crate) fn original_queue(world: &World, match_id: u64, character: u64) -> Option<u32> {
    world
        .resource::<super::super::PvpServerState>()
        .battleground_matches
        .get(&match_id)?
        .original_queues
        .get(&character)
        .copied()
}
