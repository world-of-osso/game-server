use super::*;
use server::simc_apl::Facts;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_bm_natures_ally_empowers_real_pet_command() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1273126));
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    let target = run.targets[0];
    engage(&mut run);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), run.actor, target);
    crate::pet_control::order_companion_pet(run.app.world_mut(), run.actor, Some(target));
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, 193455, Some(target), false)
        .expect("ordinary Cobra earns Nature's Ally");
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1276720));
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 471878));
    let bonus =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 1276720, 0);
    run.app
        .insert_resource(FactorCapture::new(pet, target, 83381, "kill_command", 2));
    for expected in 1..=2 {
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            34026,
            Some(target),
            false,
        )
        .expect("ordinary Kill Command");
        preparation::finish_pending_setup(&mut run).unwrap();
        for _ in 0..200 {
            run.app.update();
            if run.app.world().resource::<FactorCapture>().export()["captured_hits"]
                .as_u64()
                .unwrap()
                >= expected
            {
                break;
            }
        }
        assert!(!crate::auras::has_aura(run.app.world(), run.actor, 1276720));
        assert!(!crate::auras::has_aura(run.app.world(), run.actor, 471878));
    }
    let receipt = run.app.world().resource::<FactorCapture>().export();
    eprintln!("KC_OWNER_DIAGNOSTIC {}", receipt);
    let events = receipt["events"].as_array().unwrap();
    let results: Vec<_> = events
        .iter()
        .filter(|event| event["event"] == "result" && event["spell_id"] == 83381)
        .collect();
    let amounts: Vec<_> = events
        .iter()
        .filter(|event| event["event"] == "direct_amount" && event["spell_id"] == 83381)
        .collect();
    assert_eq!(
        (results.len(), amounts.len()),
        (2, 2),
        "two actual pet commands"
    );
    let normalized = |index: usize| {
        results[index]["factors"]["raw_amount"].as_f64().unwrap()
            / amounts[index]["factors"]["ap_term"].as_f64().unwrap()
            / results[index]["factors"]["target_product"]
                .as_f64()
                .unwrap()
    };
    let ratio = normalized(0) / normalized(1);
    let expected = 1.0 + f64::from(bonus) / 100.0;
    // SimC76f8118 hunter.cpp2779–2785 and 6585–6604: the armed
    // Nature's Ally value applies to the command before its consumption.
    assert!(
        (ratio - expected).abs() < 0.001,
        "actual armed pet command must receive Nature's Ally: actual={ratio}, expected={expected}"
    );
}

fn barbed_recharge(run: &Run) -> f64 {
    let bindings = crate::simc_apl_native::Bindings {
        actions: [("barbed_shot".into(), 217200)].into(),
        ..Default::default()
    };
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    crate::simc_apl_native::NativeFacts::new(run.app.world(), run.actor, &bindings, encounter, 1)
        .value("cooldown.barbed_shot.full_recharge_time")
        .unwrap()
}

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_bm_pack_mentality_recovers_real_barbed_recharge_after_consuming_howl() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 472358),
        "native reference must select Pack Mentality"
    );
    engage(&mut run);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), run.actor, run.targets[0]);
    run.app.update();
    let deadline = crate::spell_cast::now(run.app.world()) + 31.0;
    while !crate::auras::has_aura(run.app.world(), run.actor, 471878)
        && crate::spell_cast::now(run.app.world()) < deadline
    {
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 471878),
        "real combat clock must earn Wyvern readiness"
    );
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(19574, run.actor, Some(run.targets[0])),
    )
    .expect("ordinary Bestial Wrath advances a second earned beast");
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 472324),
        "selected native Stampede must also earn Boar readiness"
    );
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        217200,
        Some(run.targets[0]),
        false,
    )
    .expect("ordinary Barbed starts native recharge");
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 471878)
            && crate::auras::has_aura(run.app.world(), run.actor, 472324),
        "both earned beasts must remain ready after Barbed and its real GCD"
    );
    let before = barbed_recharge(&run);
    let reduction = f64::from(crate::spell_cast::scripts::script_effect_value(
        run.app.world(),
        run.actor,
        472358,
        1,
    )) / 1000.0;
    assert!(
        before > reduction,
        "the actual running charge must have time to recover"
    );
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        34026,
        Some(run.targets[0]),
        false,
    )
    .expect("ordinary KC consumes both actually earned beasts");
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 471878));
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 472324));
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 471881),
        "actual Wyvern summon must produce Cry"
    );
    let after = barbed_recharge(&run);
    // SimC76f8118 hunter3808–3811: each actual consumed beast reduces
    // already-running Barbed recharge by selected Pack Mentality's native time.
    let expected = (before - reduction * 2.0).max(0.0);
    assert!(
        (after - expected).abs() < 0.001,
        "actual Wyvern+Boar consumption must recover Barbed twice: before={before}, after={after}, per_beast={reduction}, expected={expected}"
    );
}

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_bm_pack_mentality_increases_real_kill_command_damage() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 472358));
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 269737));
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).expect("native pet");
    let target = run.targets[0];
    engage(&mut run);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), run.actor, target);
    crate::pet_control::order_companion_pet(run.app.world_mut(), run.actor, Some(target));
    for _ in 0..40 {
        run.app.update();
    }
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 471878));
    run.app
        .insert_resource(FactorCapture::new(pet, target, 83381, "kill_command", 2));
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, 34026, Some(target), false)
        .expect("ordinary KC before Howl readiness");
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..200 {
        run.app.update();
        if run.app.world().resource::<FactorCapture>().export()["captured_hits"]
            .as_u64()
            .unwrap()
            >= 1
        {
            break;
        }
    }
    let deadline = crate::spell_cast::now(run.app.world()) + 31.0;
    while !crate::auras::has_aura(run.app.world(), run.actor, 471878)
        && crate::spell_cast::now(run.app.world()) < deadline
    {
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 471878),
        "real combat earns Wyvern readiness"
    );
    let alpha =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 269737, 1);
    let pack =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 472358, 0);
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, 34026, Some(target), false)
        .expect("ordinary KC with naturally earned Howl readiness");
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..200 {
        run.app.update();
        if run.app.world().resource::<FactorCapture>().export()["captured_hits"]
            .as_u64()
            .unwrap()
            >= 2
        {
            break;
        }
    }
    let receipt = run.app.world().resource::<FactorCapture>().export();
    assert_eq!(
        receipt["captured_hits"].as_u64(),
        Some(2),
        "two actual owned-pet 83381 hits"
    );
    let events = receipt["events"].as_array().unwrap();
    let results: Vec<_> = events
        .iter()
        .filter(|event| event["event"] == "result" && event["spell_id"] == 83381)
        .collect();
    let amounts: Vec<_> = events
        .iter()
        .filter(|event| event["event"] == "direct_amount" && event["spell_id"] == 83381)
        .collect();
    assert_eq!((results.len(), amounts.len()), (2, 2));
    let normalized = |index: usize| {
        let result = &results[index]["factors"];
        let amount = &amounts[index]["factors"];
        result["raw_amount"].as_f64().unwrap()
            / amount["ap_term"].as_f64().unwrap()
            / result["target_product"].as_f64().unwrap()
    };
    let actual = normalized(1) / normalized(0);
    // SimC76f8118 sc_hunter.cpp2764–2785: Alpha Predator and selected
    // Pack Mentality add before multiplying the real pet command.
    let expected = (100.0 + f64::from(alpha) + f64::from(pack)) / (100.0 + f64::from(alpha));
    assert!(
        (actual - expected).abs() < 0.001,
        "earned readiness must amplify actual pet KC: before={}, after={}, actual={actual}, expected={expected}, alpha={alpha}, pack={pack}",
        normalized(0),
        normalized(1)
    );
}
