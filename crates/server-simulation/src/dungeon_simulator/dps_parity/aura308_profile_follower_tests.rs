use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;

#[test]
#[ignore = "requires frozen native Marksmanship profile and selected traits"]
fn marksmanship_aimed_mark_grants_one_native_critical_bonus() {
    let mut run = prepare_literal(254, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(19434)
        .unwrap()
        .clone();
    assert!(crate::auras::has_aura(run.app.world(), actor, 1253846));
    let before = crate::spell_cast::effects::crit_chance(run.app.world(), actor, target, &spell);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1301098, Some(target), 1);
    let expected =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 1253846, 1);
    assert_eq!(expected, 50.0);
    let marked = crate::spell_cast::effects::crit_chance(run.app.world(), actor, target, &spell);
    assert!((marked - before - expected).abs() < 0.001);
    run.app
        .world_mut()
        .insert_resource(FactorCapture::new(actor, target, 19434, "aimed_shot", 1));
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 19434, Some(target), 1);
    for _ in 0..80 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    let result = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "result")
        .expect("actual Aimed Shot damage must be observed");
    let actual = result["factors"]["crit"].as_f64().unwrap()
        + result["factors"]["tgt_crit"].as_f64().unwrap();
    assert!(
        (actual * 100.0 - f64::from(marked)).abs() < 0.001,
        "actual Aimed Shot must not double-count its native mark bonus: {result}"
    );
}
