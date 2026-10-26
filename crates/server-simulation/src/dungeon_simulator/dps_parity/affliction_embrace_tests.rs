//! Death's Embrace uses each actual recipient's live health and native values.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use shared::components::Health;

const EMBRACE: u32 = 234876;

fn cast_factor_at_health(run: &mut Run, spell: u32, health_pct: f32, periodic: bool) -> f64 {
    let (caster, target) = (run.actor, run.targets[0]);
    {
        let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
        health.current = health.max * health_pct / 100.0;
    }
    run.app
        .world_mut()
        .insert_resource(FactorCapture::new(caster, target, spell, "embrace", 64));
    crate::spell_cast::cast_triggered(run.app.world_mut(), caster, spell, Some(target), 0);
    for _ in 0..60 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    let kind = if periodic {
        "periodic_amount"
    } else {
        "direct_amount"
    };
    let event = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == kind && event["spell_id"] == spell)
        .expect("actual native damage must resolve");
    event["applied_multipliers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|factor| factor["factor"] == "ret_damage")
        .expect("context product is observable")["value"]
        .as_f64()
        .unwrap()
}

fn assert_factor(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 0.0001,
        "actual={actual}, native expected={expected}"
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_embrace_real_bolt_scales_below_native_threshold_not_volley() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let caster = run.actor;
    assert!(crate::auras::has_aura(run.app.world(), caster, EMBRACE));
    let amount =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), caster, EMBRACE, 0);
    let threshold =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), caster, EMBRACE, 1);
    assert_eq!((amount, threshold), (40.0, 35.0));
    assert_factor(cast_factor_at_health(&mut run, 686, 100.0, false), 1.0);
    assert_factor(cast_factor_at_health(&mut run, 686, threshold, false), 1.0);
    // SimC76f8118 actions529–535/832: live linear recipient-health scaling.
    assert_factor(
        cast_factor_at_health(&mut run, 686, threshold / 2.0, false),
        1.0 + f64::from(amount) / 200.0,
    );
    // actions809: Volley does not inherit this selected modifier.
    assert_factor(
        cast_factor_at_health(&mut run, 453176, threshold / 2.0, false),
        1.0,
    );
    crate::auras::remove_auras(
        run.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Unlearned,
        |aura| aura.spell_id == EMBRACE,
    );
    assert_factor(
        cast_factor_at_health(&mut run, 686, threshold / 2.0, false),
        1.0,
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_embrace_actual_ua_ticks_use_live_recipient_health() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    assert_factor(cast_factor_at_health(&mut run, 1259790, 100.0, true), 1.0);
    // SimC76f8118 actions1504/529–535: ordinary UA is affected by Embrace.
    assert_factor(cast_factor_at_health(&mut run, 1259790, 17.5, true), 1.2);
}
