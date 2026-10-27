//! Roaring Blaze spreads an earned Wither without a foreground cast.
use super::*;
use crate::class_progression::TraitLoadout;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn cast_paid(run: &mut Run, spell: u32) {
    let actor = run.actor;
    let target = run.targets[0];
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(spell, target, Some(target)),
    )
    .unwrap();
    preparation::finish_pending_setup(run).unwrap();
    for _ in 0..30 {
        run.app.update();
    }
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_roaring_blaze_actual_conflagrate_spreads_owned_wither() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 3).unwrap();
    let actor = run.actor;
    let targets = run.targets.clone();
    assert!(
        run.app
            .world()
            .get::<TraitLoadout>(actor)
            .unwrap()
            .active_talent_spells()
            .any(|talent| talent.spell_id == 1244310)
    );
    engage(&mut run);
    cast_paid(&mut run, 445468);
    assert!(crate::auras::has_aura(run.app.world(), targets[0], 445474));
    for &target in &targets[1..] {
        assert!(!crate::auras::has_aura(run.app.world(), target, 445474));
    }
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    cast_paid(&mut run, 17962);
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(targets[0].to_bits())
                && event.spell_id == Some(17962)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    // SimC76f8118 actions4440-4500: selected Roaring Blaze spreads the
    // primary's existing DoT to nearby targets as a free zero-direct action.
    for &target in &targets[1..] {
        assert!(
            crate::auras::has_aura(run.app.world(), target, 445474),
            "actual Conflagrate must spread Wither to {target:?}"
        );
        assert!(
            !events
                .iter()
                .any(|event| event.target == Some(target.to_bits())
                    && event.spell_id == Some(445468)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "spread must not deal direct Wither damage"
        );
    }
}
