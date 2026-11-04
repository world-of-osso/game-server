//! Earned Bulletstorm damage must survive its real Aimed Shot consumption.
use super::marksmanship_mechanics_tests::{cast, owned_buff, player};
use crate::spell_cast::effects::factor_capture::FactorCapture;

const AIMED_SHOT: u32 = 19434;
const RAPID_FIRE: u32 = 257044;
const BULLETSTORM: u32 = 389020;

#[test]
#[ignore = "requires frozen exact254 profile/traits and native world data"]
fn marksmanship_bulletstorm_survives_consumption_until_aimed_projectile_hit() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = player(1);
    cast(&mut run, RAPID_FIRE, false);
    let earned = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == BULLETSTORM && aura.caster == run.actor)
        .expect("real Rapid Fire earns Bulletstorm");
    let expected = 1.0 + f64::from(earned.effects[0].amount) / 100.0;
    assert!(expected > 1.0, "earned native Bulletstorm increases damage");
    // Accuracy By Volume can grant extra uses; exercise the final real use.
    let uses = earned.stacks;
    for _ in 1..uses {
        cast(&mut run, AIMED_SHOT, false);
    }
    for _ in 0..40 {
        run.app.update();
    }
    run.app.world_mut().insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        AIMED_SHOT,
        "aimed_shot",
        1,
    ));
    cast(&mut run, AIMED_SHOT, false);
    assert!(
        !owned_buff(&run, BULLETSTORM),
        "Aimed consumes the real use"
    );
    for _ in 0..80 {
        run.app.update();
        if run.app.world().resource::<FactorCapture>().export()["captured_hits"].as_u64() == Some(1)
        {
            break;
        }
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    let events = capture["events"].as_array().unwrap();
    let event = |kind| {
        events
            .iter()
            .find(|event| event["event"] == kind && event["spell_id"] == AIMED_SHOT)
            .expect("actual Aimed Shot projectile damages its recipient")
    };
    let actual = event("result")["factors"]["native_spell_mod_product"]
        .as_f64()
        .unwrap();
    let other_modifiers: f64 = event("direct_amount")["applied_modifiers"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|modifier| modifier["op"] == 0 && modifier["source_spell_id"] != BULLETSTORM)
        .map(|modifier| 1.0 + modifier["value"].as_f64().unwrap() / 100.0)
        .product();
    let observed = actual / other_modifiers;
    assert!(
        (observed / expected - 1.0).abs() < 0.001,
        "consumed Bulletstorm must empower the landed shot: observed{observed}, expected{expected}"
    );
}
