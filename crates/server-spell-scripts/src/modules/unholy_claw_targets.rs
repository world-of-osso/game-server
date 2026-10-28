//! Sweeping Claws' native splash excludes the primary impact target.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::{AreaTargetFilter, SpellScriptHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AreaFilter(filter)];

fn filter(spell: u32, effect: u8) -> Option<AreaTargetFilter> {
    // SimC76f8118 sc_death_knight.cpp3298–3321/3520–3543: effect3 is
    // secondary-only for both the controlled and lesser ghoul attacks.
    (matches!(spell, 91778 | 1278150) && effect == 2).then_some(exclude_primary)
}

fn exclude_primary(_: &World, _: &SpellInfo, primary: Option<Entity>, targets: &mut Vec<Entity>) {
    if let Some(primary) = primary { targets.retain(|target| *target != primary); }
}
