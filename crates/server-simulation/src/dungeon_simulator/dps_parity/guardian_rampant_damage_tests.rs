//! Guardian tier damage is earned by actual Thrash casts.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires Guardian four-piece profile and native world data"]
fn guardian_thrash_earns_three_rampant_thorn_hits() {
    let mut run = prepare_registered_player(104, 3).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let hits = |run: &Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|e| {
                e.kind == CombatLogKind::Damage
                    && e.spell_id == Some(1301157)
                    && e.source == Some(actor.to_bits())
                    && e.amount > 0
            })
            .count()
    };
    assert_eq!(hits(&run), 0);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 77758, Some(target), false)
        .unwrap();
    for _ in 0..24 {
        run.app.update();
    }
    assert_eq!(hits(&run), 9);
    for _ in 0..24 {
        run.app.update();
    }
    assert_eq!(hits(&run), 9);
}
