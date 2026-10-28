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
    let metrics = &report["metrics"];
    let effective = metrics["effective_hps"].as_f64().unwrap();
    let raw = metrics["raw_hps"].as_f64().unwrap();
    assert!(effective > 0.0 && raw >= effective);
    let spells = metrics["spell_healing"].as_object().unwrap();
    let per_spell: f64 = spells
        .values()
        .map(|spell| spell["effective_hps"].as_f64().unwrap())
        .sum();
    assert!((effective - per_spell).abs() < 1e-6);
    let mana = metrics["mana_curve"].as_array().unwrap();
    assert!(mana.len() > 1);
    assert!(
        mana.windows(2)
            .all(|pair| pair[0]["time_seconds"].as_f64().unwrap()
                < pair[1]["time_seconds"].as_f64().unwrap())
    );
    assert_eq!(
        metrics["first_death_seconds"].is_null(),
        metrics["party_deaths"] == 0
    );
    println!(
        "| spec | effective HPS | raw HPS | overheal % | first death (s) |\n| 65 | {effective:.2} | {raw:.2} | {} | {} |",
        metrics["overheal_pct"], metrics["first_death_seconds"]
    );
    println!("spell_healing={spells:?}; mana_samples={}", mana.len());
}
