//! Discipline MID2 Penance advances Mind Blast's running cooldown.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{script_registry::SpellScriptHook, scripts};
use shared::components::ActiveSpec;

const TWO_PIECE: u32 = 1296577;
const PENANCE: u32 = 47540;
const MIND_BLAST: u32 = 8092;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterNativeOnCast(after_cast)];

fn after_cast(world: &mut World, caster: Entity, spell: u32, _: u8) {
    if spell != PENANCE
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 256)
        || !combat::auras::has_aura(world, caster, TWO_PIECE)
    {
        return;
    }
    // SimC76f8118 sc_priest_discipline.cpp:596-600: Penance advances
    // Mind Blast once per cast by the set's native second effect.
    let milliseconds = scripts::script_effect_value(world, caster, TWO_PIECE, 1);
    scripts::reduce_cooldown(world, caster, MIND_BLAST, f64::from(milliseconds) / 1000.0);
}
