//! SimC76f8118 sc_druid.cpp3745–3749/11313–11316: one use per Cat action.
use super::*;

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_native_tigers_fury_earns_two_guaranteed_unseen_attacks_from_real_shreds() {
    let _seed = crate::random::seed_scope(1031263962);
    let mut run = prepare_literal(103, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(5217, actor, Some(target)),
    )
    .expect("ordinary selected native Tiger's Fury");
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    for (expected_hits, expected_stacks) in [(1, Some(1)), (2, None), (2, None)] {
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5221, Some(target), false)
            .expect("real admitted native Shred after Tiger's Fury");
        for _ in 0..40 {
            run.app.update();
        }
        let hits = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.spell_id == Some(1263890)
                    && event.kind == shared::protocol::CombatLogKind::Damage
                    && event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
            })
            .count();
        assert_eq!(
            hits, expected_hits,
            "actual Cat actions consume exactly two guaranteed uses"
        );
        let stacks = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .and_then(|auras| {
                auras
                    .iter()
                    .find(|aura| aura.spell_id == 1263962 && aura.caster == actor)
            })
            .map(|aura| aura.stacks);
        assert_eq!(
            stacks, expected_stacks,
            "one native Stalking stack is spent per ordinary Cat action"
        );
    }
}
