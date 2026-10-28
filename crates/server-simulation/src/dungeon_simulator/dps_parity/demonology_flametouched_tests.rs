use super::*;

#[test]
#[ignore = "requires canonical 266 inputs; actual Dreadstalker Flametouched"]
fn demonology_flametouched_empowers_actual_dreadstalker() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(266, 1).unwrap();
    let owner = run.actor;
    assert!(crate::auras::has_aura(run.app.world(), owner, 453699));
    crate::spell_cast::creature_cast(run.app.world_mut(), owner, 193331, Some(owner), true)
        .expect("native Dreadstalker summon");
    let dog = run
        .app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(run.app.world())
        .find(|(_, summon)| summon.owner == owner && summon.spell_id == 193331)
        .map(|(actor, _)| actor)
        .expect("actual Dreadstalker");
    let aura = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(dog)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 453704 && aura.caster == owner)
        .expect("selected Flametouched grants native Ferocity to the actual dog");
    let haste = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    let crit = aura
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .amount;
    let expected_haste =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), owner, 453699, 0);
    let expected_crit =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), owner, 453699, 1);
    assert!(
        (haste - expected_haste).abs() < 0.01,
        "native haste {haste} expected {expected_haste}"
    );
    assert!(
        (crit - expected_crit).abs() < 0.01,
        "native crit {crit} expected {expected_crit}"
    );
}
