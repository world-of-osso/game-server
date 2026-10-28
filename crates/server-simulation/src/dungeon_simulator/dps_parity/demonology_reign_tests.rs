use super::*;

#[test]
#[ignore = "requires canonical 266 inputs; Reign extends actual dogs"]
fn demonology_reign_tyrant_extends_living_dreadstalker() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(266, 1).unwrap();
    let owner = run.actor;
    assert!(crate::auras::has_aura(run.app.world(), owner, 1276748));
    crate::spell_cast::creature_cast(run.app.world_mut(), owner, 193331, Some(owner), true)
        .unwrap();
    let dog = run
        .app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(run.app.world())
        .find(|(_, summon)| summon.owner == owner && summon.spell_id == 193331)
        .map(|(entity, _)| entity)
        .expect("actual Dreadstalker");
    let before = run
        .app
        .world()
        .get::<crate::spell_cast::summon::SummonedCreature>(dog)
        .unwrap()
        .expires_at
        .unwrap();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        owner,
        265187,
        Some(run.targets[0]),
        true,
    )
    .unwrap();
    let after = run
        .app
        .world()
        .get::<crate::spell_cast::summon::SummonedCreature>(dog)
        .unwrap()
        .expires_at
        .unwrap();
    assert!(
        (after - before - 15.0).abs() < 0.001,
        "actual dog gains source 15s: {before} to {after}"
    );
}

#[test]
#[ignore = "requires canonical 266 inputs; Vilefiend counts for Tyrant power"]
fn demonology_vilefiend_counts_for_new_tyrant_power() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(266, 1).unwrap();
    let owner = run.actor;
    crate::spell_cast::creature_cast(run.app.world_mut(), owner, 455465, Some(owner), true)
        .unwrap();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        owner,
        265187,
        Some(run.targets[0]),
        true,
    )
    .unwrap();
    let tyrant = run
        .app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(run.app.world())
        .find(|(_, summon)| summon.owner == owner && summon.spell_id == 265187)
        .map(|(entity, _)| entity)
        .expect("actual Tyrant");
    assert!(
        crate::auras::has_aura(run.app.world(), tyrant, 1276788),
        "actual Gloomhound grants Demonic Power on Tyrant summon"
    );
}
