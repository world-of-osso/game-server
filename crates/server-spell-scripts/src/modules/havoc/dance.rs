//! SimC76f8118 sc_demon_hunter.cpp7040–7110,7252–7275,7324–7385.
use super::combat;
use bevy::prelude::*;
use combat::spell_info::{SpellEffect, SpellInfo};
use combat::spell_cast::effects::{EffectCtx, EffectHandler};
use combat::spell_cast::script_registry::{DamageAmountHook, SpellScriptHook};
use super::havoc::has_talent;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EffectHandler(effect_handler),
    SpellScriptHook::TargetFilter(filter_targets),
    SpellScriptHook::DamageAmount(damage_amount),
];

const FIRST_BLOOD: u32 = 206416;

pub(super) fn effect_handler(spell: u32, index: u8) -> Option<EffectHandler> {
    (matches!(spell, 188499 | 210152) && (1..=4).contains(&index)).then_some(trigger_dance)
}

fn trigger_dance(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    if !has_talent(world, ctx.caster, FIRST_BLOOD) {
        combat::spell_cast::queued_triggers::schedule(world, ctx, effect, target);
        return;
    }
    let Some(primary) = ctx.primary_target() else {
        error!(spell = ctx.spell.id, "First Blood lacks its selected primary");
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
    combat::spell_cast::queued_triggers::schedule(world, ctx, &strike, primary);
    combat::spell_cast::queued_triggers::schedule(world, ctx, effect, primary);
}

pub(super) fn filter_targets(world: &World, caster: Entity, spell: &SpellInfo, _effect: u8, primary: Option<Entity>, units: &mut Vec<Entity>) {
    if matches!(spell.id, 199552 | 200685 | 210153 | 210155)
        && has_talent(world, caster, FIRST_BLOOD)
    {
        units.retain(|unit| Some(*unit) != primary);
    }
}

pub(super) fn damage_amount(spell: u32) -> Option<DamageAmountHook> {
    matches!(spell, 391374 | 391378 | 393055 | 393054).then_some(first_blood_damage)
}

fn first_blood_damage(world: &World, caster: Entity, _target: Entity, amount: f32) -> f32 {
    if !has_talent(world, caster, FIRST_BLOOD) {
        return amount;
    }
    let percent = combat::spell_cast::scripts::script_effect_value(world, caster, FIRST_BLOOD, 0);
    amount * (1.0 + percent / 100.0)
}
