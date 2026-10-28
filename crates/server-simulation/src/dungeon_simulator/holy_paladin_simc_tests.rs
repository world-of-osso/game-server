use std::path::Path;

#[test]
#[ignore = "uses the MID2 Holy player and native Avenging Wrath"]
fn holy_avenging_wrath_enables_and_removes_hammer_override() {
    let mut run = super::super::dps_parity::prepare_registered_player(66, 1).unwrap();
    let actor = super::super::dps_parity::spawn_second_reference_player(&mut run, 65).unwrap();
    let enemy = run.targets[0];
    let resolve = |world: &bevy::prelude::World| {
        crate::spell_cast::knowledge::resolve_actor_action(world, actor, 20271).unwrap()
    };
    assert_eq!(resolve(run.app.world()), 275773);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 31884, Some(actor), false)
        .unwrap();
    run.app.update();
    assert!(crate::auras::has_aura(run.app.world(), actor, 1241410));
    let hammer = resolve(run.app.world());
    assert_eq!(hammer, 1241413);
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, hammer, Some(enemy), false)
        .unwrap();
    run.app.update();
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(hammer)
                && event.kind == shared::protocol::CombatLogKind::CastSuccess)
    );
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 31884,
    );
    assert!(!crate::auras::has_aura(run.app.world(), actor, 1241410));
    assert_eq!(resolve(run.app.world()), 275773);
}

#[test]
#[ignore = "replays the approved MID2 Holy Paladin against the SimC dummy trace"]
fn holy_paladin_heals_the_simc_dummy_party() {
    let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let report =
        super::super::healer_replay::replay_simc(65, Path::new(&trace), 300, 70448).unwrap();
    println!(
        "HOLY_SUMMARY casts={} healing={} death={}",
        report["healer_casts"],
        report["metrics"]["spell_healing"],
        report["metrics"]["first_death_seconds"]
    );
    let metrics = &report["metrics"];
    let effective = metrics["effective_hps"].as_f64().unwrap();
    let raw = metrics["raw_hps"].as_f64().unwrap();
    assert_eq!(report["tank_spec"], 66);
    assert!(report["incoming_events"].as_u64().unwrap() > 100);
    assert!(
        report["tank_casts"]
            .as_object()
            .unwrap()
            .values()
            .any(|x| x.as_u64().unwrap() > 0)
    );
    assert!(
        report["healer_casts"]["20473"].as_u64().unwrap_or(0) > 0,
        "{report}"
    );
    assert!(effective > 0.0 && raw >= effective, "{report}");
    assert!(
        metrics["idle_gcd_pct"].as_f64().unwrap() < 60.45,
        "Holy Paladin idle time did not improve from the prior policy: {report}"
    );
    assert!(
        ["20271", "275773", "35395", "24275", "26573"]
            .iter()
            .any(|spell| report["healer_casts"][*spell].as_u64().unwrap_or(0) > 0),
        "Holy Paladin did not cast damage filler: {report}"
    );
    assert_eq!(metrics["mana_curve"].as_array().unwrap().len(), 301);
    println!("HOLY_PALADIN_REPORT={report}");
}
