//! Eye points from the local CASC extraction (game-engine `data/`) and the server's
//! world.db: playable races and Northshire creatures. Heights are model-space Z in yards.
#![cfg(feature = "bake")]

use std::path::PathBuf;

use los::bake::eye::eye_rows_of;
use los::eye::{EyeRow, EyeSource};

/// `ChrModel` displays of human male and female, gnome male, tauren male.
const HUMAN_MALE: u32 = 57899;
const HUMAN_FEMALE: u32 = 56658;
const GNOME_MALE: u32 = 51877;
const TAUREN_MALE: u32 = 55077;
/// Kobold Vermin (6), Diseased Timber Wolf (69), Blackrock Invader (42937).
const KOBOLD: u32 = 10913;
const TIMBER_WOLF: u32 = 31048;
const INVADER: u32 = 33142;
const STAND: u8 = 0;
const SIT: u8 = 1;

fn rows(displays: &[u32]) -> Vec<EyeRow> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let (rows, stats) = eye_rows_of(
        &root.join("../../../game-engine/data"),
        &root.join("../../data/world.db"),
        displays,
    )
    .unwrap();
    assert!(stats.errors.is_empty(), "{:?}", stats.errors);
    for row in &rows {
        println!(
            "{}\t{}\t{:?}\t{:.3}\t{:.3}\t{:.3}",
            row.display, row.stand_state, row.source, row.eye[0], row.eye[1], row.eye[2]
        );
    }
    rows
}

fn eye(rows: &[EyeRow], display: u32, state: u8) -> &EyeRow {
    rows.iter()
        .find(|row| (row.display, row.stand_state) == (display, state))
        .unwrap_or_else(|| panic!("no eye for display {display} state {state}"))
}

#[test]
fn human_male_eyes_drop_when_he_sits() {
    let rows = rows(&[HUMAN_MALE]);
    let stand = eye(&rows, HUMAN_MALE, STAND);
    let sit = eye(&rows, HUMAN_MALE, SIT);
    assert_eq!(stand.source, EyeSource::Eyes);
    // A ~1.9 yd tall man's eyes, a little under the top of his head and in front of the
    // body's axis.
    assert!((1.6..1.9).contains(&stand.eye[2]), "stand {:?}", stand.eye);
    assert!(stand.eye[0] > 0.0, "the eyes face forward: {:?}", stand.eye);
    // Sitting on the ground puts them at roughly half his standing eye height.
    assert!(
        (0.4..0.75).contains(&(sit.eye[2] / stand.eye[2])),
        "sit {:?} vs stand {:?}",
        sit.eye,
        stand.eye
    );
}

#[test]
fn playable_races_and_creatures_have_eye_points_in_proportion() {
    let displays = [
        HUMAN_MALE,
        HUMAN_FEMALE,
        GNOME_MALE,
        TAUREN_MALE,
        KOBOLD,
        TIMBER_WOLF,
        INVADER,
    ];
    let rows = rows(&displays);
    let height = |display| eye(&rows, display, STAND).eye[2];
    assert!(height(GNOME_MALE) < 0.6 * height(HUMAN_MALE));
    assert!(height(HUMAN_FEMALE) < height(HUMAN_MALE));
    assert!(height(TAUREN_MALE) > height(HUMAN_MALE));
    // A wolf's head is lower than a standing man's eyes.
    assert!(height(TIMBER_WOLF) < height(HUMAN_MALE));
    for display in displays {
        assert!(
            height(display) > 0.3,
            "display {display} eye above its feet"
        );
    }
}
