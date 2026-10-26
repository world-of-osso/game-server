//! SimC76f8118 sc_priest_shadow.cpp2745–2765/46–58/838–879/1471–1488/2197–2199.
//! The permanent talent marker is not its earned three-DoT window.
use super::literal_rotation::execution::{self, Execution};
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::{read_native_cast_start, CastFailure};
use crate::spell_info::SpellStore;

fn cast_when_ready(run: &mut Run, id: u32) {
    let native = run
        .app
        .world()
        .resource::<SpellStore>()
        .get(id)
        .unwrap()
        .clone();
    for _ in 0..400 {
        let ready = read_native_cast_start(run.app.world_mut(), run.actor, &native, run.targets[0]);
        match ready {
            Ok(_) => {
                let action = Execution::Spell(id, run.targets[0], None);
                execution::execute(run.app.world_mut(), run.actor, action).unwrap();
                return;
            }
            Err(
                CastFailure::OnGlobalCooldown
                | CastFailure::OnCooldown
                | CastFailure::NoCharges
                | CastFailure::InProgress,
            ) => run.app.update(),
            Err(reason) => panic!("ordinary native spell{id}: {reason:?}"),
        }
    }
    panic!("native spell{id} never became ready");
}

fn finish_cast(run: &mut Run) {
    for _ in 0..240 {
        if run
            .app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_none()
        {
            return;
        }
        run.app.update();
    }
    panic!("real native channel must finish");
}

fn owns_dot(run: &Run, id: u32) -> bool {
    run.app
        .world()
        .get::<ActiveAuras>(run.targets[0])
        .unwrap()
        .iter()
        .any(|aura| aura.caster == run.actor && aura.spell_id == id)
}

fn sample_flay_source(run: &mut Run) -> f64 {
    let actual = execution::resolve_action(run.app.world(), run.actor, 15407)
        .unwrap()
        .expect("actual ordinary Flay action");
    assert!(matches!(actual, 15407 | 391403));
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        actual,
        "mind_flay",
        1,
    ));
    cast_when_ready(run, actual);
    finish_cast(run);
    let capture = run.app.world().resource::<FactorCapture>().export();
    println!("actual Flay {actual} factor capture: {capture}");
    let result = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["event"] == "result" && row["spell_id"] == actual)
        .expect("actual native Flay damage");
    assert!(result["factors"]["actual_amount"].as_f64().unwrap() > 0.0);
    let amount = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["event"] == "periodic_amount" && row["spell_id"] == actual)
        .expect("actual native Flay periodic factors");
    let mastery = amount["applied_multipliers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["factor"] == "shadow_weaving")
        .expect("actual native Shadow Weaving factor")["value"]
        .as_f64()
        .unwrap();
    result["factors"]["source_product"].as_f64().unwrap() / mastery
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn diag_ire_real_three_dot_window_boosts_actual_flay_then_naturally_expires() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(258, 1).unwrap();
    engage(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 373212));
    let percent =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 373212, 0);
    assert_eq!(
        percent, 25.0,
        "native definition108688 curve59064 rank2 sets25 over raw20"
    );
    let baseline = sample_flay_source(&mut run);
    cast_when_ready(&mut run, 1227280);
    finish_cast(&mut run);
    cast_when_ready(&mut run, 589);
    assert!(
        [589, 34914, 335467]
            .into_iter()
            .all(|id| owns_dot(&run, id)),
        "ordinary Pain and real Slam's VT/weak Madness earn all three live own DoTs"
    );
    let earned = sample_flay_source(&mut run);
    let expected = 1.0 + f64::from(percent) / 100.0;
    assert!((earned / baseline - expected).abs() < 0.005,
        "real owned DoTs earn the native Ire source bonus: baseline={baseline}, earned={earned}, expected={expected}");
    for _ in 0..800 {
        if !owns_dot(&run, 335467) {
            break;
        }
        run.app.update();
    }
    assert!(
        !owns_dot(&run, 335467),
        "actual weakest DoT reaches natural expiry"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 373212),
        "talent marker remains learned"
    );
    let expired = sample_flay_source(&mut run);
    assert!((expired / baseline - 1.0).abs() < 0.005,
        "natural earned-window expiry removes bonus without removing talent: baseline={baseline}, expired={expired}");
}
