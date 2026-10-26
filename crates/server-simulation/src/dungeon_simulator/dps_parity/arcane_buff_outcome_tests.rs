//! A merged buff grant must not create another spell's native proc inputs.
use super::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn sample_buff_then_splinter(seed: u64, queued: bool) -> Value {
    let _seed = crate::random::seed_scope(seed);
    let mut run = prepare_measurement(62, 1).unwrap();
    engage(&mut run);
    if queued {
        crate::spell_cast::queued_triggers::schedule_buff_outcome(
            run.app.world_mut(),
            run.actor,
            394195,
            true,
        );
    } else {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &AuraApplication {
                caster: run.actor,
                target: run.actor,
                spell_id: 394195,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
    for _ in 0..20 {
        run.app.update();
    }
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 394195));
    // Native394195 modifies Barrage, not this actual Splinter or its Infused
    // Salvo roll. No native crit, damage variance, stats or RNG inputs change.
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        443763,
        Some(run.targets[0]),
        1,
    );
    for _ in 0..20 {
        run.app.update();
    }
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(443763)
                && event.kind == CombatLogKind::Damage
        })
        .map(|event| (event.amount, event.crit))
        .collect::<Vec<_>>();
    assert!(!hits.is_empty(), "the real native Splinter must land");
    let salvo = run
        .app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == 1242974)
        .map_or(0, |aura| aura.stacks);
    json!({"hits": hits, "salvo": salvo, "stats": report::stats(run.app.world(), run.actor)})
}

#[test]
#[ignore = "requires private native70448 Arcane62 inputs"]
fn arcane_buff_outcome_queued_grant_preserves_unrelated_native_splinter_proc_inputs() {
    // SimC76f8118 sc_mage.cpp7272–7293/5585–5605: merged outcomes call
    // buff.trigger, not another action.execute. A native queued grant must be
    // observationally identical to direct native aura application, not a cast
    // that changes unrelated native equipment/proc rolls before the next hit.
    for seed in [70448, 1, 2, 3] {
        let direct = sample_buff_then_splinter(seed, false);
        let queued = sample_buff_then_splinter(seed, true);
        assert_eq!(
            queued, direct,
            "native merged buff seed={seed}: queued grant changed unrelated damage/Salvo/stat outcomes"
        );
    }
}
