//! Native Neltharus encounter displays must have source-model LOS eye points.
#![cfg(feature = "bake")]

use std::{fs, path::PathBuf};

use los::bake::eye::eye_rows_of;
use los::eye::EyeTable;
use rusqlite::{Connection, OpenFlags};

#[test]
#[ignore = "requires NELTHARUS_WORLD_DB, NELTHARUS_ENGINE_DATA and NELTHARUS_EYE_OUTPUT"]
fn neltharus_boss_models_bake_and_load_all_eye_poses() {
    let database = PathBuf::from(std::env::var("NELTHARUS_WORLD_DB").unwrap());
    let data = PathBuf::from(std::env::var("NELTHARUS_ENGINE_DATA").unwrap());
    let output = PathBuf::from(std::env::var("NELTHARUS_EYE_OUTPUT").unwrap());
    let connection =
        Connection::open_with_flags(&database, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut query = connection.prepare("SELECT DISTINCT m.CreatureDisplayID FROM content_creature_template_model m JOIN content_creature c ON c.id1=m.CreatureID WHERE c.map=2519 AND c.id1 IN (181861,189340,189478,189901) ORDER BY 1").unwrap();
    let displays: Vec<u32> = query
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(displays, vec![102604, 107029, 107189, 108248]);
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
    let eyes = EyeTable::load(&output).unwrap();
    for display in displays {
        for state in 0..=9 {
            assert!(
                eyes.eye(display, state)
                    .unwrap()
                    .iter()
                    .all(|value| value.is_finite())
            );
        }
    }
}
