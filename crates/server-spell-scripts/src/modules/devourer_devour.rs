//! Devour's source-defined native Fury payload.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::OnCast(on_cast)];

fn on_cast(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    if spell != 1217610 {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp2468–2474,5774–5779 executes
    // Devour's native Fury action on each cast, independent of hit outcome.
    if !spell_cast::try_cast_triggered(world, caster, 1288123, Some(caster), depth + 1) {
        warn!(?caster, "Devour native Fury payload 1288123 was refused");
    }
}
