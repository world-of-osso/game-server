//! Stormkeeper's explicit Lightning Bolt Overload damage eligibility.
use bevy::prelude::*;

use super::combat;
use combat::auras::ActiveAuras;
use combat::class_progression::ActiveSpec;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetDamageMultiplier(multiplier)];

fn multiplier(world: &World, caster: Entity, _target: Entity, spell: &SpellInfo) -> f32 {
    // SimC76f8118 sc_shaman.cpp3254–3257/7320–7327 explicitly enables
    // Stormkeeper for Elemental Lightning Bolt Overload despite its native mask.
    if spell.id != 45284 || !world.get::<ActiveSpec>(caster).is_some_and(|spec| spec.0 == 262) {
        return 1.0;
    }
    let now = spell_cast::now(world);
    let percent = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras.iter().find(|aura| aura.spell_id == 191634 && aura.caster == caster
            && aura.expires_at.is_none_or(|at| at > now))
    }).and_then(|aura| aura.effects.iter().find(|effect| effect.index == 1))
        .map_or(0.0, |effect| effect.amount);
    1.0 + percent / 100.0
}
