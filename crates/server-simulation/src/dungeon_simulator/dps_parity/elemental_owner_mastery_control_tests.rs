//! The pet mastery correction must not change the owning player's own damage.
use super::elemental_mechanics_tests::fixture;
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;

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
