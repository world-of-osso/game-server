//! SimC76f8118 sc_rogue.cpp7457–7475: selected Deepening Shadows
//! extends actual Shadow Dance duration using native haste rating.
use super::*;

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_dance_duration_native_haste_keeps_real_dance_past_six_seconds() {
    let mut run = prepare_literal(261, 1).unwrap();
    engage(&mut run);
    assert!(
        run.app
            .world()
            .get::<crate::stats::CharacterStats>(run.actor)
            .unwrap()
            .ratings
            .haste
            > 0.0,
        "reference actor carries native haste rating"
    );
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(185313, run.actor, Some(run.actor)),
    )
    .unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 185313),
        "real Shadow Dance starts"
    );
    let start = run.app.world().resource::<Time>().elapsed_secs_f64();
    for _ in 0..400 {
        if run.app.world().resource::<Time>().elapsed_secs_f64() >= start + 7.0 {
            break;
        }
        run.app.update();
    }
    assert!(
        run.app.world().resource::<Time>().elapsed_secs_f64() >= start + 7.0,
        "fixture advances seven seconds"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 185313),
        "reference haste and selected Deepening Shadows keep Dance active after seven seconds"
    );
}
