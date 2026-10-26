//! Native-flag Pandemic carries remaining duration, independent of preserved tick phase.
use super::feral_growth_tests::cast_native_spell_after_regeneration as cast_native;
use super::*;
use crate::auras::AuraInstance;
use shared::components::PowerType;

fn owned_dot(run: &Run, spell: u32) -> Option<AuraInstance> {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.targets[0])?
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == spell)
        .cloned()
}

fn cast_initial_dot(run: &mut Run, cast: u32, dot: u32) -> AuraInstance {
    cast_native(run, cast);
    for _ in 0..60 {
        if let Some(aura) = owned_dot(run, dot) {
            return aura;
        }
        run.app.update();
    }
    panic!("native root{cast} must apply actual DoT{dot}");
}

fn wait_inside_pandemic(run: &mut Run, dot: u32, fresh_seconds: f64) -> f64 {
    for _ in 0..1200 {
        let expiry = owned_dot(run, dot).unwrap().expires_at.unwrap();
        let remaining = expiry - crate::auras::aura_now(run.app.world());
        if remaining > 0.0 && remaining <= fresh_seconds * 0.25 {
            return expiry;
        }
        run.app.update();
    }
    panic!("native DoT{dot} must reach its actual refresh window");
}

fn assert_refresh_carry(run: &mut Run, cast: u32, dot: u32, fresh_seconds: f64) {
    let old_expiry = wait_inside_pandemic(run, dot, fresh_seconds);
    let stacks = owned_dot(run, dot).unwrap().stacks;
    cast_native(run, cast);
    let mut refreshed = owned_dot(run, dot).unwrap();
    for _ in 0..60 {
        if refreshed.expires_at != Some(old_expiry) {
            break;
        }
        run.app.update();
        refreshed = owned_dot(run, dot).unwrap();
    }
    let actual = refreshed.expires_at.unwrap();
    // SimC76f8118 action.cpp4612/4630–4635: inside the30% window,
    // the resulting deadline is exactly the previous deadline + fresh duration.
    let expected = old_expiry + fresh_seconds;
    assert!(
        (actual - expected).abs() < 0.001,
        "native DoT{dot}: expiry={actual}, expected carry={expected}"
    );
    if dot == 980 {
        assert_eq!(
            refreshed.stacks, stacks,
            "Agony refresh retains its actual damage counter"
        );
    }
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn native_pandemic_agony_real_refresh_retains_remaining_native_duration() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let aura = cast_initial_dot(&mut run, 980, 980);
    let fresh = f64::from(aura.duration_ms) / 1000.0;
    assert!(fresh > 0.0);
    assert_refresh_carry(&mut run, 980, 980, fresh);
}

fn earn_five_combo_points(run: &mut Run) {
    for _ in 0..10 {
        let points = crate::spell_cast::powers::power_value(
            run.app.world(),
            run.actor,
            PowerType::ComboPoints,
        )
        .unwrap()
        .0;
        if points >= 5.0 {
            return;
        }
        cast_native(run, 5221);
    }
    panic!("real Shred must earn five native combo points");
}

#[test]
#[ignore = "requires private103 profile and native70448 world DB"]
fn native_pandemic_feral_rip_rake_keep_identical_native_carry() {
    let _seed = crate::random::seed_scope(70448);
    for (cast, dot) in [(1079, 1079), (1822, 155722)] {
        let mut run = prepare_literal(103, 1).unwrap();
        engage(&mut run);
        let actor = run.actor;
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false)
            .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        if cast == 1079 {
            earn_five_combo_points(&mut run);
        }
        let aura = cast_initial_dot(&mut run, cast, dot);
        if cast == 1079 {
            earn_five_combo_points(&mut run);
        }
        assert_refresh_carry(&mut run, cast, dot, f64::from(aura.duration_ms) / 1000.0);
    }
}
