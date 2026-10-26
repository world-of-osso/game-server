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
const RIP: u32 = 1079;
const RAKE: u32 = 155722;

#[derive(Clone, Copy)]
enum RollingBudget {
    PerTick,
    NativeDuration(u32),
}

#[derive(Clone, Copy)]
enum DurationCarry {
    Pandemic,
    Extend,
    CappedExtend(u32),
}

pub(super) fn prepare_application(
    world: &mut World,
    target: Entity,
    new: &mut AuraInstance,
    spell: &SpellInfo,
) {
    let fresh_duration_ms = new.duration_ms;
    let sourced_rolling =
        crate::spell_cast::scripts::rolls_reapplication(world, new.caster, new.spell_id);
    let rolling = sourced_rolling
        || spell
            .misc
            .as_ref()
            .is_some_and(|misc| misc.attributes[10] & ROLLING_PERIODIC != 0);
    if rolling {
        settle_previous_ticks(world, target, new);
    }
    prepare_periodic_phase(world, target, new, spell, sourced_rolling);
    if crate::spell_cast::scripts::extends_reapplication(world, new.caster, new.spell_id) {
        carry_duration(world, target, new, DurationCarry::Extend);
    }
    match new.spell_id {
        IGNORE_PAIN => accumulate_ignore_pain(world, target, new),
        RIP | RAKE => carry_duration(world, target, new, DurationCarry::Pandemic),
        SHIELD_BLOCK => carry_duration(world, target, new, DurationCarry::Extend),
        SHIELD_OF_THE_RIGHTEOUS => {
            carry_duration(world, target, new, DurationCarry::CappedExtend(3));
        }
        _ => {}
    }
    preserve_expiry(world, target, new, spell);
    if rolling {
        let budget = if sourced_rolling {
            carry_rolling_duration(world, target, new);
            RollingBudget::NativeDuration(fresh_duration_ms)
        } else {
            RollingBudget::PerTick
        };
        roll_remaining_periodic(world, target, new, budget);
    }
    super::ret_crusade::preserve_refresh_duration(world, target, new);
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

/// TC a352b1fa Spell.cpp3236 and SpellAuras.cpp976–991: normal nonstacking
/// recasts reset periodic timers; FULL_MASK's DONT_RESET_PERIODIC_TIMER,
/// stacking and native Pandemic retain time-to-tick (SpellDefines.h291–293).
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
    let pandemic = spell
        .misc
        .as_ref()
        .is_some_and(|misc| misc.attributes[13] & PERIODIC_REFRESH_EXTENDS_DURATION != 0);
    let native_preserves_phase = spell.cast_dont_reset_periodic_timer || stacking || pandemic;
    let preserve_phase = native_preserves_phase || sourced_rolling;
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

/// Native per-tick carry: TC a352b1fa SpellAuraEffects.cpp832–849.
/// Source-qualified coefficient rolling: SimC76f8118 action.cpp4995–5016
/// shares old unpaid + fresh native duration budget across the new schedule.
fn roll_remaining_periodic(
    world: &World,
    target: Entity,
    new: &mut AuraInstance,
    budget: RollingBudget,
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
        let unpaid = remaining_ticks(prior.next_tick_at(), old_expiry, prior.period_ms);
        let future = remaining_ticks(fresh.next_tick_at(), new_expiry, fresh.period_ms);
        if future > 0 {
            let unpaid_budget = prior.amount * old.stack_multiplier() * unpaid as f32;
            fresh.amount = match budget {
                RollingBudget::PerTick => fresh.amount + unpaid_budget / future as f32,
                RollingBudget::NativeDuration(duration) => {
                    let fresh_ticks = duration as f32 / fresh.period_ms as f32;
                    (fresh.amount * fresh_ticks + unpaid_budget) / future as f32
                }
            };
        }
    }
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

///190456s2: every hit only spends its authored fraction of incoming damage.
pub(super) fn absorb_fraction(aura: &AuraInstance) -> f32 {
    if aura.spell_id != IGNORE_PAIN {
        return 1.0;
    }
    let percent = aura
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("Ignore Pain absorb fraction field")
        .amount;
    (percent / 100.0).clamp(0.0, 1.0)
}
