//! Shared raid-haste lockouts, matching spell_gen_bloodlust in TrinityCore.
use bevy::prelude::*;

pub const LOCKOUTS: &[u32] = &[57724, 57723, 80354, 264689, 390435];

pub fn has_lockout(world: &World, target: Entity) -> bool {
    LOCKOUTS
        .iter()
        .any(|&spell| super::has_aura(world, target, spell))
}

pub fn lockout_for(spell: u32) -> Option<u32> {
    match spell {
        2825 => Some(57724),
        32182 => Some(57723),
        80353 => Some(80354),
        264667 => Some(264689),
        390386 => Some(390435),
        _ => None,
    }
}
