//! Actual Soul Fire applies a free background Wither before impact.
use super::*;
use crate::auras::ActiveAuras;
use crate::class_progression::TraitLoadout;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_soul_fire_actual_cast_applies_background_wither() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    assert!(
        run.app
            .world()
            .get::<TraitLoadout>(actor)
            .unwrap()
            .active_talent_spells()
            .any(|talent| talent.spell_id == 445465)
    );
    engage(&mut run);
    assert!(!crate::auras::has_aura(run.app.world(), target, 445474));
    let actual = literal_rotation::execution::resolve_action(run.app.world(), actor, 6353)
        .unwrap()
        .expect("selected native Soul Fire");
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(actual, target, Some(target)),
    )
    .unwrap();
    for _ in 0..120 {
        run.app.update();
        let events = &run.app.world().resource::<ObservedCombatEvents>().0;
        if events.iter().any(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(6353)
                && event.kind == CombatLogKind::CastSuccess
        }) {
            let owned = run
                .app
                .world()
                .get::<ActiveAuras>(target)
                .is_some_and(|auras| {
                    auras
                        .iter()
                        .any(|aura| aura.spell_id == 445474 && aura.caster == actor)
                });
            // SimC76f8118 actions5102–5138: Soul Fire's actual execute
            // applies the free dot to its chosen target, before missile impact.
            assert!(
                owned,
                "actual paid Soul Fire execution must apply owned Wither"
            );
            assert!(
                !events
                    .iter()
                    .any(|event| event.target == Some(target.to_bits())
                        && event.spell_id == Some(445468)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0),
                "background dot deals no direct Wither damage"
            );
            return;
        }
    }
    panic!("native paid Soul Fire must complete");
}
