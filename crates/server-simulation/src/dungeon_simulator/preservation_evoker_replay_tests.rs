use std::path::Path;

fn echo_percent(
    world: &bevy::prelude::World,
    caster: bevy::prelude::Entity,
    unit: bevy::prelude::Entity,
) -> Option<f32> {
    world
        .get::<crate::auras::ActiveAuras>(unit)?
        .iter()
        .find(|aura| aura.spell_id == 364343 && aura.caster == caster)?
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .map(|effect| effect.amount)
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn echo_replicates_living_flame_on_every_holder_including_anomaly_echoes() {
    use shared::protocol::CombatLogKind;
    let mut run = super::healer_simc_replay::snapshot_party(1468, 90, 1);
    let healer = run.healer;
    super::healer_simc_replay::hold(&mut run, healer);
    let [_, _, full_holder, primary, _] = run.slots;
    crate::spell_cast::cast_triggered(run.app.world_mut(), healer, 364343, Some(full_holder), 1);
    let full = echo_percent(run.app.world(), healer, full_holder).expect("Echo applied");
    crate::spell_cast::creature_cast(run.app.world_mut(), healer, 373861, None, false)
        .expect("Temporal Anomaly cast starts");
    for _ in 0..60 {
        run.app.update();
    }
    // Anomaly keeps an existing Echo and adds a 373861 s4 (30%) Echo elsewhere.
    assert_eq!(
        echo_percent(run.app.world(), healer, full_holder),
        Some(full)
    );
    let partial = full * 0.3;
    for unit in run.slots.into_iter().filter(|&unit| unit != full_holder) {
        let percent = echo_percent(run.app.world(), healer, unit)
            .unwrap_or_else(|| panic!("{unit:?} has no Anomaly Echo"));
        assert!((percent - partial).abs() < 0.01, "{percent} vs {partial}");
    }
    for _ in 0..40 {
        run.app.update();
    }
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(healer)
        .unwrap()
        .crit_pct = 0.0;
    let cursor = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    crate::spell_cast::creature_cast(run.app.world_mut(), healer, 361469, Some(primary), false)
        .expect("Living Flame cast starts");
    for _ in 0..80 {
        run.app.update();
    }
    let heals: Vec<_> = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[cursor..]
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Heal
                && event.spell_id == Some(361509)
                && event.source == Some(healer.to_bits())
                && !event.periodic
        })
        .map(|event| (event.target.unwrap(), event.amount as f32, event.crit))
        .collect();
    assert!(heals.iter().all(|heal| !heal.2), "{heals:?}");
    let on = |unit: bevy::prelude::Entity| -> Vec<f32> {
        heals
            .iter()
            .filter(|heal| heal.0 == unit.to_bits())
            .map(|heal| heal.1)
            .collect()
    };
    // Full-health targets: raw amounts share one mastery and differ only by Echo.
    let mut on_primary = on(primary);
    on_primary.sort_by(f32::total_cmp);
    assert_eq!(
        on_primary.len(),
        2,
        "root heal and its Echo copy: {heals:?}"
    );
    let root = on_primary[1];
    assert!(
        (on_primary[0] / root - partial / 100.0).abs() < 0.01,
        "{heals:?}"
    );
    let full_copy = on(full_holder);
    assert_eq!(full_copy.len(), 1, "{heals:?}");
    assert!(
        (full_copy[0] / root - full / 100.0).abs() < 0.01,
        "{heals:?}"
    );
    for unit in run.slots {
        assert_eq!(
            echo_percent(run.app.world(), healer, unit),
            None,
            "Echo consumed"
        );
        if unit != primary && unit != full_holder {
            assert_eq!(
                on(unit).len(),
                1,
                "Anomaly Echo copy on {unit:?}: {heals:?}"
            );
        }
    }
}

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
