//! Actual Brewmaster strikes can trigger the native Walk with the Ox Stomp.
use super::*;
use shared::{components::Health, protocol::CombatLogKind};

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn strikes_trigger_walk_with_the_ox() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    for _ in 0..60 {
        {
            let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
            health.current = health.max;
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 100780, Some(target), false).unwrap();
        for _ in 0..150 {
            run.app.update();
        }
    }
    let events = &run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
    assert!(events.iter().any(|event| event.spell_id == Some(1242373)
        && event.kind == CombatLogKind::Damage
        && event.target == Some(target.to_bits()) && event.amount > 0),
        "eligible strikes must trigger the native Stomp");
}
