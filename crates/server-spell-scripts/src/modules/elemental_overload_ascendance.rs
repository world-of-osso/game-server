//! Ascendance's native damage bonus applies to Elemental Overload children.
use bevy::prelude::*;

use super::combat;
use combat::auras::ActiveAuras;
use combat::class_progression::ActiveSpec;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::TargetDamageMultiplier(multiplier)];

const ASCENDANCE: u32 = 1219480;

fn multiplier(world: &World, caster: Entity, _target: Entity, spell: &SpellInfo) -> f32 {
    // SimC76f8118 sc_shaman.cpp4884–4891: live Ascendance applies native effect8 to overloads.
    if !spell_cast::scripts::is_overload_damage(spell.id)
        || !world.get::<ActiveSpec>(caster).is_some_and(|spec| spec.0 == 262)
    {
        return 1.0;
    }
    let now = spell_cast::now(world);
    let percent = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras.iter().find(|aura| {
                aura.spell_id == ASCENDANCE
                    && aura.caster == caster
                    && aura.expires_at.is_none_or(|at| at > now)
            })
        })
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 7))
        .map_or(0.0, |effect| effect.amount);
    1.0 + percent / 100.0
}
