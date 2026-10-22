//! Same observation-only contract for baseline and healer-policy batches.
#[test]
fn healer_triage_benchmark_records_native_pull_end_mana_and_hot_uptime() {
    let report = super::campaign::run(1).unwrap();
    let metrics = &report.performance["healer_metrics"];
    let pulls = metrics["pull_ends"]
        .as_array()
        .expect("native pull-end observations");
    assert!(
        !pulls.is_empty(),
        "pull-end mana belongs to the real last pack death, not next-pull recovery"
    );
    for pull in pulls {
        assert!(
            pull["last_native_death_at"].as_f64().unwrap() >= pull["engaged_at"].as_f64().unwrap()
        );
        match (
            pull["healer_mana"].as_f64(),
            pull["healer_max_mana"].as_f64(),
        ) {
            (Some(mana), Some(max)) => assert!((0.0..=max).contains(&mana)),
            (None, None) => {
                assert!(pull["healer_mana"].is_null() && pull["healer_max_mana"].is_null())
            }
            _ => panic!("native mana current/max must be observed together"),
        }
    }
    assert_eq!(metrics["sample_period_seconds"], 0.05);
    let combat = metrics["combat_seconds"].as_f64().unwrap();
    let alive_tank = metrics["alive_tank_combat_seconds"].as_f64().unwrap();
    assert!(combat > 0.0 && alive_tank <= combat);
    for seconds in metrics["tank_hot_seconds"].as_object().unwrap().values() {
        assert!((0.0..=alive_tank + 1e-6).contains(&seconds.as_f64().unwrap()));
    }
    for usage in metrics["proc_usage"].as_object().unwrap().values() {
        let count = |field: &str| usage[field].as_u64().unwrap();
        let available = count("initial_charges") + count("charges_gained");
        assert_eq!(
            available,
            count("consumed")
                + count("expired")
                + count("other_removed")
                + count("remaining_charges"),
            "native observed charges must reconcile without inferring consumption from AuraRemoved"
        );
        if available == 0 {
            assert!(
                usage["charge_consumption_rate"].is_null(),
                "no native proc opportunities are N/A, not zero-percent use"
            );
        } else {
            assert!((0.0..=1.0).contains(&usage["charge_consumption_rate"].as_f64().unwrap()));
        }
    }
}
