//! Skyriding abilities (Retail 12.x, DB2 12.1.0.69933).
//!
//! Vigor is Skyriding Charges: Surge Forward 372608 and Skyward Ascent 372610 share
//! `ChargeCategory` 2391 ("Skryriding Charges - Core", MaxCharges 6, ChargeRecoveryTime
//! 15000), whose recovery the Skyriding mount aura 406095 speeds up by its
//! CHARGE_RECOVERY_MULTIPLIER 454 -31% (10.35 s a charge; `SpellHistory::GetChargeRecoveryTime`).
//! Both spend it through the spell pipeline (`spell_cast`); Second Wind 425782 gives one back
//! (MODIFY_CHARGES 293). Their effect 0 DUMMY flaps the mount: the flap's impulse
//! (`shared::skyriding::ability_impulse`) raises the movement bound of the skyrider
//! (`networking_movement::MovementClocks`), so the server follows the faster glider.
use bevy::prelude::*;

/// Flap impulse, yards/s, a Skyriding ability gave the unit since its movement last ran.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct SkyridingImpulse(pub f32);

/// Surge Forward 372608 effect 0 (DUMMY, the caster).
pub fn surge_forward(world: &mut World, _caster: Entity, target: Entity, _depth: u8) {
    flap(world, target, shared::skyriding::SURGE_FORWARD);
}

/// Skyward Ascent 372610 effect 0 (DUMMY, the caster).
pub fn skyward_ascent(world: &mut World, _caster: Entity, target: Entity, _depth: u8) {
    flap(world, target, shared::skyriding::SKYWARD_ASCENT);
}

fn flap(world: &mut World, unit: Entity, spell_id: u32) {
    let Some(speed) = shared::skyriding::ability_impulse(spell_id) else {
        return;
    };
    let mut entity = world.entity_mut(unit);
    let mut impulse = entity
        .entry::<SkyridingImpulse>()
        .or_insert(SkyridingImpulse(0.0));
    impulse.get_mut().0 += speed;
}

#[cfg(test)]
#[path = "skyriding_tests.rs"]
mod tests;
