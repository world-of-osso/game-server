//! Real Takedown supplies native owner Focus and selected Twin Fangs uses.
use super::marksmanship_mechanics_tests::cast;
use super::*;

fn player() -> Run {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(255, 1).unwrap();
    hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
    cast(&mut run, 883, true);
    engage(&mut run);
    run
}

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_takedown_earns_its_owner_focus_without_pet_duplicate() {
    // SimC76f8118 sc_hunter.cpp3096–3105/6368–6379: owner Takedown
    // earns native1258571 Focus once; its pet child does not energize.
    let mut run = player();
    cast(&mut run, 186270, false);
    cast(&mut run, 186270, false);
    let amount = crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 1258571, 0);
    let spent = cast(&mut run, 1250646, false);
    assert!(amount > 0.0);
    assert!((spent + amount).abs() < 0.01,
        "actual Takedown must earn exactly native owner Focus once: spent={spent}, native gain={amount}");
}

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_takedown_selected_twin_fangs_grants_tip_before_owner_impact() {
    // SimC76f8118 sc_hunter.cpp6390–6404: selected Twin Fangs grants
    // native uses before the actual owner damage impact consumes one.
    let mut run = player();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1272139));
    assert_eq!(owned_tip(&run), 0);
    let count = crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 1272139, 0) as u8;
    cast(&mut run, 1250646, false);
    assert!(count > 0);
    assert_eq!(owned_tip(&run), count - 1,
        "actual Takedown must grant selected native Twin Fangs uses before its damage consumes one");
}

fn owned_tip(run: &Run) -> u8 {
    run.app.world().get::<crate::auras::ActiveAuras>(run.actor).unwrap().iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == 260286)
        .map_or(0, |aura| aura.stacks)
}
