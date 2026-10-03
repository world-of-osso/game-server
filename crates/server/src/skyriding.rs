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

use crate::spell_info::SpellInfo;

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
