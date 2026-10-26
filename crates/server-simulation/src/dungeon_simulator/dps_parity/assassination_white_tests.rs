//! Earned Deathstalker white-hit children with real marked/unmarked victims.
use super::*;
use crate::spell_cast::test_harness::{ObservedCombatEvents, ObservedTriggeredCasts};
use shared::protocol::CombatLogKind;

fn observe_white_hits(run: &mut Run, target: Entity) -> (usize, usize) {
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    run.app
        .world_mut()
        .resource_mut::<ObservedTriggeredCasts>()
        .0
        .clear();
    for _ in 0..200 {
        run.app.update();
    }
    let world = run.app.world();
    let whites = world
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id.is_none()
                && event.kind == CombatLogKind::Damage
        })
        .count();
    let children = world
        .resource::<ObservedTriggeredCasts>()
        .0
        .iter()
        .filter(|&&(caster, spell, victim)| {
            caster == run.actor && spell == 457193 && victim == Some(target)
        })
        .count();
    assert!(
        whites > 5,
        "the actual weapon victim must receive enough landed swings"
    );
    (whites, children)
}

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn assassination_white_hunt_them_down_follows_owned_mark_not_selected_target() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 3).unwrap();
    run.app.init_resource::<ObservedTriggeredCasts>();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 457054));
    engage(&mut run);
    let actor = run.actor;
    let marked = run.targets[0];
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(703, marked, Some(marked)),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), marked, 457129));
    let (white_hits, children) = observe_white_hits(&mut run, marked);
    assert_eq!(
        children, white_hits,
        "each actual landed white hit against the owned mark must launch native457193 once"
    );
    let unmarked = run.targets[1];
    assert!(!crate::auras::has_aura(run.app.world(), unmarked, 457129));
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::RetargetAutoAttack(unmarked),
    )
    .unwrap();
    let (_, children) = observe_white_hits(&mut run, unmarked);
    assert_eq!(
        children, 0,
        "retargeted weapons must not dispatch to a mark on a different target"
    );
}
