//! Landed Feral finishers apply the caster-owned Sabertooth vulnerability.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{
    self,
    effects::EffectCtx,
    script_registry::{EnemyContextHitHook, SpellScriptHook},
};
use shared::components::PowerType;

const BITE: u32 = 22568;
const TALENT: u32 = 202031;
const DEBUFF: u32 = 391722;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResourceContext(&[BITE]),
    SpellScriptHook::EnemyContextHit(hit),
];

fn hit(spell: u32) -> Option<EnemyContextHitHook> {
    (spell == BITE).then_some(apply_sabertooth)
}

// SimC76f8118 sc_druid.cpp4295–4304/13852–13854: a landed primary Bite
// adds its spent combo points, capped at five, and refreshes the owned debuff.
fn apply_sabertooth(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    if ctx.spell.cast_primary_target != Some(target) {
        return;
    }
    let value = world.get::<ActiveAuras>(ctx.caster).and_then(|auras| {
        auras
            .iter()
            .find(|a| a.spell_id == TALENT && a.caster == ctx.caster)
            .and_then(|a| a.effects.iter().find(|effect| effect.index == 1))
            .map(|effect| effect.amount)
    });
    let points = ctx.spell.cast_policy_context.as_ref().and_then(|policy| {
        policy
            .resources
            .iter()
            .find(|(power, _)| *power == PowerType::ComboPoints)
            .map(|(_, points)| *points)
    });
    let (Some(value), Some(points)) = (value, points) else {
        return;
    };
    if points < 1.0 {
        return;
    }
    let Some(mut spell) = spell_cast::load_triggered_spell(world, DEBUFF, ctx.depth + 1) else {
        return;
    };
    spell.cast_aura_stacks = Some((points as u8).min(5));
    for effect in &mut spell.effects {
        effect.base_points = value;
    }
    spell_cast::launch_triggered_spell(world, ctx.caster, &spell, Some(target), ctx.depth + 1);
}
