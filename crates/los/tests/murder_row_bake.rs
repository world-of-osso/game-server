//! Source-model eye receipt for Nibbles' scripted friendly display.
#![cfg(feature = "bake")]

use std::{fs, path::PathBuf};

use los::bake::eye::eye_rows_of;
use los::eye::EyeTable;
use rusqlite::{Connection, OpenFlags};

#[test]
#[ignore = "requires MURDER_ROW_WORLD_DB, MURDER_ROW_ENGINE_DATA and MURDER_ROW_EYE_OUTPUT"]
fn scripted_friendly_nibbles_has_all_runtime_eye_poses() {
    let database = PathBuf::from(std::env::var("MURDER_ROW_WORLD_DB").unwrap());
    let data = PathBuf::from(std::env::var("MURDER_ROW_ENGINE_DATA").unwrap());
    let output = PathBuf::from(std::env::var("MURDER_ROW_EYE_OUTPUT").unwrap());
    let connection =
        Connection::open_with_flags(&database, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut query = connection
        .prepare(include_str!("../../../scripts/los-runtime-displays.sql"))
        .unwrap();
    let displays: Vec<u32> = query
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(
        displays.contains(&128332),
        "scripted Nibbles display must be required"
    );
    let (rows, stats) = eye_rows_of(&data, &database, &[128332], &[]).unwrap();
    assert_eq!(stats.displays, 1);
    assert_eq!(rows.len(), 10);
    fs::create_dir_all(&output).unwrap();
    fs::write(output.join("eye_heights.tsv"), EyeTable::to_tsv(&rows)).unwrap();
    fs::copy(
        data.join("los/player_displays.tsv"),
        output.join("player_displays.tsv"),
    )
    .unwrap();
    let loaded = EyeTable::load(&output).unwrap();
    for state in 0..=9 {
        assert!(
            loaded
                .eye(128332, state)
                .unwrap()
                .iter()
                .all(|value| value.is_finite())
        );
    }
}
