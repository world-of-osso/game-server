//! SimC76f8118 sc_druid.cpp5192–5212,5428–5433,5600–5605,11959–11962.
use super::{combat, marksmanship::bank_residual};
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook};

const TALENT: u32 = 1269617;
const RESIDUAL: u32 = 1270065;
const SOURCES: &[u32] = &[33917, 6807, 400254, 441605];

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedDamage(after_impact)];

fn after_impact(world: &mut World, ctx: &EffectCtx, target: Entity, _crit: bool, damage: f32) {
    if damage <= 0.0 || !SOURCES.contains(&ctx.spell.id) {
        return;
    }
    let percent = world.get::<ActiveAuras>(ctx.caster).and_then(|auras| {
        let talent = auras
            .iter()
            .find(|aura| aura.spell_id == TALENT && aura.caster == ctx.caster)?;
        Some(
            talent
                .effects
                .iter()
                .find(|effect| effect.index == 1)
                .expect("Wild Guardian requires its native Vicious Brambles fraction")
                .amount,
        )
    });
    if let Some(percent) = percent {
        bank_residual(world, ctx, target, RESIDUAL, percent, damage);
    }
}
