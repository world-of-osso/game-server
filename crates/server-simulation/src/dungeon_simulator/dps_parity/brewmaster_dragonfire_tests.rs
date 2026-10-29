//! Dragonfire Brew follows real Breath of Fire with two delayed impacts.
use super::*;
use shared::protocol::CombatLogKind;

fn hits(run: &Run, target: Entity) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.spell_id == Some(387621)
                && event.kind == CombatLogKind::Damage
                && event.target == Some(target.to_bits())
                && event.amount > 0
        })
        .count()
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn breath_of_fire_schedules_both_dragonfire_impacts() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 115181, Some(target), false)
        .unwrap();
    let start = crate::spell_cast::now(run.app.world());
    while crate::spell_cast::now(run.app.world()) < start + 1.2 {
        run.app.update();
    }
    assert_eq!(hits(&run, target), 0);
    while crate::spell_cast::now(run.app.world()) < start + 1.9 {
        run.app.update();
    }
    assert_eq!(hits(&run, target), 1);
    while crate::spell_cast::now(run.app.world()) < start + 3.4 {
        run.app.update();
    }
    assert_eq!(hits(&run, target), 2);
}
