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
    capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "direct_amount")
        .expect("actual native damage result")["applied_multipliers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|factor| factor["factor"] == "giantkiller")
        .expect("target-qualified multiplier")["value"]
        .as_f64()
        .unwrap()
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
    let unrelated = captured_target_factor(&mut run, 285466);
    assert!((plain - 1.0).abs() < 0.001, "no Flame Shock: {plain}");
    assert!(
        (affected - (1.0 + percent / 100.0)).abs() < 0.001,
        "real Flame Shock amplifies native Chain Overload: {affected}"
    );
    assert!(
        (unrelated - 1.0).abs() < 0.001,
        "Lava Overload is excluded: {unrelated}"
    );
}
