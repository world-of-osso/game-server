//! Native Volley impacts produce Salvo's capped background Explosive Shots once.
use super::marksmanship_mechanics_tests::{cast, owned_buff, player};
use super::*;
use shared::protocol::CombatLogKind;

fn explosive_expiries(run: &Run) -> Vec<(Entity, f64)> {
    run.targets
        .iter()
        .filter_map(|&target| {
            let aura = run
                .app
                .world()
                .get::<crate::auras::ActiveAuras>(target)?
                .iter()
                .find(|aura| aura.caster == run.actor && aura.spell_id == 212431)?;
            Some((target, aura.expires_at.expect("native Explosive duration")))
        })
        .collect()
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn native_mm_salvo_volley_applies_capped_background_explosives_only_once() {
    // SimC76f8118 sc_hunter.cpp6940–6965: the first Volley pulse launches
    // background Explosive Shot on native Salvo's first N actual recipients.
    let mut run = player(3);
    let cap = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(400456)
        .unwrap()
        .effects[0]
        .base_points as usize;
    assert!(cap > 0 && cap < run.targets.len());
    cast(&mut run, 260243, false);
    for _ in 0..20 {
        run.app.update();
    }
    let initial = explosive_expiries(&run);
    assert_eq!(
        initial.len(),
        cap,
        "actual Salvo applies its native target cap"
    );
    assert!(
        !owned_buff(&run, 1301778),
        "background shots do not earn Unstable Trigger"
    );
    for _ in 0..40 {
        run.app.update();
    }
    assert_eq!(
        explosive_expiries(&run),
        initial,
        "later Volley pulses must not refresh or spread the Salvo applications"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for (target, _) in initial {
        assert!(
            events.iter().any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(212431)
                    && event.kind == CombatLogKind::Damage
                    && event.periodic
                    && event.amount > 0
            }),
            "actual background Explosive Shot ticks on {target:?}"
        );
    }
}
