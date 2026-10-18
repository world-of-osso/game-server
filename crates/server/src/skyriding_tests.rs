//! Skyriding abilities on a Golden Gryphon skyrider against the real spell DB2 tables in
//! data/world.db: Skyriding Charges (ChargeCategory 2391), cooldowns, the impulse the
//! movement bound takes, and Second Wind.

use shared::components::PlayerMotion;
use shared::protocol::ActionRef;
use shared::skyriding::{
    AERIAL_HALT, LAUNCH_SPEED, SKYWARD_ASCENT, SURGE_FORWARD, SURGE_SPEED, WHIRLING_SURGE,
    WHIRLING_SURGE_SPEED,
};
use shared::spell_data::CastFailReason;

use super::SkyridingImpulse;
use crate::auras::mount::{FlightStyle, SKYRIDING_BASICS, set_flight_style};
use crate::auras::mount_tests::{mount, rider};
use crate::profession::PlayerProfessions;
use crate::spell_cast::test_harness::Harness;

/// Second Wind: MODIFY_CHARGES 293, one charge of ChargeCategory 2391.
const SECOND_WIND: u32 = 425_782;
/// Lift Off: APPLY_AURA DUMMY on the caster, 2 s (SpellDuration 39), which Skyward Ascent's
/// effect 1 FORCE_CAST 140 casts.
const LIFT_OFF: u32 = 404_191;
/// Skyriding 406095 effect 1 LINKED 285 applies this native action prerequisite.
const DRAGONRIDER_ENERGY: u32 = 372_773;
/// Standing Lift Off: DUMMY 450 and FORCE_CAST of the two-second Lift Off aura.
const STANDING_LIFT_OFF: u32 = 374_763;
/// Tutorial highlights removed by Surge Forward / Skyward Ascent and Whirling Surge.
const TUTORIAL_HIGHLIGHTS: [u32; 3] = [1_259_214, 1_259_215, 1_259_208];
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
            DRAGONRIDER_ENERGY,
            SURGE_FORWARD,
            SKYWARD_ASCENT,
            AERIAL_HALT,
            WHIRLING_SURGE,
            SECOND_WIND,
            LIFT_OFF,
            STANDING_LIFT_OFF,
            TUTORIAL_HIGHLIGHTS[0],
            TUTORIAL_HIGHLIGHTS[1],
            TUTORIAL_HIGHLIGHTS[2],
        ],
    )?;
    h.app
        .world_mut()
        .get_mut::<PlayerProfessions>(h.caster)
        .unwrap()
        .spells
        .extend([SKYRIDING_BASICS, AERIAL_HALT, WHIRLING_SURGE, SECOND_WIND]);
    // Only what the rider learned counts as known: the Golden Gryphon, not the harness's
    // other fixture spells.
    h.app.world_mut().entity_mut(h.caster).insert((
        crate::class_progression::KnownSpells::fixture(vec![GOLDEN_GRYPHON]),
        crate::class_progression::ActionBar::default(),
    ));
    set_flight_style(h.app.world_mut(), h.caster, FlightStyle::Skyriding);
    mount(&mut h);
    let motion = *h.app.world().get::<PlayerMotion>(h.caster).unwrap();
    assert!(motion.contains(PlayerMotion::CAN_ADV_FLY), "{motion:?}");
    assert!(
        crate::auras::has_aura(h.app.world(), h.caster, DRAGONRIDER_ENERGY),
        "Mount capability's native linked aura supplies the skyriding action prerequisite"
    );
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

/// Skyriding Charges' `ChargeCategory`.
const SKYRIDING_CHARGES: u32 = 2391;

fn failures(h: &Harness) -> Vec<CastFailReason> {
    h.inbox.failed.iter().map(|f| f.reason).collect()
}

/// TC a352b1fa SpellEffects.cpp:295,5154-5163: REMOVE_AURA_2 removes every
/// instance of TriggerSpell from the effect's target, not other spells or units.
#[test]
fn skyriding_remove_aura_2_removes_only_its_targeted_tutorial_highlights() {
    for spell_id in [SURGE_FORWARD, SKYWARD_ASCENT, WHIRLING_SURGE] {
        let mut h = skyrider().expect("real spell and mount data");
        for unit in [h.caster, h.npc] {
            for spell_id in TUTORIAL_HIGHLIGHTS.into_iter().chain([LIFT_OFF]) {
                crate::auras::apply_aura(
                    h.app.world_mut(),
                    &crate::auras::AuraApplication {
                        caster: h.caster,
                        target: unit,
                        spell_id,
                        effect_mask: 1,
                        source: crate::auras::AuraSource::Cast,
                    },
                );
                assert!(crate::auras::has_aura(h.app.world(), unit, spell_id));
            }
        }
        h.cast_and_deliver(spell_id, None);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        for highlight in TUTORIAL_HIGHLIGHTS {
            let removed = if spell_id == WHIRLING_SURGE {
                highlight == TUTORIAL_HIGHLIGHTS[2]
            } else {
                highlight != TUTORIAL_HIGHLIGHTS[2]
            };
            assert_eq!(
                crate::auras::has_aura(h.app.world(), h.caster, highlight),
                !removed,
                "spell {spell_id}, highlight {highlight}"
            );
            assert!(crate::auras::has_aura(h.app.world(), h.npc, highlight));
        }
        assert!(crate::auras::has_aura(h.app.world(), h.caster, LIFT_OFF));
        let warnings = h
            .app
            .world()
            .resource::<crate::spell_cast::effects::UnhandledEffects>();
        assert!(!warnings.0.contains(&(spell_id, 203)));
    }
}

/// The actual data has no LEAP/JUMP effect: FORCE_CAST applies the 404191 aura.
#[test]
fn skyriding_standing_lift_off_applies_its_force_cast_aura() {
    let mut h = skyrider().expect("real spell and mount data");
    h.app
        .world_mut()
        .get_mut::<PlayerProfessions>(h.caster)
        .unwrap()
        .spells
        .push(STANDING_LIFT_OFF);
    h.cast_and_deliver(STANDING_LIFT_OFF, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, LIFT_OFF));
    // 404191's SpellDuration 39 is two seconds; the aura expires normally.
    h.advance(41);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, LIFT_OFF));
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
    let categories: Vec<u32> = h.inbox.charges.iter().map(|c| c.category).collect();
    assert_eq!(categories, [SKYRIDING_CHARGES, SKYRIDING_CHARGES]);
    assert_eq!(impulse(&h), Some(LAUNCH_SPEED));
}

/// The takeoff: Skyward Ascent cast on the ground spends a Skyriding Charge, flaps upward
/// (the launch the server's movement bound takes off with) and its effect 1 FORCE_CAST
/// makes the rider cast Lift Off 404191 on itself.
#[test]
fn a_ground_takeoff_casts_skyward_ascent_and_lift_off() {
    let Some(mut h) = skyrider() else {
        return;
    };
    h.cast_and_deliver(SKYWARD_ASCENT, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(vigor(&h), Some((SKYWARD_ASCENT, 5, 6, RECHARGE_MS)));
    assert_eq!(impulse(&h), Some(LAUNCH_SPEED));
    assert!(crate::auras::has_aura(h.app.world(), h.caster, LIFT_OFF));
}

/// Gaining the Skyriding aura's ADV_FLYING puts the abilities the rider knows on bonus bar 5
/// (slots 120..=131, the page the main bar shows while skyriding), in Retail order, and
/// sends the bar.
#[test]
fn a_skyriding_mount_fills_bonus_bar_5_with_the_known_abilities() {
    let Some(h) = skyrider() else {
        return;
    };
    let expected = [
        (120, SURGE_FORWARD),
        (121, SKYWARD_ASCENT),
        (122, AERIAL_HALT),
        (123, WHIRLING_SURGE),
        (124, SECOND_WIND),
    ]
    .map(|(slot, spell)| (slot, ActionRef::Spell(spell)));
    let sent = h.inbox.bars.last().expect("ActionBarSnapshot");
    assert_eq!(sent.slots, expected);
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

#[test]
fn whirling_surge_costs_no_charge_surges_forward_and_cools_down_for_30_seconds() {
    let Some(mut h) = skyrider() else {
        return;
    };
    h.cast_and_deliver(WHIRLING_SURGE, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.charges.is_empty(), "{:?}", h.inbox.charges);
    assert_eq!(take_impulse(&mut h), Some(WHIRLING_SURGE_SPEED));
    let cooldown = h.inbox.cooldowns.last().expect("Whirling Surge cooldown");
    assert_eq!(
        (cooldown.spell_id, cooldown.duration_ms),
        (WHIRLING_SURGE, 30_000)
    );
    h.advance(PAST_GCD);
    h.cast_and_deliver(WHIRLING_SURGE, None);
    assert_eq!(failures(&h), [CastFailReason::OnCooldown]);
    assert_eq!(impulse(&h), None);
}
