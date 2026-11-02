//! Selected Twin Claw may follow a landed foreground Feral melee ability.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::random::Rng;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use shared::components::ActiveSpec;

const TALENT: u32 = 1271635;
const CLAW: u32 = 1271636;
const DRUID_CLASS_SET: u32 = 7;
const MELEE_DEFENSE: u8 = 2;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OwnsPolicyProc(owns_proc),
    SpellScriptHook::AfterPolicyDirectDamage(after_melee),
];
fn owns_proc(spell: u32) -> bool {
    spell == TALENT
}

// SimC76f8118 sc_druid.cpp12890–12904: selected Feral melee class
// ability impacts independently roll native Twin Claw effect1 chance.
fn after_melee(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool) {
    if ctx.depth != 0
        || world
            .get::<ActiveSpec>(ctx.caster)
            .is_none_or(|spec| spec.0 != 103)
        || ctx
            .spell
            .class_options
            .as_ref()
            .is_none_or(|class| class.spell_class_set != DRUID_CLASS_SET)
        || ctx
            .spell
            .categories
            .as_ref()
            .is_none_or(|category| category.defense_type != MELEE_DEFENSE as _)
    {
        return;
    }
    let chance = world.get::<ActiveAuras>(ctx.caster).and_then(|auras| {
        auras
            .iter()
            .find(|a| a.spell_id == TALENT && a.caster == ctx.caster)
            .and_then(|a| a.effects.iter().find(|effect| effect.index == 0))
            .map(|effect| effect.amount)
    });
    if let Some(chance) = chance
        && combat::random::thread_rng().gen_range(0.0..100.0) < chance
    {
        spell_cast::cast_triggered(world, ctx.caster, CLAW, Some(target), ctx.depth + 1);
    }
}
