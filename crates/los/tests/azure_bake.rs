//! Reproducible source-model eye fragment for Azure's native follower population.
#![cfg(feature = "bake")]

use std::{collections::BTreeSet, fs, path::PathBuf};

use los::bake::eye::{eye_rows_of, player_displays};
use los::eye::EyeTable;
use rusqlite::{Connection, OpenFlags};

#[test]
#[ignore = "requires AZURE_WORLD_DB, AZURE_ENGINE_DATA and AZURE_EYE_OUTPUT"]
fn azure_follower_and_player_models_bake_every_runtime_eye_pose() {
    let database = PathBuf::from(std::env::var("AZURE_WORLD_DB").unwrap());
    let data = PathBuf::from(std::env::var("AZURE_ENGINE_DATA").unwrap());
    let output = PathBuf::from(std::env::var("AZURE_EYE_OUTPUT").unwrap());
    let connection =
        Connection::open_with_flags(&database, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut query = connection
        .prepare(
            "SELECT DISTINCT m.CreatureDisplayID FROM content_creature_template_model m
             WHERE m.CreatureID IN (
                 SELECT id1 FROM content_creature WHERE map=2515
                 AND (','||spawnDifficulties||',') LIKE '%,1,%'
             ) OR m.CreatureID IN (SELECT entry FROM follower_profiles)
             UNION SELECT modelid FROM content_creature WHERE map=2515 AND modelid!=0
                 AND (','||spawnDifficulties||',') LIKE '%,1,%'
             ORDER BY 1",
        )
        .unwrap();
    let mut displays: BTreeSet<u32> = query
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(displays.contains(&109087), "Telash's sourced display");
    for display in [113752, 113753, 113757] {
        assert!(displays.contains(&display), "imported follower {display}");
    }
    let players = player_displays(&data).unwrap();
    assert!(!players.is_empty(), "installed playable race/sex displays");
    displays.extend(players.iter().map(|(_, display)| *display));
    let displays: Vec<u32> = displays.into_iter().collect();
    let (rows, stats) = eye_rows_of(&data, &database, &displays, &[]).unwrap();
    assert_eq!(stats.displays, displays.len());
    assert_eq!(rows.len(), displays.len() * 10);
    assert!(stats.skipped.is_empty());

    fs::create_dir_all(&output).unwrap();
    fs::write(output.join("eye_heights.tsv"), EyeTable::to_tsv(&rows)).unwrap();
    fs::write(
        output.join("player_displays.tsv"),
        EyeTable::players_to_tsv(&players),
    )
    .unwrap();
    let loaded = EyeTable::load(&output).unwrap();
    for display in &displays {
        for state in 0..=9 {
            let point = loaded
                .eye(*display, state)
                .expect("every required runtime pose has baked eyes");
            assert!(point.iter().all(|coordinate| coordinate.is_finite()));
        }
    }
    for ((race, sex), display) in &players {
        assert_eq!(loaded.player_display(*race, *sex), Some(*display));
    }
    println!(
        "{} required Azure/follower/player displays, {} eye rows, {} player mappings; \
         eyes {}, head {}, bounds {}, authored Stand holds {}",
        stats.displays,
        rows.len(),
        players.len(),
        stats.from_eyes,
        stats.from_head,
        stats.from_bounds,
        stats.stand_pose,
    );
}
