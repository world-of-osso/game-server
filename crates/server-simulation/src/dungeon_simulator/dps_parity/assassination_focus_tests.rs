//! SimC76f8118 sc_rogue.cpp11568–11605: real off-mark damage feeds the owned mark.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn assassination_focus_copies_actual_direct_and_periodic_damage_to_owned_mark() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 3).unwrap();
    let actor = run.actor;
    let marked = run.targets[0];
    let secondary = run.targets[1];
    let fraction = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 457055)
        .expect("native Singular Focus selected")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount
        / 100.0;
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(703, marked, Some(marked)),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), marked, 457129));
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    // Native damage on the marked target must never copy back into itself.
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 315585, Some(marked), 1);
    assert!(
        !run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.spell_id == Some(457236))
    );
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 315585, Some(secondary), 1);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 2818, Some(secondary), 1);
    for _ in 0..120 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let sources: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(secondary.to_bits())
                && event.kind == CombatLogKind::Damage
        })
        .collect();
    assert!(
        sources.iter().any(|event| event.periodic),
        "real native periodic source"
    );
    assert!(
        sources.iter().any(|event| !event.periodic),
        "real native direct source"
    );
    let copies: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(457236)
                && event.kind == CombatLogKind::Damage
        })
        .collect();
    assert_eq!(
        copies.len(),
        sources.len(),
        "each real off-mark hit must copy once"
    );
    assert!(
        copies
            .iter()
            .all(|event| event.target == Some(marked.to_bits()) && !event.crit)
    );
    let expected = sources.iter().map(|event| event.amount as f32).sum::<f32>() * fraction;
    let actual = copies.iter().map(|event| event.amount as f32).sum::<f32>();
    assert!(
        (actual - expected).abs() <= sources.len() as f32 * 2.0,
        "native fraction of actual damage, without another crit or damage multiplier: {actual}/{expected}"
    );
}
