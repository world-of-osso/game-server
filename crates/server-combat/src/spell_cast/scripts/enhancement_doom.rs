//! SimC76f8118 sc_shaman.cpp11809–11827: Ascendance's earned Doom deck
//! draws only outside its actual active aura; spending within the aura neither draws nor resets it.
use bevy::prelude::*;

const ASCENDANCE: u32 = 114051;

pub(super) fn can_draw(world: &World, caster: Entity, talent: u32) -> bool {
    talent != ASCENDANCE || !crate::auras::has_aura(world, caster, ASCENDANCE)
}
