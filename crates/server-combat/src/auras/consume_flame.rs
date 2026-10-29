//! 70448 Consume Flame healing from an owned Dream Breath.
use bevy::prelude::*;

use super::{ActiveAuras, RemoveReason};
use crate::spell_cast::effects::{self, EffectCtx};

const TALENT: u32 = 444088;
const DREAM_BREATH: u32 = 355941;
const CONSUME_HEAL: u32 = 445495;

pub(crate) fn consume_dream_breath(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    let duration_effect = match ctx.spell.id {
        361195 => 4,
        355916 | 361361 => 0,
        _ => return,
    };
    let caster = ctx.caster;
    if !super::has_aura(world, caster, TALENT) {
        return;
    }
    let now = super::aura_now(world);
    let Some(aura) = world.get::<ActiveAuras>(target).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == DREAM_BREATH && aura.caster == caster)
            .cloned()
    }) else {
        return;
    };
    let Some(expiry) = aura.expires_at else {
        return;
    };
    let Some(periodic) = aura.effects.iter().find(|effect| effect.index == 0) else {
        return;
    };
    if periodic.period_ms == 0 {
        return;
    }
    let requested =
        crate::spell_cast::scripts::script_effect_value(world, caster, TALENT, duration_effect);
    let remaining = ((expiry - now).max(0.0) * 1000.0).round() as u32;
    let consumed = remaining.min(requested.max(0.0) as u32);
    if consumed == 0 {
        return;
    }
    let multiplier =
        crate::spell_cast::scripts::script_effect_value(world, caster, TALENT, 5) / 100.0;
    let amount = periodic.amount * consumed as f32 / periodic.period_ms as f32 * multiplier;
    if consumed == remaining {
        super::remove_instance(world, target, aura.instance_id, RemoveReason::Cancelled);
    } else {
        super::set_owned_duration(world, caster, target, DREAM_BREATH, remaining - consumed);
    }
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(CONSUME_HEAL)
        .expect("Consume Flame requires spell 445495")
        .clone();
    let transfer = EffectCtx {
        caster,
        spell: &spell,
        depth: ctx.depth + 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    effects::apply_calculated_heal(world, &transfer, target, amount);
}
