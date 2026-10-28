use std::path::Path;

#[test]
#[ignore = "requires native MID2 tank, Druid and canonical world DB"]
fn restoration_druid_multi_injury_uses_area_heals() {
    let mut run = super::super::dps_parity::prepare_registered_player(66, 1).unwrap();
    let healer = super::super::dps_parity::spawn_second_reference_player(&mut run, 105).unwrap();
    let tank = run.actor;
    let faction = *run
        .app
        .world()
        .get::<shared::components::UnitFactionTemplate>(tank)
        .unwrap();
    run.app.world_mut().entity_mut(healer).insert(faction);
    let party = crate::followers::simulator::spawn_party_for_reference_healer(
        run.app.world_mut(),
        tank,
        healer,
    )
    .unwrap();
    let members: Vec<_> = party.members.iter().copied().chain([tank]).collect();
    for &unit in &members {
        if let Some(mut health) = run
            .app
            .world_mut()
            .get_mut::<shared::components::Health>(unit)
        {
            health.current = health.max * 0.5;
        }
    }
    let known = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(healer)
        .unwrap();
    println!(
        "DRUID_KNOWN efflo={} wild_growth={} guardians={} convoke={}",
        known.contains(145205),
        known.contains(48438),
        known.contains(102693),
        known.contains(391528)
    );
    let mut policy = super::prepare(&mut run, healer).unwrap();
    for _ in 0..200 {
        policy.drive(run.app.world_mut(), healer, &members).unwrap();
        run.app.update();
    }
    let world = run.app.world_mut();
    let efflo = world
        .query::<&crate::spell_cast::summon::SummonedCreature>()
        .iter(world)
        .any(|summon| summon.owner == healer && summon.spell_id == 145205);
    assert!(efflo, "multi-injury must create Efflorescence");
    assert!(
        members
            .iter()
            .any(|unit| crate::auras::has_aura(world, *unit, 48438)),
        "multi-injury must apply Wild Growth"
    );
}

#[test]
#[ignore = "replays the MID2 Restoration Druid against the SimC dummy trace"]
fn restoration_druid_heals_the_simc_dummy_party() {
    let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let report =
        super::super::healer_replay::replay_simc(105, Path::new(&trace), 300, 70448).unwrap();
    let metrics = &report["metrics"];
    let effective = metrics["effective_hps"].as_f64().unwrap();
    let raw = metrics["raw_hps"].as_f64().unwrap();
    assert!(report["incoming_events"].as_u64().unwrap() > 100);
    assert!(effective > 0.0 && raw >= effective, "{report}");
    assert!(!metrics["spell_healing"].as_object().unwrap().is_empty());
    assert_eq!(metrics["mana_curve"].as_array().unwrap().len(), 301);
    let casts = report["healer_casts"].as_object().unwrap();
    assert!(
        casts.get("18562").and_then(|v| v.as_u64()).unwrap_or(0) > 0,
        "Swiftmend must be used: {report}"
    );
    assert!(
        casts.get("8921").and_then(|v| v.as_u64()).unwrap_or(0) > 0,
        "Moonfire must be used: {report}"
    );
    assert!(
        casts.get("5176").and_then(|v| v.as_u64()).unwrap_or(0) > 0,
        "Wrath must be used: {report}"
    );
    println!("RESTORATION_DRUID_REPORT={report}");
}
