//! SimC76f8118 sc_rogue.cpp5131–5135,6278–6287,7993–7999.
use super::*;
use crate::auras::{ActiveAuras, RemoveReason};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn assassination_doomblade_both_real_mutilate_hands_fund_native_bleed() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let native_percent = |spell| {
        run.app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == spell && aura.caster == actor)
            .unwrap()
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap()
            .amount
    };
    let fraction = native_percent(381673) / 100.0;
    let dose = native_percent(381640) / 100.0;
    // Keep the target's only counted bleed the earned Doomblade itself.
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        RemoveReason::Cancelled,
        |aura| matches!(aura.spell_id, 2823 | 381664 | 3408 | 381637),
    );
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    run.app
        .world_mut()
        .insert_resource(FactorCapture::new(actor, target, 394021, "doomblade", 32));
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1329, Some(target), 1);
    let initial_bleed = run
        .app
        .world()
        .get::<ActiveAuras>(target)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 394021 && aura.caster == actor)
                .cloned()
        });
    for _ in 0..180 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let hands: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && matches!(event.spell_id, Some(5374 | 27576))
                && event.kind == CombatLogKind::Damage
        })
        .collect();
    assert_eq!(
        hands.len(),
        2,
        "one actual Mutilate must land both native hands"
    );
    let ticks: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(394021)
                && event.kind == CombatLogKind::Damage
                && event.periodic
        })
        .collect();
    assert!(
        !ticks.is_empty(),
        "actual Mutilate hands must earn native394021 periodic damage"
    );
    assert!(
        ticks.iter().all(|event| !event.crit),
        "a residual retains the source's crit once"
    );
    let expected =
        hands.iter().map(|event| event.amount as f32).sum::<f32>() * fraction * (1.0 + dose);
    let actual = ticks.iter().map(|event| event.amount as f32).sum::<f32>();
    assert!(
        (actual - expected).abs() <= 6.0,
        "both resolved hits fund the native bleed fraction and its dynamic Lethal Dose: {actual}/{expected}; fraction{fraction},dose{dose}; hands{:?}; ticks{:?}; initial{initial_bleed:?}; factors{}",
        hands
            .iter()
            .map(|event| (event.spell_id, event.amount, event.crit))
            .collect::<Vec<_>>(),
        ticks.iter().map(|event| event.amount).collect::<Vec<_>>(),
        run.app.world().resource::<FactorCapture>().export()
    );
}
