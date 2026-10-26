//! Source-backed Affliction modifiers read actual owned target auras.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::script_effect_value};
use combat::spell_info::SpellInfo;

const WITHERING: u32 = 386976;
const IMPETUOUS: u32 = 1312998;
const HAUNT: u32 = 48181;
const WITHERING_ACTIONS: &[u32] = &[686, 453176, 1261153, 198590];
// SimC76f8118 sc_warlock.cpp248–264: UA counts once, not once per stack.
const AFFLICTION_DOTS: &[u32] = &[980, 146739, 445474, 27243, 1259790];

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::TargetDamageMultiplier(target_multiplier),
    SpellScriptHook::ConsumedEffectValues(&[
        (WITHERING, 0),
        (WITHERING, 1),
        (IMPETUOUS, 0),
        (IMPETUOUS, 1),
        (IMPETUOUS, 2),
        (IMPETUOUS, 3),
    ]),
];

fn has_owned_active_aura(world: &World, caster: Entity, target: Entity, spell: u32) -> bool {
    let now = combat::auras::aura_now(world);
    world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras.iter().any(|aura| {
            if aura.caster != caster || aura.spell_id != spell {
                return false;
            }
            aura.stacks > 0 && aura.expires_at.is_none_or(|expiry| expiry > now)
        })
    })
}

// SimC76f8118 actions817–818/915–916/2223–2224/2368–2369: native
// percentage times distinct owned DoTs, capped by the selected native effect2.
fn withering_multiplier(world: &World, caster: Entity, target: Entity, spell: u32) -> f32 {
    if !WITHERING_ACTIONS.contains(&spell)
        || !has_owned_active_aura(world, caster, caster, WITHERING)
    {
        return 1.0;
    }
    let cap = script_effect_value(world, caster, WITHERING, 1) as usize;
    let dots = AFFLICTION_DOTS
        .iter()
        .filter(|&&dot| has_owned_active_aura(world, caster, target, dot))
        .count();
    1.0 + script_effect_value(world, caster, WITHERING, 0) / 100.0 * dots.min(cap) as f32
}

// SimC76f8118 actions918–919/2226–2227/2371–2372/2452–2453:
// Volley has Withering but no Impetuous Wrath; Harvest uses native effects3/4.
fn target_multiplier(world: &World, caster: Entity, target: Entity, spell: &SpellInfo) -> f32 {
    let withering = withering_multiplier(world, caster, target, spell.id);
    let base = match spell.id {
        686 | 1261153 | 198590 => 0,
        1257065 => 2,
        _ => return withering,
    };
    if !has_owned_active_aura(world, caster, caster, IMPETUOUS) {
        return withering;
    }
    let haunted = has_owned_active_aura(world, caster, target, HAUNT);
    let index = base + u8::from(haunted);
    withering * (1.0 + script_effect_value(world, caster, IMPETUOUS, index) / 100.0)
}
