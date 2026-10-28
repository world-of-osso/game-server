//! Flashing Claws repeats a real Thrash after its native delay.
use super::*;
use crate::auras::{self, ActiveAuras, RemoveReason};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires Guardian Flashing Claws profile and native world data"]
fn guardian_flashing_claws_repeats_thrash_after_half_second() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let instance = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 393427 && a.caster == actor)
        .unwrap()
        .instance_id;
    assert!(auras::set_effect_amount(
        run.app.world_mut(),
        actor,
        instance,
        0,
        100.0
    ));
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1308399, Some(actor), 1);
    let wrath_hits = |run: &Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|e| {
                e.kind == CombatLogKind::Damage
                    && e.spell_id == Some(1308093)
                    && e.target == Some(target.to_bits())
            })
            .count()
    };
    let hits = |run: &Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|e| {
                e.kind == CombatLogKind::Damage
                    && e.spell_id == Some(77758)
                    && e.source == Some(actor.to_bits())
                    && e.target == Some(target.to_bits())
            })
            .count()
    };
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 77758, Some(target), false)
        .unwrap();
    assert_eq!(hits(&run), 1);
    auras::remove_auras(run.app.world_mut(), actor, RemoveReason::Consumed, |a| {
        a.instance_id == instance
    });
    for _ in 0..8 {
        run.app.update();
    }
    assert_eq!(hits(&run), 1);
    for _ in 0..4 {
        run.app.update();
    }
    assert_eq!(hits(&run), 2);
    for _ in 0..6 {
        run.app.update();
    }
    assert_eq!(
        wrath_hits(&run),
        2,
        "both the original and repeated Thrash trigger Spirit's Wrath"
    );
}
