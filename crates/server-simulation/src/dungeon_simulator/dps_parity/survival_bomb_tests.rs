//! Native Wildfire recipients and selected Shrapnel payload, from real casts.
use super::marksmanship_mechanics_tests::cast;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_wildfire_hits_pack_and_uses_selected_shrapnel_bleed() {
    // SimC76f8118 sc_hunter.cpp7030/7062–7068/7083–7089: actual
    // explosion recipients share the selected Shrapnel native periodic payload.
    let mut run = prepare_literal(255, 3).unwrap();
    engage(&mut run);
    cast(&mut run, 259495, false);
    for _ in 0..60 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for &target in &run.targets {
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(265157)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "actual in-radius enemy {target:?} must take the real Wildfire explosion"
        );
        let auras = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(target)
            .unwrap();
        assert!(
            auras
                .iter()
                .any(|aura| aura.caster == run.actor && aura.spell_id == 1253171),
            "selected Shrapnel must apply its native bleed to actual explosion target {target:?}"
        );
        assert!(
            !auras
                .iter()
                .any(|aura| aura.caster == run.actor && aura.spell_id == 269747),
            "selected Shrapnel replaces the ordinary fire DoT; it cannot apply both"
        );
    }
}
