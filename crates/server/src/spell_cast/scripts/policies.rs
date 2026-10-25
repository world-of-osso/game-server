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

fn grant_counter(world: &mut World, caster: Entity, policy: &Policy) {
    let Relationship::CounterAfterCast {
        counter,
        count_effect,
        ..
    } = policy.relationship
    else {
        return;
    };
    let count = super::script_effect_value(world, caster, policy.talent, count_effect);
    if !count.is_finite() || !(1.0..=f32::from(u8::MAX)).contains(&count) {
        warn!(
            policy = policy.name,
            ?caster,
            count,
            "Native policy counter amount invalid"
        );
        return;
    }
    let Some(mut spell) = world.resource::<SpellStore>().get(counter).cloned() else {
        warn!(
            policy = policy.name,
            spell_id = counter,
            "Native policy counter unavailable"
        );
        return;
    };
    spell.cast_aura_stacks = Some(count as u8);
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
    let targets = std::iter::once(primary)
        .chain(selected.into_iter().filter(|&target| target != primary))
        .collect();
    Some(Box::new(CastPolicyContext { powers, targets }))
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

fn consume_on_hit(world: &mut World, caster: Entity, policy: &Policy, context: &CastPolicyContext) {
    let Relationship::SpenderHit {
        counter, payload, ..
    } = policy.relationship
    else {
        return;
    };
    let Some(instance) = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == counter && aura.caster == caster && aura.stacks > 0)
            .map(|aura| aura.instance_id)
    }) else {
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
