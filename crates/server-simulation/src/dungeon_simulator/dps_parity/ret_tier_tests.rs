//! Native2062 and child spell data; relationship policy is pinned SimC76f8118.
use super::*;
use crate::spell_cast::{CastFailure, test_harness::ObservedCombatEvents};
use shared::protocol::CombatLogKind;

const DIVINE_PURPOSE: u32 = 223819;
const DIVINE_POWER: u32 = 1305230;
const VERDICT_ARBITER: u32 = 1306161;
const STORM_ARBITER: u32 = 1306162;
const DIVINE_ARBITER: u32 = 1306923;
const FINAL_VERDICT: u32 = 383328;
const DIVINE_STORM: u32 = 53385;
const SET_PIECES: &[u32] = &[271465, 271463, 271468, 271466, 271464];

pub(super) fn native_ret(piece_count: usize) -> Run {
    let (mut profile, traits) = read_reference(70).unwrap();
    profile.equipment.retain(|item| {
        SET_PIECES
            .iter()
            .position(|&id| id == item.item_id)
            .is_none_or(|index| index < piece_count)
    });
    let mut run = prepare_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    engage(&mut run);
    literal_rotation::capture_fixture(
        run.app.world_mut(),
        run.actor,
        crate::simc_apl_native::Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        },
    )
    .unwrap();
    assert_eq!(has(&run, 1296660), piece_count >= 2);
    assert_eq!(has(&run, 1296661), piece_count >= 4);
    run
}

pub(super) fn has(run: &Run, spell: u32) -> bool {
    crate::auras::has_aura(run.app.world(), run.actor, spell)
}

pub(super) fn grant_native_purpose(run: &mut Run) {
    // Isolate the consumption relationship, not the independent random producer.
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        DIVINE_PURPOSE,
        Some(run.actor),
        0,
    );
    assert!(has(run, DIVINE_PURPOSE));
}

pub(super) fn cast_finisher(run: &mut Run, spell: u32) {
    for _ in 0..200 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(run.targets[0]),
            false,
        ) {
            Ok(()) => {
                for _ in 0..4 {
                    run.app.update();
                }
                return;
            }
            Err(
                CastFailure::OnGlobalCooldown
                | CastFailure::OnCooldown
                | CastFailure::NoCharges
                | CastFailure::InProgress,
            ) => run.app.update(),
            Err(reason) => panic!("native Ret finisher{spell} failed: {reason:?}"),
        }
    }
    panic!("native Ret finisher{spell} did not become ready");
}

fn child_damage(run: &Run) -> i64 {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(DIVINE_ARBITER)
                && event.kind == CombatLogKind::Damage
        })
        .map(|event| i64::from(event.amount))
        .sum()
}

#[test]
#[ignore = "native Ret2062 tier relationship policy: two-piece consumes real Divine Purpose"]
fn dps_parity_ret_tier_two_piece_consumption_grants_native_divine_power_only() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(2);
    grant_native_purpose(&mut run);
    cast_finisher(&mut run, FINAL_VERDICT);
    assert!(
        !has(&run, DIVINE_PURPOSE),
        "foreground spender consumes native Purpose"
    );
    assert!(
        has(&run, DIVINE_POWER),
        "two-piece consumption grants Divine Power1305230"
    );
    assert!(!has(&run, VERDICT_ARBITER));
    assert!(!has(&run, STORM_ARBITER));
    assert_eq!(child_damage(&run), 0);
}

#[test]
#[ignore = "native Ret2062 tier relationship policy: pending kind and actual child damage"]
fn dps_parity_ret_tier_four_piece_same_kind_preserves_cross_kind_emits_child() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    grant_native_purpose(&mut run);
    cast_finisher(&mut run, FINAL_VERDICT);
    assert!(
        has(&run, VERDICT_ARBITER),
        "Verdict consumption arms its actual native kind"
    );
    assert_eq!(child_damage(&run), 0);
    grant_native_purpose(&mut run);
    cast_finisher(&mut run, FINAL_VERDICT);
    assert!(
        has(&run, VERDICT_ARBITER),
        "same kind must preserve pending Arbiter"
    );
    assert_eq!(child_damage(&run), 0);
    grant_native_purpose(&mut run);
    cast_finisher(&mut run, DIVINE_STORM);
    // Native1306923 has Speed50. The actual follower can move while waiting
    // on its GCD; wait for the real projectile outcome, not a guessed flight.
    for _ in 0..200 {
        let outcome = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.spell_id == Some(DIVINE_ARBITER)
                    && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
            });
        if outcome {
            break;
        }
        run.app.update();
    }
    assert!(!has(&run, VERDICT_ARBITER));
    assert!(
        !has(&run, STORM_ARBITER),
        "existing pending kind prevents a new kind grant"
    );
    assert!(
        child_damage(&run) > 0,
        "cross-kind casts native child1306923 with real damage; actor={:?}, target={:?}, child events={:?}",
        run.app
            .world()
            .get::<shared::components::Position>(run.actor),
        run.app
            .world()
            .get::<shared::components::Position>(run.targets[0]),
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| event.spell_id == Some(DIVINE_ARBITER))
            .collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "native Ret2062 no-set control: actual Purpose consumption is not tier eligibility"]
fn dps_parity_ret_tier_no_set_does_not_produce_tier_children() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(0);
    grant_native_purpose(&mut run);
    cast_finisher(&mut run, FINAL_VERDICT);
    assert!(!has(&run, DIVINE_PURPOSE));
    assert!(!has(&run, DIVINE_POWER));
    assert!(!has(&run, VERDICT_ARBITER));
    assert!(!has(&run, STORM_ARBITER));
    assert_eq!(child_damage(&run), 0);
}
