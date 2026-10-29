//! Equipped Omnium's Fire rune can proc from real healing.
use super::*;
use shared::{
    components::{Health, PowerType},
    protocol::CombatLogKind,
};

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn omnium_fire_healing_procs_heal_the_triggering_player() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1279599),
        "fixture has Fire rune"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1287555),
        "fixture has Lingering rune"
    );
    for _ in 0..60 {
        {
            let mut health = run.app.world_mut().get_mut::<Health>(actor).unwrap();
            health.current = health.max * 0.5;
        }
        {
            let mut pools = run
                .app
                .world_mut()
                .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
                .unwrap();
            pools.get_mut(PowerType::Energy).unwrap().current = 100.0;
        }
        crate::spell_cast::scripts::reduce_cooldown(run.app.world_mut(), actor, 322101, 1000.0);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 322101, Some(actor), false)
            .unwrap();
        for _ in 0..300 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        if events.iter().any(|event| {
            event.spell_id == Some(1287665) && event.kind == CombatLogKind::Heal && event.amount > 0
        }) {
            assert!(events.iter().any(|event| event.spell_id == Some(1263002)
                && event.kind == CombatLogKind::Heal
                && event.target == Some(actor.to_bits())
                && event.amount > 0));
            return;
        }
    }
    panic!("healing must trigger both direct and lingering Fire rune heals");
}
