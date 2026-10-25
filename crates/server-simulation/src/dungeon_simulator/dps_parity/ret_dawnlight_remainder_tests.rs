//! SimC76f8118 sc_paladin.hpp1492–1496/1581–1594 and cpp3048–3053.
use super::ret_dawnlight_consumer_tests::{owned_dot_at, wait_for_damage};
use super::ret_dawnlight_tests::{budget, cast_and_damage};
use super::ret_purpose_tests::earn_finisher_power;
use super::ret_tier_tests::{cast_finisher, native_ret, native_ret_with_targets};
use super::*;
use crate::spell_cast::{CastTargets, test_harness::ObservedCombatEvents};
use shared::protocol::CombatLogKind;

fn wait_native_gcd(run: &mut Run) {
    for _ in 0..200 {
        let world = run.app.world();
        let now = crate::spell_cast::history_now(world);
        if !world
            .get::<crate::spell_cast::SpellHistory>(run.actor)
            .is_some_and(|history| history.has_active_gcd(now))
        {
            return;
        }
        run.app.update();
    }
    panic!("native GCD never became ready");
}

fn native_dot(run: &mut Run, target: Entity) {
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 431380, Some(target), 0);
    assert!(owned_dot_at(run, target).is_some());
}

#[test]
#[ignore = "real paid Verdict extends existing Dawnlight by native Endless Gleam300ms"]
fn dps_parity_ret_dawnlight_remainder_impact_extends_without_reapplication() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    cast_and_damage(&mut run, 20271);
    earn_finisher_power(&mut run, 383328);
    wait_native_gcd(&mut run);
    let target = run.targets[0];
    native_dot(&mut run, target);
    assert!(budget(&run).is_none(), "no Wake budget can reapply the DoT");
    let before = owned_dot_at(&run, target).unwrap().expires_at.unwrap();
    cast_finisher(&mut run, 383328);
    wait_for_damage(&mut run, 383328);
    let after = owned_dot_at(&run, target).unwrap().expires_at.unwrap();
    assert!(
        (after - before - 0.3).abs() < 0.001,
        "native300ms impact extension: before={before}, after={after}"
    );
}

#[test]
#[ignore = "one actual three-target Storm extends ticking Dawnlights by impact300ms plus area500ms"]
fn dps_parity_ret_dawnlight_remainder_storm_extends_each_owned_dot() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret_with_targets(4, 3);
    cast_and_damage(&mut run, 255937);
    wait_native_gcd(&mut run);
    for target in run.targets.clone() {
        native_dot(&mut run, target);
    }
    assert_eq!(budget(&run).unwrap().stacks, 3);
    let before = run
        .targets
        .iter()
        .map(|&target| owned_dot_at(&run, target).unwrap().expires_at.unwrap())
        .collect::<Vec<_>>();
    // Observe the instant root before queued Sunrise/Tempest actions run.
    crate::spell_cast::creature_cast_with_targets(
        run.app.world_mut(),
        run.actor,
        53385,
        CastTargets {
            unit: run.actor,
            primary: Some(run.targets[0]),
            destination: None,
        },
    )
    .unwrap();
    let recipients = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(53385)
                && event.kind == CombatLogKind::Damage
        })
        .filter_map(|event| event.target)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        recipients.len(),
        3,
        "the actual root must hit all three native enemies"
    );
    assert_eq!(
        budget(&run).unwrap().stacks,
        3,
        "all ticking: no new Dawnlight application"
    );
    for (&target, old) in run.targets.iter().zip(before) {
        let after = owned_dot_at(&run, target).unwrap().expires_at.unwrap();
        assert!(
            (after - old - 0.8).abs() < 0.001,
            "native300+500ms extension on{target:?}: before={old}, after={after}"
        );
    }
}

fn splash_damage(run: &Run, target: Entity) -> i64 {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(431399)
                && event.kind == CombatLogKind::Damage
        })
        .map(|event| i64::from(event.amount))
        .sum()
}

#[test]
#[ignore = "real Dawnlight tick radiates native431399 to secondary enemies but not its own primary"]
fn dps_parity_ret_dawnlight_remainder_tick_radiates_only_to_secondaries() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret_with_targets(4, 3);
    let primary = run.targets[0];
    native_dot(&mut run, primary);
    wait_for_damage(&mut run, 431380);
    for _ in 0..100 {
        if run.targets[1..]
            .iter()
            .all(|&target| splash_damage(&run, target) > 0)
        {
            break;
        }
        run.app.update();
    }
    assert!(
        run.targets[1..]
            .iter()
            .all(|&target| splash_damage(&run, target) > 0),
        "a real native periodic tick must radiate damage to both nearby enemies"
    );
    assert_eq!(
        splash_damage(&run, primary),
        0,
        "the primary must not receive its own splash"
    );
}
