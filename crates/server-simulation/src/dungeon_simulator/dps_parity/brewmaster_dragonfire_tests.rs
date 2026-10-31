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
#[ignore = "native Dragonfire after a lethal Breath of Fire"]
fn dragonfire_hits_survivor_after_lethal_breath() {
    use shared::components::Health;

    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 3).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let primary = run.targets[0];
    let survivor = run.targets[1];
    run.app.world_mut().get_mut::<Health>(primary).unwrap().current = 1.0;
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 115181, Some(primary), false)
        .unwrap();
    for _ in 0..100 { run.app.update(); }
    assert!(run.app.world().get::<Health>(primary).is_none_or(|health| health.current <= 0.0));
    let events = &run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
    assert!(events.iter().any(|event| event.source == Some(actor.to_bits())
        && event.target == Some(primary.to_bits())
        && event.spell_id == Some(115181)
        && event.kind == CombatLogKind::Damage
        && event.amount > 0), "Breath of Fire must kill the primary");
    assert!(hits(&run, survivor) > 0, "the destination cone must still hit the survivor");
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
