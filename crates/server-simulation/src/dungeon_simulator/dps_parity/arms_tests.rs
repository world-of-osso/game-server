//! Actual reference profile must execute its precombat stance and combat APL.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "private current native DB and exact spec71 inputs"]
fn arms_literal_reference_executes_stance_and_rend_on_three_targets() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(71, 3).unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 386164),
        "pinned Arms precombat must actually cast Battle Stance"
    );
    let started = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started);
    engage(&mut run);
    for _ in 0..400 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(845)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }),
        "the literal opener must land native Cleave before checking its Rend child"
    );
    let rend_targets: std::collections::BTreeSet<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(388539)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .filter_map(|event| event.target)
        .collect();
    assert_eq!(
        rend_targets.len(),
        3,
        "literal Slayer AoE opens with Cleave to maintain native Rend"
    );
}
