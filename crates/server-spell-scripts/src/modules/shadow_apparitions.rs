//! SimC76f8118 sc_priest_shadow.cpp291–308/439–445/450–549/1296/2609–2650.
//! Native projectile parents transport their damage payload; only the source parent bursts.
use super::{combat, shadow_link, shadow_priest};
use bevy::prelude::*;
use combat::auras;
use combat::spell_cast::script_registry::{ContextHitHook, DamageAmountHook, SpellScriptHook};
use combat::spell_cast::{self, effects::EffectCtx};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ContextHit(impact_hook),
    SpellScriptHook::DamageAmount(damage_hook),
    SpellScriptHook::TargetContextDamageMultiplier(shadeburst_scale),
    SpellScriptHook::ConsumedEffectValues(&[
        (VOID_APPARITIONS, 1),
        (SPECTRAL_HORRORS, 0),
        (SHADEBURST_TALENT, 0),
    ]),
];
const MADNESS: u32 = 335467;
const APPARITIONS: u32 = 341491;
const NORMAL_MISSILE: u32 = 148859;
const VOID_MISSILE: u32 = 1264175;
const NORMAL_DAMAGE: u32 = 413231;
const VOID_DAMAGE: u32 = 1264176;
const VOID_APPARITIONS: u32 = 1264104;
const VOID_BOLT: u32 = 1264177;
const SHADEBURST_TALENT: u32 = 73510;
const SHADEBURST: u32 = 1231479;
const SPECTRAL_HORRORS: u32 = 407469;

fn impact_hook(spell: u32) -> Option<ContextHitHook> {
    match spell {
        MADNESS => Some(spawn_apparitions),
        NORMAL_MISSILE | VOID_MISSILE => Some(burst_at_parent),
        _ => None,
    }
}

pub(super) fn spawn_apparitions(world: &mut World, ctx: &EffectCtx<'_>, primary: Entity) {
    if !auras::has_aura(world, ctx.caster, APPARITIONS) {
        return;
    }
    // SimC1296/2609–2650: Madness impact and earned Tormented cohorts share launches.
    for target in shadow_link::read_vampiric_touch_targets(world, ctx.caster) {
        let parent = roll_missile(world, ctx.caster);
        launch_apparition(world, ctx, primary, target, parent);
    }
}

fn roll_missile(world: &World, caster: Entity) -> u32 {
    if !auras::has_aura(world, caster, VOID_APPARITIONS) {
        return NORMAL_MISSILE;
    }
    let chance = spell_cast::scripts::script_effect_value(world, caster, VOID_APPARITIONS, 1);
    if combat::random::random::<f32>() * 100.0 < chance {
        VOID_MISSILE
    } else {
        NORMAL_MISSILE
    }
}

fn launch_apparition(
    world: &mut World,
    source: &EffectCtx<'_>,
    primary: Entity,
    target: Entity,
    parent: u32,
) {
    let depth = source.depth + 1;
    let Some(mut missile) = spell_cast::load_triggered_spell(world, parent, depth) else {
        return;
    };
    // SimC450–549: native E0 owns the damage missile. E1 resource generation
    // is conditional Auspicious Spirits; E2 Shadeburst belongs only to the parent.
    missile.effects.retain(|effect| effect.index == 0);
    missile.cast_primary_target = Some(primary);
    let Some(plan) = plan_apparition(world, source.caster, &missile, target, depth) else {
        return;
    };
    if parent == VOID_MISSILE {
        // SimC543–547/2468: each Void Apparition executes1264177 Void Bolt,
        // not the unrelated373279 area Void Spike.
        spell_cast::cast_triggered(world, source.caster, VOID_BOLT, Some(primary), depth);
    }
    // Keep the original source primary distinct from this missile's recipient.
    spell_cast::launch_planned_spell(world, source.caster, &missile, plan, depth);
}

fn plan_apparition(
    world: &mut World,
    caster: Entity,
    missile: &combat::spell_info::SpellInfo,
    target: Entity,
    depth: u8,
) -> Option<spell_cast::TargetPlan> {
    match spell_cast::plan_triggered_spell(world, caster, missile, Some(target), depth) {
        Ok(plan) => Some(plan),
        Err(reason) => {
            warn!(?reason, "Apparition native recipient refused");
            None
        }
    }
}

fn burst_at_parent(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    let parent_hit = ctx.primary_target() == Some(target);
    if parent_hit && auras::has_aura(world, ctx.caster, SHADEBURST_TALENT) {
        // The native E0 damage child has resolved synchronously before this hit hook.
        spell_cast::cast_triggered(world, ctx.caster, SHADEBURST, Some(target), ctx.depth + 1);
    }
}

fn shadeburst_scale(world: &World, ctx: &EffectCtx<'_>, _: Entity, _: bool) -> f32 {
    if ctx.spell.id != SHADEBURST {
        return 1.0;
    }
    // SimC291–308: native talent E0 supplies the reduced-AoE soft cap.
    let cap = spell_cast::scripts::script_effect_value(world, ctx.caster, SHADEBURST_TALENT, 0);
    let count = spell_cast::resolving_targets(world, ctx)
        .expect("Shadeburst native area selection")
        .len() as f32;
    (cap / count).min(1.0).sqrt()
}

fn damage_hook(spell: u32) -> Option<DamageAmountHook> {
    match spell {
        NORMAL_DAMAGE => Some(spectral_damage),
        VOID_DAMAGE => Some(void_damage),
        _ => None,
    }
}

fn spectral_damage(world: &World, caster: Entity, target: Entity, amount: f32) -> f32 {
    let spectral = auras::has_aura(world, caster, SPECTRAL_HORRORS)
        && shadow_priest::owns_dot(world, caster, target, MADNESS);
    if spectral {
        let percent = spell_cast::scripts::script_effect_value(world, caster, SPECTRAL_HORRORS, 0);
        amount * (1.0 + percent / 100.0)
    } else {
        amount
    }
}

fn void_damage(world: &World, caster: Entity, target: Entity, amount: f32) -> f32 {
    let percent = spell_cast::scripts::script_effect_value(world, caster, VOID_APPARITIONS, 1);
    spectral_damage(world, caster, target, amount) * (1.0 + percent / 100.0)
}
