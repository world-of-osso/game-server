//! SimC76f8118 sc_druid.cpp4058–4165: Frenzy parents execute native tick children.
use bevy::prelude::*;

use super::combat;
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::auras::{AuraInstance, RemoveReason};
use combat::spell_cast::script_registry::{ContextHitHook, SpellScriptHook};
use combat::spell_cast::{self, effects::EffectCtx, queued_triggers};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::ContextHit(context_hit),
];

fn context_hit(spell: u32) -> Option<ContextHitHook> {
    match spell {
        FERAL | FRANTIC | HEART_CAT => Some(begin),
        FRANTIC_CHILD => Some(energize_frantic),
        _ => None,
    }
}

pub(super) const FERAL: u32 = 274837;
pub(super) const FRANTIC: u32 = 1243807;
pub(super) const FRANTIC_CHILD: u32 = 1244079;
const FERAL_CHILD: u32 = 274838;
const HEART_CAT: u32 = 1261868;
const HEART_TALENT: u32 = 1261867;
const FRANTIC_ENERGIZE: u32 = 1278969;
// SimC4060–4102 explicitly supplies this script-only interval for Frantic Frenzy.
const FRANTIC_INTERVAL_MS: u32 = 200;

#[derive(Component)]
struct FrenzyTarget(Entity);

pub(crate) fn register(scripts: &mut AuraScripts) {
    for spell in [FERAL, HEART_CAT] {
        scripts.0.insert(
            spell,
            AuraScript {
                on_periodic: Some(feral_tick),
                on_remove: Some(clear_target),
                ..Default::default()
            },
        );
    }
}

pub(super) fn begin(world: &mut World, ctx: &EffectCtx, target: Entity) {
    if ctx.spell.cast_primary_target != Some(target) {
        return;
    }
    if matches!(ctx.spell.id, FERAL | HEART_CAT) {
        world.entity_mut(ctx.caster).insert(FrenzyTarget(target));
        return;
    }
    let count = ctx
        .spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Frantic Frenzy retains its strike count");
    let count = spell_cast::effects::modded_base_points(world, ctx.caster, ctx.spell, count) as u32;
    for index in 1..=count {
        queued_triggers::schedule_full_trigger(
            world,
            ctx,
            target,
            FRANTIC_CHILD,
            index * FRANTIC_INTERVAL_MS,
        );
    }
}

fn feral_tick(world: &mut World, unit: Entity, aura: &AuraInstance, effect_index: u8) {
    if effect_index != 1 {
        return;
    }
    let Some(target) = world.get::<FrenzyTarget>(unit).map(|state| state.0) else {
        return;
    };
    // SimC76f8118 sc_druid.cpp11754–11765: Heart's Cat override uses the
    // Feral child, scaled by the native Heart talent's second effect.
    let damage_multiplier = (aura.spell_id == HEART_CAT).then(|| {
        spell_cast::scripts::script_effect_value(world, aura.caster, HEART_TALENT, 1) / 100.0
    });
    spell_cast::cast_triggered_with_overrides(
        world,
        aura.caster,
        FERAL_CHILD,
        Some(target),
        1,
        spell_cast::TriggeredSpellOverrides {
            damage_multiplier,
            ..Default::default()
        },
    );
}

fn clear_target(world: &mut World, unit: Entity, _: &AuraInstance, _: RemoveReason) {
    world.entity_mut(unit).remove::<FrenzyTarget>();
}

pub(super) fn energize_frantic(world: &mut World, ctx: &EffectCtx, target: Entity) {
    if ctx.spell.cast_primary_target == Some(target) {
        // SimC4100–4104: one native energize per strike, not per AoE recipient.
        spell_cast::cast_triggered(
            world,
            ctx.caster,
            FRANTIC_ENERGIZE,
            Some(ctx.caster),
            ctx.depth + 1,
        );
    }
}
