use super::*;

#[test]
fn high_spawn_and_formation_guids_round_trip_through_graph() {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch(
        "CREATE TABLE content_creature_formations(leaderGUID INTEGER, memberGUID INTEGER, dist REAL, angle REAL, groupAI INTEGER);
         INSERT INTO content_creature_formations VALUES(8100243029,8100243029,0,0,0), (8100243029,8100245912,3,0,0);",
    )
    .unwrap();
    let spawn = db
        .query_row(
            "SELECT 8100245912,245912,2859,0,1351.79,1339.61,-12.8,0,0,0,0,0,0,'1,8,23',0",
            [],
            spawn_from_row,
        )
        .unwrap();
    let formations = load_formations(&db);
    assert_eq!(spawn.guid, 8_100_245_912);
    assert_eq!(formations[&spawn.guid].leader_guid, 8_100_243_029);
    let pack = crate::test_support::graph::Pack {
        id: formations[&spawn.guid].leader_guid,
        position: [spawn.position_x, -spawn.position_y],
        height: spawn.position_z,
        spawns: vec![crate::test_support::graph::Spawn {
            guid: spawn.guid,
            entry: spawn.template_entry,
            position: [spawn.position_x, -spawn.position_y],
        }],
    };
    let json = serde_json::to_vec(&pack).unwrap();
    let decoded: crate::test_support::graph::Pack = serde_json::from_slice(&json).unwrap();
    assert_eq!(decoded.id, 8_100_243_029);
    assert_eq!(decoded.spawns[0].guid, 8_100_245_912);
}
