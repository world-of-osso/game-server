//! Source-qualified landed-impact PRD relationships; native effects own probabilities.
use bevy::prelude::*;
use shared::components::ActiveSpec;
use std::collections::BTreeMap;

use crate::auras::{ActiveAuras, procs};
use crate::spell_cast::effects::EffectCtx;

struct ImpactProc {
    passive: u32,
    payload: u32,
    chance_effects: &'static [(u32, u8)],
    primary_actions: &'static [u32],
    per_target_actions: &'static [u32],
}

const PROCS: &[ImpactProc] = &[
    // SimC76f8118 sc_warrior.cpp1651–1661/1693–1703: shared unit-wide
    // PRD attempts, native per-spec chance and ICD, selected primary unless Cleave.
    ImpactProc {
        passive: 444767,
        payload: 445579,
        chance_effects: &[(71, 1), (72, 0)],
        // Eligibility:2584/2617,3036,3466,4286,4438,4925,5092,5438,5724.
        primary_actions: &[
            23881, 335096, 12294, 1464, 260798, 280849, 96103, 85384, 335098, 335100, 7384, 201363,
        ],
        per_target_actions: &[845],
    },
];

#[derive(Component, Default)]
struct ImpactProcAttempts(BTreeMap<u32, u32>);

pub(super) fn after_damage(world: &mut World, ctx: &EffectCtx, target: Entity) {
    for policy in PROCS {
        let primary = ctx.spell.cast_primary_target == Some(target);
        let eligible = policy.per_target_actions.contains(&ctx.spell.id)
            || (primary && policy.primary_actions.contains(&ctx.spell.id));
        if !eligible {
            continue;
        }
        roll_impact_proc(world, ctx, target, policy);
    }
}

fn roll_impact_proc(world: &mut World, ctx: &EffectCtx, target: Entity, policy: &ImpactProc) {
    let Some((instance, probability)) = read_native_probability(world, ctx.caster, policy) else {
        return;
    };
    let attempts = advance_attempts(world, ctx.caster, policy.passive);
    let constant = crate::random::accumulated_proc_constant(probability);
    let chance = (constant * f64::from(attempts) * 100.0).min(100.0) as f32;
    if procs::try_policy_proc(world, ctx.caster, instance, chance) {
        cast_impact_proc(world, ctx, target, policy);
    }
}

fn cast_impact_proc(world: &mut World, ctx: &EffectCtx, target: Entity, policy: &ImpactProc) {
    world
        .get_mut::<ImpactProcAttempts>(ctx.caster)
        .unwrap()
        .0
        .insert(policy.passive, 0);
    if !crate::spell_cast::try_cast_triggered(
        world,
        ctx.caster,
        policy.payload,
        Some(target),
        ctx.depth + 1,
    ) {
        warn!(caster = ?ctx.caster, payload = policy.payload, "Native impact proc refused");
    }
}

fn read_native_probability(
    world: &World,
    caster: Entity,
    policy: &ImpactProc,
) -> Option<(u32, f64)> {
    let spec = world.get::<ActiveSpec>(caster)?.0;
    let (_, index) = policy
        .chance_effects
        .iter()
        .find(|(allowed, _)| *allowed == spec)?;
    let now = crate::spell_cast::now(world);
    let aura = world.get::<ActiveAuras>(caster)?.iter().find(|aura| {
        aura.spell_id == policy.passive && aura.caster == caster && now >= aura.proc.next_ready_at
    })?;
    let chance = aura
        .effects
        .iter()
        .find(|effect| effect.index == *index)
        .expect("native impact proc chance effect")
        .amount;
    Some((aura.instance_id, f64::from(chance) / 100.0))
}

fn advance_attempts(world: &mut World, caster: Entity, passive: u32) -> u32 {
    let mut entity = world.entity_mut(caster);
    let mut attempts = entity.entry::<ImpactProcAttempts>().or_default().into_mut();
    let count = attempts.0.entry(passive).or_default();
    *count += 1;
    *count
}
