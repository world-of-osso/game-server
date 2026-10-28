use super::healer_intake::Intake;
use std::path::Path;

#[test]
#[ignore = "requires the MID2 Protection Paladin profile and canonical world database"]
fn protection_paladin_uses_parity_player_profile_and_equipment() {
    let canonical = Path::new("/home/osso/Projects/world-of-osso/game-server/data/world.db")
        .canonicalize()
        .unwrap();
    assert_eq!(
        crate::test_data::world_db_path().canonicalize().unwrap(),
        canonical
    );
    assert_eq!(
        Path::new(&std::env::var("DPS_PARITY_WORLD_DB").unwrap())
            .canonicalize()
            .unwrap(),
        canonical
    );
    let (profile, traits) = super::dps_parity::read_reference(66).unwrap();
    assert_eq!(profile.level, "90");
    assert_eq!(profile.race, "tauren");
    let mut expected: Vec<_> = profile.equipment.iter().map(|item| item.item_id).collect();
    expected.sort_unstable();
    let run = super::dps_parity::spawn_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 6 },
    )
    .unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<shared::components::Level>(run.actor)
            .unwrap()
            .0,
        90
    );
    let mut actual: Vec<_> = run
        .app
        .world()
        .get::<crate::stats::EquippedItems>(run.actor)
        .unwrap()
        .items
        .iter()
        .map(|(_, _, id)| *id)
        .collect();
    actual.sort_unstable();
    assert_eq!(actual, expected);
    assert_eq!(run.targets.len(), 1);
}

#[test]
#[ignore = "replays a recorded Stockade intake against the canonical world database"]
fn protection_paladin_replays_recorded_damage_without_healer() {
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
    let report = super::tank_replay::replay(&intake).unwrap();
    assert_eq!(report["spec"], 66);
    let raw = report["raw_damage"].as_f64().unwrap();
    let taken = report["damage_taken"].as_f64().unwrap();
    assert!(raw > 0.0 && taken > 0.0 && taken <= raw, "{report}");
    assert!(
        (report["mitigation_pct"].as_f64().unwrap() - 100.0 * (raw - taken) / raw).abs() < 1e-6
    );
    let duration = report["duration_seconds"].as_f64().unwrap();
    assert!(duration > 0.0);
    assert!((0.0..=1.0).contains(&report["active_mitigation_uptime"].as_f64().unwrap()));
    assert!(report["largest_spike"].as_f64().unwrap() > 0.0);
    assert!(
        !report["tank_casts"].as_object().unwrap().is_empty(),
        "{report}"
    );
    if !report["time_to_death"].is_null() {
        assert_eq!(report["time_to_death"].as_f64().unwrap(), duration);
    }
    println!("TANK_REPLAY {report}");
}
