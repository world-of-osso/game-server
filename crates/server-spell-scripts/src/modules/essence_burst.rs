//! TC a352b1fa spell_evoker.cpp322–350: one native Azure talent roll per completed cast.
//! Dragonrage guarantee: native70448 375087s2; SimC76f8118 sc_evoker.cpp5625/7012.
//! Azure Sweep shares the base Azure execute producer: sc_evoker.cpp5603–5628/5671–5678.
//! Ruby rolls per actual Living Flame hit (`evoker_flames`, sc_evoker.cpp7004–7023).
use super::combat;
use combat::random as rand;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::AfterCastHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::NativeAfterCast(hook)];

fn hook(spell: u32) -> Option<AfterCastHook> {
    match spell {
        AZURE_STRIKE | AZURE_SWEEP => Some(roll_azure_essence_burst),
        _ => None,
    }
}
use bevy::prelude::*;

use combat::auras::{ActiveAuras, has_aura};
use combat::spell_info::SpellInfo;

pub(super) const AZURE_STRIKE: u32 = 362969;
const AZURE_SWEEP: u32 = 1265872;
const AZURE_ESSENCE_BURST: u32 = 375721;
const RUBY_ESSENCE_BURST: u32 = 376872;
const ESSENCE_BURST: u32 = 359618;
const DRAGONRAGE: u32 = 375087;

fn roll_azure_essence_burst(world: &mut World, caster: Entity, _: &SpellInfo) {
    roll_essence_burst(world, caster, AZURE_ESSENCE_BURST);
}

pub(super) fn roll_ruby_essence_burst(world: &mut World, caster: Entity) {
    roll_essence_burst(world, caster, RUBY_ESSENCE_BURST);
}

fn roll_essence_burst(world: &mut World, caster: Entity, talent: u32) {
    let Some(chance) = talent_chance(world, caster, talent) else {
        return;
    };
    let chance = if has_aura(world, caster, DRAGONRAGE) {
        combat::spell_cast::scripts::script_effect_value(world, caster, DRAGONRAGE, 1)
    } else {
        chance
    };
    if rand::random::<f32>() * 100.0 < chance {
        combat::spell_cast::cast_triggered(world, caster, ESSENCE_BURST, Some(caster), 1);
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
