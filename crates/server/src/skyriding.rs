//! Skyriding abilities (Retail 12.x, DB2 12.1.0.69933).
//!
//! Vigor is Skyriding Charges: Surge Forward 372608 and Skyward Ascent 372610 share
//! `ChargeCategory` 2391 ("Skryriding Charges - Core", MaxCharges 6, ChargeRecoveryTime
//! 15000), whose recovery the Skyriding mount aura 406095 speeds up by its
//! CHARGE_RECOVERY_MULTIPLIER 454 -31% (10.35 s a charge; `SpellHistory::GetChargeRecoveryTime`).
//! Both spend it through the spell pipeline (`spell_cast`); Second Wind 425782 gives one back
//! (MODIFY_CHARGES 293). Their DUMMY effects (and Whirling Surge 361584's, no charge, 30 s)
//! flap the mount after the cast: the flap's impulse
//! (`shared::skyriding::ability_impulse`) raises the movement bound of the skyrider
//! (`networking_movement::MovementClocks`), so the server follows the faster glider.
use bevy::prelude::*;
use shared::protocol::ActionRef;
use shared::skyriding::{AERIAL_HALT, SKYWARD_ASCENT, SURGE_FORWARD, WHIRLING_SURGE};

use crate::spell_info::SpellInfo;

/// Second Wind: MODIFY_CHARGES 293, one Skyriding Charge back.
const SECOND_WIND: u32 = 425_782;
/// The Skyriding abilities in their bar order (team-lead's Retail order; no DB2 table or
/// TrinityCore code lists the Skyriding bar).
const SKYRIDING_BAR: [u32; 5] = [
    SURGE_FORWARD,
    SKYWARD_ASCENT,
    AERIAL_HALT,
    WHIRLING_SURGE,
    SECOND_WIND,
];
/// First slot of bonus bar 5, the page the main bar shows while skyriding:
/// `(NUM_ACTIONBAR_PAGES + GetBonusBarOffset() - 1) * NUM_ACTIONBAR_BUTTONS`
/// (Blizzard_Tutorials_RPE.lua:446, `NUM_ACTIONBAR_PAGES` 6, offset 5, 12 buttons).
const SKYRIDING_BAR_FIRST_SLOT: u8 = (6 + 5 - 1) * 12;

/// The unit can now skyride (an ADV_FLYING aura, `auras::mount::reconcile_can_fly`): put
/// the Skyriding abilities it knows on the Skyriding bar.
pub fn fill_skyriding_bar(world: &mut World, unit: Entity) {
    let actions: Vec<(u8, ActionRef)> = (SKYRIDING_BAR_FIRST_SLOT..)
        .zip(SKYRIDING_BAR)
        .filter(|&(_, spell)| crate::spell_cast::caster_knows_spell(world, unit, spell))
        .map(|(slot, spell)| (slot, ActionRef::Spell(spell)))
        .collect();
    crate::class_progression::place_actions(world, unit, &actions);
}

/// Flap impulse, yards/s, a Skyriding ability gave the unit since its movement last ran.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct SkyridingImpulse(pub f32);

/// Surge Forward, Skyward Ascent and Whirling Surge after their cast
/// (`spell_cast::scripts::after_cast`): the caster flaps by the ability's impulse.
pub fn flap(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let Some(speed) = shared::skyriding::ability_impulse(spell.id) else {
        return;
    };
    let mut entity = world.entity_mut(caster);
    let mut impulse = entity
        .entry::<SkyridingImpulse>()
        .or_insert(SkyridingImpulse(0.0));
    impulse.get_mut().0 += speed;
}

#[cfg(test)]
#[path = "skyriding_tests.rs"]
mod tests;
