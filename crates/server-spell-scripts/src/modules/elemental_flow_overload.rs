//! Flowing Elements' explicit Lightning Bolt and Lava Burst Overload eligibility.
use bevy::prelude::*;

use super::combat;
use combat::auras::ActiveAuras;
use combat::class_progression::ActiveSpec;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::CaptureConsumedDamageModifier(snapshot)];

fn snapshot(world: &World, caster: Entity, spell: &mut SpellInfo, _foreground: bool) -> Option<u32> {
    let factor = multiplier(world, caster, spell);
    if factor != 1.0 {
        spell.cast_damage_multiplier = Some(spell.cast_damage_multiplier.unwrap_or(1.0) * factor);
    }
    None // This modifier does not consume the native aura.
}

fn multiplier(world: &World, caster: Entity, spell: &SpellInfo) -> f32 {
    // SimC76f8118 sc_shaman.cpp3265–3267/6795/7323–7324: these two
    // background overloads explicitly qualify despite the native class mask.
    if !matches!(spell.id, 45284 | 285466)
        || !world.get::<ActiveSpec>(caster).is_some_and(|spec| spec.0 == 262)
    {
        return 1.0;
    }
    let now = spell_cast::now(world);
    let percent = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras.iter().find(|aura| aura.spell_id == 1300219 && aura.caster == caster
            && aura.expires_at.is_none_or(|at| at > now))
    }).and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map_or(0.0, |effect| effect.amount);
    1.0 + percent / 100.0
}
