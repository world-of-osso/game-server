//! Owned Leymor body input; no tile rebake, guessed eyes or canonical writes.
#![cfg(feature = "bake")]

use los::bake::eye::eye_rows_of;
use los::bake::m2::M2File;
use los::eye::EyeTable;
use rusqlite::{Connection, OpenFlags};
use std::{fs, path::PathBuf};

const LEYMOR_DISPLAY: u32 = 107127;
const LEYMOR_MODEL: u32 = 4279060;

#[test]
#[ignore = "requires AZURE_EYE_WORLD_DB, AZURE_EYE_MODEL_DATA and AZURE_EYE_OUTPUT"]
fn azure_leymor_source_model_bakes_ten_native_eye_poses() {
    let database = PathBuf::from(std::env::var_os("AZURE_EYE_WORLD_DB").unwrap());
    let data = PathBuf::from(std::env::var_os("AZURE_EYE_MODEL_DATA").unwrap());
    let output = PathBuf::from(std::env::var_os("AZURE_EYE_OUTPUT").unwrap());
    let db = Connection::open_with_flags(&database, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let model: u32 = db.query_row(
        "SELECT model.FileDataID FROM creature_display_info display JOIN creature_model_data model ON model.ID=display.ModelID WHERE display.ID=?",
        [LEYMOR_DISPLAY], |row| row.get(0),
    ).unwrap();
    assert_eq!(
        model, LEYMOR_MODEL,
        "use Leymor's actual native display/model binding"
    );
    let bytes = fs::read(data.join(format!("models/{model}.m2"))).unwrap();
    let parsed = M2File::parse(&bytes).unwrap();
    println!(
        "AZURE_EYE_DEPENDENCIES model={model} skin={:?} skeleton={:?} animations={:?}",
        parsed.skin_fdid, parsed.skeleton_fdid, parsed.anim_files
    );
    let (rows, stats) = eye_rows_of(&data, &database, &[LEYMOR_DISPLAY], &[]).unwrap();
    assert_eq!(stats.displays, 1);
    assert_eq!(rows.len(), 10);
    assert!(stats.skipped.is_empty());
    assert!(rows.iter().all(|row| row.display == LEYMOR_DISPLAY));
    fs::create_dir_all(&output).unwrap();
    fs::write(output.join("eye_heights.tsv"), EyeTable::to_tsv(&rows)).unwrap();
    fs::copy(
        data.join("los/player_displays.tsv"),
        output.join("player_displays.tsv"),
    )
    .unwrap();
    let loaded = EyeTable::load(&output).unwrap();
    for state in 0..=9 {
        let eye = loaded
            .eye(LEYMOR_DISPLAY, state)
            .expect("every native runtime pose needs a sourced eye");
        assert!(eye.iter().all(|coordinate| coordinate.is_finite()));
    }
    println!("AZURE_EYE_SOURCE {stats:?}");
}
