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
#[ignore = "requires native MID2 tank, Shaman and canonical world DB"]
fn restoration_shaman_healing_stream_summon_applies_native_pulse() {
    let mut run = super::super::dps_parity::prepare_registered_player(66, 1).unwrap();
    let healer = super::super::dps_parity::spawn_second_reference_player(&mut run, 264).unwrap();
    let tank = run.actor;
    let party = crate::followers::simulator::spawn_party_for_reference_healer(
        run.app.world_mut(),
        tank,
        healer,
    )
    .unwrap();
    let mut health = run
        .app
        .world_mut()
        .get_mut::<shared::components::Health>(tank)
        .unwrap();
    health.current = health.max * 0.5;
    drop(health);
    super::super::dps_parity::literal_rotation::execution::execute(
        run.app.world_mut(),
        healer,
        super::super::dps_parity::literal_rotation::execution::Execution::Spell(
            5394,
            healer,
            Some(tank),
        ),
    )
    .unwrap();
    for _ in 0..50 {
        run.app.update();
    }
    let world = run.app.world_mut();
    let actors: Vec<_> = world
        .query::<(
            bevy::prelude::Entity,
            &shared::components::Npc,
            &crate::spell_cast::summon::SummonedCreature,
        )>()
        .iter(world)
        .filter(|(_, npc, summon)| npc.template_id == 3527 && summon.owner == healer)
        .map(|(entity, _, _)| entity)
        .collect();
    assert!(
        !actors.is_empty(),
        "actual Healing Stream summon must exist"
    );
    assert!(
        actors
            .iter()
            .any(|actor| crate::auras::has_aura(world, *actor, 5672)),
        "totem must apply its native periodic aura: {actors:?}"
    );
    let events = &world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| event.spell_id == Some(52042)
            && event.amount > 0
            && event.target == Some(tank.to_bits())),
        "native pulse must heal the injured tank; store={}, totem faction={:?}, tank faction={:?}, events={:?}",
        world
            .resource::<crate::spell_info::SpellStore>()
            .get(52042)
            .is_some(),
        world.get::<shared::components::UnitFactionTemplate>(actors[0]),
        world.get::<shared::components::UnitFactionTemplate>(tank),
        events
            .iter()
            .filter(|event| matches!(event.spell_id, Some(5672 | 52042)))
            .collect::<Vec<_>>()
    );
    let _ = party;
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
