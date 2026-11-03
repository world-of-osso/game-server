use super::*;
use crate::auras;
use crate::spell_cast::effects::factor_capture::FactorCapture;

#[test]
#[ignore = "requires MID2 Brewmaster profile and world data"]
fn dps_parity_brewmaster_balanced_stratagem_alternates_real_casts() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_registered_player(268, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    assert!(auras::has_aura(run.app.world(), actor, 450889));
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 115181, Some(target), false)
        .unwrap();
    assert!(
        auras::has_aura(run.app.world(), actor, 451514),
        "Breath of Fire earns physical bonus"
    );
    for _ in 0..40 {
        run.app.update();
    }
    run.app
        .insert_resource(FactorCapture::new(actor, target, 100780, "tiger_palm", 1));
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 100780, Some(target), false)
        .unwrap();
    let capture = run.app.world().resource::<FactorCapture>().export();
    let direct = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["spell_id"] == 100780 && event["event"] == "direct_amount")
        .unwrap();
    assert!(
        direct["applied_modifiers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|modifier| modifier["source_spell_id"] == 451514 && modifier["value"] == 5.0),
        "physical strike uses 5% native Brewmaster bonus: {direct}"
    );
    assert!(
        !auras::has_aura(run.app.world(), actor, 451514),
        "Tiger Palm consumes physical bonus"
    );
    assert!(
        auras::has_aura(run.app.world(), actor, 451508),
        "Tiger Palm earns magic bonus"
    );
    for _ in 0..300 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 115181, Some(target), false)
        .unwrap();
    assert!(
        !auras::has_aura(run.app.world(), actor, 451508),
        "Breath consumes magic bonus"
    );
    assert!(
        auras::has_aura(run.app.world(), actor, 451514),
        "Breath earns another physical bonus"
    );
}

#[test]
#[ignore = "requires MID2 Brewmaster profile and world data"]
fn dps_parity_brewmaster_chi_burst_consumes_magic_before_granting_physical() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_registered_player(268, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 100780, Some(target), false)
        .unwrap();
    assert!(auras::has_aura(run.app.world(), actor, 451508));
    for _ in 0..40 {
        run.app.update();
    }
    run.app.insert_resource(FactorCapture::new(
        actor,
        target,
        148135,
        "chi_burst_damage",
        1,
    ));
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 123986, Some(target), false)
        .unwrap();
    for _ in 0..60 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    let direct = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["spell_id"] == 148135 && event["event"] == "direct_amount")
        .unwrap();
    let multiplier = direct["applied_multipliers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["factor"] == "cast_effectiveness")
        .unwrap()["value"]
        .as_f64()
        .unwrap();
    assert!(
        (multiplier - 1.05).abs() < 0.001,
        "Chi Burst child must consume the earned 5%: {direct}"
    );
    assert!(!auras::has_aura(run.app.world(), actor, 451508));
    assert!(auras::has_aura(run.app.world(), actor, 451514));
}
