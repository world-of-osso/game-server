//! Pinned76f8118's unconditional precombat Living Flame, through the real300s fixture.
//! Inspect native cast events; no controller labels, grants or invented action facts.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const LIVING_FLAME: u32 = 361469;
const FIRE_BREATH: u32 = 357208;
const FONT_FIRE_BREATH: u32 = 382266;
const ETERNITY_SURGE: u32 = 359073;
const FONT_ETERNITY_SURGE: u32 = 382411;

fn assert_living_flame_precedes_empowers(targets: usize) {
    let mut run = prepare(1467, targets).unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::followers::Follower>(run.actor)
            .is_some()
    );
    assert!(
        run.app
            .world()
            .get::<shared::components::Player>(run.actor)
            .is_none()
    );
    engage(&mut run);
    advance_combat(&mut run).unwrap();
    let relevant: Vec<_> = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits()) && event.kind == CombatLogKind::CastSuccess
        })
        .filter_map(|event| event.spell_id)
        .filter(|spell| {
            matches!(
                *spell,
                LIVING_FLAME
                    | FIRE_BREATH
                    | FONT_FIRE_BREATH
                    | ETERNITY_SURGE
                    | FONT_ETERNITY_SURGE
            )
        })
        .collect();
    assert!(
        relevant
            .iter()
            .any(|spell| matches!(*spell, FIRE_BREATH | FONT_FIRE_BREATH)),
        "native empower execution must actually occur, not just a synthetic precombat receipt: {relevant:?}"
    );
    assert_eq!(
        relevant.first(),
        Some(&LIVING_FLAME),
        "literal precombat Living Flame must complete before native empower casts on {targets} targets: {relevant:?}"
    );
}

#[test]
#[ignore = "literal Dev native RED: prepared1467/current-canonical300s fixture"]
fn literal_dev_native_single_target_casts_precombat_living_flame_before_empowers() {
    assert_living_flame_precedes_empowers(1);
}

#[test]
#[ignore = "literal Dev native RED: prepared1467/current-canonical300s add fixture"]
fn literal_dev_native_three_targets_casts_precombat_living_flame_before_empowers() {
    assert_living_flame_precedes_empowers(3);
}
