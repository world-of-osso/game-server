//! TC a352b1fa spell_evoker.cpp322–350: one native talent roll per completed cast.
//! Dragonrage guarantee: native70448 375087s2; SimC76f8118 sc_evoker.cpp5625.
use bevy::prelude::*;

use crate::auras::{ActiveAuras, has_aura};
use crate::spell_info::SpellInfo;

pub(super) const AZURE_STRIKE: u32 = 362969;
pub(super) const LIVING_FLAME: u32 = 361469;
const AZURE_ESSENCE_BURST: u32 = 375721;
const RUBY_ESSENCE_BURST: u32 = 376872;
const ESSENCE_BURST: u32 = 359618;
const DRAGONRAGE: u32 = 375087;

pub(super) fn roll_essence_burst(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let talent = match spell.id {
        AZURE_STRIKE => AZURE_ESSENCE_BURST,
        LIVING_FLAME => RUBY_ESSENCE_BURST,
        _ => unreachable!("Essence Burst source registration"),
    };
    let Some(chance) = talent_chance(world, caster, talent) else {
        return;
    };
    let chance = if has_aura(world, caster, DRAGONRAGE) {
        super::script_effect_value(world, caster, DRAGONRAGE, 1)
    } else {
        chance
    };
    if rand::random::<f32>() * 100.0 < chance {
        crate::spell_cast::cast_triggered(world, caster, ESSENCE_BURST, Some(caster), 1);
    }
}

fn talent_chance(world: &World, caster: Entity, talent: u32) -> Option<f32> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == talent)
        .map(|aura| {
            aura.effects
                .iter()
                .find(|effect| effect.index == 0)
                .expect("native Essence Burst talent retains its chance effect")
                .amount
        })
}
