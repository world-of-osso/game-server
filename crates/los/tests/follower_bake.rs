//! Private follower-run eye receipt from authenticated models, never a LOS bypass.
#![cfg(feature = "bake")]

use std::{fs, path::PathBuf};

use los::bake::eye::eye_rows_of;
use los::eye::EyeTable;
use rusqlite::{Connection, OpenFlags};

#[test]
#[ignore = "requires FOLLOWER_WORLD_DB, FOLLOWER_ENGINE_DATA and FOLLOWER_EYE_OUTPUT"]
fn follower_ruby_models_bake_every_eye_pose_and_load_in_runtime() {
    let db_path = PathBuf::from(std::env::var("FOLLOWER_WORLD_DB").unwrap());
    let data = PathBuf::from(std::env::var("FOLLOWER_ENGINE_DATA").unwrap());
    let output = PathBuf::from(std::env::var("FOLLOWER_EYE_OUTPUT").unwrap());
    let db = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut query = db.prepare("SELECT DISTINCT m.CreatureDisplayID FROM content_creature_template_model m WHERE m.CreatureID IN (SELECT id1 FROM content_creature WHERE map=2521 AND (','||spawnDifficulties||',') LIKE '%,1,%') OR m.CreatureID IN (SELECT entry FROM follower_profiles) ORDER BY m.CreatureDisplayID").unwrap();
    let displays: Vec<u32> = query
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(displays.contains(&113757), "Crenna's authored display");
    let (rows, stats) = eye_rows_of(&data, &db_path, &displays, &[]).unwrap();
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
            let eye = loaded
                .eye(display, state)
                .expect("every runtime pose has eyes");
            assert!(eye.is_finite());
        }
    }
    println!(
        "{} authored display receipts, {} eye rows",
        stats.displays,
        rows.len()
    );
}
