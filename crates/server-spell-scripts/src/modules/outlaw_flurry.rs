//! Blade Flurry's independent activation hit, not its secondary residual copies.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::spell_cast::scripts::{AfterCastHook, script_effect_value};
use combat::spell_cast::{
    self,
    effects::{self, EffectCtx},
    script_registry::SpellScriptHook,
};
use combat::spell_info::SpellInfo;
use shared::components::PowerType;

const BLADE_FLURRY: u32 = 13877;
const INSTANT_ATTACK: u32 = 429951;
const DEFT_MANEUVERS: u32 = 381878;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AuraRollAfterCast(after_cast),
    SpellScriptHook::ResolvedDamage(after_hit),
];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == BLADE_FLURRY).then_some(launch_activation)
}

fn launch_activation(world: &mut World, caster: Entity, _: &SpellInfo) {
    // SimC76f8118 sc_rogue.cpp3993–3999: the self-cast buff launches one
    // native area attack; its own native target plan selects nearby enemies.
    spell_cast::cast_triggered(world, caster, INSTANT_ATTACK, Some(caster), 1);
}

fn after_hit(world: &mut World, ctx: &EffectCtx, _: Entity, _: bool, damage: f32) {
    if ctx.spell.id != INSTANT_ATTACK
        || damage <= 0.0
        || !auras::has_aura(world, ctx.caster, DEFT_MANEUVERS)
    {
        return;
    }
    // sc_rogue.cpp3916–3922: selected Deft Maneuvers pays once per hit.
    let points = script_effect_value(world, ctx.caster, DEFT_MANEUVERS, 1);
    effects::energize_unit(
        world,
        ctx,
        ctx.caster,
        PowerType::ComboPoints,
        points,
        false,
    );
}
