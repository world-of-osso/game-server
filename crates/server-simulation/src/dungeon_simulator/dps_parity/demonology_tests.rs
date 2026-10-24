//! Native fixture proof: the pinned Demo priority must actually summon its selected grimoire.
use serde_json::Value;

#[test]
#[ignore = "300s native Demo literal APL; requires private DB, spec266 profile and DPS_PARITY_TARGETS=1|3"]
fn dps_parity_demo_literal_executes_selected_grimoire_without_native_kit_curses() {
    assert_eq!(std::env::var("DPS_PARITY_SPEC").unwrap(), "266");
    super::dps_parity_native_bis_dummy_records_300s();
    let count = std::env::var("DPS_PARITY_TARGETS").unwrap();
    let report: Value = super::inputs::read_json(
        &super::inputs::directory()
            .unwrap()
            .join(format!("266.native.{count}.json")),
    )
    .unwrap();
    let actions = report["combat"]["actions"].as_array().unwrap();
    let casts = |spell| {
        actions
            .iter()
            .filter(|row| row["spell_id"].as_u64() == Some(spell))
            .map(|row| row["count"].as_u64().unwrap())
            .sum::<u64>()
    };
    assert!(
        casts(1276452) > 0,
        "literal Diabolist must cast selected Grimoire: Imp Lord1276452, not only the old three-action kit"
    );
    assert!(
        casts(686) > 0,
        "literal priority must reach native Shadow Bolt"
    );
    assert!(
        casts(105174) > 0,
        "literal priority must spend earned shards"
    );
    assert_eq!(
        casts(702),
        0,
        "Curse of Weakness is absent from the pinned APL"
    );
    assert_eq!(
        report["driver"], "LITERAL_SIMC_DEMONOLOGY_76f8118",
        "fixture report must identify its actual controller"
    );
}
