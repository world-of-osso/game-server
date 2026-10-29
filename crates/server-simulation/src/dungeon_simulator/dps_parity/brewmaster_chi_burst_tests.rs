//! Chi Burst's native projectile emits damage and healing on actual casts.
use super::*;
use shared::{components::Health, protocol::CombatLogKind};

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn chi_burst_damages_the_enemy_and_heals_the_caster() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    {
        let mut health = run.app.world_mut().get_mut::<Health>(actor).unwrap();
        health.current = health.max * 0.5;
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 123986, Some(target), false)
        .unwrap();
    for _ in 0..300 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(events.iter().any(|event| event.spell_id == Some(148135)
        && event.kind == CombatLogKind::Damage
        && event.target == Some(target.to_bits())
        && event.amount > 0));
    assert!(events.iter().any(|event| event.spell_id == Some(130654)
        && event.kind == CombatLogKind::Heal
        && event.target == Some(actor.to_bits())
        && event.amount > 0));
}
