//! Native Devourer metamorphosis drain.
use super::*;

fn enter_metamorphosis(run: &mut Run) {
    let actor = run.actor;
    // Establish the native ordinary-cast admission aura for this isolated fixture.
    // Triggered casts themselves ignore caster-aura restrictions.
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1213809, Some(actor), 1);
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1213809),
        "native Meta admission setup"
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1217605, Some(actor), 1);
    // Effect64 queues its native child; let the actual trigger queue run.
    for _ in 0..20 {
        if crate::auras::has_aura(run.app.world(), actor, 1217607) {
            break;
        }
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1217607),
        "native Void Metamorphosis must start"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_metamorphosis_drains_fury_over_time() {
    use shared::components::PowerType;
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    engage(&mut run);
    enter_metamorphosis(&mut run);
    let initial = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury)
        .unwrap()
        .0;
    // SimC76f8118 sc_demon_hunter.cpp1424–1478,12462–12500: events drain Fury.
    for _ in 0..40 {
        run.app.update();
    }
    let after = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury)
        .unwrap()
        .0;
    assert!(
        after < initial,
        "Void Metamorphosis must drain Fury: {initial} -> {after}"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_metamorphosis_fills_fury_on_entry() {
    use shared::components::PowerType;
    let mut run = prepare_literal(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    engage(&mut run);
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Fury, -1000.0)
        .expect("native Fury pool");
    enter_metamorphosis(&mut run);
    let (current, maximum) =
        crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury).unwrap();
    assert!(
        maximum > 0.0 && maximum <= 200.0,
        "native Fury cap: {maximum}"
    );
    // SimC76f8118 sc_demon_hunter.cpp12445–12452 grants 200 Fury on entry, capped by the native pool.
    assert_eq!(current, maximum, "Void Metamorphosis entry must fill Fury");
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn real_meta_survives_outgoing_void_ray_damage() {
    use shared::protocol::CombatLogKind;
    let mut run = prepare(1480, 3).expect("native Devourer profile");
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        473662,
        Some(target),
    )
    .expect("real Consume engages combat");
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .get::<shared::components::CombatStatus>(actor)
            .is_some_and(|status| status.0),
        "real combat"
    );
    for _ in 0..35 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1225789, Some(actor), 1);
    }
    for _ in 0..20 {
        if crate::auras::has_aura(run.app.world(), actor, 1213809) {
            break;
        }
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1213809),
        "native Meta ready"
    );
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        1217605,
        Some(actor),
    )
    .expect("ordinary Meta entry");
    for _ in 0..30 {
        if crate::auras::has_aura(run.app.world(), actor, 1217607)
            && !crate::auras::has_aura(run.app.world(), actor, 201453)
        {
            break;
        }
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1217607),
        "Meta entered"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 201453),
        "pacify expired naturally"
    );
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        473728,
        Some(target),
    )
    .expect("real Void Ray in Meta");
    for _ in 0..40 {
        run.app.update();
        if run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id == Some(1214595)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
        {
            break;
        }
    }
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(1214595)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "real Meta ray dealt outgoing damage"
    );
    for _ in 0..5 {
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1217607),
        "outgoing ray damage must not interrupt caster's Meta"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn selected_voidrush_without_charge_does_not_reduce_meta_drain() {
    use shared::components::PowerType;
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .is_some_and(|known| known.contains(1272422)),
        "selected Voidrush talent"
    );
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        473662,
        Some(target),
    )
    .expect("real Consume engages combat");
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .get::<shared::components::CombatStatus>(actor)
            .is_some_and(|status| status.0),
        "actor in combat"
    );
    for _ in 0..35 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1225789, Some(actor), 1);
    }
    for _ in 0..20 {
        if crate::auras::has_aura(run.app.world(), actor, 1213809) {
            break;
        }
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1213809),
        "native Meta ready"
    );
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        1217605,
        Some(actor),
    )
    .expect("ordinary Meta entry");
    for _ in 0..20 {
        if crate::auras::has_aura(run.app.world(), actor, 1217607) {
            break;
        }
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1217607),
        "real Meta active"
    );
    let initial = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury)
        .unwrap()
        .0;
    assert_eq!(initial, 140.0, "selected native Fury cap");
    let started = crate::spell_cast::now(run.app.world());
    for _ in 0..80 {
        if crate::spell_cast::now(run.app.world()) - started >= 3.0 {
            break;
        }
        run.app.update();
    }
    assert!(
        crate::spell_cast::now(run.app.world()) - started >= 3.0,
        "advanced three seconds"
    );
    assert!(
        run.app
            .world()
            .get::<shared::components::CombatStatus>(actor)
            .is_some_and(|status| status.0),
        "still in combat"
    );
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(actor)
            .is_none(),
        "not channeling or casting"
    );
    assert!(
        !run.app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .is_some_and(crate::auras::ActiveAuras::is_stunned),
        "not stunned"
    );
    let remaining = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury)
        .unwrap()
        .0;
    // SimC76f8118 cpp12471–12488: no Voidrush movement means normal rate, not 10% rate.
    assert!(
        remaining < 100.0,
        "normal in-combat drain after three seconds, with no Voidrush charge: {initial}->{remaining}"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1217607),
        "Meta remains active"
    );
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        1245483,
        Some(target),
    )
    .expect("real Pierce triggers Voidrush");
    let charge_start =
        crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury)
            .unwrap()
            .0;
    for _ in 0..9 {
        run.app.update();
    }
    let during = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury)
        .unwrap()
        .0;
    assert!(
        charge_start - during <= 4.0,
        "real charge reduces drain during its short window: {charge_start}->{during}"
    );
    for _ in 0..30 {
        run.app.update();
    }
    let after = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury)
        .unwrap()
        .0;
    assert!(
        during - after > 4.0,
        "normal drain resumes after charge: {during}->{after}"
    );
}
