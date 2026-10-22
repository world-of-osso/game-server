//! Native Echo producer proof. Never manufacture Henry's mastery amount.
use super::{GroupFixture, apply_pressure, cast_binding_source, native_group, native_group_at};
use crate::auras::{ActiveAuras, AuraInstance, aura_now};
use crate::followers::kit_clearcasting_tests::{frame_seconds, own_aura, wait_for_gcd};
use crate::spell_cast::creature_cast;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use bevy::prelude::*;
use shared::protocol::{CombatLogEvent, CombatLogKind};

const ECHO_MASTERY: u32 = 77_485;
const ECHO_HEAL: u32 = 77_489;
const FLASH_HEAL: u32 = 2_061;

fn native_mastery_percent(group: &GroupFixture) -> f32 {
    let aura = own_aura(&group.h, group.priest, ECHO_MASTERY).expect("native learned Echo mastery");
    let amount = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert!(
        amount > 0.0,
        "follower mastery bridge prerequisite, not an Echo producer RED"
    );
    amount
}

fn owned_echo(group: &GroupFixture, patient: Entity) -> Option<AuraInstance> {
    group
        .h
        .app
        .world()
        .get::<ActiveAuras>(patient)?
        .iter()
        .find(|aura| aura.spell_id == ECHO_HEAL && aura.caster == group.priest)
        .cloned()
}

/// Stop after the real direct hit and event processing, before Echo's first tick.
fn complete_flash_hit(group: &mut GroupFixture, patient: Entity) -> CombatLogEvent {
    wait_for_gcd(&mut group.h, group.priest, FLASH_HEAL);
    let start = group
        .h
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .len();
    creature_cast(
        group.h.app.world_mut(),
        group.priest,
        FLASH_HEAL,
        Some(patient),
        false,
    )
    .unwrap();
    for _ in 0..200 {
        group.h.advance(1);
        let heal = group.h.app.world().resource::<ObservedCombatEvents>().0[start..]
            .iter()
            .find(|event| {
                event.kind == CombatLogKind::Heal
                    && event.source == Some(group.priest.to_bits())
                    && event.target == Some(patient.to_bits())
                    && event.spell_id == Some(FLASH_HEAL)
            })
            .cloned();
        if let Some(heal) = heal {
            group.h.advance(2);
            return heal;
        }
    }
    panic!("ordinary native Flash Heal did not land");
}

fn finish_echo(group: &mut GroupFixture, echo: &AuraInstance) {
    let remaining =
        echo.expires_at.expect("native finite Echo duration") - aura_now(group.h.app.world());
    let frames = (remaining.max(0.0) / frame_seconds(&group.h)).ceil() as usize + 2;
    group.h.advance(frames);
}

fn echo_ticks(group: &GroupFixture, patient: Entity, start: usize) -> Vec<CombatLogEvent> {
    group.h.app.world().resource::<ObservedCombatEvents>().0[start..]
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Heal
                && event.source == Some(group.priest.to_bits())
                && event.target == Some(patient.to_bits())
                && event.spell_id == Some(ECHO_HEAL)
        })
        .cloned()
        .collect()
}

#[test]
fn holy_echo_of_light_native90_direct_overheal_produces_two_native_ticks_and_expires() {
    let mut group = native_group();
    let percent = native_mastery_percent(&group);
    apply_pressure(&mut group, 1);
    let patient = group.h.caster;
    let start = group
        .h
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .len();
    let direct = complete_flash_hit(&mut group, patient);
    assert!(
        direct.amount > 0 && direct.overflow > 0,
        "real pre-overheal boundary: {direct:?}"
    );
    let echo =
        owned_echo(&group, patient).expect("actual native Flash Heal must produce owned77489");
    assert_eq!(echo.duration_ms, 4_000);
    let effect = echo
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap();
    assert_eq!(effect.period_ms, 2_000);
    let count = echo.duration_ms / effect.period_ms;
    let expected = (direct.amount as f32 * percent / 100.0 / count as f32).round() as i32;
    finish_echo(&mut group, &echo);
    let ticks = echo_ticks(&group, patient, start);
    assert_eq!(
        ticks.len(),
        count as usize,
        "no recursive Echo producer: {ticks:?}"
    );
    for tick in ticks {
        assert!(tick.periodic);
        assert!(
            (tick.amount - expected).abs() <= 1,
            "native amount, not effective direct heal: expected{expected}, {tick:?}"
        );
    }
    assert!(owned_echo(&group, patient).is_none());
}

#[test]
fn holy_echo_of_light_native90_self_flash_is_eligible_unlike_binding_heals() {
    let mut group = native_group();
    native_mastery_percent(&group);
    apply_pressure(&mut group, 6);
    let priest = group.priest;
    let start = group
        .h
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .len();
    let direct = complete_flash_hit(&mut group, priest);
    assert!(direct.amount > 0);
    let echo = owned_echo(&group, priest).expect("direct self healing also produces native Echo");
    finish_echo(&mut group, &echo);
    let ticks = echo_ticks(&group, priest, start);
    assert_eq!(ticks.len(), 2);
    assert!(
        ticks.iter().any(|tick| tick.amount > tick.overflow),
        "Echo must actually heal the wounded self patient: {ticks:?}"
    );
    assert!(owned_echo(&group, group.h.caster).is_none());
}

#[test]
fn holy_echo_of_light_native90_renew_ticks_do_not_produce_echo() {
    let mut group = native_group();
    native_mastery_percent(&group);
    apply_pressure(&mut group, 2);
    let patient = group.h.caster;
    let start = group
        .h
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .len();
    cast_binding_source(&mut group, 139, patient);
    let events = &group.h.app.world().resource::<ObservedCombatEvents>().0[start..];
    assert!(events.iter().any(|event| event.kind == CombatLogKind::Heal
        && event.periodic
        && event.spell_id == Some(139)));
    assert!(echo_ticks(&group, patient, start).is_empty());
    assert!(owned_echo(&group, patient).is_none());
}

#[test]
fn holy_echo_of_light_native9_unlearned_mastery_does_not_produce_echo() {
    let mut group = native_group_at(9);
    assert!(own_aura(&group.h, group.priest, ECHO_MASTERY).is_none());
    apply_pressure(&mut group, 1);
    let patient = group.h.caster;
    let start = group
        .h
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .len();
    let direct = complete_flash_hit(&mut group, patient);
    assert!(direct.amount > 0);
    assert!(echo_ticks(&group, patient, start).is_empty());
    assert!(owned_echo(&group, patient).is_none());
}

#[test]
fn holy_echo_of_light_native90_two_native_priests_keep_separate_patient_pools() {
    let mut group = native_group();
    let first_priest = group.priest;
    let patient = group.h.caster;
    // A second real Henry is a friendly caster, not a claim of six-member
    // follower-party admission. No stats, aura amounts or spells are granted.
    let second_priest =
        crate::followers::native_fixture::spawn(group.h.app.world_mut(), patient, super::HENRY)
            .expect("second native Henry fixture");
    group.h.advance(3);
    native_mastery_percent(&group);
    apply_pressure(&mut group, 2);
    let start = group
        .h
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .len();
    complete_flash_hit(&mut group, patient);
    owned_echo(&group, patient).expect("first real priest owns an Echo");
    group.priest = second_priest;
    let second_percent = native_mastery_percent(&group);
    let second_direct = complete_flash_hit(&mut group, patient);
    let second_echo = owned_echo(&group, patient).expect("second priest owns a distinct Echo");
    group.priest = first_priest;
    complete_flash_hit(&mut group, patient);
    let first_refresh = owned_echo(&group, patient).expect("first priest refresh remains owned");
    group.priest = second_priest;
    let second_after = owned_echo(&group, patient).expect("other caster's aura survives refresh");
    assert_eq!(second_after.expires_at, second_echo.expires_at);
    assert!(first_refresh.expires_at.unwrap() >= second_echo.expires_at.unwrap());
    finish_echo(&mut group, &first_refresh);
    let ticks = echo_ticks(&group, patient, start);
    let period = second_echo
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .period_ms;
    let count = second_echo.duration_ms / period;
    let expected =
        (second_direct.amount as f32 * second_percent / 100.0 / count as f32).round() as i32;
    assert_eq!(ticks.len(), count as usize);
    for tick in ticks {
        assert!(
            (tick.amount - expected).abs() <= 1,
            "first priest's refresh must not borrow or replace the second pool: {tick:?}"
        );
    }
    assert!(owned_echo(&group, patient).is_none());
}

#[test]
fn holy_echo_of_light_native90_reheal_preserves_unpaid_pool_and_tick_phase() {
    let mut group = native_group();
    let percent = native_mastery_percent(&group);
    apply_pressure(&mut group, 2);
    let patient = group.h.caster;
    let first = complete_flash_hit(&mut group, patient);
    let initial = owned_echo(&group, patient).expect("qualified Echo producer prerequisite");
    let start = group
        .h
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .len();
    for _ in 0..200 {
        if !echo_ticks(&group, patient, start).is_empty() {
            break;
        }
        group.h.advance(1);
    }
    let first_ticks = echo_ticks(&group, patient, start);
    assert_eq!(
        first_ticks.len(),
        1,
        "exactly one original native tick must be paid before refresh"
    );
    let second = complete_flash_hit(&mut group, patient);
    let refreshed = owned_echo(&group, patient).expect("second native direct heal refreshes Echo");
    assert!(refreshed.expires_at.unwrap() > initial.expires_at.unwrap());
    assert!(
        aura_now(group.h.app.world()) < initial.expires_at.unwrap(),
        "second native hit must precede the original final tick"
    );
    let remaining_pool = first.amount as f32 * percent / 100.0 - first_ticks[0].amount as f32;
    let new_pool = second.amount as f32 * percent / 100.0;
    let after_refresh = group
        .h
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .len();
    let original_tick_deadline = initial.expires_at.unwrap() + frame_seconds(&group.h);
    while aura_now(group.h.app.world()) <= original_tick_deadline {
        group.h.advance(1);
    }
    assert_eq!(
        echo_ticks(&group, patient, after_refresh).len(),
        1,
        "the next heal lands on the original tick phase, not two seconds after refresh"
    );
    finish_echo(&mut group, &refreshed);
    let remaining_ticks = echo_ticks(&group, patient, after_refresh);
    assert_eq!(
        remaining_ticks.len(),
        2,
        "refresh retains the native two-tick future window"
    );
    let paid: i32 = remaining_ticks.iter().map(|event| event.amount).sum();
    assert!(
        (paid as f32 - remaining_pool - new_pool).abs() <= 3.0,
        "unpaid first pool must survive native refresh: paid{paid}, prior{remaining_pool}, fresh{new_pool}"
    );
    assert!(owned_echo(&group, patient).is_none());
}
