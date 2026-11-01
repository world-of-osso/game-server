use std::path::Path;

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn mistweaver_gust_heals_primary_and_scales_with_mastery_rating() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::components::Health;
    use shared::protocol::CombatLogKind;

    let mut run = super::healer_simc_replay::snapshot_party(270, 90, 1);
    let healer = run.healer;
    let tank = run.tank;
    assert!(crate::auras::has_aura(run.app.world(), healer, 117907));
    let cast = |run: &mut super::healer_simc_replay::ClassParty, spell| {
        let world = run.app.world_mut();
        let max = world.get::<Health>(tank).unwrap().max;
        world.get_mut::<Health>(tank).unwrap().current = max * 0.1;
        world
            .get_mut::<crate::stats::CharacterStats>(healer)
            .unwrap()
            .crit_pct = 0.0;
        let start = world.resource::<ObservedCombatEvents>().0.len();
        crate::spell_cast::creature_cast(world, healer, spell, Some(tank), false).unwrap();
        for _ in 0..80 {
            run.app.update();
        }
        let events = &run.app.world().resource::<ObservedCombatEvents>().0[start..];
        let gusts: Vec<_> = events
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Heal
                    && event.spell_id == Some(191894)
                    && event.target == Some(tank.to_bits())
            })
            .collect();
        assert_eq!(gusts.len(), 1, "{events:?}");
        gusts[0].amount
    };
    let high_rating = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(healer)
        .unwrap()
        .ratings
        .mastery;
    let high = cast(&mut run, 124682);
    crate::auras::remove_auras(
        run.app.world_mut(),
        healer,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 1235108,
    );
    for _ in 0..3 {
        run.app.update();
    }
    let low_rating = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(healer)
        .unwrap()
        .ratings
        .mastery;
    assert!(
        high_rating > low_rating,
        "flask must increase mastery rating: {high_rating} <= {low_rating}"
    );
    let low = cast(&mut run, 124682);
    assert!(
        high > low && low > 0,
        "Gust must scale with mastery rating: {high} vs {low}"
    );
    let renewing = cast(&mut run, 115151);
    assert!(renewing > 0);
}

#[test]
#[ignore = "checks 70448 Ancient Teachings from an actual Monk attack"]
fn mistweaver_ancient_teachings_splits_damage_and_preserves_critical_result() {
    use shared::components::Health;
    use shared::protocol::CombatLogKind;

    let spells = [100780, 388023, 388024, 388025, 467293];
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &spells, 10).unwrap();
    h.app
        .init_resource::<crate::spell_cast::test_harness::ObservedCombatEvents>();
    let ally = crate::spell_cast::test_harness::spawn_player(&mut h.app, 10, 3.0);
    for unit in [h.caster, ally] {
        *h.app.world_mut().get_mut::<Health>(unit).unwrap() = Health {
            current: 1.0,
            max: 100_000.0,
        };
    }
    *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
        current: 100_000.0,
        max: 100_000.0,
    };
    for spell_id in [388023, 467293] {
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: h.caster,
                target: h.caster,
                spell_id,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    assert_eq!(
        crate::spell_cast::scripts::script_effect_value(h.app.world(), h.caster, 388023, 0),
        345.0
    );
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 0.0;
    crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 100780, Some(h.npc), true)
        .unwrap();
    h.advance(2);
    let events = &h
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let damage = events
        .iter()
        .find(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(100780))
        .expect("Tiger Palm must deal damage");
    assert!(!damage.crit);
    for unit in [h.caster, ally] {
        let heal = events
            .iter()
            .find(|event| {
                event.kind == CombatLogKind::Heal
                    && event.spell_id == Some(388024)
                    && event.target == Some(unit.to_bits())
            })
            .expect("Ancient Teachings must heal each ally");
        assert!(!heal.crit);
        assert!((heal.amount as f32 - damage.amount as f32 * 3.45 / 2.0).abs() <= 3.0);
    }
    let start = events.len();
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 100.0;
    crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 100780, Some(h.npc), true)
        .unwrap();
    h.advance(2);
    let events = &h
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    let damage = events
        .iter()
        .find(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(100780))
        .expect("critical Tiger Palm must deal damage");
    assert!(damage.crit);
    for unit in [h.caster, ally] {
        let heal = events
            .iter()
            .find(|event| {
                event.kind == CombatLogKind::Heal
                    && event.spell_id == Some(388025)
                    && event.target == Some(unit.to_bits())
            })
            .expect("critical Ancient Teachings must heal each ally");
        assert!(heal.crit);
        assert!((heal.amount as f32 - damage.amount as f32 * 3.45 / 2.0).abs() <= 3.0);
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn mistweaver_waits_for_four_mana_tea_stacks() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::components::Mana;
    use shared::protocol::CombatLogKind;
    let mut run = super::healer_simc_replay::snapshot_party(270, 90, 1);
    super::healer_simc_replay::engage(&mut run);
    let healer = run.healer;
    {
        let mut mana = run.app.world_mut().get_mut::<Mana>(healer).unwrap();
        mana.current = mana.max * 0.5;
    }
    let apply = |world: &mut bevy::prelude::World| {
        crate::auras::apply_aura(
            world,
            &crate::auras::AuraApplication {
                caster: healer,
                target: healer,
                spell_id: 115867,
                effect_mask: 7,
                source: crate::auras::AuraSource::Cast,
            },
        )
    };
    apply(run.app.world_mut());
    for _ in 0..10 {
        run.app.update();
    }
    assert!(
        !run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|e| e.source == Some(healer.to_bits())
                && e.spell_id == Some(115294)
                && e.kind == CombatLogKind::CastSuccess)
    );
    for _ in 0..3 {
        apply(run.app.world_mut());
    }
    for _ in 0..30 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|e| e.source == Some(healer.to_bits())
                && e.spell_id == Some(115294)
                && e.kind == CombatLogKind::CastSuccess),
        "four earned Mana Tea stacks must be used at low mana"
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot and SimC traces"]
fn mistweaver_prioritizes_rising_sun_kick_and_accumulates_mana_tea() {
    let trace = std::env::var("HEALER_TANK_TIMELINE").unwrap();
    let party = std::env::var("HEALER_TANK_PARTY_TRACE").unwrap();
    let report = super::healer_simc_replay::replay_simc_party(
        270,
        90,
        Path::new(&trace),
        Path::new(&party),
        300,
        70448,
    )
    .unwrap();
    let casts = &report["healer_casts"];
    let count = |id| casts[id].as_u64().unwrap_or(0);
    assert!(
        count("107428") > 8,
        "Rising Sun Kick must not be starved: {report}"
    );
    assert!(
        count("399491") < 100,
        "Sheilun's Gift must not be cast as single-target filler: {report}"
    );
    assert!(
        count("115294") < 30,
        "Mana Tea must accumulate stacks: {report}"
    );
    println!("MISTWEAVER_POLICY_REPORT={report}");
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn mistweaver_uses_spinning_crane_kick_only_against_a_pack() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::components::Health;
    use shared::protocol::CombatLogKind;
    let mut run = super::healer_simc_replay::snapshot_party(270, 90, 2);
    super::healer_simc_replay::engage(&mut run);
    let (tank, healer) = (run.tank, run.healer);
    let enemy = run.sources[0];
    let add = run.sources[1];
    run.app
        .world_mut()
        .entity_mut(tank)
        .insert(crate::networking::CombatTarget(Some(enemy)));
    run.app.world_mut().get_mut::<Health>(add).unwrap().current = 0.0;
    for _ in 0..150 {
        run.app.update();
    }
    assert!(
        !run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|e| e.source == Some(healer.to_bits())
                && e.spell_id == Some(101546)
                && e.kind == CombatLogKind::CastSuccess)
    );
    {
        let mut hp = run.app.world_mut().get_mut::<Health>(add).unwrap();
        hp.current = hp.max;
    }
    for _ in 0..250 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let casts: Vec<_> = events
        .iter()
        .filter(|e| e.source == Some(healer.to_bits()) && e.kind == CombatLogKind::CastSuccess)
        .map(|e| e.spell_id)
        .collect();
    assert!(
        casts.contains(&Some(101546)),
        "Mistweaver must use Spinning Crane Kick against a pack"
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot and SimC traces"]
fn mistweaver_uses_sheilun_on_party_damage() {
    let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let party_trace =
        std::env::var("HEALER_TANK_PARTY_TRACE").expect("HEALER_TANK_PARTY_TRACE required");
    let report = super::healer_simc_replay::replay_simc_party(
        270,
        90,
        Path::new(&trace),
        Path::new(&party_trace),
        300,
        70448,
    )
    .unwrap();
    assert!(
        report["party_incoming_events"].as_u64().unwrap() > 100,
        "{report}"
    );
    assert!(
        report["healer_casts"]["399491"].as_u64().unwrap_or(0) > 0,
        "Sheilun's Gift never cast: {report}"
    );
    assert!(
        report["metrics"]["spell_healing"]["399491"]["effective_hps"]
            .as_f64()
            .unwrap_or(0.0)
            > 0.0,
        "Sheilun's Gift did not heal: {report}"
    );
    println!("MISTWEAVER_PARTY_REPORT={report}");
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot and SimC traces"]
fn mistweaver_heals_the_simc_dummy_party() {
    let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let report =
        super::healer_simc_replay::replay_simc(270, 90, Path::new(&trace), 300, 70448).unwrap();
    let metrics = &report["metrics"];
    let effective = metrics["effective_hps"].as_f64().unwrap();
    let raw = metrics["raw_hps"].as_f64().unwrap();
    assert!((0.0..=100.0).contains(&metrics["overheal_pct"].as_f64().unwrap()));
    assert!(report["incoming_events"].as_u64().unwrap() > 100);
    assert!(
        ["107428", "100784", "205523", "100780"]
            .iter()
            .any(|spell| report["healer_casts"][*spell].as_u64().unwrap_or(0) > 0),
        "Mistweaver never cast damage filler: {report}"
    );
    assert_eq!(report["tank_spec"], 66);
    assert!(
        report["tank_casts"]
            .as_object()
            .unwrap()
            .values()
            .any(|count| count.as_u64().unwrap() > 0)
    );
    assert!(report["tank_active_mitigation_seconds"].as_f64().unwrap() > 0.0);
    assert!(
        metrics["first_death_seconds"]
            .as_f64()
            .is_none_or(|time| time > 4.0)
    );
    assert!(
        report["healer_casts"]["115175"].as_u64().unwrap_or(0) > 0,
        "Soothing Mist never cast: {report}"
    );
    assert!(
        metrics["spell_healing"]["115175"]["effective_hps"]
            .as_f64()
            .unwrap_or(0.0)
            > 0.0,
        "Soothing Mist did not heal: {report}"
    );
    assert!(effective > 0.0 && raw >= effective, "{report}");
    let spells = metrics["spell_healing"].as_object().unwrap();
    assert!(!spells.is_empty());
    let spell_total: f64 = spells
        .values()
        .map(|spell| spell["effective_hps"].as_f64().unwrap())
        .sum();
    assert!((spell_total - effective).abs() < 0.000001);
    assert_eq!(metrics["mana_curve"].as_array().unwrap().len(), 301);
    assert_eq!(metrics["tank_health_curve"].as_array().unwrap().len(), 301);
    assert!(metrics["tank_dtps"].as_f64().unwrap() > 0.0);
    assert!(metrics["tank_minimum_health_pct"].as_f64().unwrap() < 100.0);
    assert!(metrics["tank_seconds_below_35_alive"].as_f64().unwrap() >= 0.0);
    println!("MISTWEAVER_REPORT={report}");
}
