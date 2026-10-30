//! Missing bounce dispatch; native child records retain damage, crit and armor rules.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::TraitLoadout;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{
    effects::EffectCtx,
    queued_triggers::{self, ScaledTrigger},
    targets,
};

pub const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EffectHandler(hook),
    SpellScriptHook::DeclaredReach(declared_reach),
];

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
    finisher_talent: u32,
    finisher: u32,
}

// SimC76f8118 sc_hunter.cpp4410–4417/4490–4502: initial hit plus native
// bounce count, round-robin native targets,640ms ST/130ms multi and Radiant Edge.
// sc_hunter.cpp4427–4434/4504–4507: selected Twilight Requiem executes its
// native area payload around the actual primary with the final bounce.
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
    finisher_talent: 1264904,
    finisher: 1266096,
}];

fn declared_reach(
    world: &World,
    caster: Entity,
    spell: &combat::spell_info::SpellInfo,
) -> combat::spell_cast::script_registry::DeclaredReach {
    let Some(policy) = POLICIES.iter().find(|policy| policy.parent == spell.id) else {
        return Default::default();
    };
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == policy.dispatch_effect)
        .expect("native bounce dispatch effect");
    let (jumps, jump_radius) = targets::chain_jump_parameters(world, caster, spell, effect);
    let finisher = world
        .resource::<combat::spell_info::SpellStore>()
        .get(policy.finisher)
        .expect("native bounce finisher");
    let payload_radius = finisher
        .effects
        .iter()
        .map(targets::effect_radius)
        .fold(0.0, f32::max);
    combat::spell_cast::script_registry::DeclaredReach {
        caster_radius: 0.0,
        target_radius: jumps as f32 * jump_radius + payload_radius,
    }
}

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
    effect: &combat::spell_info::SpellEffect,
    target: Entity,
) {
    let Some(policy) = POLICIES
        .iter()
        .find(|policy| policy.parent == ctx.spell.id && selected(world, ctx.caster, policy))
    else {
        return;
    };
    // SimC76f8118 sc_hunter.cpp4487–4503: one sequence per execute, at the
    // actual primary. The native missile lands each chained unit separately.
    if ctx.primary_target() != Some(target) {
        return;
    }
    let targets: Vec<Entity> = std::iter::once(target)
        .chain(targets::chain_targets(
            world,
            ctx.caster,
            ctx.spell,
            effect,
            target,
            targets::Relation::Enemy,
        ))
        .collect();
    // Native1280140 modifies bounce label5870 through shared spell mods;
    // it only needs to live until the final scheduled bounce (C++4492–4498).
    let tipped = super::survival::consume_chakram_tip(world, ctx, target);
    let impacts = read_impacts(world, ctx.caster, policy, &targets);
    let last_delay = impacts.last().map(|impact| impact.delay_ms);
    for request in impacts {
        queued_triggers::schedule_scaled_full_trigger(world, ctx, request);
    }
    if has_talent(world, ctx.caster, policy.finisher_talent)
        && let Some(delay) = last_delay
    {
        queued_triggers::schedule_full_trigger(world, ctx, target, policy.finisher, delay);
    }
    if tipped && let Some(delay) = last_delay {
        queued_triggers::schedule_callback(world, ctx, ctx.caster, delay, expire_chakram_tip);
    }
}

fn expire_chakram_tip(world: &mut World, caster: Entity, _target: Entity, _depth: u8) {
    combat::auras::remove_auras(
        world,
        caster,
        combat::auras::RemoveReason::Consumed,
        |aura| aura.caster == caster && aura.spell_id == 1280140,
    );
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
        let native = script_effect_value(world, caster, policy.ramp_talent, policy.ramp_effect);
        super::survival::radiant_edge_percent(world, caster, native) / 100.0
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
