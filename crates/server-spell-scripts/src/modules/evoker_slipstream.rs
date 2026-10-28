//! SimC76f8118 sc_evoker.cpp6314–6317/8027–8030: Slipstream resets Hover on breath.
use bevy::prelude::*;

use super::combat;
use combat::auras::has_aura;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::{AfterCastHook, reduce_cooldown};
use combat::spell_info::{SpellInfo, SpellStore};

const SLIPSTREAM: u32 = 441257;
const HOVER: u32 = 358267;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterResourceValues(cast_hook)];

fn cast_hook(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 357210 | 433874 | 403631 | 442204).then_some(reset_hover)
}

fn reset_hover(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if !has_aura(world, caster, SLIPSTREAM) {
        return;
    }
    let hover = world.resource::<SpellStore>().get(HOVER).expect("native Hover metadata");
    let category = hover.categories.as_ref().expect("native Hover charge category").charge_category;
    let recovery = hover.cooldowns.as_ref().expect("native Hover cooldown").recovery_ms;
    combat::spell_cast::restore_charges(world, caster, category, u8::MAX);
    reduce_cooldown(world, caster, HOVER, f64::from(recovery) / 1000.0);
}
