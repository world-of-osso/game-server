//! TC a352b1fa SpellAuras.cpp486–494/863–899: a finite aura's `ManaPerSecond` and
//! `PowerPctPerSecond` costs, charged to its caster every second from application.
use bevy::prelude::*;
use shared::components::{Health, PowerType};

use super::{ActiveAuras, RemoveReason};
use crate::spell_cast::attributes;
use crate::spell_info::{SpellInfo, SpellPowerCost, SpellStore};

const ATTR2_NO_TARGET_PER_SECOND_COSTS: u32 = 0x0000_0800;
const ATTR6_DO_NOT_CONSUME_RESOURCES: u32 = 0x0000_0020;
const POWER_HEALTH: i32 = -2;

fn per_second(cost: &SpellPowerCost) -> bool {
    cost.mana_per_second != 0 || cost.power_pct_per_second > 0.0
}

/// `m_timeCla`: the first charge falls one second after application. TC charges
/// only while `m_duration > 0`, so a permanent aura never pays.
pub(super) fn first_charge_at(spell: &SpellInfo, now: f64, finite: bool) -> Option<f64> {
    (finite
        && attributes(spell, 6) & ATTR6_DO_NOT_CONSUME_RESOURCES == 0
        && spell.power_costs.iter().any(per_second))
    .then_some(now + 1.0)
}

/// Charge every due second of `unit`'s auras, removing those the caster cannot pay.
pub(super) fn charge_due(world: &mut World, unit: Entity, now: f64) {
    let due: Vec<(u32, Entity, u32)> = world
        .get::<ActiveAuras>(unit)
        .map(|auras| {
            auras
                .iter()
                .filter(|aura| aura.next_cost_at.is_some_and(|at| at <= now))
                .map(|aura| (aura.instance_id, aura.caster, aura.spell_id))
                .collect()
        })
        .unwrap_or_default();
    for (instance_id, caster, spell_id) in due {
        let spell = world
            .resource::<SpellStore>()
            .get(spell_id)
            .cloned()
            .expect("aura with per-second costs is a native spell");
        while advance_due(world, unit, instance_id, now) {
            if !charges_caster(world, unit, caster, &spell) {
                continue;
            }
            if !pay_second(world, caster, &spell) {
                super::remove_instance(world, unit, instance_id, RemoveReason::Cancelled);
                end_channel(world, caster, spell_id);
                break;
            }
        }
    }
}

/// `m_timeCla += 1000`: step this aura's next charge, if one is still due.
fn advance_due(world: &mut World, unit: Entity, instance_id: u32, now: f64) -> bool {
    let Some(mut auras) = world.get_mut::<ActiveAuras>(unit) else {
        return false;
    };
    let Some(aura) = auras.get_mut(instance_id) else {
        return false;
    };
    match aura.next_cost_at {
        Some(at) if at <= now => {
            aura.next_cost_at = Some(at + 1.0);
            true
        }
        _ => false,
    }
}

/// `caster && (caster == GetOwner() || !NO_TARGET_PER_SECOND_COSTS)`.
fn charges_caster(world: &World, unit: Entity, caster: Entity, spell: &SpellInfo) -> bool {
    world.get_entity(caster).is_ok()
        && (caster == unit || attributes(spell, 2) & ATTR2_NO_TARGET_PER_SECOND_COSTS == 0)
}

/// One second of every applicable row, in row order; false when a row is short.
fn pay_second(world: &mut World, caster: Entity, spell: &SpellInfo) -> bool {
    for cost in spell.power_costs.iter().filter(|cost| per_second(cost)) {
        if cost.required_aura_spell_id != 0
            && !super::has_aura(world, caster, cost.required_aura_spell_id)
        {
            continue;
        }
        if !pay_row(world, caster, cost) {
            return false;
        }
    }
    true
}

fn pay_row(world: &mut World, caster: Entity, cost: &SpellPowerCost) -> bool {
    if cost.power_type == POWER_HEALTH {
        let Some(health) = world.get::<Health>(caster) else {
            return false;
        };
        let amount = row_amount(cost, health.max);
        if amount == 0 {
            return true;
        }
        if health.current as i32 <= amount {
            return false;
        }
        world.get_mut::<Health>(caster).unwrap().current -= amount as f32;
        crate::spell_cast::mark_dirty(world, caster);
        return true;
    }
    let Some(power) = PowerType::from_db(cost.power_type) else {
        return false;
    };
    let (current, max) =
        crate::spell_cast::powers::power_value(world, caster, power).unwrap_or((0.0, 0.0));
    let amount = row_amount(cost, max);
    if amount == 0 {
        return true;
    }
    if (current as i32) < amount {
        return false;
    }
    crate::spell_cast::powers::change_power(world, caster, power, -(amount as f32));
    crate::spell_cast::mark_dirty(world, caster);
    true
}

/// `ManaPerSecond + int32(CalculatePct(max, PowerPctPerSecond))`.
fn row_amount(cost: &SpellPowerCost, max: f32) -> i32 {
    cost.mana_per_second + (max * cost.power_pct_per_second / 100.0) as i32
}

/// TC Spell::update: a channel whose aura is gone finishes (`UpdateChanneledTargetList`).
fn end_channel(world: &mut World, caster: Entity, spell_id: u32) {
    let channeling = world
        .get::<shared::casting::CastState>(caster)
        .is_some_and(|cast| {
            cast.spell_id == spell_id && cast.cast_type == shared::casting::CastType::Channel
        });
    if channeling {
        crate::spell_cast::end_cast(world, caster);
    }
}
