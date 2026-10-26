//! Native Reap payload must follow an actual Sudden Death Execute, not paid Rage.
use super::literal_rotation::execution::{Execution, execute};
use super::*;
use crate::spell_cast::powers::PowerPools;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::{Health, PowerType};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_reap_the_storm_requires_an_actual_sudden_death_execute() {
    for sudden_death in [false, true] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_measurement(71, 3).unwrap();
        engage(&mut run);
        let primary = run.targets[0];
        {
            let mut health = run.app.world_mut().get_mut::<Health>(primary).unwrap();
            health.current = health.max * 0.1;
        }
        run.app
            .world_mut()
            .get_mut::<PowerPools>(run.actor)
            .unwrap()
            .get_mut(PowerType::Rage)
            .unwrap()
            .current = 400.0;
        if sudden_death {
            crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 52437, Some(run.actor), 1);
        }
        execute(
            run.app.world_mut(),
            run.actor,
            Execution::Spell(163201, primary, Some(primary)),
        )
        .unwrap();
        run.app.update();
        let hits: Vec<_> = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(446005)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .filter_map(|event| event.target)
            .collect();
        assert_eq!(
            hits.len(),
            if sudden_death { 3 } else { 0 },
            "native Imminent Demise100% must dispatch one native Reap, not any paid Execute: sudden_death={sudden_death}"
        );
        let targets: std::collections::BTreeSet<_> = hits.into_iter().collect();
        assert_eq!(targets.len(), if sudden_death { 3 } else { 0 });
    }
}
