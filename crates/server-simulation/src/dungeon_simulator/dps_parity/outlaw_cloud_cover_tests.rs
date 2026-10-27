//! Cloud Cover is earned by Adrenaline Rush and adds owned Fazed stacks.
use super::*;
use shared::components::PowerType;

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn outlaw_cloud_cover_earns_window_and_fazed_from_actual_attacks() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 441429));
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 441640));
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(13750, run.actor, Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 441640),
        "actual Adrenaline Rush must earn the Cloud Cover window"
    );
    for _ in 0..4 {
        {
            let mut powers = run
                .app
                .world_mut()
                .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
                .unwrap();
            let energy = powers.get_mut(PowerType::Energy).unwrap();
            energy.current = energy.max;
        }
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                193315,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
    }
    let stacks = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.targets[0])
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 441224 && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks);
    // SimC76f8118 sc_rogue.cpp3579/8434–8465/8720–8729: actual
    // landed attacks in the earned window add native owned Fazed stacks.
    assert!(
        stacks >= 2,
        "Cloud Cover attacks must earn multiple owned Fazed stacks; got {stacks}"
    );
}
