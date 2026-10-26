//! Swipe rank2's native primary contribution is not an all-recipient aura.
use super::cast_relationships::has_talent;
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::script_effect_value};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::TargetDamageMultiplier(primary_multiplier)];

const SWIPE: u32 = 1262293;
const SWIPE_RANK_TWO: u32 = 1259017;

fn primary_multiplier(world: &World, caster: Entity, target: Entity, spell: &SpellInfo) -> f32 {
    // SimC76f8118 sc_hunter.cpp6195–6196: selected rank2's second
    // effect modifies only chain_target0; native selected modifiers own its value.
    if spell.id != SWIPE || spell.cast_primary_target != Some(target) {
        return 1.0;
    }
    if !has_talent(world, caster, SWIPE_RANK_TWO) {
        return 1.0;
    }
    1.0 + script_effect_value(world, caster, SWIPE_RANK_TWO, 1) / 100.0
}
