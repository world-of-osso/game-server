//! Devourer fragment heal's out-of-Meta stack production.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::{HitHook, SpellScriptHook}};
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::Hit(hit_hook)];

fn hit_hook(spell: u32) -> Option<HitHook> {
    (spell == 1266301).then_some(on_fragment_heal)
}

fn on_fragment_heal(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if caster != target
        || world.get::<ActiveSpec>(caster).map(|spec| spec.0) != Some(1480)
        || combat::auras::has_aura(world, caster, 1217607)
    {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp3678–3692: each fragment heal
    // impact outside Meta triggers one native Void Metamorphosis stack.
    if !spell_cast::try_cast_triggered(world, caster, 1225789, Some(caster), depth + 1) {
        warn!(?caster, "native fragment Meta stack 1225789 was refused");
    }
}
