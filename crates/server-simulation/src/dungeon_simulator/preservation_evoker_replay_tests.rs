use std::path::Path;

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn temporal_anomaly_shields_the_party_with_its_description_amount() {
    let mut run = super::healer_simc_replay::snapshot_party(1468, 90, 1);
    let healer = run.healer;
    super::healer_simc_replay::hold(&mut run, healer);
    crate::spell_cast::creature_cast(run.app.world_mut(), healer, 373861, None, false)
        .expect("Temporal Anomaly cast starts");
    for _ in 0..60 {
        run.app.update();
    }
    let stats = *run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(healer)
        .unwrap();
    // DescriptionVariables 746: $shield=${$SP*6*(1+$@versadmg)}.
    let expected = stats.healing_power * 6.0 * (1.0 + stats.versatility_pct / 100.0);
    assert!(expected > 1000.0, "{expected}");
    for unit in run.slots {
        let shield = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(unit)
            .and_then(|auras| auras.iter().find(|aura| aura.spell_id == 373862).cloned())
            .unwrap_or_else(|| panic!("party member {unit:?} has no Temporal Anomaly shield"));
        assert_eq!(shield.caster, healer);
        let amount = shield.effects.iter().find(|e| e.index == 0).unwrap().amount;
        assert!((amount - expected).abs() <= 1.0, "{amount} vs {expected}");
    }
    let tank = run.tank;
    let (remaining, absorbed) =
        crate::auras::absorb_damage(run.app.world_mut(), tank, None, 1, expected + 500.0);
    assert!((absorbed - expected).abs() <= 1.0, "absorbed {absorbed}");
    assert!((remaining - 500.0).abs() <= 1.0, "remaining {remaining}");
    assert!(!crate::auras::has_aura(run.app.world(), tank, 373862));
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot and SimC traces"]
fn preservation_casts_fire_breath_and_heals_from_damage() {
    let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let party = std::env::var("HEALER_TANK_PARTY_TRACE").expect("HEALER_TANK_PARTY_TRACE required");
    let report = super::healer_simc_replay::replay_simc_party(
        1468,
        90,
        Path::new(&trace),
        Path::new(&party),
        300,
        70448,
    )
    .unwrap();
    assert!(
        report["healer_casts"]["357208"].as_u64().unwrap_or(0) > 0,
        "Fire Breath was never cast: {}",
        report["healer_casts"]
    );
    assert!(
        report["metrics"]["spell_healing"]["371441"]["raw"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "Life-Giver's Flame did not heal: {}",
        report["metrics"]["spell_healing"]
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot and SimC traces"]
fn preservation_evoker_heals_the_simc_dummy_party() {
    let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let report =
        super::healer_simc_replay::replay_simc(1468, 90, Path::new(&trace), 300, 70448).unwrap();
    let metrics = &report["metrics"];
    let effective = metrics["effective_hps"].as_f64().unwrap();
    let raw = metrics["raw_hps"].as_f64().unwrap();
    assert!(report["incoming_events"].as_u64().unwrap() > 100);
    assert!(
        report["healer_casts"]["364343"].as_u64().unwrap_or(0) > 0,
        "Echo never cast: {report}"
    );
    assert!(
        report["healer_casts"]["355936"].as_u64().unwrap_or(0) > 0,
        "Dream Breath never cast: {report}"
    );
    assert_eq!(report["tank_spec"], 66);
    assert!(report["tank_active_mitigation_seconds"].as_f64().unwrap() > 0.0);
    assert!(
        report["healer_casts"]["366155"].as_u64().unwrap_or(0) > 0,
        "ordinary Reversion never cast: {report}"
    );
    assert!(
        metrics["first_death_seconds"]
            .as_f64()
            .is_none_or(|time| time > 4.0)
    );
    assert!(metrics["tank_dtps"].as_f64().unwrap() > 0.0);
    assert_eq!(metrics["tank_health_curve"].as_array().unwrap().len(), 301);
    assert!(effective > 0.0 && raw >= effective, "{report}");
    let spells = metrics["spell_healing"].as_object().unwrap();
    assert!(!spells.is_empty());
    let spell_total: f64 = spells
        .values()
        .map(|spell| spell["effective_hps"].as_f64().unwrap())
        .sum();
    assert!((spell_total - effective).abs() < 0.000001);
    assert_eq!(metrics["mana_curve"].as_array().unwrap().len(), 301);
    println!("PRESERVATION_EVOKER_REPORT={report}");
}
