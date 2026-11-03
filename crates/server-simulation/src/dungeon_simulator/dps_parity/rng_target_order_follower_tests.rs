//! Actual casts retain seeded target order across Bevy archetype changes.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::{Npc, UnitSummonedBy};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires native262 profile and ancestor template"]
fn ancestor_echo_casts_follow_entity_order_after_archetype_change() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(262, 1).unwrap();
    engage(&mut run);
    for _ in 0..2 {
        crate::spell_cast::scripts::summon_ancestor(run.app.world_mut(), run.actor, 0);
    }
    let mut ancestors: Vec<_> = run.app.world_mut()
        .query::<(Entity, &Npc, &UnitSummonedBy)>()
        .iter(run.app.world())
        .filter(|(_, npc, owner)| npc.template_id == 221177 && owner.0 == run.actor.to_bits())
        .map(|(entity, _, _)| entity)
        .collect();
    ancestors.sort_unstable();
    assert!(ancestors.len() >= 2);
    run.app.world_mut().entity_mut(ancestors[0]).insert(Name::new("reordered ancestor"));
    run.app.world_mut().resource_mut::<ObservedCombatEvents>().0.clear();
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, 188196, Some(run.targets[0]), false).unwrap();
    for _ in 0..60 { run.app.update(); }
    let casts: Vec<_> = run.app.world().resource::<ObservedCombatEvents>().0.iter()
        .filter(|event| event.spell_id == Some(447419) && event.kind == CombatLogKind::Damage)
        .filter_map(|event| event.source.and_then(Entity::try_from_bits))
        .filter(|actor| ancestors.contains(actor))
        .collect();
    assert_eq!(casts, ancestors, "foreground Bolt commands each ancestor in Entity order");
}

fn roaring_spread(reorder: bool) -> Vec<usize> {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 5).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let primary = run.targets[0];
    literal_rotation::execution::execute(run.app.world_mut(), actor,
        literal_rotation::execution::Execution::Spell(445468, primary, Some(primary))).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..30 { run.app.update(); }
    if reorder {
        run.app.world_mut().entity_mut(run.targets[1]).insert(Name::new("reordered opponent"));
    }
    {
        let _cast_seed = crate::random::seed_scope(90210);
        literal_rotation::execution::execute(run.app.world_mut(), actor,
            literal_rotation::execution::Execution::Spell(17962, primary, Some(primary))).unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        for _ in 0..30 { run.app.update(); }
    }
    run.targets.iter().enumerate().skip(1)
        .filter_map(|(index, &target)| crate::auras::has_aura(run.app.world(), target, 445474).then_some(index))
        .collect()
}

#[test]
#[ignore = "requires native267 profile and world data"]
fn roaring_blaze_seeded_spread_ignores_opponent_archetype_order() {
    let normal = roaring_spread(false);
    let reordered = roaring_spread(true);
    assert!(!normal.is_empty() && normal.len() < 4, "the spread must select a subset: {normal:?}");
    assert_eq!(normal, reordered);
}
