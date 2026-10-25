use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_hunter_pet_focus_inherits_owner_rate_continuously() {
    use server::simc_apl::Facts;
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    let bindings = crate::simc_apl_native::Bindings {
        resources: [("focus".into(), shared::components::PowerType::Focus)].into(),
        ..Default::default()
    };
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let fact = |run: &Run, actor, name| {
        crate::simc_apl_native::NativeFacts::new(run.app.world(), actor, &bindings, encounter, 1)
            .value(name)
            .unwrap()
    };
    let rate = fact(&run, run.actor, "focus.regen");
    crate::spell_cast::creature_cast(run.app.world_mut(), pet, 49966, Some(run.targets[0]), false)
        .expect("ordinary pet ability spends real Focus");
    let before = fact(&run, pet, "focus");
    let started = crate::spell_cast::now(run.app.world());
    assert!(before < fact(&run, pet, "focus.max") - 5.0);
    for _ in 0..5 {
        run.app.update();
    }
    let elapsed = crate::spell_cast::now(run.app.world()) - started;
    let gained = fact(&run, pet, "focus") - before;
    // SimC76f8118 sc_hunter.cpp2355–2360: owner native Focus rate times1.25.
    let expected = rate * 1.25 * elapsed;
    assert!(
        (gained - expected).abs() < 0.05,
        "pet Focus must use continuous native owner regen, not legacy4s ticks: gained={gained}, expected={expected}, elapsed={elapsed}, owner_rate={rate}"
    );
}
