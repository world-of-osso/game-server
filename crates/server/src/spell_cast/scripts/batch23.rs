//! TC a352b1fa spell_dh.cpp:1118-1140, spell_dh_enduring_torment_buff: for player
//! casters 453314 keeps effects 0/1 only for Havoc and 2/3 only for Devourer.
use bevy::prelude::*;
use shared::components::{ActiveSpec, Player};

const ENDURING_TORMENT_BUFF: u32 = 453_314;
const SPEC_HAVOC: u32 = 577;
const SPEC_DEVOURER: u32 = 1480;

pub(in crate::spell_cast) fn effect_enabled(
    world: &World,
    caster: Entity,
    spell: u32,
    index: u8,
) -> bool {
    if spell != ENDURING_TORMENT_BUFF || world.get::<Player>(caster).is_none() {
        return true;
    }
    let spec = world.get::<ActiveSpec>(caster).map(|spec| spec.0);
    let required = if index < 2 { SPEC_HAVOC } else { SPEC_DEVOURER };
    spec == Some(required)
}
