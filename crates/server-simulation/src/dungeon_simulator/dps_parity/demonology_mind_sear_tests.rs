use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires canonical 266 inputs; real Inquisitor Mind Sear initial tick"]
fn demonology_inquisitor_mind_sear_ticks_immediately() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(266, 3).unwrap();
    let owner = run.actor;
    engage(&mut run);
    run.app
        .world_mut()
        .entity_mut(owner)
        .insert(crate::networking::CombatTarget(Some(run.targets[0])));
    crate::spell_cast::creature_cast(run.app.world_mut(), owner, 1276283, Some(owner), true)
        .unwrap();
    let inquisitor = run
        .app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(run.app.world())
        .find(|(_, summon)| summon.owner == owner && summon.spell_id == 1276283)
        .map(|(actor, _)| actor)
        .expect("real Antoran Inquisitor");
    for _ in 0..8 {
        run.app.update();
    }
    let hits = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for target in &run.targets {
        assert!(
            hits.iter()
                .any(|event| event.source == Some(inquisitor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(1280460)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "the native channel begins with an immediate tick against each target"
        );
    }
}
