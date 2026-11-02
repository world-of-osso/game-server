//! Native 373202 and SimC76f8118 sc_priest.cpp266–270 / sc_priest_shadow.cpp1262–1287.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras};
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

const TALENT: u32 = 373202;
const BUFF: u32 = 373204;
const MIND_BLAST: u32 = 8092;
const VOID_BLAST: u32 = 450983;
const MADNESS: u32 = 335467;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResolvedDamage(after_mind_blast),
    SpellScriptHook::CaptureConsumedDamageModifier(snapshot_madness),
    SpellScriptHook::AfterForegroundSpecial(consume_buff),
];

fn after_mind_blast(world: &mut World, ctx: &EffectCtx<'_>, _: Entity, _: bool, _: f32) {
    if !matches!(ctx.spell.id, MIND_BLAST | VOID_BLAST)
        || !auras::has_aura(world, ctx.caster, TALENT)
    {
        return;
    }
    let chance = spell_cast::scripts::script_effect_value(world, ctx.caster, TALENT, 0) / 100.0;
    if combat::random::random::<f32>() < chance {
        spell_cast::cast_triggered(world, ctx.caster, BUFF, Some(ctx.caster), ctx.depth + 1);
    }
}

fn snapshot_madness(
    world: &World,
    caster: Entity,
    spell: &mut SpellInfo,
    foreground: bool,
) -> Option<u32> {
    if foreground && spell.id == MADNESS && auras::has_aura(world, caster, BUFF) {
        let bonus = spell_cast::scripts::script_effect_value(world, caster, BUFF, 1) / 100.0;
        spell.cast_damage_multiplier =
            Some(spell.cast_damage_multiplier.unwrap_or(1.0) * (1.0 + bonus));
    }
    None
}

fn consume_buff(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != MADNESS {
        return;
    }
    let instance = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.caster == caster && aura.spell_id == BUFF)
            .map(|aura| aura.instance_id)
    });
    if let Some(instance) = instance {
        auras::spend_stack(world, caster, instance);
    }
}
