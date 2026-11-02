//! Ravage applies the selected Dreadful Wound to each landed target.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{
    self,
    effects::EffectCtx,
    script_registry::{EnemyContextHitHook, SpellScriptHook},
};

const RAVAGE: u32 = 441591;
const TALENT: u32 = 441809;
const EXACERBATING_WOUNDS: u32 = 1271839;
const WOUND: u32 = 441812;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EnemyContextHit(hit)];

fn selected(world: &World, actor: Entity, spell: u32) -> bool {
    world.get::<ActiveAuras>(actor).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.spell_id == spell && aura.caster == actor)
    })
}

fn hit(spell: u32) -> Option<EnemyContextHitHook> {
    (spell == RAVAGE).then_some(apply_wound)
}

// SimC76f8118 sc_druid.cpp2334–2343,2393–2397,14308–14310:
// selected Ravage impacts apply the dot; the vulnerability requires the separate talent.
fn apply_wound(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    if !selected(world, ctx.caster, TALENT) {
        return;
    }
    let Some(mut spell) = spell_cast::load_triggered_spell(world, WOUND, ctx.depth + 1) else {
        return;
    };
    if !selected(world, ctx.caster, EXACERBATING_WOUNDS) {
        spell
            .effects
            .retain(|effect| !matches!(effect.index, 2 | 3));
    }
    spell_cast::launch_triggered_spell(world, ctx.caster, &spell, Some(target), ctx.depth + 1);
}
