//! Bound actual guardian creation by source-qualified native producers.
use super::*;
use crate::spell_cast::summon::SummonedCreature;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::Npc;
use shared::protocol::CombatLogKind;
use std::collections::HashSet;

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_overfiend_literal_actual_spawns_require_source_producers() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(267, 1).unwrap();
    let started = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started);
    engage(&mut run);
    let mut actual_actors = HashSet::new();
    // SimC76f8118 actions4210–4240/5124–5136: actual Soul Fire produces
    // one Overfiend; a Dimensional Rift can produce at most one. Neither
    // ordinary spell hits nor the Overfiend's own bolts summon another.
    for _ in 0..1200 {
        drive_player(&mut run).unwrap();
        run.app.update();
        let owner = run.actor;
        let world = run.app.world_mut();
        for (entity, summon, npc) in world
            .query::<(Entity, &SummonedCreature, &Npc)>()
            .iter(world)
        {
            if (summon.owner, summon.spell_id, npc.template_id) == (owner, 434587, 217429) {
                actual_actors.insert(entity);
            }
        }
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let soul_fire = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(6353)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    let rifts = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(196586)
                && event.kind == CombatLogKind::CastSuccess
        })
        .count();
    assert!(
        soul_fire > 0,
        "literal native player must execute real Soul Fire"
    );
    assert!(
        actual_actors.len() >= soul_fire,
        "each actual Soul Fire must summon an Overfiend"
    );
    assert!(
        actual_actors.len() <= soul_fire + rifts,
        "actual Overfiend actors must be bounded by Soul Fire plus Rift casts: {} actors, {soul_fire} Soul Fire, {rifts} Rifts",
        actual_actors.len()
    );
}
