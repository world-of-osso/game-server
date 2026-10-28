//! Native Devourer literal controller qualification.
use super::*;

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_void_ray_produces_native_tick_damage() {
    use shared::components::PowerType;
    use shared::protocol::CombatLogKind;

    let mut run = prepare_literal(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Fury, 140.0)
        .expect("native Fury pool");
    engage(&mut run);
    // SimC76f8118 sc_demon_hunter.cpp6354-6367: Void Ray channels tick spell 1213649 outside metamorphosis.
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        473728,
        Some(target),
    )
    .expect("native Void Ray cast");
    for _ in 0..100 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id == Some(1213649)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            }),
        "Void Ray must deal its native tick damage"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_rotation_casts_hungering_slash_and_reapers_toll() {
    let report = measure_seed(1480, 1, 70448).expect("real Devourer 300s rotation");
    let actions = report["combat"]["actions"].as_array().unwrap();
    let counts: Vec<_> = [1239123, 1245470]
        .into_iter()
        .map(|spell| {
            actions
                .iter()
                .find(|a| a["spell_id"] == spell)
                .and_then(|a| a["count"].as_u64())
                .unwrap_or(0)
        })
        .collect();
    assert!(
        counts.iter().all(|&count| count > 0),
        "rotation casts Hungering Slash and Reaper's Toll: {counts:?}"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn batch_c_explicit_embellishments_earn_insight() {
    let report = measure_seed(1480, 1, 70448).expect("native Devourer rotation");
    assert_eq!(
        report["buff_uptime_pct"]["1283697"].as_f64(),
        Some(200.0),
        "both equipped Lining drivers remain active"
    );
    assert!(
        report["buff_uptime_pct"]["1229746"].as_f64().unwrap_or(0.0) > 0.0,
        "actual combat earns Insight"
    );
    assert!(
        !report["gaps"].as_array().unwrap().iter().any(|gap| gap
            .as_str()
            .is_some_and(|gap| gap.contains("embellishment=arcanoweave_lining"))),
        "modeled option must not remain a gap"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn batch2_real_rotation_produces_soulburst() {
    let report = measure_seed(1480, 1, 70448).expect("native Devourer rotation");
    let actions = report["combat"]["actions"].as_array().unwrap();
    assert!(
        actions
            .iter()
            .any(|action| action["spell_id"] == 1297432
                && action["damage"].as_u64().unwrap_or(0) > 0),
        "selected tier proc must produce real Soulburst damage"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn batch6_real_rotation_produces_voidsurge() {
    let report = measure_seed(1480, 1, 70448).expect("native Devourer rotation");
    let actions = report["combat"]["actions"].as_array().unwrap();
    assert!(
        actions
            .iter()
            .any(|action| action["spell_id"] == 1246160
                && action["damage"].as_u64().unwrap_or(0) > 0),
        "selected Demonsurge must produce real Voidsurge damage"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn batch8_monster_rising_follows_metamorphosis() {
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    engage(&mut run);
    for _ in 0..30 {
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 452550),
        "Monster Rising active outside Meta"
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1217605, Some(actor), 1);
    for _ in 0..30 {
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1217607),
        "native Meta active"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 452550),
        "Monster Rising absent during Meta"
    );
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 1217607,
    );
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 452550),
        "Monster Rising returns after Meta"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_native_measurement_selects_literal_driver() {
    let run = prepare_measurement(1480, 1).expect("native Devourer profile and precombat");
    assert_eq!(
        run.spec_driver.as_ref().map(|driver| driver.label()),
        Some("LITERAL_SIMC_DEVOURER_76f8118")
    );
}

fn singular_strikes_hits(selected: bool) -> Vec<(i32, bool)> {
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(1480, 3).expect("native Devourer profile");
    let actor = run.actor;
    let known = run.app.world().get::<crate::class_progression::KnownSpells>(actor).unwrap();
    assert!(known.contains(1272770), "profile selects Singular Strikes");
    if !selected {
        let spells: Vec<_> = known.iter().filter(|&id| id != 1272770).collect();
        run.app.world_mut().entity_mut(actor).insert(crate::class_progression::KnownSpells::from_baseline(spells));
    }
    engage(&mut run);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1239525, Some(actor), 1);
    let _hit_seed = crate::random::seed_scope(91);
    crate::spell_cast::cast_for_player_with_target(run.app.world_mut(), actor, 1239123, Some(run.targets[0])).expect("real Hungering Slash");
    for _ in 0..40 { run.app.update(); }
    let events = &run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
    run.targets.iter().map(|target| {
        let hit = events.iter().find(|event| event.spell_id == Some(1239127) && event.target == Some(target.to_bits()) && event.kind == CombatLogKind::Damage && event.amount > 0).expect("actual Hungering Slash hit");
        (hit.amount, hit.crit)
    }).collect()
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn singular_strikes_amplifies_only_the_primary_hungering_slash_hit() {
    let without = singular_strikes_hits(false);
    let with = singular_strikes_hits(true);
    for index in 0..3 {
        assert_eq!(without[index].1, with[index].1, "same hit outcome");
        let factor = if index == 0 { 1.5 } else { 1.0 };
        let expected = f64::from(without[index].0) * factor;
        assert!((f64::from(with[index].0) - expected).abs() < expected * 0.005, "target {index}: {} vs {expected}", with[index].0);
    }
}
