//! Native paid-resource cooldown relationships, never inferred from cast counts.
use super::combat;
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterPayment(after_payment)];

use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::CostPool;
use shared::components::{ActiveSpec, PowerType};

struct PaidCooldown {
    spec: u32,
    passive: u32,
    resource: PowerType,
    divisor_effect: u8,
    cooldowns: &'static [u32],
}

const POLICIES: &[PaidCooldown] = &[
    // SimC76f8118 sc_warrior.cpp1450–1455/1506–1517: actual paid Rage
    // divided by native Anger Management effect3 reduces Fury Avatar/Recklessness.
    PaidCooldown {
        spec: 72,
        passive: 152278,
        resource: PowerType::Rage,
        divisor_effect: 2,
        cooldowns: &[107574, 1719],
    },
];

fn after_payment(world: &mut World, caster: Entity, costs: &[(CostPool, f32)]) {
    for policy in POLICIES {
        let paid = combat::spell_cast::spent(costs, policy.resource);
        if paid <= 0.0 {
            continue;
        }
        let Some(seconds) = read_reduction(world, caster, policy, paid) else {
            continue;
        };
        for cooldown in policy.cooldowns {
            combat::spell_cast::scripts::reduce_cooldown(world, caster, *cooldown, seconds);
        }
    }
}

fn read_reduction(
    world: &World,
    caster: Entity,
    policy: &PaidCooldown,
    raw_paid: f32,
) -> Option<f64> {
    let divisor = read_native_divisor(world, caster, policy)?;
    let units = world
        .resource::<combat::spell_cast::powers::PowerTypeTable>()
        .get(policy.resource)
        .expect("native paid-resource display units")
        .display_modifier;
    assert!(
        divisor > 0.0 && units > 0,
        "native cooldown divisors must be positive"
    );
    Some(f64::from(raw_paid) / f64::from(units) / f64::from(divisor))
}

fn read_native_divisor(world: &World, caster: Entity, policy: &PaidCooldown) -> Option<f32> {
    if world.get::<ActiveSpec>(caster)?.0 != policy.spec {
        return None;
    }
    let passive = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == policy.passive && aura.caster == caster)?;
    Some(
        passive
            .effects
            .iter()
            .find(|effect| effect.index == policy.divisor_effect)
            .expect("native paid-resource cooldown divisor")
            .amount,
    )
}
