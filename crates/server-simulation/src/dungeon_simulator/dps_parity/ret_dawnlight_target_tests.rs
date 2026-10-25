//! Native pre-existing DoTs isolate selection/refresh, never budget or power.
use super::ret_dawnlight_consumer_tests::{damage, owned_dot_at, wait_for_damage};
use super::ret_dawnlight_tests::{budget, cast_and_damage, native_count};
use super::ret_purpose_tests::current_holy_power;
use super::ret_tier_tests::{cast_finisher, grant_native_purpose, native_ret_with_targets};
use super::*;
use crate::spell_cast::{CostPool, test_harness::ObservedCombatEvents};
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

const WAKE: u32 = 255937;
const VERDICT: u32 = 383328;
const STORM: u32 = 53385;
const DAWNLIGHT: u32 = 431380;
const SHIELD: u32 = 642;

fn wake_with_existing_dots(count: usize, existing: &[usize]) -> Run {
    let mut run = native_ret_with_targets(4, count);
    assert_eq!(native_count(&run), 3.0);
    assert!(budget(&run).is_none());
    // Actual native payload applications model already-ticking target state.
    // The tested counter and spend still come only from ordinary Wake/spenders.
    for &index in existing {
        crate::spell_cast::cast_triggered(
            run.app.world_mut(),
            run.actor,
            DAWNLIGHT,
            Some(run.targets[index]),
            0,
        );
        assert!(owned_dot_at(&run, run.targets[index]).is_some());
    }
    cast_and_damage(&mut run, WAKE);
    assert_eq!(current_holy_power(&run), 3.0);
    assert_eq!(budget(&run).unwrap().stacks, 3);
    run
}

fn dot_count(run: &Run) -> usize {
    run.targets
        .iter()
        .filter(|&&target| owned_dot_at(run, target).is_some())
        .count()
}

#[test]
#[ignore = "native three-target Verdict retains a single hit but spreads Dawnlight"]
fn dps_parity_dawnlight_targets_verdict_uses_available_secondary() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = wake_with_existing_dots(3, &[0]);
    cast_finisher(&mut run, VERDICT);
    wait_for_damage(&mut run, VERDICT);
    assert_eq!(current_holy_power(&run), 0.0, "real Verdict paid3HP");
    let recipients = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(VERDICT)
                && event.kind == CombatLogKind::Damage
        })
        .filter_map(|event| event.target)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        recipients,
        std::collections::HashSet::from([run.targets[0].to_bits()]),
        "Verdict remains single-target"
    );
    assert_eq!(
        dot_count(&run),
        2,
        "SimC target_list includes a nearby unhit enemy for the Dawnlight application"
    );
    assert_eq!(budget(&run).unwrap().stacks, 2);
}

fn cast_paid_storm(run: &mut Run) {
    cast_finisher(run, STORM);
    wait_for_damage(run, STORM);
    assert_eq!(
        current_holy_power(run),
        0.0,
        "actual foreground Storm paid3HP"
    );
}

#[test]
#[ignore = "one-target native Dawnlight refresh consumes exactly one use"]
fn dps_parity_dawnlight_targets_single_refreshes_owned_periodic() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = wake_with_existing_dots(1, &[0]);
    let old_expiry = owned_dot_at(&run, run.targets[0])
        .unwrap()
        .expires_at
        .unwrap();
    let counter_expiry = budget(&run).unwrap().expires_at;
    cast_finisher(&mut run, VERDICT);
    wait_for_damage(&mut run, VERDICT);
    assert_eq!(current_holy_power(&run), 0.0);
    assert_eq!(dot_count(&run), 1);
    assert!(
        owned_dot_at(&run, run.targets[0])
            .unwrap()
            .expires_at
            .unwrap()
            > old_expiry
    );
    assert_eq!(budget(&run).unwrap().stacks, 2);
    assert_eq!(
        budget(&run).unwrap().expires_at,
        counter_expiry,
        "loss does not refresh counter time"
    );
}

#[test]
#[ignore = "native three-target spender skips ticking primary and consumes once per cast"]
fn dps_parity_dawnlight_targets_aoe_uses_unticking_secondary_once() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = wake_with_existing_dots(3, &[0]);
    cast_paid_storm(&mut run);
    assert_eq!(
        dot_count(&run),
        2,
        "one actual secondary gains owned periodic"
    );
    assert_eq!(budget(&run).unwrap().stacks, 2, "not one use per area hit");
    let recipients = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| event.spell_id == Some(STORM) && event.kind == CombatLogKind::Damage)
        .filter_map(|event| event.target)
        .collect::<std::collections::HashSet<_>>();
    assert!(
        recipients.len() >= 2,
        "actual multi-target hit prerequisite"
    );
}

#[test]
#[ignore = "all actual action targets already ticking preserve earned counter"]
fn dps_parity_dawnlight_targets_all_ticking_preserve_use() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = wake_with_existing_dots(3, &[0, 1, 2]);
    cast_paid_storm(&mut run);
    assert_eq!(dot_count(&run), 3);
    assert_eq!(budget(&run).unwrap().stacks, 3);
}

#[test]
#[ignore = "native Purpose waives cost but foreground HolyPower pool still consumes a Dawnlight use"]
fn dps_parity_dawnlight_targets_real_zero_cost_pool_qualifies() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = wake_with_existing_dots(1, &[]);
    // Isolate the already-qualified modifier, not its independent random producer.
    grant_native_purpose(&mut run);
    let world = run.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(VERDICT)
        .unwrap();
    let costs = crate::spell_cast::applicable_costs(world, run.actor, spell).unwrap();
    assert!(
        costs.iter().any(
            |(pool, amount, _)| *pool == CostPool::Power(PowerType::HolyPower) && *amount == 0.0
        )
    );
    cast_finisher(&mut run, VERDICT);
    wait_for_damage(&mut run, VERDICT);
    assert_eq!(
        current_holy_power(&run),
        3.0,
        "no synthetic power or paid cost"
    );
    assert!(owned_dot_at(&run, run.targets[0]).is_some());
    assert_eq!(budget(&run).unwrap().stacks, 2);
}

#[test]
#[ignore = "actual native immunity is not a successful first hit and cannot spend a use"]
fn dps_parity_dawnlight_targets_native_immune_primary_keeps_use() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = wake_with_existing_dots(1, &[]);
    let target = run.targets[0];
    crate::spell_cast::cast_triggered(run.app.world_mut(), target, SHIELD, Some(target), 0);
    assert!(crate::auras::has_aura(run.app.world(), target, SHIELD));
    cast_finisher(&mut run, VERDICT);
    assert_eq!(damage(&run, VERDICT), 0);
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.spell_id == Some(VERDICT)
                && event.target == Some(target.to_bits())
                && matches!(event.kind, CombatLogKind::Miss(_)))
    );
    assert!(owned_dot_at(&run, target).is_none());
    assert_eq!(budget(&run).unwrap().stacks, 3);
}
