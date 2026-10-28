//! Reapplication of a real Magus periodic effect retains its tick phase.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::summon::SummonedCreature;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires native252 profile and Magus templates"]
fn unholy_magus_recast_preserves_grasp_tick_phase() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(42650, run.actor, None),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let owner = run.actor;
    let magus = {
        let world = run.app.world_mut();
        world
            .query::<(Entity, &SummonedCreature)>()
            .iter(world)
            .find_map(|(entity, summon)| {
                (summon.owner == owner && summon.spell_id == 317776).then_some(entity)
            })
            .expect("actual Army summons a Magus")
    };
    let mut cursor = run.app.world().resource::<ObservedCombatEvents>().0.len();
    let mut last_tick = None;
    let mut refreshed = false;
    for _ in 0..400 {
        run.app.update();
        let world = run.app.world();
        let now = world.resource::<Time>().elapsed_secs_f64();
        let events = &world.resource::<ObservedCombatEvents>().0;
        for event in &events[cursor..] {
            if event.source != Some(magus.to_bits()) || event.spell_id != Some(1297091) {
                continue;
            }
            if event.kind == CombatLogKind::CastSuccess && last_tick.is_some() {
                refreshed = true;
                let remaining = world
                    .get::<ActiveAuras>(run.targets[0])
                    .unwrap()
                    .iter()
                    .find(|aura| aura.spell_id == 1297091 && aura.caster == magus)
                    .and_then(|aura| aura.expires_at)
                    .unwrap()
                    - now;
                assert!(
                    remaining > 3.0,
                    "real refresh must carry the next tick into its 3s duration: {remaining}"
                );
            }
            if event.kind == CombatLogKind::Damage && event.periodic && event.amount > 0 {
                if let Some(before) = last_tick
                    && refreshed
                {
                    let elapsed = now - before;
                    assert!(
                        elapsed <= 1.05,
                        "refresh must not restart the 1s tick phase: {elapsed}s"
                    );
                    return;
                }
                last_tick = Some(now);
            }
        }
        cursor = events.len();
    }
    panic!("real Magus must refresh Withering Grasp between consecutive ticks");
}
