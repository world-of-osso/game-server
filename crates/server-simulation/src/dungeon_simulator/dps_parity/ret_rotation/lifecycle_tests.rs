//! Exercise the actual measurement lifecycle, not a manually dispatched Ret rotation.
use super::super::{advance_combat_observed, engage, prepare};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use bevy::prelude::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "native Ret measurement must naturally cast Wake and a HolyPower spender within30s"]
fn dps_parity_ret_literal_native_first_30s_reaches_wake_and_spender() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare(70, 3).unwrap();
    engage(&mut run);
    let started = run.app.world().resource::<Time>().elapsed_secs_f64();
    let mut checked = false;
    advance_combat_observed(&mut run, |run| {
        let elapsed = run.app.world().resource::<Time>().elapsed_secs_f64() - started;
        if checked || elapsed < 30.0 {
            return;
        }
        checked = true;
        let casts: Vec<_> = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits()) && event.kind == CombatLogKind::CastSuccess
            })
            .filter_map(|event| event.spell_id)
            .collect();
        assert!(
            casts.contains(&255937) && casts.iter().any(|spell| matches!(spell, 85256 | 383328 | 53385)),
            "actual first30s measurement must naturally cast Wake and a HolyPower spender; casts={casts:?}"
        );
    })
    .unwrap();
    assert!(
        checked,
        "actual native clock must reach the30s proof boundary"
    );
}
