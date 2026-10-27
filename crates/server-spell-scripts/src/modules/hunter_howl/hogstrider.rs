//! Hogstrider's earned Boar impacts empower the next Cobra Shot.
use super::{ActiveAuras, RemoveReason, auras, combat, has_talent, spell_cast};
use bevy::prelude::*;
use combat::class_progression::ActiveSpec;
use combat::spell_cast::effects::EffectCtx;
use combat::spell_info::SpellInfo;

const TALENT: u32 = 472639;
const BUFF: u32 = 472640;
const COBRA: u32 = 193455;

pub(super) fn owns_proc(spell: u32) -> bool {
    spell == BUFF
}

pub(super) fn after_boar_impact(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    _: Entity,
    _: bool,
    _: f32,
) {
    if matches!(ctx.spell.id, 471936 | 471938)
        && world.get::<ActiveSpec>(ctx.caster).is_some_and(|spec| spec.0 == 253)
        && has_talent(world, ctx.caster, TALENT)
    {
        // SimC76f8118 hunter.cpp4817–4846: each actual Boar impact increments.
        spell_cast::cast_triggered(world, ctx.caster, BUFF, Some(ctx.caster), ctx.depth + 1);
    }
}

pub(super) fn capture_cobra(world: &World, caster: Entity, spell: &mut SpellInfo) {
    if spell.id != COBRA {
        return;
    }
    let Some(aura) = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras.iter().find(|aura| aura.spell_id == BUFF && aura.caster == caster)
    }) else {
        return;
    };
    // SimC76f8118 hunter.cpp4979–4980: effect1 applies once, not per stack.
    let bonus = aura.effects.iter().find(|effect| effect.index == 0)
        .expect("native Hogstrider damage effect").amount;
    spell.cast_damage_multiplier = Some(spell.cast_damage_multiplier.unwrap_or(1.0) * (1.0 + bonus / 100.0));
}

pub(super) fn consume_cobra(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id == COBRA {
        auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == BUFF && aura.caster == caster
        });
    }
}
