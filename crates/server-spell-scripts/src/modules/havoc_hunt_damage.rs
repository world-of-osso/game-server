//! SimC76f8118 sc_demon_hunter.cpp8486–8501: bugs-mode Hunt DoT double-dips Demon Hide.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::KnownSpells;
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::script_effect_value};
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::DotMasteryMultiplier(hunt_dot)];

fn hunt_dot(world: &World, caster: Entity, _: Entity, spell: u32) -> f32 {
    if spell == 370969
        && world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 577)
        && world
            .get::<KnownSpells>(caster)
            .is_some_and(|spells| spells.contains(428241))
    {
        1.0 + script_effect_value(world, caster, 428241, 0) / 100.0
    } else {
        1.0
    }
}
