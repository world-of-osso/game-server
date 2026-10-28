use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires canonical 266 inputs; actual Gloomhound inherits owner haste"]
fn demonology_gloomhound_melee_uses_owner_attack_haste() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(266, 1).unwrap();
    super::channel_timing_tests::remove_equipped_proc_drivers(&mut run);
    let owner = run.actor;
    let now = crate::spell_cast::now(run.app.world());
    crate::auras::remove_auras(run.app.world_mut(), owner, crate::auras::RemoveReason::Cancelled, |aura| {
        aura.expires_at.is_some_and(|at| at <= now + 60.0)
    });
    run.app.update();
    let haste = run.app.world().get::<crate::stats::CharacterStats>(owner).unwrap().haste_pct;
    assert!(haste > 10.0, "native owner has real haste");
    engage(&mut run);
    run.app.world_mut().entity_mut(owner).insert(crate::networking::CombatTarget(Some(run.targets[0])));
    crate::spell_cast::creature_cast(run.app.world_mut(), owner, 455465, Some(owner), true).unwrap();
    let dog = run.app.world_mut().query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(run.app.world()).find(|(_, summon)| summon.owner == owner && summon.spell_id == 455465)
        .map(|(actor,_)| actor).expect("actual Gloomhound");
    let mut seen = 0;
    let mut times = Vec::new();
    for _ in 0..180 {
        run.app.update();
        let events = &run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
        let count = events.iter().filter(|event| event.source == Some(dog.to_bits())
            && event.spell_id.is_none() && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))).count();
        if count > seen { times.push(crate::spell_cast::now(run.app.world())); seen = count; }
        if times.len() >= 3 { break; }
    }
    assert!(times.len() >= 3, "actual Gloomhound must melee repeatedly: {times:?}");
    let expected = 2.0 / (1.0 + f64::from(haste) / 100.0);
    for pair in times.windows(2) {
        assert!(((pair[1] - pair[0]) - expected).abs() < 0.11,
            "native dog swings {:?} must use owner haste {haste}% and source 2s base", times);
    }
}
