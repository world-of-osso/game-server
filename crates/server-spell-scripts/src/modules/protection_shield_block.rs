//! Shield Charge grants the same cumulative Shield Block as the paid action.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AuraRollAfterCast(hook)];

fn hook(spell: u32) -> Option<spell_cast::scripts::AfterCastHook> {
    (spell == 385952).then_some(grant_shield_block)
}

fn grant_shield_block(world: &mut World, caster: Entity, _: &SpellInfo) {
    spell_cast::scripts::extend_or_apply_shield_block(world, caster, 0);
}
