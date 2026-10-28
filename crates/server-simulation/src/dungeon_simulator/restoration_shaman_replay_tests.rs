use std::path::Path;

#[test]
fn zero_capacity_charge_category_keeps_native_spell_cooldown() {
    use crate::spell_cast::test_harness::{Inbox, harness};
    // Native Dark Soul: Instability has category1447 with zero base charges
    // and its own 120-second recovery.
    let mut h = harness(Vec::new(), &[113858], 9).unwrap();
    h.cast_and_deliver(113858, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(31);
    h.inbox = Inbox::default();
    h.cast_and_deliver(113858, None);
    assert_eq!(
        h.inbox.failed[0].reason,
        shared::spell_data::CastFailReason::OnCooldown
    );
}

#[test]
#[ignore = "replays the MID2 Restoration Shaman against the SimC dummy trace"]
fn restoration_shaman_heals_the_simc_dummy_party() {
    let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let report =
        super::super::healer_replay::replay_simc(264, Path::new(&trace), 300, 70448).unwrap();
    let metrics = &report["metrics"];
    let effective = metrics["effective_hps"].as_f64().unwrap();
    let raw = metrics["raw_hps"].as_f64().unwrap();
    assert!(report["incoming_events"].as_u64().unwrap() > 100);
    assert!(effective > 0.0 && raw >= effective, "{report}");
    let spells = metrics["spell_healing"].as_object().unwrap();
    assert!(!spells.is_empty());
    let spell_total: f64 = spells
        .values()
        .map(|spell| spell["effective_hps"].as_f64().unwrap())
        .sum();
    assert!((spell_total - effective).abs() < 0.000001);
    assert_eq!(metrics["mana_curve"].as_array().unwrap().len(), 301);
    assert!(metrics["tank_dtps"].as_f64().unwrap() > 0.0, "{report}");
    assert!(
        metrics["tank_health_min"].as_f64().unwrap() >= 0.0,
        "{report}"
    );
    assert!(
        metrics["mana_min"].as_f64().unwrap()
            <= metrics["mana_curve"][0]["current"].as_f64().unwrap(),
        "{report}"
    );
    assert!(
        report["healer_casts"]["974"].as_u64().unwrap_or(0) > 0,
        "Earth Shield must be cast: {report}"
    );
    let streams = report["healer_casts"]["5394"].as_u64().unwrap_or(0);
    assert!(
        (1..=22).contains(&streams),
        "native 30s Healing Stream charges must bound casts: {report}"
    );
    assert!(
        metrics["spell_healing"]["52042"]["effective_hps"]
            .as_f64()
            .unwrap_or(0.0)
            > 0.0,
        "real Healing Stream Totem must heal the injured party: {report}"
    );
    assert!(
        report["healer_casts"]["77472"].as_u64().unwrap_or(0) > 0,
        "Healing Wave must be available between totem charges: {report}"
    );
    println!("RESTORATION_SHAMAN_REPORT={report}");
}
