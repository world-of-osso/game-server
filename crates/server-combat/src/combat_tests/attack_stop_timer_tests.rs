//! Melee cooldowns belong to the unit, not to its current victim.
use super::*;
use crate::spell_cast::test_harness::harness;

#[test]
fn creature_stop_restart_keeps_its_swing_timer() {
    let mut h = harness(Vec::new(), &[], 1).expect("native combat fixture");
    let creature = h.npc;
    let victim = h.caster;
    h.app
        .world_mut()
        .entity_mut(creature)
        .insert(CreatureAttackTime(2.0));
    h.app.world_mut().get_mut::<Health>(victim).unwrap().current = 1_000_000.0;
    let start = |h: &mut crate::spell_cast::test_harness::Harness| {
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, h.app.world());
        start_attack(&mut commands, creature, victim, None);
        queue.apply(h.app.world_mut());
    };
    start(&mut h);
    h.advance(3);
    let swings = |h: &crate::spell_cast::test_harness::Harness| {
        h.inbox
            .log
            .iter()
            .filter(|event| event.source == Some(creature.to_bits()) && event.spell_id.is_none())
            .count()
    };
    assert_eq!(swings(&h), 1, "the first attack must swing");
    for _ in 0..10 {
        attack_stop(h.app.world_mut(), creature);
        start(&mut h);
        h.advance(2);
    }
    assert_eq!(
        swings(&h),
        1,
        "restarts within one second must not grant new swings"
    );
    h.advance(25);
    assert_eq!(
        swings(&h),
        2,
        "the preserved two-second cooldown eventually expires"
    );
}
