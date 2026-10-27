//! Native Focused Ray target-count differential.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};
use std::collections::BTreeSet;

fn first_ray_tick(count: usize) -> (i32, bool) {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(1480, count).expect("native Devourer profile");
    let actor = run.actor;
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .is_some_and(|known| known.contains(1240203)),
        "native profile must select Focused Ray"
    );
    let primary = run.targets[0];
    engage(&mut run);
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Fury, 140.0)
        .unwrap();
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        473728,
        Some(primary),
    )
    .expect("native Void Ray cast");
    for _ in 0..12 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let ticks: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(1213649)
                && event.amount > 0
        })
        .collect();
    let targets: BTreeSet<_> = ticks.iter().filter_map(|event| event.target).collect();
    assert_eq!(
        targets.len(),
        count,
        "native Void Ray must hit every intended target before checking the multiplier"
    );
    let first = ticks
        .iter()
        .find(|event| event.target == Some(primary.to_bits()))
        .expect("native primary tick");
    (first.amount, first.crit)
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_focused_ray_amplifies_three_targets_but_not_four() {
    let (three, crit_three) = first_ray_tick(3);
    let (four, crit_four) = first_ray_tick(4);
    assert_eq!(
        crit_three, crit_four,
        "compare matching native hit outcomes"
    );
    // SimC76f8118 sc_demon_hunter.cpp6304–6321: selected1240203 effect1
    // amplifies each tick when resolved targets <= native effect2 threshold.
    // Native70448 has +80% and threshold3; tolerate integer damage rounding.
    assert!(
        (f64::from(three) - f64::from(four) * 1.8).abs() <= 2.0,
        "Focused Ray must amplify the same primary tick at 3 targets: three={three}, four={four}"
    );
}
