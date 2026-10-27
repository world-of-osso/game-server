//! SimC76f8118 sc_demon_hunter.cpp7579–7608,12585–12619.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::NativeAfterCast(after_cast),
    SpellScriptHook::OwnsPolicyProc(|spell| matches!(spell, 206478 | 1266330)),
];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 162794 | 201427).then_some(appetite)
}

fn appetite(world: &mut World, owner: Entity, _: &SpellInfo) {
    if world
        .get::<ActiveSpec>(owner)
        .is_none_or(|spec| spec.0 != 577)
    {
        return;
    }
    if combat::soul_fragments::roll_owned(world, owner, 206478)
        && let Err(error) = combat::soul_fragments::spawn(
            world,
            owner,
            combat::soul_fragments::FragmentKind::Lesser,
            false,
        )
    {
        error!(%error, "Demonic Appetite failed");
    }
}
