//! Dragonrage's native dummy launches the same real Pyre explosions as the APL.
use super::*;

#[test]
#[ignore = "requires native Devastation follower profile and world DB"]
fn devastation_dragonrage_cast_launches_a_pyre_per_target() {
    for target_count in [1, 3] {
        let mut run = prepare_literal(1467, target_count).unwrap();
        engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, 375087, Some(run.actor), false).unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        for _ in 0..100 { run.app.update(); }
        let events = &run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
        let hits: Vec<_> = events.iter().filter(|event| event.source == Some(run.actor.to_bits())
            && event.spell_id == Some(357212)
            && event.kind == shared::protocol::CombatLogKind::Damage).collect();
        assert_eq!(hits.len(), target_count * target_count, "each selected target produces a real area Pyre explosion");
        assert!(hits.iter().all(|event| event.amount > 0));
    }
}
