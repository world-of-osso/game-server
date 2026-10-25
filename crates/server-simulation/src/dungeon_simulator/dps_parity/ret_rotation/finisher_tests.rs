//! Real pinned finisher-list controls; no default-list/adds.up substitute.
use super::super::{
    Run, engage, install_fixture_lifetimes,
    literal_rotation::{Driver, equipment::Equipment, execution::Prepared},
    prepare_literal,
    ret_purpose_tests::earn_finisher_power,
};
use super::{context::Context, *};
use crate::simc_apl_native::{Encounter, FightStyle};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use bevy::prelude::*;
use shared::protocol::CombatLogKind;

fn prepare_finisher(targets: usize, spell: u32) -> Run {
    let mut run = prepare_literal(70, targets).unwrap();
    engage(&mut run);
    earn_finisher_power(&mut run, spell);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    run
}

fn dispatch_finisher(run: &mut Run) -> String {
    let mut bindings = bindings();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings).unwrap();
    let mut driver = Driver::new(read_program().unwrap());
    for _ in 0..200 {
        let prepared = Prepared::read(
            run.app.world_mut(),
            run.actor,
            &run.targets,
            &equipment,
            execution_actions(),
            false,
        )
        .unwrap();
        let facts = Context::read(
            run.app.world(),
            run.actor,
            &bindings,
            &equipment,
            &prepared,
            Encounter::Fixture {
                started_at: 0.0,
                duration: 300.0,
                style: FightStyle::Patchwerk,
            },
            &run.targets,
        );
        if let Some((action, execution)) = driver
            .select("finishers", &facts, &prepared, &equipment)
            .unwrap()
        {
            let name = action.name.clone();
            driver
                .execute(run.app.world_mut(), run.actor, action, execution)
                .unwrap();
            return name;
        }
        run.app.update();
    }
    panic!("actual naturally funded source finisher never became ready");
}

fn assert_own_damage(run: &mut Run, spell: u32) {
    for _ in 0..200 {
        run.app.update();
        if run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && run
                        .targets
                        .iter()
                        .any(|target| Some(target.to_bits()) == event.target)
            })
        {
            return;
        }
    }
    panic!("actual source-selected native finisher{spell} did not damage a real supplied target");
}

#[test]
#[ignore = "pinned Ret finisher rows must select native FV on a real single target without resource grants"]
fn dps_parity_ret_literal_finisher_single_selects_native_verdict() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_finisher(1, 383328);
    assert_eq!(dispatch_finisher(&mut run), "templars_verdict");
    assert_own_damage(&mut run, 383328);
}

#[test]
#[ignore = "pinned Ret finisher rows must select native DivineStorm with three real admitted enemies"]
fn dps_parity_ret_literal_finisher_three_selects_native_storm() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_finisher(3, 53385);
    assert_eq!(dispatch_finisher(&mut run), "divine_storm");
    assert_own_damage(&mut run, 53385);
}
