//! SimC76f8118 sc_demon_hunter.cpp7040–7110,7252–7275,7324–7385.
use crate::spell_cast::effects::EffectCtx;
use crate::spell_info::{SpellEffect, SpellInfo};
use bevy::prelude::*;

const FIRST_BLOOD: u32 = 206416;

pub(crate) fn trigger_dance(
    world: &mut World,
    ctx: &EffectCtx,
    effect: &SpellEffect,
    target: Entity,
) {
    if !super::has_talent(world, ctx.caster, FIRST_BLOOD) {
        crate::spell_cast::queued_triggers::schedule(world, ctx, effect, target);
        return;
    }
    let Some(primary) = ctx.primary_target() else {
        error!(
            spell = ctx.spell.id,
            "First Blood lacks its selected primary"
        );
        return;
    };
    let mut strike = effect.clone();
    strike.trigger_spell = match (ctx.spell.id, effect.index) {
        (188499, 4) => 391378,
        (188499, _) => 391374,
        (210152, 4) => 393054,
        (210152, _) => 393055,
        _ => unreachable!("registered native dance trigger"),
    };
    // Preserve every authored effect64 delay and ordinary child admission.
    crate::spell_cast::queued_triggers::schedule(world, ctx, &strike, primary);
    crate::spell_cast::queued_triggers::schedule(world, ctx, effect, primary);
}

pub(crate) fn filter_dance_targets(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    primary: Option<Entity>,
    units: &mut Vec<Entity>,
) {
    if matches!(spell.id, 199552 | 200685 | 210153 | 210155)
        && super::has_talent(world, caster, FIRST_BLOOD)
    {
        units.retain(|unit| Some(*unit) != primary);
    }
}

pub(crate) fn first_blood_damage(
    world: &World,
    caster: Entity,
    _target: Entity,
    amount: f32,
) -> f32 {
    if !super::has_talent(world, caster, FIRST_BLOOD) {
        return amount;
    }
    let percent = super::super::script_effect_value(world, caster, FIRST_BLOOD, 0);
    amount * (1.0 + percent / 100.0)
}
