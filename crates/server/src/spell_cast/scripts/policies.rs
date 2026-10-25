//! Explicit missing-script policies; native spell records still own payloads.
use bevy::prelude::*;
use shared::components::{ActiveSpec, PowerType};

use super::AfterCastHook;
use crate::auras::ActiveAuras;
use crate::class_progression::TraitLoadout;
use crate::spell_cast::{CostPool, TargetPlan, effects::EffectCtx, targets};
use crate::spell_info::{CastPolicyContext, SpellInfo, SpellStore};

#[derive(Clone, Copy)]
enum Relationship {
    ResourceOnPrimaryHit {
        parent: u32,
        payload: u32,
    },
    CounterAfterCast {
        parent: u32,
        counter: u32,
        count_effect: u8,
    },
    SpenderHit {
        resource: PowerType,
        counter: u32,
        payload: u32,
    },
}

struct Policy {
    name: &'static str,
    spec: u32,
    talent: u32,
    relationship: Relationship,
}

// Missing edges are explicit SimC policies, never invented DB2/TDB records.
const POLICIES: &[Policy] = &[
    // SimC76f8118 sc_paladin.cpp888–916: each successful primary strike
    // grants the native406834 effect0, independently of the old stack counter.
    Policy {
        name: "SIMC_RET_CRUSADING_STRIKE_HOLY_POWER",
        spec: 70,
        talent: 404542,
        relationship: Relationship::ResourceOnPrimaryHit {
            parent: 408385,
            payload: 406834,
        },
    },
    // sc_paladin_retribution.cpp846–876; native raw2 + specialization1 wins.
    Policy {
        name: "SIMC_RET_DAWNLIGHT_WAKE_PRODUCER",
        spec: 70,
        talent: 431377,
        relationship: Relationship::CounterAfterCast {
            parent: 255937,
            counter: 431522,
            count_effect: 0,
        },
    },
    // sc_paladin.hpp1512–1535: successful foreground first hit, one use.
    Policy {
        name: "SIMC_RET_DAWNLIGHT_SPENDER_CONSUMER",
        spec: 70,
        talent: 431377,
        relationship: Relationship::SpenderHit {
            resource: PowerType::HolyPower,
            counter: 431522,
            payload: 431380,
        },
    },
];

fn hit_resources(parent: u32) -> impl Iterator<Item = &'static Policy> {
    POLICIES.iter().filter(move |policy| {
        matches!(policy.relationship,
            Relationship::ResourceOnPrimaryHit { parent: id, .. } if id == parent)
    })
}

pub(super) fn hit_hook(parent: u32) -> Option<super::ContextHitHook> {
    hit_resources(parent)
        .next()
        .map(|_| grant_hit_resource as super::ContextHitHook)
}

fn grant_hit_resource(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    if super::super::resolving_targets(world, ctx).and_then(|targets| targets.first())
        != Some(&target)
    {
        return;
    }
    for policy in hit_resources(ctx.spell.id) {
        if !selected(world, ctx.caster, policy) {
            continue;
        }
        let Relationship::ResourceOnPrimaryHit { payload, .. } = policy.relationship else {
            unreachable!("resource-hit policy iterator");
        };
        super::super::cast_triggered(world, ctx.caster, payload, Some(ctx.caster), ctx.depth + 1);
    }
}

fn producers(parent: u32) -> impl Iterator<Item = &'static Policy> {
    POLICIES.iter().filter(move |policy| {
        matches!(policy.relationship,
        Relationship::CounterAfterCast { parent: trigger, .. } if trigger == parent)
    })
}

fn consumers(powers: &[PowerType]) -> impl Iterator<Item = &'static Policy> + '_ {
    POLICIES.iter().filter(move |policy| {
        matches!(policy.relationship,
        Relationship::SpenderHit { resource, .. } if powers.contains(&resource))
    })
}

pub(super) fn after_cast_hook(parent: u32) -> Option<AfterCastHook> {
    producers(parent)
        .next()
        .map(|_| apply_after_cast as AfterCastHook)
}

fn selected(world: &World, caster: Entity, policy: &Policy) -> bool {
    let correct_spec = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == policy.spec);
    let owns_talent = world.get::<TraitLoadout>(caster).is_some_and(|talents| {
        talents
            .active_talent_spells()
            .any(|spell| spell.spell_id == policy.talent)
    });
    correct_spec && owns_talent
}

fn apply_after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for policy in producers(spell.id) {
        if selected(world, caster, policy) {
            grant_counter(world, caster, policy);
        }
    }
}

fn read_counter_amount(world: &World, caster: Entity, policy: &Policy, effect: u8) -> Option<u8> {
    let count = super::script_effect_value(world, caster, policy.talent, effect);
    if !count.is_finite() || !(1.0..=f32::from(u8::MAX)).contains(&count) {
        warn!(
            policy = policy.name,
            ?caster,
            count,
            "Native policy counter amount invalid"
        );
        return None;
    }
    Some(count as u8)
}

fn grant_counter(world: &mut World, caster: Entity, policy: &Policy) {
    let Relationship::CounterAfterCast {
        counter,
        count_effect,
        ..
    } = policy.relationship
    else {
        return;
    };
    let Some(count) = read_counter_amount(world, caster, policy, count_effect) else {
        return;
    };
    launch_counter(world, caster, policy, counter, count);
}

fn launch_counter(world: &mut World, caster: Entity, policy: &Policy, counter: u32, count: u8) {
    let Some(mut spell) = world.resource::<SpellStore>().get(counter).cloned() else {
        warn!(
            policy = policy.name,
            spell_id = counter,
            "Native policy counter unavailable"
        );
        return;
    };
    spell.cast_aura_stacks = Some(count);
    if !super::super::launch_triggered_spell(world, caster, &spell, Some(caster), 1) {
        warn!(
            policy = policy.name,
            ?caster,
            "Native policy counter refused"
        );
    }
}

/// Only ordinary successful launch snapshots enter this path; FULL_MASK does not.
/// Retain applicable pools even at zero final cost, and original selected targets.
pub(in crate::spell_cast) fn snapshot(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    plan: &TargetPlan,
    costs: &[(CostPool, f32)],
) -> Option<Box<CastPolicyContext>> {
    let powers = costs
        .iter()
        .filter_map(|(pool, _)| match pool {
            CostPool::Power(power) => Some(*power),
            CostPool::Health => None,
        })
        .collect::<Vec<_>>();
    if spell.cast_parent_owns_modifier_charges
        || !consumers(&powers).any(|policy| selected(world, caster, policy))
    {
        return None;
    }
    let targets = capture_primary_targets(caster, spell, plan)?;
    Some(Box::new(CastPolicyContext { powers, targets }))
}

fn capture_primary_targets(
    caster: Entity,
    spell: &SpellInfo,
    plan: &TargetPlan,
) -> Option<Vec<Entity>> {
    let Some(primary) = spell.cast_primary_target else {
        warn!(
            ?caster,
            spell = spell.id,
            "NATIVE_POLICY_PRIMARY_UNAVAILABLE: no request context"
        );
        return None;
    };
    let selected = super::super::selected_units(plan);
    if !selected.contains(&primary) {
        warn!(
            ?caster,
            spell = spell.id,
            ?primary,
            "NATIVE_POLICY_PRIMARY_UNAVAILABLE: not selected"
        );
        return None;
    }
    Some(
        std::iter::once(primary)
            .chain(selected.into_iter().filter(|&target| target != primary))
            .collect(),
    )
}

pub(in crate::spell_cast) fn on_hit(world: &mut World, ctx: &EffectCtx<'_>, hit: Entity) {
    let Some(context) = &ctx.spell.cast_policy_context else {
        return;
    };
    if context.targets.first() != Some(&hit) {
        return;
    }
    for policy in consumers(&context.powers) {
        if selected(world, ctx.caster, policy) {
            consume_on_hit(world, ctx.caster, policy, context);
        }
    }
}

fn read_counter_instance(world: &World, caster: Entity, counter: u32) -> Option<u32> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == counter && aura.caster == caster && aura.stacks > 0)
        .map(|aura| aura.instance_id)
}

fn consume_on_hit(world: &mut World, caster: Entity, policy: &Policy, context: &CastPolicyContext) {
    let Relationship::SpenderHit {
        counter, payload, ..
    } = policy.relationship
    else {
        return;
    };
    let Some(instance) = read_counter_instance(world, caster, counter) else {
        return;
    };
    let Some(targets) = read_payload_targets(world, caster, policy, payload, &context.targets)
    else {
        return;
    };
    let Some(target) = choose_target(world, caster, payload, &targets) else {
        return;
    };
    if !super::super::try_cast_triggered(world, caster, payload, Some(target), 1) {
        warn!(
            policy = policy.name,
            ?caster,
            ?target,
            "Native policy payload refused"
        );
        return;
    }
    crate::auras::spend_stack(world, caster, instance);
}

fn read_payload_targets(
    world: &World,
    caster: Entity,
    policy: &Policy,
    payload: u32,
    selected: &[Entity],
) -> Option<Vec<Entity>> {
    let Some(spell) = world.resource::<SpellStore>().get(payload) else {
        warn!(
            policy = policy.name,
            payload, "Native policy payload unavailable"
        );
        return None;
    };
    let relation = targets::explicit_requirement(spell.effects.iter());
    let accepts = |&unit: &Entity| {
        relation.is_none_or(|relation| targets::relation_matches(world, caster, unit, relation))
            && (spell.allows_dead_target() || targets::is_alive(world, unit))
    };
    let primary = selected.first()?;
    if !accepts(primary) {
        warn!(
            policy = policy.name,
            ?primary,
            "NATIVE_POLICY_PRIMARY_REFUSED: no target substitute"
        );
        return None;
    }
    Some(
        selected
            .iter()
            .filter(|unit| accepts(unit))
            .copied()
            .collect(),
    )
}

fn owns_periodic(world: &World, caster: Entity, target: Entity, payload: u32) -> bool {
    world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.caster == caster
                && aura.spell_id == payload
                && aura.effects.iter().any(|effect| effect.period_ms > 0)
        })
    })
}

fn choose_target(
    world: &World,
    caster: Entity,
    payload: u32,
    targets: &[Entity],
) -> Option<Entity> {
    let primary = *targets.first()?;
    if targets.len() == 1 || !owns_periodic(world, caster, primary, payload) {
        return Some(primary);
    }
    targets
        .iter()
        .copied()
        .find(|&target| !owns_periodic(world, caster, target, payload))
}
