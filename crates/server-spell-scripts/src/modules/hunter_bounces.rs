//! Missing bounce dispatch; native child records retain damage, crit and armor rules.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::TraitLoadout;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{
    effects::EffectCtx,
    queued_triggers::{self, ScaledTrigger},
};

pub const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EffectHandler(hook)];

struct BouncePolicy {
    parent: u32,
    payload: u32,
    talent: u32,
    dispatch_effect: u8,
    count_effect: u8,
    ramp_talent: u32,
    ramp_effect: u8,
    single_delay_ms: u32,
    multi_delay_ms: u32,
}

// SimC76f8118 sc_hunter.cpp4410–4417/4490–4502: initial hit plus native
// bounce count, round-robin native targets,640ms ST/130ms multi and Radiant Edge.
const POLICIES: &[BouncePolicy] = &[BouncePolicy {
    parent: 1264949,
    payload: 1266081,
    talent: 1264902,
    dispatch_effect: 0,
    count_effect: 1,
    ramp_talent: 1264903,
    ramp_effect: 0,
    single_delay_ms: 640,
    multi_delay_ms: 130,
}];

fn selected(world: &World, caster: Entity, policy: &BouncePolicy) -> bool {
    // SimC76f8118 sc_hunter.cpp7864–7877: the active Sentinel hero
    // talent supplies this same action for both native Hunter specializations.
    has_talent(world, caster, policy.talent)
}

fn has_talent(world: &World, caster: Entity, talent: u32) -> bool {
    world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|spell| spell.spell_id == talent)
    })
}

pub(super) fn hook(spell: u32, index: u8) -> Option<combat::spell_cast::effects::EffectHandler> {
    POLICIES
        .iter()
        .any(|policy| policy.parent == spell && policy.dispatch_effect == index)
        .then_some(schedule_bounces)
}

fn schedule_bounces(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    _effect: &combat::spell_info::SpellEffect,
    target: Entity,
) {
    let Some(policy) = POLICIES
        .iter()
        .find(|policy| policy.parent == ctx.spell.id && selected(world, ctx.caster, policy))
    else {
        return;
    };
    let Some(targets) = combat::spell_cast::resolving_targets(world, ctx).map(<[Entity]>::to_vec)
    else {
        error!(caster=?ctx.caster,spell=ctx.spell.id,"Bounce dispatch lacks native TargetInfos");
        return;
    };
    if targets.first() != Some(&target) {
        return;
    }
    for request in read_impacts(world, ctx.caster, policy, &targets) {
        queued_triggers::schedule_scaled_full_trigger(world, ctx, request);
    }
}

fn read_impacts(
    world: &World,
    caster: Entity,
    policy: &BouncePolicy,
    targets: &[Entity],
) -> Vec<ScaledTrigger> {
    let count = script_effect_value(world, caster, policy.parent, policy.count_effect);
    if !count.is_finite() || !(0.0..=u32::MAX as f32).contains(&count) {
        error!(count, spell = policy.parent, "Native bounce count invalid");
        return Vec::new();
    }
    let ramp = if has_talent(world, caster, policy.ramp_talent) {
        script_effect_value(world, caster, policy.ramp_talent, policy.ramp_effect) / 100.0
    } else {
        0.0
    };
    let spacing = if targets.len() == 1 {
        policy.single_delay_ms
    } else {
        policy.multi_delay_ms
    };
    (0..=count as u32)
        .map(|index| ScaledTrigger {
            target: targets[index as usize % targets.len()],
            spell_id: policy.payload,
            delay_ms: index * spacing,
            multiplier: (1.0 + ramp).powi(index as i32 + 1),
        })
        .collect()
}
