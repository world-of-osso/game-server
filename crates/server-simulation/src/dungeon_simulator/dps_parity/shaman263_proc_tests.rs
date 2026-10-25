//! Real earned Tempest charges and native linked Strike payloads, independent of APL selection.
use super::enhancement_generation_tests::{cast, earn_tempest_by_spending};
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn tempest_procs(run: &Run) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 454015 && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks)
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn shaman263_tempest_cast_spends_one_of_two_actually_earned_procs() {
    // SimC76f8118 sc_shaman.cpp9966 decrements one, before the spender can earn its next proc.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    earn_tempest_by_spending(&mut run);
    assert_eq!(tempest_procs(&run), 1);
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .remove::<crate::combat::AutoAttack>();
    let actor = run.actor;
    cast(&mut run, 114051, actor);
    assert_eq!(
        tempest_procs(&run),
        2,
        "actual Ascendance earns the second proc through selected Descending Skies"
    );
    let target = run.targets[0];
    cast(&mut run, 452201, target);
    assert_eq!(
        tempest_procs(&run),
        1,
        "real Tempest must not discard the other earned proc"
    );
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn shaman263_real_strike_dispatches_ride_the_lightning_to_native_chain_targets() {
    // SimC76f8118 sc_shaman.cpp5647/10757–10766/12532–12551 uses native211094, not another foreground CL.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 3).unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(289874)
    );
    engage(&mut run);
    let target = run.targets[0];
    cast(&mut run, 17364, target);
    for _ in 0..60 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    for &target in &run.targets {
        let hits = events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(211094)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count();
        assert_eq!(
            hits, 1,
            "one real Strike emits one native Ride the Lightning bounce per actual target"
        );
    }
}
