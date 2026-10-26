//! Pack Mentality rewards each actual consumed Howl beast with native recovery.
use super::{combat, has_talent};
use bevy::prelude::*;
use combat::spell_cast::scripts::{reduce_cooldown, script_effect_value};

const PACK_MENTALITY: u32 = 472358;
const RECOVERY_EFFECTS: &[(u32, u8)] = &[(217200, 1), (259495, 2)];

pub(super) fn recover_cooldowns(world: &mut World, caster: Entity, consumed: u32) {
    if !has_talent(world, caster, PACK_MENTALITY) {
        return;
    }
    // SimC76f8118 hunter3808–3811: both clocks recover per consumed beast,
    // before Incoming Stampede starts. The shared reducer never starts an
    // inactive clock; native472358 effects2/3 supply milliseconds.
    for &(spell, effect) in RECOVERY_EFFECTS {
        let milliseconds = script_effect_value(world, caster, PACK_MENTALITY, effect);
        let seconds = f64::from(milliseconds) * f64::from(consumed) / 1000.0;
        reduce_cooldown(world, caster, spell, seconds);
    }
}
