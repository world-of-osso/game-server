//! Native Inferno Arc uses the actual recipient's owned Flame Shock.
use super::elemental_mechanics_tests::{fixture, own_aura};
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;

fn captured_target_factor(run: &mut Run, spell: u32) -> f64 {
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        spell,
        "inferno_arc",
        1,
    ));
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        spell,
        Some(run.targets[0]),
        1,
    ));
    for _ in 0..60 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    let factors = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "direct_amount" && event["spell_id"] == spell)
        .expect("actual native damage result")["applied_multipliers"]
        .as_array()
        .unwrap();
    ["giantkiller", "target_damage"]
        .into_iter()
        .map(|name| {
            factors
                .iter()
                .find(|factor| factor["factor"] == name)
                .unwrap()["value"]
                .as_f64()
                .unwrap()
        })
        .product()
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_inferno_arc_requires_owned_flame_shock_on_recipient() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    let percent = own_aura(&run, 1259047)
        .expect("selected Inferno Arc")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount as f64;
    assert!(percent > 0.0);
    let plain = captured_target_factor(&mut run, 45297);
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        188389,
        Some(run.targets[0]),
        1,
    ));
    let affected = captured_target_factor(&mut run, 45297);
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.targets[0],
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 188389 && aura.caster == run.actor,
    );
    let removed = captured_target_factor(&mut run, 45297);
    assert!((plain - 1.0).abs() < 0.001, "no Flame Shock: {plain}");
    assert!(
        (affected - (1.0 + percent / 100.0)).abs() < 0.001,
        "real Flame Shock amplifies native Chain Overload once: {affected}"
    );
    assert!(
        (removed - 1.0).abs() < 0.001,
        "removing the owned Flame Shock removes its factor: {removed}"
    );
}
