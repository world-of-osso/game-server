//! Native earned Mongoose Fury and Stargazer producers; no primed buffs or stats.
use super::marksmanship_mechanics_tests::cast;
use super::*;

fn player() -> Run {
    let mut run = prepare_literal(255, 1).unwrap();
    hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
    cast(&mut run, 883, true);
    engage(&mut run);
    run
}

fn owned_stacks(run: &Run, spell: u32) -> u8 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == spell)
        .map_or(0, |aura| aura.stacks)
}

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_real_raptor_and_swipe_casts_earn_mongoose_fury_stacks() {
    // SimC76f8118 sc_hunter.cpp6149–6155: both actual base executions
    // trigger native259388; the ordinary first Strike earns the next Swipe.
    let mut run = player();
    assert_eq!(owned_stacks(&run, 259388), 0);
    cast(&mut run, 186270, false);
    assert_eq!(owned_stacks(&run, 259388), 1, "actual Raptor earned no native Mongoose Fury");
    cast(&mut run, 186270, false);
    assert_eq!(owned_stacks(&run, 259388), 2, "actual Swipe did not add a native Mongoose Fury stack");
}

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_real_tip_consumption_earns_selected_stargazer_stack() {
    // SimC76f8118 sc_hunter.cpp1358–1361/8303–8305: actual primary
    // Tip consumption earns the selected asynchronous native Stargazer buff.
    let mut run = player();
    assert_eq!(owned_stacks(&run, 1253750), 0);
    cast(&mut run, 259489, false);
    let before = owned_stacks(&run, 260286);
    assert!(before > 0, "prerequisite: actual Kill Command earned no Tip");
    cast(&mut run, 186270, false);
    assert_eq!(owned_stacks(&run, 260286), before - 1);
    assert_eq!(owned_stacks(&run, 1253750), 1, "actual Tip consumption earned no native Stargazer");
}
