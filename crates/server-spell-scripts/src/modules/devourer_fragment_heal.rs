//! Devourer fragment heal's source-defined stack production.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::KnownSpells;
use combat::spell_cast::{
    self,
    script_registry::{HitHook, SpellScriptHook},
};
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::Hit(hit_hook)];

fn hit_hook(spell: u32) -> Option<HitHook> {
    (spell == 1266301).then_some(on_fragment_heal)
}

fn on_fragment_heal(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if caster != target || world.get::<ActiveSpec>(caster).map(|spec| spec.0) != Some(1480) {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp3678–3698 selects the Meta stack
    // outside Meta and the selected Collapsing Star stack inside Meta.
    let stack = if !combat::auras::has_aura(world, caster, 1217607) {
        1225789
    } else if world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(1221167))
    {
        1227702
    } else {
        return;
    };
    if !spell_cast::try_cast_triggered(world, caster, stack, Some(caster), depth + 1) {
        warn!(?caster, stack, "native fragment stack was refused");
    }
}
