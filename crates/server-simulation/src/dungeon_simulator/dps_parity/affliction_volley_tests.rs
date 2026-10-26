//! Improved Shadow Bolt's source whitelist includes Volley without duplicating Bolt.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;

const IMPROVED: u32 = 453080;
const BOLT: u32 = 686;
const VOLLEY: u32 = 453176;

fn cast_damage_product(run: &mut Run, spell: u32) -> f64 {
    let (caster, target) = (run.actor, run.targets[0]);
    run.app.world_mut().insert_resource(FactorCapture::new(
        caster,
        target,
        spell,
        "improved_bolt",
        32,
    ));
    crate::spell_cast::cast_triggered(run.app.world_mut(), caster, spell, Some(target), 0);
    for _ in 0..8 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    let event = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "result" && event["spell_id"] == spell)
        .expect("real native spell must resolve damage");
    let spell_mods = event["factors"]["native_spell_mod_product"]
        .as_f64()
        .unwrap();
    let amount = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "direct_amount" && event["spell_id"] == spell)
        .expect("actual native direct amount must be captured");
    let target_mods = amount["applied_multipliers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|factor| factor["factor"] == "giantkiller")
        .expect("actual native damage retains its script product")["value"]
        .as_f64()
        .unwrap();
    spell_mods * target_mods
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_volley_native_improved_shadow_bolt_applies_once_to_both_actions() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let caster = run.actor;
    assert!(crate::auras::has_aura(run.app.world(), caster, IMPROVED));
    let percent =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), caster, IMPROVED, 1);
    assert_eq!(percent, 40.0);
    let selected_bolt = cast_damage_product(&mut run, BOLT);
    let selected_volley = cast_damage_product(&mut run, VOLLEY);
    crate::auras::remove_auras(
        run.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Unlearned,
        |aura| aura.caster == caster && aura.spell_id == IMPROVED,
    );
    let plain_bolt = cast_damage_product(&mut run, BOLT);
    let plain_volley = cast_damage_product(&mut run, VOLLEY);
    let expected = 1.0 + f64::from(percent) / 100.0;
    // SimC76f8118 sc_warlock_init.cpp85–86 explicitly adds Volley to
    // effect2's affect list; ordinary Bolt already has its native mask.
    let bolt = selected_bolt / plain_bolt;
    let volley = selected_volley / plain_volley;
    assert!(
        (bolt - expected).abs() < 0.00001,
        "ordinary Bolt must retain exactly one native bonus: ratio={bolt}, expected={expected}"
    );
    assert!(
        (volley - expected).abs() < 0.00001,
        "actual Volley must receive the same native bonus: ratio={volley}, expected={expected}"
    );
}
