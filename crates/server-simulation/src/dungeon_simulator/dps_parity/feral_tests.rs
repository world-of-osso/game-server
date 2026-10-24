//! Spec103 fixture proof; production dungeon control remains outside this adapter.
use super::*;

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec103 exact profile/traits"]
fn feral_native_kit_does_not_cast_an_unselected_talent() {
    let mut run = prepare(103, 1).expect("native Feral selected-trait fixture");
    let traits = run
        .app
        .world()
        .get::<crate::class_progression::TraitLoadout>(run.actor)
        .unwrap();
    assert!(
        traits
            .active_talent_spells()
            .all(|talent| talent.spell_id != 102543)
    );
    engage(&mut run);
    for _ in 0..600 {
        run.app.update();
    }
    let combat = report::combat(run.app.world(), &run.damage_targets, 30.0);
    let actions = combat["actions"].as_array().unwrap();
    let casts = |id| {
        actions
            .iter()
            .filter(|action| action["spell_id"] == id)
            .map(|action| action["count"].as_u64().unwrap())
            .sum::<u64>()
    };
    assert!(casts(5221) > 0, "native granted Shred remains usable");
    assert_eq!(
        casts(102543),
        0,
        "dungeon kit must not admit Incarnation absent from actual selected traits"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec103 exact profile/traits"]
fn feral_literal_fixture_records_native_bleeds_for_300s() {
    let count: usize = std::env::var("DPS_PARITY_TARGETS")
        .expect("DPS_PARITY_TARGETS")
        .parse()
        .expect("numeric target count");
    assert!(matches!(count, 1 | 3));
    let report = measure(103, count).expect("native Feral fixture");
    let output = inputs::directory()
        .unwrap()
        .join(format!("103.feral-literal.{count}.json"));
    std::fs::write(output, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    assert_eq!(report["driver"], "LITERAL_SIMC_FERAL_76f8118");
    assert!((report["simulated_seconds"].as_f64().unwrap() - 300.0).abs() < 0.001);
    assert!(report["combat"]["dps"].as_f64().unwrap() > 0.0);
}
