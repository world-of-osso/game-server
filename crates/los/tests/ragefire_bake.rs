//! Private source-model eye receipts for native Ragefire encounter acceptance.
#![cfg(feature = "bake")]

use std::{fs, path::PathBuf};

use los::bake::eye::eye_rows_of;
use los::eye::EyeTable;
use rusqlite::{Connection, OpenFlags};

#[test]
#[ignore = "requires RAGEFIRE_WORLD_DB, RAGEFIRE_ENGINE_DATA and RAGEFIRE_EYE_OUTPUT"]
fn ragefire_models_bake_every_eye_pose_and_load_in_runtime() {
    let database = PathBuf::from(std::env::var("RAGEFIRE_WORLD_DB").unwrap());
    let data = PathBuf::from(std::env::var("RAGEFIRE_ENGINE_DATA").unwrap());
    let output = PathBuf::from(std::env::var("RAGEFIRE_EYE_OUTPUT").unwrap());
    let connection =
        Connection::open_with_flags(&database, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut query = connection
        .prepare(
            "SELECT DISTINCT CreatureDisplayID FROM content_creature_template_model
         WHERE CreatureID IN (SELECT id1 FROM content_creature WHERE map=389)
            OR CreatureID=61800
         UNION SELECT CAST(modelid AS INTEGER) FROM content_creature WHERE map=389 AND modelid!=0
         ORDER BY 1",
        )
        .unwrap();
    let displays: Vec<u32> = query
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    for display in [38759, 42101, 42247, 42100] {
        assert!(
            displays.contains(&display),
            "source primary display {display}"
        );
    }
    let (rows, stats) = eye_rows_of(&data, &database, &displays, &[]).unwrap();
    assert_eq!(stats.displays, displays.len());
    assert!(stats.skipped.is_empty());
    fs::create_dir_all(&output).unwrap();
    fs::write(output.join("eye_heights.tsv"), EyeTable::to_tsv(&rows)).unwrap();
    fs::copy(
        data.join("los/player_displays.tsv"),
        output.join("player_displays.tsv"),
    )
    .unwrap();
    let loaded = EyeTable::load(&output).unwrap();
    for display in displays {
        for state in 0..=9 {
            let point = loaded
                .eye(display, state)
                .expect("every authored pose baked");
            assert!(point.iter().all(|coordinate| coordinate.is_finite()));
            assert!(
                point[2] > 0.0,
                "display {display} state {state} eyes above feet"
            );
        }
    }
    println!(
        "{} Ragefire display receipts, {} eye rows",
        stats.displays,
        rows.len()
    );
}
