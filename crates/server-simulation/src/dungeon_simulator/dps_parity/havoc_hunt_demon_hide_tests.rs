//! SimC76f8118 sc_demon_hunter.cpp8486–8501: Hunt's bugs-mode periodic multiplier.
use super::*;
use shared::protocol::CombatLogKind;

fn hunt_tick(selected: bool) -> i32 {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(577, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    let known = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(actor)
        .unwrap();
    assert!(known.contains(428241));
    let spells = known
        .iter()
        .filter(|spell| selected || *spell != 428241)
        .collect();
    run.app
        .world_mut()
        .entity_mut(actor)
        .insert(crate::class_progression::KnownSpells::fixture(spells));
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 370969, Some(target), 1);
    for _ in 0..80 {
        run.app.update();
    }
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .find(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(370969)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .expect("real Hunt periodic damage")
        .amount
}

#[test]
#[ignore = "requires prepared native spec577 player profile"]
fn havoc_hunt_dot_uses_demon_hide_twice() {
    let normal = hunt_tick(false);
    let selected = hunt_tick(true);
    assert!(
        (selected as f32 / normal as f32 - 1.03).abs() < 0.002,
        "native Hunt tick {normal} -> {selected}"
    );
}
