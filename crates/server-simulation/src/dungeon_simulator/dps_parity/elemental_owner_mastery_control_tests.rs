//! The pet mastery correction must not change the owning player's own damage.
use super::elemental_mechanics_tests::fixture;
use crate::spell_cast::effects::factor_capture::FactorCapture;

#[test]
#[ignore = "requires native Elemental262 profile and world data"]
fn elemental_late_family_modifiers_recalculate_fury_and_overload_mastery() {
    // SimC76f8118 sc_shaman.cpp4848–4850 and the passive processing in
    // player.cpp: selected Primordial Fury and Feedback Loop raise Fury;
    // Echo Chamber changes the mastery effect used by real overloads.
    let mut run = fixture();
    let amount = |run: &super::Run, spell, index| {
        run.app.world().get::<crate::auras::ActiveAuras>(run.actor).unwrap().iter()
            .find(|aura| aura.caster == run.actor && aura.spell_id == spell).unwrap()
            .effects.iter().find(|effect| effect.index == index).unwrap().amount
    };
    assert_eq!(amount(&run, 60188, 0), 105.0);
    assert_eq!(amount(&run, 168534, 1), 31.25);
    assert!((overload_effectiveness(&mut run) - 0.3125).abs() < 0.001);
    crate::auras::remove_auras(run.app.world_mut(), run.actor, crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 1270062);
    assert_eq!(amount(&run, 60188, 0), 75.0);
    crate::auras::remove_auras(run.app.world_mut(), run.actor, crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 378193 || aura.spell_id == 382032);
    assert_eq!(amount(&run, 60188, 0), 50.0);
    assert_eq!(amount(&run, 168534, 1), 25.0);
    assert!((overload_effectiveness(&mut run) - 0.25).abs() < 0.001);
}

fn overload_effectiveness(run: &mut super::Run) -> f64 {
    run.app.insert_resource(FactorCapture::new(run.actor, run.targets[0], 45297, "overload", 1));
    assert!(crate::spell_cast::try_cast_triggered(run.app.world_mut(), run.actor, 45297, Some(run.targets[0]), 1));
    for _ in 0..80 { run.app.update(); }
    let capture = run.app.world().resource::<FactorCapture>().export();
    capture["events"].as_array().unwrap().iter()
        .find(|event| event["event"] == "result")
        .expect("actual Chain Lightning Overload hit")["factors"]["native_script_product"]
        .as_f64().unwrap()
}

fn player_overload_factor(remove_mastery: bool) -> f64 {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    if remove_mastery {
        crate::auras::remove_auras(
            run.app.world_mut(),
            run.actor,
            crate::auras::RemoveReason::Consumed,
            |aura| aura.spell_id == 168534,
        );
    }
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        45284,
        "player_overload",
        1,
    ));
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        45284,
        Some(run.targets[0]),
        1
    ));
    for _ in 0..80 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "result")
        .expect("real player overload damage")["factors"]["native_spell_mod_product"]
        .as_f64()
        .unwrap()
}

#[test]
#[ignore = "requires native Elemental262 profile and world data"]
fn elemental_owner_overload_retains_its_own_mastery_modifier() {
    let present = player_overload_factor(false);
    let absent = player_overload_factor(true);
    assert!(
        present > absent * 1.1,
        "real player mastery must remain: {present} versus {absent}"
    );
}
