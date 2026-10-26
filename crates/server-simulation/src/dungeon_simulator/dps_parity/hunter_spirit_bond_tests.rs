//! Read actual native Spirit Bond payload before inferring a missing modifier.
use super::marksmanship_mechanics_tests::cast;
use super::*;

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn native_spirit_bond_selected_payload_includes_native_mastery_dependency() {
    // SimC76f8118 sc_hunter.cpp1392–1395/7900: the actual selected
    // mastery contribution includes459722's native percentage, not guessed stats.
    let mut run = prepare_literal(255, 1).unwrap();
    hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
    cast(&mut run, 883, true);
    engage(&mut run);
    for _ in 0..4 {
        run.app.update();
    }
    let world = run.app.world();
    let points = world
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .mastery_points;
    let store = world.resource::<crate::spell_info::SpellStore>();
    let mastery = store.get(263135).unwrap();
    let coefficient = mastery
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .bonus_coefficient;
    let dependency = store.get(459722).unwrap();
    let percent = dependency
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points;
    let active = world.get::<crate::auras::ActiveAuras>(run.actor).unwrap();
    let aura = active
        .iter()
        .find(|aura| aura.spell_id == 263135 && aura.caster == run.actor)
        .expect("actual selected native Spirit Bond must exist");
    let amount = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    let expected = points * coefficient * (1.0 + percent / 100.0);
    println!(
        "NATIVE_SPIRIT_BOND points={points} coefficient={coefficient} dependency_pct={percent} actual={amount} expected={expected}"
    );
    assert!(points > 0.0 && coefficient > 0.0 && percent > 0.0);
    assert!(
        (amount - expected).abs() < 0.01,
        "actual native Spirit Bond magnitude lacks its sourced native percentage: actual={amount}, expected={expected}"
    );
}
