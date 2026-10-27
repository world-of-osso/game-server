//! TC a352b1fa spell_dh.cpp:1118-1140, spell_dh_enduring_torment_buff: for player
//! casters 453314 keeps effects 0/1 only for Havoc and 2/3 only for Devourer.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::SpellScriptHook;

pub const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EffectEnabled(effect_enabled)];
use shared::components::{ActiveSpec, Player};

const ENDURING_TORMENT_BUFF: u32 = 453_314;
const UNTETHERED_FURY: u32 = 452_411;
const SPEC_HAVOC: u32 = 577;
const SPEC_DEVOURER: u32 = 1480;

fn effect_enabled(world: &World, caster: Entity, spell: u32, index: u8) -> bool {
    if world.get::<Player>(caster).is_none() {
        return true;
    }
    let spec = world.get::<ActiveSpec>(caster).map(|spec| spec.0);
    match spell {
        ENDURING_TORMENT_BUFF => {
            let required = if index < 2 { SPEC_HAVOC } else { SPEC_DEVOURER };
            spec == Some(required)
        }
        // SimC76f8118 sc_demon_hunter.cpp11438–11445 deregisters the
        // other specialization's native max-Fury effect.
        UNTETHERED_FURY => !matches!((spec, index), (Some(SPEC_DEVOURER), 0) | (Some(577 | 581), 1)),
        _ => true,
    }
}
