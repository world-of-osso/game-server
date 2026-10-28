//! SimC76f8118 sc_druid.cpp5148–5188: earned Guardian damage feeds Ursoc's Fury.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedDamage(grant_shield)];
const TALENT: u32 = 377210;
const SHIELD: u32 = 372505;
// Thrash direct and bleed, Maul, Raze, and the native Ravage-Maul child.
const SOURCES: &[u32] = &[77758, 192090, 6807, 400254, 441605];

fn selected_amount(world: &World, caster: Entity, spell: u32) -> Option<f32> {
    world.get::<ActiveAuras>(caster)?.iter()
        .find(|aura| aura.spell_id == spell && aura.caster == caster)?
        .effects.iter().find(|effect| effect.index == 0).map(|effect| effect.amount)
}

fn grant_shield(world: &mut World, ctx: &EffectCtx<'_>, _target: Entity, _crit: bool, damage: f32) {
    if damage <= 0.0 || !SOURCES.contains(&ctx.spell.id) { return; }
    let caster = ctx.caster;
    let Some(percent) = selected_amount(world, caster, TALENT) else { return };
    let nurturing = selected_amount(world, caster, 33873).unwrap_or(0.0);
    let recovery = selected_amount(world, caster, 377796).unwrap_or(0.0);
    let multiplier = 1.0 + percent / 100.0 * (1.0 + nurturing / 100.0) * (1.0 + recovery / 100.0);
    let mut shield = world.resource::<SpellStore>().get(SHIELD)
        .expect("selected Ursoc's Fury requires native shield").clone();
    shield.effects.iter_mut().find(|effect| effect.index == 0)
        .expect("native Ursoc's Fury absorb").base_points = damage * multiplier;
    // SimC uses actual dealt damage and does not apply outgoing absorb modifiers again.
    shield.misc.as_mut().expect("native Ursoc's Fury metadata").attributes[11] |= 0x0008_0000;
    spell_cast::launch_triggered_spell(world, caster, &shield, Some(caster), ctx.depth + 1);
}
