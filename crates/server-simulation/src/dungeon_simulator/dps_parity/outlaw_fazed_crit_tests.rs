//! Real Trickster Fazed modifies only its owner's matching critical attacks.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::effects::factor_capture::FactorCapture;

fn crit_multiplier(run: &mut Run, spell: u32) -> f64 {
    let actor = run.actor;
    let target = run.targets[0];
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .crit_pct = 100.0;
    run.app
        .insert_resource(FactorCapture::new(actor, target, spell, "fazed_crit", 1));
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        actor,
        spell,
        Some(target),
        1
    ));
    for _ in 0..100 {
        run.app.update();
        let capture = run.app.world().resource::<FactorCapture>().export();
        if let Some(hit) = capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == "direct_amount" && event["spell_id"] == spell)
        {
            return hit["applied_multipliers"]
                .as_array()
                .unwrap()
                .iter()
                .find(|factor| factor["factor"] == "critical_damage")
                .unwrap()["value"]
                .as_f64()
                .unwrap();
        }
    }
    panic!("real attack {spell} did not hit");
}

#[test]
#[ignore = "requires native260 profile and Trickster talents"]
fn outlaw_surprising_strikes_uses_real_fazed_critical_bonus() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(260, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 441273));
    let pistol_before = crit_multiplier(&mut run, 185763);
    let dispatch_before = crit_multiplier(&mut run, 2098);
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        actor,
        441144,
        Some(target),
        1
    ));
    for _ in 0..60 {
        run.app.update();
        if crate::auras::has_aura(run.app.world(), target, 441224) {
            break;
        }
    }
    let aura = run
        .app
        .world()
        .get::<ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 441224 && aura.caster == actor)
        .expect("real Unseen Blade applies owned Fazed");
    let bonus = aura
        .effects
        .iter()
        .find(|effect| effect.index == 3)
        .unwrap()
        .amount;
    assert!(
        bonus > 0.0,
        "selected native talent must modify Fazed effect4"
    );
    let pistol_after = crit_multiplier(&mut run, 185763);
    let dispatch_after = crit_multiplier(&mut run, 2098);
    assert!(
        (pistol_after / pistol_before - (1.0 + f64::from(bonus) / 200.0)).abs() < 0.02,
        "owned Fazed must increase Pistol Shot critical bonus by native effect4: {pistol_before} -> {pistol_after}"
    );
    assert!(
        (dispatch_after / dispatch_before - 1.0).abs() < 0.01,
        "nonmatching finisher must not receive the combo-generator critical bonus"
    );
}
