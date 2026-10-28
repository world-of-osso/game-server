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

#[test]
#[ignore = "requires canonical 266 inputs; imploded Imps preserve Tyrant power"]
fn demonology_imploded_imps_preserve_tyrant_demonic_power() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(266, 1).unwrap();
    let owner = run.actor;
    crate::auras::remove_auras(
        run.app.world_mut(),
        owner,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 267216,
    );
    for _ in 0..3 {
        crate::spell_cast::creature_cast(run.app.world_mut(), owner, 104317, Some(owner), true)
            .unwrap();
    }
    let imps: Vec<_> = run
        .app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(run.app.world())
        .filter(|(_, summon)| summon.owner == owner && summon.spell_id == 104317)
        .map(|(entity, _)| entity)
        .collect();
    assert_eq!(imps.len(), 3);
    engage(&mut run);
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
    let stacks = |world: &World| {
        world
            .get::<crate::auras::ActiveAuras>(tyrant)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 1276788)
            .unwrap()
            .stacks
    };
    let before = stacks(run.app.world());
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        owner,
        196277,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    for _ in 0..100 {
        run.app.update();
        if imps
            .iter()
            .all(|imp| run.app.world().get_entity(*imp).is_err())
        {
            break;
        }
    }
    assert!(
        imps.iter()
            .all(|imp| run.app.world().get_entity(*imp).is_err()),
        "actual imps must implode"
    );
    // Implosion can summon new Inner Demons, which add their own stacks.
    let new_imps = run
        .app
        .world_mut()
        .query::<&crate::spell_cast::summon::SummonedCreature>()
        .iter(run.app.world())
        .filter(|summon| summon.owner == owner && summon.spell_id == 279910)
        .count() as u16;
    assert_eq!(
        u16::from(stacks(run.app.world())),
        u16::from(before) + new_imps,
        "imploded Imps retain their stacks and new Imps add theirs"
    );
}
