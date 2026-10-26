//! Counts and named resource bridges from local Mainline69933 and70448.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::{AfterCastLookup, SpellScriptHook};
use combat::spell_info::SpellInfo;

const BARBED_SHOT: u32 = 217200;
const BARBED_FOCUS: u32 = 246152;
// Native70448 Pack321014 names instant Focus child1282660 in its description.
const PACK_TACTICS: u32 = 321014;
const PACK_FOCUS: u32 = 1282660;
const STORMKEEPER: u32 = 191634;

pub const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::NativeAfterCast(after_cast as AfterCastLookup),
    SpellScriptHook::ProcCharges(proc_charges),
];

fn after_cast(spell: u32) -> Option<combat::spell_cast::scripts::AfterCastHook> {
    (spell == BARBED_SHOT).then_some(grant_barbed_focus)
}

///217200 explicitly names246152's amount, duration and period. Start the
///native periodic resource aura once per completed root cast, not per tick.
pub(super) fn grant_barbed_focus(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    combat::spell_cast::cast_triggered(world, caster, BARBED_FOCUS, Some(caster), 1);
    if combat::auras::has_aura(world, caster, PACK_TACTICS)
        && !combat::spell_cast::try_cast_triggered(world, caster, PACK_FOCUS, Some(caster), 1)
    {
        warn!(?caster, "Native Pack Tactics instant Focus child refused");
    }
}

///191634s5 specifies two empowered spells; the options row's single charge
///does not implement that count. Ordinary matching spell mods consume uses.
pub(crate) fn proc_charges(world: &World, caster: Entity, spell: &SpellInfo, native: f32) -> f32 {
    if spell.id == STORMKEEPER {
        combat::spell_cast::scripts::script_effect_value(world, caster, STORMKEEPER, 4)
    } else {
        native
    }
}
