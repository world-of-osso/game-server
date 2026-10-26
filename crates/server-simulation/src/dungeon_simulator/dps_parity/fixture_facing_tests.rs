//! Actual encounter setup must make native directional damage reachable.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires prepared native spec577 player profile"]
fn parity_fixture_facing_reaches_native_eye_beam_line() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(577, 3).unwrap();
    engage(&mut run);
    // Repeated real children keep ordinary native hit rolls; no test damage/geometry.
    for _ in 0..10 {
        crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 198030, None, 1);
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for target in &run.targets {
        assert!(
            events
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(198030)
                    && event.amount > 0
                    && event.target == Some(target.to_bits())),
            "native Eye Beam line must damage fixture enemy {target:?}"
        );
    }
}
