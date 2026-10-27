//! Native Void Ray channel tick Fury spending.
use super::*;
use shared::{casting::CastState, components::PowerType};

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_meta_void_ray_cast_count_respects_cooldown() {
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1217605, Some(actor), 1);
    for _ in 0..30 { run.app.update(); }
    assert!(crate::auras::has_aura(run.app.world(), actor, 1217607), "real Meta active");
    crate::spell_cast::cast_for_player_with_target(run.app.world_mut(), actor, 473728, Some(target)).expect("first Void Ray");
    for _ in 0..80 { run.app.update(); }
    assert!(crate::auras::has_aura(run.app.world(), actor, 1217607), "Meta remains active");
    let _ = crate::spell_cast::cast_for_player_with_target(run.app.world_mut(), actor, 473728, Some(target));
    let casts = run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.iter().filter(|event| event.source == Some(actor.to_bits()) && event.spell_id == Some(473728) && event.kind == shared::protocol::CombatLogKind::CastSuccess).count();
    assert_eq!(casts, 1, "Void Ray cannot cast twice within four seconds in Meta");
}

fn start_ray(targets: usize) -> Run {
    let mut run = prepare(1480, targets).expect("native Devourer profile");
    run.app
        .init_resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 1217607),
        "out-of-Meta fixture"
    );
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Fury, -1000.0)
        .expect("native Fury pool");
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Fury, 100.0)
        .expect("native Fury pool");
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        473728,
        Some(target),
    )
    .expect("native Void Ray admission at 100 Fury");
    let initial = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury)
        .unwrap()
        .0;
    assert_eq!(initial, 100.0, "native channel must not spend Fury upfront");
    run
}

fn ray_ticks(run: &Run) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>()
        .0
        .iter()
        .filter(|(caster, spell, _)| *caster == run.actor && *spell == 1213649)
        .count()
}

fn advance_to_ticks(run: &mut Run, minimum: usize) {
    for _ in 0..40 {
        if ray_ticks(run) >= minimum {
            return;
        }
        run.app.update();
    }
    panic!("real native Void Ray child ticks must execute");
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_void_ray_three_target_hits_generate_shattered_souls() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = start_ray(3);
    let actor = run.actor;
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .is_some_and(|known| known.contains(1227619)),
        "selected Shattered Souls"
    );
    advance_to_ticks(&mut run, 15);
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(1213649)
                && event.amount > 0),
        "real ray child deals positive damage"
    );
    for _ in 0..30 {
        run.app.update();
    }
    assert!(
        server::soul_fragments::active_count(
            run.app.world(),
            actor,
            server::soul_fragments::FragmentFilter::Lesser
        ) > 0,
        "seeded ray hits create active Shattered Souls fragments"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_void_ray_spends_fury_for_each_native_tick() {
    let mut run = start_ray(1);
    advance_to_ticks(&mut run, 3);
    let ticks = ray_ticks(&run);
    let after = crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::Fury)
        .unwrap()
        .0;
    // SimC76f8118 sc_demon_hunter.cpp6431–6455 pays native1213649's
    // 70448 cost of 5 Fury for each non-Meta channel tick.
    // SimC76f8118 cpp3583–3698: a consumed soul can independently energize
    // the actor during the real channel; include only observed native gains.
    let soul_fury: i32 = run.app.world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0.iter()
        .filter(|event| event.target == Some(run.actor.to_bits())
            && event.spell_id == Some(1223628)
            && event.kind == shared::protocol::CombatLogKind::Energize)
        .map(|event| event.amount)
        .sum();
    assert_eq!(
        after,
        100.0 + soul_fury as f32 - ticks as f32 * 5.0,
        "every real Void Ray tick must pay its native Fury cost"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_void_ray_stops_before_an_unaffordable_tick() {
    let mut run = start_ray(1);
    advance_to_ticks(&mut run, 1);
    let actor = run.actor;
    let current = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury)
        .unwrap()
        .0;
    // Model an external Fury loss after a real paid channel tick.
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        actor,
        PowerType::Fury,
        4.0 - current,
    )
    .expect("native Fury pool");
    let paid_ticks = ray_ticks(&run);
    for _ in 0..10 {
        run.app.update();
        if run.app.world().get::<CastState>(actor).is_none() || ray_ticks(&run) > paid_ticks {
            break;
        }
    }
    assert_eq!(
        ray_ticks(&run),
        paid_ticks,
        "an unaffordable tick must not launch native damage"
    );
    assert!(
        run.app.world().get::<CastState>(actor).is_none(),
        "the native channel must stop when its next tick cannot be paid"
    );
}
