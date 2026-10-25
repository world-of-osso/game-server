//! The Dev adapter must use the shared channel decision seam during native ticks.
use super::*;
use crate::dungeon_simulator::dps_parity::{engage, prepare_literal};
use server::simc_apl::Program;

#[test]
#[ignore = "requires private native DB and exact Devastation profile"]
fn devastation_adapter_interrupts_channel_for_a_ready_higher_row() {
    let mut run = prepare_literal(1467, 1).unwrap();
    engage(&mut run);
    let started_at = crate::spell_cast::now(run.app.world());
    let mut rotation = Rotation::load(
        run.app.world(),
        run.actor,
        Encounter::Fixture {
            started_at,
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        },
    )
    .unwrap();
    rotation.driver = Driver::new(
        Program::parse(&[
            ("default", "azure_strike,if=time>0.1"),
            ("default", "disintegrate,interrupt=1"),
        ])
        .unwrap(),
    );
    rotation
        .drive(run.app.world_mut(), run.actor, &run.targets)
        .unwrap();
    let duration = run
        .app
        .world()
        .get::<shared::casting::CastState>(run.actor)
        .expect("native Disintegrate must start its actual channel")
        .duration;
    let deadline = started_at + f64::from(duration) * 0.9;
    while crate::spell_cast::now(run.app.world()) < deadline {
        run.app.update();
        rotation
            .drive(run.app.world_mut(), run.actor, &run.targets)
            .unwrap();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(362969)
                && event.kind == shared::protocol::CombatLogKind::CastSuccess
        }),
        "the higher ready Azure Strike must execute before Disintegrate's natural end; duration={duration}, elapsed={}, cast={:?}, decisions={}, events={events:?}",
        crate::spell_cast::now(run.app.world()) - started_at,
        run.app.world().get::<shared::casting::CastState>(run.actor),
        rotation.driver.decisions,
    );
}
