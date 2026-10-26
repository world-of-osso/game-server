//! Earned Coup de Grace action admission.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras};
use combat::spell_cast::script_registry::SpellScriptHook;

const COUP_TALENT: u32 = 441423;
const COUP: u32 = 441776;
const DISPATCH: u32 = 2098;
const ESCALATING: u32 = 441786;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ActionAvailable(available)];

fn available(world: &World, caster: Entity, spell: u32) -> bool {
    if spell != COUP && spell != DISPATCH {
        return true;
    }
    // SimC76f8118 sc_rogue.cpp3797–3802/6898–6903: maximum earned
    // Escalating Blade replaces Dispatch with Coup until it is consumed.
    let earned = auras::has_aura(world, caster, COUP_TALENT)
        && world.get::<ActiveAuras>(caster).is_some_and(|auras| {
            auras.iter().any(|aura| {
                aura.spell_id == ESCALATING
                    && aura.caster == caster
                    && aura.max_stacks > 0
                    && aura.stacks == aura.max_stacks
            })
        });
    if spell == COUP { earned } else { !earned }
}
