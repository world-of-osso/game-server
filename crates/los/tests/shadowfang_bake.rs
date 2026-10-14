//! Private source-model eye receipt for native Silverlaine acceptance.
#![cfg(feature = "bake")]

use los::bake::eye::eye_rows_of;
use los::eye::EyeTable;
use rusqlite::{Connection, OpenFlags};
use std::{fs, path::PathBuf};

const SILVERLAINE: u32 = 3887;
const SILVERLAINE_DISPLAY: u32 = 37288;

#[test]
#[ignore = "requires SHADOWFANG_WORLD_DB, SHADOWFANG_ENGINE_DATA and SHADOWFANG_EYE_OUTPUT"]
fn shadowfang_silverlaine_bakes_all_source_eye_poses() {
    let database = PathBuf::from(std::env::var("SHADOWFANG_WORLD_DB").unwrap());
    let data = PathBuf::from(std::env::var("SHADOWFANG_ENGINE_DATA").unwrap());
    let output = PathBuf::from(std::env::var("SHADOWFANG_EYE_OUTPUT").unwrap());
    let connection =
        Connection::open_with_flags(&database, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut query = connection.prepare(
        "SELECT DISTINCT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID=? ORDER BY 1",
    ).unwrap();
    let displays: Vec<u32> = query
        .query_map([SILVERLAINE], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(displays, vec![SILVERLAINE_DISPLAY]);
    let (rows, stats) = eye_rows_of(&data, &database, &displays, &[]).unwrap();
    assert_eq!(stats.displays, displays.len());
    assert!(
        stats.skipped.is_empty(),
        "source-model eye bake has no substituted model"
    );
    fs::create_dir_all(&output).unwrap();
    fs::write(output.join("eye_heights.tsv"), EyeTable::to_tsv(&rows)).unwrap();
    fs::copy(
        data.join("los/player_displays.tsv"),
        output.join("player_displays.tsv"),
    )
    .unwrap();
    let loaded = EyeTable::load(&output).unwrap();
    for state in 0..=9 {
        let point = loaded
            .eye(SILVERLAINE_DISPLAY, state)
            .expect("authored pose eye baked");
        assert!(point.iter().all(|coordinate| coordinate.is_finite()));
    }
    println!(
        "{} Silverlaine display, {} source eye poses",
        stats.displays,
        rows.len()
    );
}
