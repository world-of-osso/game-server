//! Darkglare Eye Beam reads its real owner's distinct active Affliction DoTs.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{
    script_registry::SpellScriptHook, scripts::script_effect_value, summon::SummonedCreature,
};
use combat::spell_info::SpellInfo;
use shared::components::Npc;

const DARKGLARE: u32 = 103673;
const EYE_BEAM: u32 = 205231;
const SUMMON_DARKGLARE: u32 = 205180;
const DOTS: &[u32] = &[980, 146739, 445474, 27243, 1259790];

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::TargetDamageMultiplier(damage_multiplier)];

// SimC76f8118 sc_warlock.cpp248–264: UA counts once regardless of stacks;
// another caster's auras are not this guardian owner's ticking DoTs.
fn count_owned_dots(world: &World, owner: Entity, target: Entity) -> usize {
    let Some(auras) = world.get::<ActiveAuras>(target) else {
        return 0;
    };
    let now = combat::auras::aura_now(world);
    DOTS.iter()
        .filter(|&&spell| {
            auras.iter().any(|aura| {
                aura.caster == owner
                    && aura.spell_id == spell
                    && aura.stacks > 0
                    && aura.expires_at.is_none_or(|expiry| expiry > now)
            })
        })
        .count()
}

// SimC76f8118 pets2445–2455: the ordinary Eye Beam increases by native
// Summon Darkglare effect2 per distinct DoT on its actual recipient.
fn damage_multiplier(world: &World, caster: Entity, target: Entity, spell: &SpellInfo) -> f32 {
    if spell.id != EYE_BEAM
        || world
            .get::<Npc>(caster)
            .is_none_or(|npc| npc.template_id != DARKGLARE)
    {
        return 1.0;
    }
    let Some(summon) = world.get::<SummonedCreature>(caster) else {
        return 1.0;
    };
    let dots = count_owned_dots(world, summon.owner, target);
    let percent = script_effect_value(world, summon.owner, SUMMON_DARKGLARE, 2);
    1.0 + dots as f32 * percent / 100.0
}
