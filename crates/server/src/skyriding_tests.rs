//! Skyriding abilities on a Golden Gryphon skyrider against the real spell DB2 tables in
//! data/world.db: Skyriding Charges (ChargeCategory 2391), cooldowns, the impulse the
//! movement bound takes, and Second Wind.

use shared::components::PlayerMotion;
use shared::skyriding::{AERIAL_HALT, LAUNCH_SPEED, SKYWARD_ASCENT, SURGE_FORWARD, SURGE_SPEED};
use shared::spell_data::CastFailReason;

use super::{SECOND_WIND, SKYRIDING_CHARGES, SkyridingImpulse};
use crate::auras::mount::{FlightStyle, SKYRIDING_BASICS, set_flight_style};
use crate::auras::mount_tests::{mount, rider};
use crate::profession::PlayerProfessions;
use crate::spell_cast::test_harness::Harness;

/// The Golden Gryphon (`auras::mount_tests`).
const GOLDEN_GRYPHON: u32 = 32235;
/// 50 ms frames: 1.55 s, past the 1.5 s global cooldown of Surge Forward and Skyward Ascent
/// (SpellCooldowns StartRecoveryTime 1500, StartRecoveryCategory 133).
const PAST_GCD: usize = 31;
/// SpellCategory 2391 ChargeRecoveryTime 15000 × Skyriding 406095 effect 8
/// CHARGE_RECOVERY_MULTIPLIER -31%.
const RECHARGE_MS: u32 = 10_350;

/// Mounted on Flight Style: Skyriding with Skyriding Basics (which teaches Surge Forward
/// and Skyward Ascent, LEARN_SPELL 195), Aerial Halt and Second Wind learned.
fn skyrider() -> Option<Harness> {
    let mut h = rider(
        225,
        false,
        &[
            SKYRIDING_BASICS,
            SURGE_FORWARD,
            SKYWARD_ASCENT,
            AERIAL_HALT,
            SECOND_WIND,
        ],
    )?;
    h.app
        .world_mut()
        .get_mut::<PlayerProfessions>(h.caster)
        .unwrap()
        .spells
        .extend([SKYRIDING_BASICS, AERIAL_HALT, SECOND_WIND]);
    // Only what the rider learned counts as known: the Golden Gryphon, not the harness's
    // other fixture spells.
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            GOLDEN_GRYPHON,
        ]));
    set_flight_style(h.app.world_mut(), h.caster, FlightStyle::Skyriding);
    mount(&mut h);
    let motion = *h.app.world().get::<PlayerMotion>(h.caster).unwrap();
    assert!(motion.contains(PlayerMotion::CAN_ADV_FLY), "{motion:?}");
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    Some(h)
}

fn impulse(h: &Harness) -> Option<f32> {
    h.app.world().get::<SkyridingImpulse>(h.caster).map(|i| i.0)
}

fn take_impulse(h: &mut Harness) -> Option<f32> {
    let taken = impulse(h);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .remove::<SkyridingImpulse>();
    taken
}

fn vigor(h: &Harness) -> Option<(u32, u8, u8, u32)> {
    h.inbox
        .charges
        .last()
        .map(|c| (c.spell_id, c.current, c.max, c.recharge_ms))
}

fn failures(h: &Harness) -> Vec<CastFailReason> {
    h.inbox.failed.iter().map(|f| f.reason).collect()
}

#[test]
fn surge_forward_spends_a_skyriding_charge_and_flaps_forward() {
    let Some(mut h) = skyrider() else {
        return;
    };
    h.cast_and_deliver(SURGE_FORWARD, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(vigor(&h), Some((SURGE_FORWARD, 5, 6, RECHARGE_MS)));
    assert_eq!(impulse(&h), Some(SURGE_SPEED));
}

#[test]
fn skyward_ascent_shares_the_charges_and_flaps_upward() {
    let Some(mut h) = skyrider() else {
        return;
    };
    h.cast_and_deliver(SURGE_FORWARD, None);
    take_impulse(&mut h);
    h.advance(PAST_GCD);
    h.cast_and_deliver(SKYWARD_ASCENT, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(vigor(&h), Some((SKYWARD_ASCENT, 4, 6, RECHARGE_MS)));
    assert_eq!(impulse(&h), Some(LAUNCH_SPEED));
}

#[test]
fn a_second_flap_waits_for_the_global_cooldown() {
    let Some(mut h) = skyrider() else {
        return;
    };
    h.cast_and_deliver(SURGE_FORWARD, None);
    take_impulse(&mut h);
    h.cast_and_deliver(SURGE_FORWARD, None);
    assert_eq!(failures(&h), [CastFailReason::OnGlobalCooldown]);
    assert_eq!(impulse(&h), None);
}

/// Six charges, then nothing until one recovers 10.35 s after the first flap.
#[test]
fn without_skyriding_charges_nothing_flaps_until_one_recovers() {
    let Some(mut h) = skyrider() else {
        return;
    };
    for flap in 0..6 {
        h.cast_and_deliver(SURGE_FORWARD, None);
        assert_eq!(take_impulse(&mut h), Some(SURGE_SPEED), "flap {flap}");
        h.advance(PAST_GCD - 3);
    }
    assert_eq!(vigor(&h), Some((SURGE_FORWARD, 0, 6, RECHARGE_MS)));
    // 6 × 1.55 s = 9.3 s after the first flap.
    h.cast_and_deliver(SURGE_FORWARD, None);
    assert_eq!(failures(&h), [CastFailReason::NoChargesRemain]);
    assert_eq!(impulse(&h), None);

    // 10.35 s after the first flap (frame 3) one charge is back.
    h.advance(20);
    h.cast_and_deliver(SURGE_FORWARD, None);
    assert_eq!(failures(&h).len(), 1, "{:?}", h.inbox.failed);
    assert_eq!(take_impulse(&mut h), Some(SURGE_SPEED));
    assert_eq!(vigor(&h).map(|v| v.1), Some(0));
}

#[test]
fn skyriding_charges_recover_one_per_10_35_seconds() {
    let Some(mut h) = skyrider() else {
        return;
    };
    h.cast_and_deliver(SURGE_FORWARD, None);
    h.advance(PAST_GCD);
    h.cast_and_deliver(SURGE_FORWARD, None);
    assert_eq!(vigor(&h).map(|v| v.1), Some(4));
    // 21 s after the first flap both are back: the next flap leaves 5.
    h.advance(21 * 20 - PAST_GCD - 3);
    h.cast_and_deliver(SURGE_FORWARD, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(vigor(&h).map(|v| v.1), Some(5));
}

#[test]
fn aerial_halt_costs_no_charge_and_cools_down_for_10_seconds() {
    let Some(mut h) = skyrider() else {
        return;
    };
    h.cast_and_deliver(AERIAL_HALT, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.charges.is_empty(), "{:?}", h.inbox.charges);
    assert_eq!(impulse(&h), None);
    let cooldown = h.inbox.cooldowns.last().expect("Aerial Halt cooldown");
    assert_eq!(
        (cooldown.spell_id, cooldown.duration_ms),
        (AERIAL_HALT, 10_000)
    );
    h.advance(PAST_GCD);
    h.cast_and_deliver(AERIAL_HALT, None);
    assert_eq!(failures(&h), [CastFailReason::OnCooldown]);
}

#[test]
fn second_wind_restores_a_skyriding_charge() {
    let Some(mut h) = skyrider() else {
        return;
    };
    h.cast_and_deliver(SURGE_FORWARD, None);
    h.advance(PAST_GCD);
    h.cast_and_deliver(SURGE_FORWARD, None);
    assert_eq!(vigor(&h).map(|v| v.1), Some(4));
    h.advance(PAST_GCD);
    h.cast_and_deliver(SECOND_WIND, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let restored = h
        .inbox
        .charges
        .iter()
        .rev()
        .find(|c| c.spell_id == SURGE_FORWARD)
        .expect("Skyriding Charges update");
    assert_eq!((restored.current, restored.max), (5, 6));
    assert_eq!(
        crate::spell_cast::charges_left(h.app.world(), h.caster, SKYRIDING_CHARGES),
        Some(5)
    );
}

#[test]
fn surge_forward_needs_skyriding_basics() {
    let Some(mut h) = skyrider() else {
        return;
    };
    h.app
        .world_mut()
        .get_mut::<PlayerProfessions>(h.caster)
        .unwrap()
        .spells
        .retain(|&spell| spell != SKYRIDING_BASICS);
    h.cast_and_deliver(SURGE_FORWARD, None);
    assert_eq!(failures(&h), [CastFailReason::NotSupported]);
    assert_eq!(impulse(&h), None);
}
