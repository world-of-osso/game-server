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
