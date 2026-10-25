//! The real Crash Lightning root must schedule its native Unleashed weapon strikes.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn unleashed_hits(run: &Run) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1252431)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count()
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_unleashed_real_crash_emits_two_delayed_native_pack_pulses_then_stops() {
    // SimC76f8118 sc_shaman.cpp6110–6124: two one-second pulses;1490 gives the90% occupancy roll.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 3).unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(1252373)
    );
    engage(&mut run);
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, 187874, Some(target), false)
        .unwrap();
    assert_eq!(
        unleashed_hits(&run),
        0,
        "Unleashed is not immediate root damage"
    );
    for _ in 0..21 {
        run.app.update();
    }
    let first = unleashed_hits(&run);
    assert!(
        (1..=3).contains(&first),
        "the first native one-second Unleashed pulse must hit the actual pack: {first}"
    );
    for _ in 0..20 {
        run.app.update();
    }
    let second = unleashed_hits(&run);
    assert!(
        (1..=3).contains(&(second - first)),
        "the second native pulse must execute separately: first={first}, second={second}"
    );
    for _ in 0..40 {
        run.app.update();
    }
    assert_eq!(
        unleashed_hits(&run),
        second,
        "native1252373 effect1 limits the real pulses to two"
    );
}
