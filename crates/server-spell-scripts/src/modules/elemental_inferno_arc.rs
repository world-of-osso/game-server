//! Inferno Arc amplifies qualifying damage against the caster's Flame Shock.
use bevy::prelude::*;

use super::combat;
use combat::auras::ActiveAuras;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetDamageMultiplier(multiplier)];

fn multiplier(world: &World, caster: Entity, target: Entity, spell: &SpellInfo) -> f32 {
    // SimC76f8118 sc_shaman.cpp6324–6330,6618–6624,7330–7336,
    // 7411–7417,7589–7595,7678–7684,7810–7812,8192–8198,
    // 8221–8227,9858–9864,10069–10075: check this target's owned dot.
    if !matches!(spell.id, 188443 | 45297 | 188196 | 45284 | 117014 | 120588
        | 77478 | 298765 | 8042 | 381725 | 452201 | 463351)
    {
        return 1.0;
    }
    let now = spell_cast::now(world);
    let active = |unit, id| {
        world.get::<ActiveAuras>(unit).is_some_and(|auras| {
            auras.iter().any(|aura| {
                aura.spell_id == id && aura.caster == caster
                    && aura.expires_at.is_none_or(|at| at > now)
            })
        })
    };
    if !active(caster, 1259047) || !active(target, 188389) {
        return 1.0;
    }
    1.0 + spell_cast::scripts::script_effect_value(world, caster, 1259047, 0) / 100.0
}
