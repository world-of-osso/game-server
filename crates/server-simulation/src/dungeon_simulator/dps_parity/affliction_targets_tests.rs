//! Actual native Shadow Bolt damage observes owned target state, not aura labels.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;

const BOLT: u32 = 686;
const WITHERING: u32 = 386976;
const IMPETUOUS: u32 = 1312998;
const FACTOR_EPSILON: f64 = 0.00001;

fn apply_owned(run: &mut Run, caster: Entity, target: Entity, spell: u32) {
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: spell,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn cast_bolt(run: &mut Run, target: Entity) {
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, BOLT, Some(target), 0);
    for _ in 0..8 {
        run.app.update();
    }
}

fn observed_target_factors(run: &Run) -> Vec<f64> {
    let capture = run.app.world().resource::<FactorCapture>().export();
    let events = capture["events"].as_array().unwrap();
    let bolts = events
        .iter()
        .filter(|event| event["event"] == "direct_amount" && event["spell_id"] == BOLT);
    bolts
        .map(|event| {
            event["applied_multipliers"].as_array().unwrap().iter()
        .find(|factor| factor["factor"] == "giantkiller")
        .expect("actual damage calculation retains its native target-script product")
        ["value"].as_f64().unwrap()
        })
        .collect()
}

fn assert_factor(actual: f64, expected: f64, context: &str) {
    assert!(
        (actual - expected).abs() < FACTOR_EPSILON,
        "{context}: actual={actual}, native expected={expected}"
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_targets_withering_counts_owned_unique_dots_and_native_cap() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 3).unwrap();
    engage(&mut run);
    let (caster, target, foreign) = (run.actor, run.targets[0], run.targets[1]);
    assert!(crate::auras::has_aura(run.app.world(), caster, WITHERING));
    let percent = f64::from(crate::spell_cast::scripts::script_effect_value(
        run.app.world(),
        caster,
        WITHERING,
        0,
    )) / 100.0;
    let cap = crate::spell_cast::scripts::script_effect_value(run.app.world(), caster, WITHERING, 1)
        as usize;
    assert!(percent > 0.0 && cap > 0);
    run.app.world_mut().insert_resource(FactorCapture::new(
        caster,
        target,
        BOLT,
        "shadow_bolt",
        32,
    ));
    for spell in [980, 1259790, 445474] {
        apply_owned(&mut run, foreign, target, spell);
    }
    cast_bolt(&mut run, target);
    for _ in 0..3 {
        apply_owned(&mut run, caster, target, 1259790);
    }
    cast_bolt(&mut run, target);
    for spell in [980, 445474] {
        apply_owned(&mut run, caster, target, spell);
    }
    cast_bolt(&mut run, target);
    apply_owned(&mut run, caster, target, 27243);
    cast_bolt(&mut run, target);
    let factors = observed_target_factors(&run);
    assert_eq!(factors.len(), 4, "four real native Shadow Bolts must land");
    // SimC76f8118 actions915–916 and sc_warlock.cpp248–264: foreign
    // DoTs do not count, UA counts once regardless of stacks, native cap wins.
    assert_factor(
        factors[1] / factors[0],
        1.0 + percent,
        "three UA stacks are one owned DoT",
    );
    assert_factor(
        factors[2] / factors[0],
        1.0 + percent * cap.min(3) as f64,
        "three owned DoT kinds",
    );
    assert_factor(
        factors[3] / factors[0],
        1.0 + percent * cap.min(4) as f64,
        "native owned-DoT cap",
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_targets_impetuous_uses_owned_haunt_and_selected_native_values() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 3).unwrap();
    engage(&mut run);
    let (caster, target, foreign) = (run.actor, run.targets[0], run.targets[1]);
    assert!(crate::auras::has_aura(run.app.world(), caster, IMPETUOUS));
    let base = f64::from(crate::spell_cast::scripts::script_effect_value(
        run.app.world(),
        caster,
        IMPETUOUS,
        0,
    )) / 100.0;
    let haunted = f64::from(crate::spell_cast::scripts::script_effect_value(
        run.app.world(),
        caster,
        IMPETUOUS,
        1,
    )) / 100.0;
    assert!(haunted > base && base > 0.0);
    run.app.world_mut().insert_resource(FactorCapture::new(
        caster,
        target,
        BOLT,
        "shadow_bolt",
        32,
    ));
    apply_owned(&mut run, foreign, target, 48181);
    cast_bolt(&mut run, target);
    apply_owned(&mut run, caster, target, 48181);
    cast_bolt(&mut run, target);
    crate::auras::remove_auras(
        run.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.caster == caster && aura.spell_id == IMPETUOUS,
    );
    cast_bolt(&mut run, target);
    let factors = observed_target_factors(&run);
    assert_eq!(factors.len(), 3, "three real native Shadow Bolts must land");
    // SimC76f8118 actions918–919: the caster's own Haunt selects effect2;
    // removing the passive removes this multiplier without changing Haunt.
    assert_factor(
        factors[0],
        1.0 + base,
        "foreign Haunt is not this caster's Haunt",
    );
    assert_factor(
        factors[1],
        1.0 + haunted,
        "owned Haunt uses its native enhancement",
    );
    assert_factor(
        factors[2],
        1.0,
        "unselected Impetuous Wrath cannot amplify damage",
    );
}
