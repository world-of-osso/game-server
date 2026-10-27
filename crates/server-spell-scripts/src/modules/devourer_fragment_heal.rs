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
    match spell {
        1266301 => Some(on_fragment_heal),
        1227702 => Some(on_collapsing_star_stack),
        _ => None,
    }
}

fn on_collapsing_star_stack(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if caster != target {
        return;
    }
    let threshold = spell_cast::scripts::script_effect_value(world, caster, 1221167, 0) as u8;
    let stacks = world.get::<combat::auras::ActiveAuras>(caster)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == 1227702 && aura.caster == caster))
        .map_or(0, |aura| aura.stacks);
    // SimC76f8118 sc_demon_hunter.cpp9318–9334 grants readiness on crossing the threshold.
    if threshold > 0 && stacks == threshold
        && !spell_cast::try_cast_triggered(world, caster, 1221171, Some(caster), depth + 1)
    {
        warn!(?caster, "native Collapsing Star ready aura 1221171 was refused");
    }
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
