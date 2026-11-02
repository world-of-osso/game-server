//! Melee cooldowns belong to the unit, not to its current victim.
use super::*;
use crate::spell_cast::test_harness::harness;

fn assert_stop_restart(haste: f32, restarts: usize, finish_ticks: usize) {
    let mut h = harness(Vec::new(), &[], 1).expect("native combat fixture");
    let creature = h.npc;
    let victim = h.caster;
    h.app.world_mut().entity_mut(creature).insert((
        CreatureAttackTime(2.0),
        crate::stats::StatModifiers {
            haste_pct: haste,
            ..Default::default()
        },
    ));
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
    for _ in 0..restarts {
        attack_stop(h.app.world_mut(), creature);
        start(&mut h);
        h.advance(2);
    }
    assert_eq!(
        swings(&h),
        1,
        "restarts before the haste-adjusted cooldown must not grant new swings"
    );
    h.advance(finish_ticks);
    assert_eq!(swings(&h), 2, "the preserved cooldown eventually expires");
}

#[test]
fn creature_stop_restart_keeps_its_swing_timer() {
    assert_stop_restart(0.0, 10, 25);
}

#[test]
fn creature_stop_restart_keeps_haste_adjusted_swing_timer() {
    assert_stop_restart(100.0, 4, 15);
}
