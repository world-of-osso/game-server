use super::healer_intake::Intake;
use std::path::Path;

#[test]
#[ignore = "replays a complete recorded Stockade intake against the canonical world database"]
fn holy_paladin_heals_recorded_native_incoming_damage() {
    let canonical = Path::new("/home/osso/Projects/world-of-osso/game-server/data/world.db")
        .canonicalize()
        .unwrap();
    assert_eq!(
        crate::test_data::world_db_path().canonicalize().unwrap(),
        canonical
    );
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/dungeons/healer-benchmark/stockade-seed1.intake.json");
    let intake: Intake = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let report = super::healer_replay::replay(&intake, 65).unwrap();
    assert_eq!(report["status"], "native");
    assert_eq!(report["intake_hits"], intake.hits.len());
    assert!(report["native_damage_receipts"].as_u64().unwrap() > 0);
    assert!(
        report["metrics"]["effective_healing"].as_f64().unwrap() > 0.0,
        "{report}"
    );
}
