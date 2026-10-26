//! Native Reap payload must follow an actual Sudden Death Execute, not paid Rage.
use super::literal_rotation::execution::{Execution, execute};
use super::*;
use crate::spell_cast::powers::PowerPools;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::{Health, PowerType};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_reap_cleave_requires_native_three_target_threshold() {
    for target_count in [2, 3] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_measurement(71, target_count).unwrap();
        let started = run.app.world().resource::<Time>().elapsed();
        install_fixture_lifetimes(&mut run, started);
        engage(&mut run);
        for _ in 0..40 {
            run.app
                .world_mut()
                .get_mut::<PowerPools>(run.actor)
                .unwrap()
                .get_mut(PowerType::Rage)
                .unwrap()
                .current = 1000.0;
            execute(
                run.app.world_mut(),
                run.actor,
                Execution::Spell(845, run.targets[0], Some(run.targets[0])),
            )
            .unwrap();
            for _ in 0..100 {
                run.app.update();
            }
        }
        let hits = run
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
            .count();
        if target_count == 2 {
            assert_eq!(hits, 0, "Cleave below native3-target threshold never rolls Reap");
        } else {
            assert!(hits > 0, "actual3-target Cleaves must roll native Reap's20% chance");
            assert_eq!(hits % 3, 0, "each Reap dispatch hits the native pack once");
            assert!(hits <= 40 * 3, "at most one proc per Cleave execution");
        }
    }
}

