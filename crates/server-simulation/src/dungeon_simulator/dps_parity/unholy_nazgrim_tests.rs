//! Real Nazgrim lifetime and paid rune behavior.
use super::*;
use crate::auras::ActiveAuras;
use shared::components::PowerType;

fn conquest(run: &Run) -> Option<f32> {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)?
        .iter()
        .find(|aura| aura.spell_id == 444763 && aura.caster == run.actor)?
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .map(|effect| effect.amount)
}

#[test]
#[ignore = "requires native252 profile and Rider templates"]
fn unholy_nazgrim_conquest_follows_real_rider_and_paid_runes() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    assert!(conquest(&run).is_none());
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(42650, run.actor, None),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let initial = conquest(&run).expect("real Army Nazgrim grants owner Conquest");
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 444052));
    let per_rune =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 444052, 1);
    let before =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::Runes)
            .unwrap()
            .0;
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(85948, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    let after =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::Runes)
            .unwrap()
            .0;
    assert!(before > after, "actual Festering Strike must spend runes");
    let boosted = conquest(&run).unwrap();
    assert!(
        (boosted - initial - (before - after) * per_rune).abs() < 0.001,
        "earned strength must match actual rune spend: {initial} -> {boosted}"
    );
    for _ in 0..800 {
        run.app.update();
        if conquest(&run).is_none() {
            return;
        }
    }
    panic!("owner Conquest must expire when actual Nazgrim departs");
}
