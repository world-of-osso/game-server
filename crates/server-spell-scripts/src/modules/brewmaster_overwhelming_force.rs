//! SimC76f8118 sc_monk.cpp787–835: copy the resolved primary Monk strike.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{
    effects::{self, EffectCtx},
    script_registry::SpellScriptHook,
    scripts::script_effect_value,
};
use combat::spell_info::SpellStore;

const TALENT: u32 = 451024;
const DAMAGE: u32 = 452333;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedDamage(copy_primary_hit)];

fn copy_primary_hit(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool, amount: f32) {
    if !matches!(ctx.spell.id, 100780 | 205523)
        || ctx.spell.cast_primary_target != Some(target)
        || amount <= 0.0
        || !world
            .get::<ActiveSpec>(ctx.caster)
            .is_some_and(|spec| spec.0 == 268)
        || !world.get::<TraitLoadout>(ctx.caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == TALENT)
        })
    {
        return;
    }
    let percent = script_effect_value(world, ctx.caster, TALENT, 0) / 100.0;
    let spell = world
        .resource::<SpellStore>()
        .get(DAMAGE)
        .expect("native Overwhelming Force damage")
        .clone();
    let copied = EffectCtx {
        caster: ctx.caster,
        spell: &spell,
        depth: ctx.depth + 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    effects::deal_copied_damage(world, &copied, target, amount * percent);
}
