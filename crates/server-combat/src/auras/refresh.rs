//! Native rolling-periodic carry and sourced reapplication duration/absorb rules.
use bevy::prelude::*;
use shared::components::{ActiveSpec, Health};

use super::{ActiveAuras, AuraInstance, aura_now, periodic};
use crate::spell_info::SpellInfo;

const ROLLING_PERIODIC: u32 = 0x4000;
const PERIODIC_REFRESH_EXTENDS_DURATION: u32 = 0x0010_0000;
const AURA_DOES_NOT_REFRESH: u32 = 0x200;
const IGNORE_PAIN: u32 = 190456;
const PROTECTION_WARRIOR: u32 = 73;
const SHIELD_BLOCK: u32 = 132404;
const SHIELD_OF_THE_RIGHTEOUS: u32 = 132403;
const HOLY_BULWARK_ABSORB: u32 = 432607;

#[derive(Clone, Copy)]
enum DurationCarry {
    Pandemic,
    Extend,
    CappedExtend(u32),
}

fn has_native_pandemic(spell: &SpellInfo) -> bool {
    spell
        .misc
        .as_ref()
        .is_some_and(|misc| misc.attributes[13] & PERIODIC_REFRESH_EXTENDS_DURATION != 0)
}

pub(super) fn prepare_application(
    world: &mut World,
    target: Entity,
    new: &mut AuraInstance,
    spell: &SpellInfo,
    forced_rolling: bool,
) {
    let fresh_duration_ms = new.duration_ms;
    let sourced_rolling =
        crate::spell_cast::scripts::rolls_reapplication(world, new.caster, new.spell_id);
    let rolling = forced_rolling
        || sourced_rolling
        || spell
            .misc
            .as_ref()
            .is_some_and(|misc| misc.attributes[10] & ROLLING_PERIODIC != 0);
    if rolling {
        settle_previous_ticks(world, target, new);
    }
    prepare_periodic_phase(world, target, new, spell, sourced_rolling || forced_rolling);
    if crate::spell_cast::scripts::extends_reapplication(world, new.caster, new.spell_id) {
        carry_duration(world, target, new, DurationCarry::Extend);
    } else if has_native_pandemic(spell) && !rolling {
        // SimC76f8118 action.cpp684–685/4612: the native refresh flag
        // selects Pandemic, not a spell-ID whitelist. Explicit Extend and
        // Rolling retain their source precedence (action.cpp2729–2734).
        carry_duration(world, target, new, DurationCarry::Pandemic);
    } else if !rolling && has_periodic_health_effect(new) {
        carry_next_tick_duration(world, target, new);
    }
    match new.spell_id {
        IGNORE_PAIN => accumulate_ignore_pain(world, target, new),
        HOLY_BULWARK_ABSORB => accumulate_holy_bulwark(world, target, new),
        77535 => accumulate_blood_shield(world, target, new),
        372505 => accumulate_ursocs_fury(world, target, new),
        SHIELD_BLOCK => carry_duration(world, target, new, DurationCarry::Extend),
        SHIELD_OF_THE_RIGHTEOUS => {
            carry_duration(world, target, new, DurationCarry::CappedExtend(3));
        }
        _ => {}
    }
    preserve_expiry(world, target, new, spell);
    if rolling {
        // SimC76f8118 action.cpp2730–2734/4995–5015 uses the same
        // next-tick schedule and remaining-plus-fresh budget for both.
        if forced_rolling {
            carry_duration(world, target, new, DurationCarry::Pandemic);
        } else {
            carry_rolling_duration(world, target, new);
        }
        roll_remaining_periodic(world, target, new, fresh_duration_ms);
    }
    preserve_counter_only_stacks(world, target, new, spell);
    super::enhancement_power::preserve_refresh_duration(world, target, new);
    super::ret_crusade::preserve_refresh_duration(world, target, new);
}

pub(super) fn set_rolling_deposit(
    aura: &mut AuraInstance,
    duration_ms: u32,
    deposit: f32,
    now: f64,
) {
    let mut periodic = aura
        .effects
        .iter_mut()
        .filter(|effect| effect.period_ms > 0);
    let effect = periodic
        .next()
        .expect("rolling deposit requires a periodic effect");
    assert!(
        periodic.next().is_none(),
        "rolling deposit requires exactly one periodic effect"
    );
    effect.amount = deposit * effect.period_ms as f32 / duration_ms as f32;
    effect.periodic_power_multiplier = 0.0;
    aura.duration_ms = duration_ms;
    aura.expires_at = Some(now + f64::from(duration_ms) / 1000.0);
}

fn has_periodic_health_effect(aura: &AuraInstance) -> bool {
    aura.effects.iter().any(|effect| {
        effect.period_ms > 0
            && matches!(
                effect.aura_type,
                super::handlers::PERIODIC_DAMAGE
                    | super::handlers::PERIODIC_DAMAGE_PERCENT
                    | super::handlers::PERIODIC_LEECH
                    | super::handlers::PERIODIC_HEAL
            )
    })
}

fn carry_next_tick_duration(world: &World, target: Entity, new: &mut AuraInstance) {
    let Some(old) = previous(world, target, new) else {
        return;
    };
    let now = aura_now(world);
    if old.expires_at.is_some_and(|expiry| expiry <= now) {
        return;
    }
    let Some(effect) = old.effects.iter().find(|effect| {
        effect.period_ms > 0
            && matches!(
                effect.aura_type,
                super::handlers::PERIODIC_DAMAGE
                    | super::handlers::PERIODIC_DAMAGE_PERCENT
                    | super::handlers::PERIODIC_LEECH
                    | super::handlers::PERIODIC_HEAL
            )
    }) else {
        return;
    };
    // SimC76f8118 action.cpp419/4618–4619: ordinary DoT refresh adds
    // the time to the next full tick to the newly triggered duration.
    let until_tick = (effect.next_tick_at() - now).max(0.0);
    let duration = f64::from(new.duration_ms) / 1000.0 + until_tick;
    new.duration_ms = (duration * 1000.0).round() as u32;
    new.expires_at = Some(now + duration);
}

fn preserve_counter_only_stacks(
    world: &World,
    target: Entity,
    new: &mut AuraInstance,
    spell: &SpellInfo,
) {
    let counter_only = world
        .resource::<crate::spell_cast::script_registry::SpellScriptRegistry>()
        .counter_only_dot_stacks(spell.id);
    if counter_only && spell.cast_aura_stacks.is_none() && previous(world, target, new).is_some() {
        // Ordinary refresh contributes no counter stacks; explicit native stack
        // applications still advance the counter through the normal merge.
        new.stacks = 0;
    }
}

fn preserve_expiry(world: &World, target: Entity, new: &mut AuraInstance, spell: &SpellInfo) {
    // SimC76f8118 buff.cpp731–743: native Attr15 bit9 disables duration
    // refresh, but not stack advancement. Native Pandemic takes precedence.
    let disabled = spell.misc.as_ref().is_some_and(|misc| {
        misc.attributes[15] & AURA_DOES_NOT_REFRESH != 0
            && misc.attributes[13] & PERIODIC_REFRESH_EXTENDS_DURATION == 0
    });
    if disabled
        && let Some(old) = previous(world, target, new)
        && old.expires_at.is_none_or(|expiry| expiry > aura_now(world))
    {
        new.expires_at = old.expires_at;
    }
}

fn previous(world: &World, target: Entity, new: &AuraInstance) -> Option<AuraInstance> {
    world
        .get::<ActiveAuras>(target)?
        .iter()
        .find(|old| {
            old.spell_id == new.spell_id && old.caster == new.caster && old.source == new.source
        })
        .cloned()
}

/// SimC76f8118: action.cpp4612 Pandemic; sc_warrior.cpp7196 Extend;
/// sc_paladin.cpp2870 caps SoTR at three freshly calculated native durations.
/// Snapshot amounts and existing periodic phase remain owned by refresh_from.
fn carry_duration(world: &World, target: Entity, new: &mut AuraInstance, rule: DurationCarry) {
    let Some(old) = previous(world, target, new) else {
        return;
    };
    let Some((old_expiry, _)) = old.expires_at.zip(new.expires_at) else {
        return;
    };
    let now = aura_now(world);
    let remaining = (old_expiry - now).max(0.0);
    let duration = f64::from(new.duration_ms) / 1_000.0;
    let refreshed = match rule {
        DurationCarry::Pandemic => remaining.max(duration + remaining.min(duration * 0.30)),
        DurationCarry::Extend => duration + remaining,
        DurationCarry::CappedExtend(count) => {
            (duration + remaining).min(duration * f64::from(count))
        }
    };
    new.duration_ms = (refreshed * 1_000.0).round() as u32;
    new.expires_at = Some(now + refreshed);
}

/// Count scheduled ticks, including a tick exactly at expiry, without altering phase.
pub(super) fn remaining_ticks(next: f64, expiry: f64, period_ms: u32) -> u32 {
    if period_ms == 0 || next > expiry + 1e-6 {
        return 0;
    }
    (((expiry - next) / (f64::from(period_ms) / 1000.0) + 1e-6).floor() as u32) + 1
}

/// Settle due ticks before reading the unpaid pool, so refresh cannot pay them twice.
fn settle_previous_ticks(world: &mut World, target: Entity, new: &AuraInstance) {
    if let Some(old) = previous(world, target, new) {
        periodic::run_ticks_until(world, target, old.instance_id, aura_now(world));
    }
}

/// SimC76f8118 action.cpp419/4618–4619 retains health-periodic phase on
/// refresh. Native full-mask, stacking, Pandemic and sourced policies also
/// preserve phase for other periodic effects.
fn prepare_periodic_phase(
    world: &World,
    target: Entity,
    new: &mut AuraInstance,
    spell: &SpellInfo,
    sourced_rolling: bool,
) {
    let stacking = spell
        .aura_options
        .as_ref()
        .is_some_and(|options| options.cumulative_aura >= 2);
    let pandemic = has_native_pandemic(spell);
    let native_preserves_phase = spell.cast_dont_reset_periodic_timer || stacking || pandemic;
    let scripted_phase = world
        .resource::<crate::spell_cast::script_registry::SpellScriptRegistry>()
        .preserves_periodic_phase(spell.id);
    let preserve_phase = native_preserves_phase
        || sourced_rolling
        || scripted_phase
        || has_periodic_health_effect(new);
    if !preserve_phase {
        return;
    }
    let Some(old) = previous(world, target, new) else {
        return;
    };
    for fresh in &mut new.effects {
        if let Some(prior) = old
            .effects
            .iter()
            .find(|effect| effect.index == fresh.index)
        {
            fresh.periodic_started_at = prior.next_tick_at();
            fresh.next_tick_index = 0;
            // A carried future tick is scheduled, not a new tick_zero application.
            fresh.initial_period_pending = false;
        }
    }
}

/// SimC76f8118 action.cpp4614–4617: a rolling refresh adds the pending
/// full tick's remaining time to the fresh duration, unless only a partial tick remains.
fn carry_rolling_duration(world: &World, target: Entity, new: &mut AuraInstance) {
    let Some(old) = previous(world, target, new) else {
        return;
    };
    let Some(expiry) = old.expires_at else {
        return;
    };
    let Some(periodic) = old
        .effects
        .iter()
        .find(|effect| remaining_ticks(effect.next_tick_at(), expiry, effect.period_ms) > 0)
    else {
        return;
    };
    let now = aura_now(world);
    let pending_ms = ((periodic.next_tick_at() - now).max(0.0) * 1000.0).round() as u32;
    new.duration_ms += pending_ms;
    new.expires_at = Some(now + f64::from(new.duration_ms) / 1000.0);
}

/// SimC76f8118 action.cpp4995–5015 rolls both the fractional unpaid
/// duration and the fresh duration across the new fractional tick schedule.
fn roll_remaining_periodic(
    world: &World,
    target: Entity,
    new: &mut AuraInstance,
    fresh_duration_ms: u32,
) {
    let Some(old) = previous(world, target, new) else {
        return;
    };
    let Some((old_expiry, new_expiry)) = old.expires_at.zip(new.expires_at) else {
        return;
    };
    for fresh in new.effects.iter_mut().filter(|effect| effect.period_ms > 0) {
        let Some(prior) = old
            .effects
            .iter()
            .find(|effect| effect.index == fresh.index && effect.aura_type == fresh.aura_type)
        else {
            continue;
        };
        if prior.period_ms == 0 {
            continue;
        }
        let unpaid = (1.0
            + (old_expiry - prior.next_tick_at()) * 1000.0 / f64::from(prior.period_ms))
        .max(0.0);
        let future = (1.0
            + (new_expiry - fresh.next_tick_at()) * 1000.0 / f64::from(fresh.period_ms))
        .max(0.0);
        if future > 0.0 {
            let fresh_ticks = f64::from(fresh_duration_ms) / f64::from(fresh.period_ms);
            let spell = world
                .resource::<crate::spell_info::SpellStore>()
                .get(old.spell_id)
                .expect("rolling periodic spell must exist");
            let prior_amount =
                super::periodic::periodic_live_amount(world, old.caster, spell, prior);
            let stacks = old.stack_multiplier();
            fresh.amount = redistribute_rolling_pool(
                prior_amount * stacks,
                unpaid,
                fresh.amount,
                fresh_ticks,
                future,
            );
            fresh.periodic_power_multiplier = redistribute_rolling_pool(
                prior.periodic_power_multiplier * stacks,
                unpaid,
                fresh.periodic_power_multiplier,
                fresh_ticks,
                future,
            );
        }
    }
}

fn redistribute_rolling_pool(
    prior: f32,
    unpaid_ticks: f64,
    fresh: f32,
    fresh_ticks: f64,
    future_ticks: f64,
) -> f32 {
    ((f64::from(prior) * unpaid_ticks + f64::from(fresh) * fresh_ticks) / future_ticks) as f32
}

///190456 c3/s4: remaining shields accumulate to the native percent-health cap.
fn accumulate_ignore_pain(world: &World, target: Entity, new: &mut AuraInstance) {
    if !world
        .get::<ActiveSpec>(new.caster)
        .is_some_and(|spec| spec.0 == PROTECTION_WARRIOR)
    {
        return;
    }
    let percent = new
        .effects
        .iter()
        .find(|effect| effect.index == 3)
        .expect("Ignore Pain cap field")
        .amount;
    let max_health = world
        .get::<Health>(target)
        .expect("living shield target")
        .max;
    let previous = previous(world, target, new)
        .and_then(|old| {
            old.effects
                .iter()
                .find(|effect| effect.index == 0)
                .map(|effect| effect.absorb_remaining)
        })
        .unwrap_or(0.0);
    if let Some(shield) = new.effects.iter_mut().find(|effect| effect.index == 0) {
        shield.absorb_remaining =
            (shield.absorb_remaining + previous).min(max_health * percent / 100.0);
    }
}

/// SimC76f8118 sc_paladin.hpp:990–999: retain unspent 432607 up to the native cap.
fn accumulate_holy_bulwark(world: &World, target: Entity, new: &mut AuraInstance) {
    let cap_percent = world
        .resource::<crate::spell_info::SpellStore>()
        .get(432496)
        .expect("Holy Bulwark source spell")
        .effects
        .iter()
        .find(|effect| effect.index == 4)
        .expect("Holy Bulwark cap effect")
        .base_points;
    let max_health = world
        .get::<Health>(target)
        .expect("living Holy Bulwark target")
        .max;
    let previous = previous(world, target, new)
        .and_then(|old| {
            old.effects
                .iter()
                .find(|effect| effect.index == 0)
                .map(|effect| effect.absorb_remaining)
        })
        .unwrap_or(0.0);
    if let Some(shield) = new.effects.iter_mut().find(|effect| effect.index == 0) {
        shield.absorb_remaining =
            (shield.absorb_remaining + previous).min(max_health * cap_percent / 100.0);
    }
}

/// SimC76f8118 sc_death_knight.cpp6952–6983: add unpaid Blood Shield,
/// capped by the native mastery health percentage.
fn accumulate_blood_shield(world: &World, target: Entity, new: &mut AuraInstance) {
    let cap_percent = crate::spell_cast::scripts::script_effect_value(world, new.caster, 77513, 2);
    let max_health = world
        .get::<Health>(target)
        .expect("living Blood Shield target")
        .max;
    let previous = previous(world, target, new)
        .and_then(|old| {
            old.effects
                .iter()
                .find(|effect| effect.index == 0)
                .map(|effect| effect.absorb_remaining)
        })
        .unwrap_or(0.0);
    if let Some(shield) = new.effects.iter_mut().find(|effect| effect.index == 0) {
        shield.absorb_remaining =
            (shield.absorb_remaining + previous).min(max_health * cap_percent / 100.0);
    }
}

/// SimC76f8118 sc_druid.cpp5166–5173: cumulative Ursoc's Fury is capped at30% max health.
fn accumulate_ursocs_fury(world: &World, target: Entity, new: &mut AuraInstance) {
    let max_health = world
        .get::<Health>(target)
        .expect("living Ursoc's Fury target")
        .max;
    let previous = previous(world, target, new)
        .and_then(|old| {
            old.effects
                .iter()
                .find(|effect| effect.index == 0)
                .map(|effect| effect.absorb_remaining)
        })
        .unwrap_or(0.0);
    if let Some(shield) = new.effects.iter_mut().find(|effect| effect.index == 0) {
        shield.absorb_remaining = (shield.absorb_remaining + previous).min(max_health * 0.3);
    }
}

/// Ignore Pain and Celestial Infusion spend their authored fraction of each hit.
pub(super) fn absorb_fraction(aura: &AuraInstance) -> f32 {
    if !matches!(aura.spell_id, IGNORE_PAIN | 1241059) {
        return 1.0;
    }
    let percent = aura
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("fractional absorb percentage")
        .amount;
    (percent / 100.0).clamp(0.0, 1.0)
}
