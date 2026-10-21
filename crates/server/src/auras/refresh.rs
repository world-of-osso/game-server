//! Named reapplication rules explicitly described in local Mainline spell data.
use bevy::prelude::*;
use shared::components::{ActiveSpec, Health};

use super::{ActiveAuras, AuraInstance, aura_now, periodic};

const SHADOW_WORD_MADNESS: u32 = 335467;
const BARBED_SHOT: u32 = 217200;
const IGNORE_PAIN: u32 = 190456;
const PROTECTION_WARRIOR: u32 = 73;
const SHIELD_BLOCK: u32 = 132404;
const SHIELD_OF_THE_RIGHTEOUS: u32 = 132403;
const RIP: u32 = 1079;
const RAKE: u32 = 155722;

#[derive(Clone, Copy)]
enum DurationCarry {
    Pandemic,
    Extend,
    CappedExtend(u32),
}

pub(super) fn prepare_application(world: &mut World, target: Entity, new: &mut AuraInstance) {
    match new.spell_id {
        SHADOW_WORD_MADNESS => roll_remaining_damage(world, target, new),
        BARBED_SHOT => roll_barbed_damage(world, target, new),
        IGNORE_PAIN => accumulate_ignore_pain(world, target, new),
        RIP | RAKE => carry_duration(world, target, new, DurationCarry::Pandemic),
        SHIELD_BLOCK => carry_duration(world, target, new, DurationCarry::Extend),
        SHIELD_OF_THE_RIGHTEOUS => {
            carry_duration(world, target, new, DurationCarry::CappedExtend(3));
        }
        _ => {}
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
fn remaining_ticks(next: f64, expiry: f64, period_ms: u32) -> u32 {
    if period_ms == 0 || next > expiry + 1e-6 {
        return 0;
    }
    (((expiry - next) / (f64::from(period_ms) / 1000.0) + 1e-6).floor() as u32) + 1
}

///335467: "all remaining damage is added to the new effect", not Pandemic.
fn roll_remaining_damage(world: &mut World, target: Entity, new: &mut AuraInstance) {
    let Some(old) = previous(world, target, new) else {
        return;
    };
    // Resolve old due ticks before taking its unpaid damage; they cannot be paid twice.
    periodic::run_ticks_until(world, target, old.instance_id, aura_now(world));
    let Some(old) = previous(world, target, new) else {
        return;
    };
    let Some((old_expiry, new_expiry)) = old.expires_at.zip(new.expires_at) else {
        return;
    };
    for fresh in &mut new.effects {
        if fresh.aura_type != super::handlers::PERIODIC_LEECH {
            continue;
        }
        let Some(prior) = old
            .effects
            .iter()
            .find(|effect| effect.index == fresh.index)
        else {
            continue;
        };
        let unpaid = remaining_ticks(prior.next_tick_at(), old_expiry, prior.period_ms);
        let future = remaining_ticks(prior.next_tick_at(), new_expiry, prior.period_ms);
        if future > 0 {
            fresh.amount += prior.amount * old.stack_multiplier() * unpaid as f32 / future as f32;
        }
    }
}

///217200's description totals five damage amounts, while the native clock
///has seven ticks. Distribute that total without replacing the native clock.
fn roll_barbed_damage(world: &mut World, target: Entity, new: &mut AuraInstance) {
    if let Some(old) = previous(world, target, new) {
        periodic::run_ticks_until(world, target, old.instance_id, aura_now(world));
    }
    let old = previous(world, target, new);
    let count = crate::spell_cast::scripts::script_effect_value(world, new.caster, BARBED_SHOT, 1);
    let expiry = new.expires_at.expect("Barbed Shot native duration");
    let fresh = new
        .effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("Barbed Shot periodic damage");
    let new_budget = fresh.amount * count;
    let (next, old_budget) = old
        .as_ref()
        .and_then(|aura| unpaid_barbed_damage(aura, fresh.index))
        .unwrap_or((fresh.next_tick_at(), 0.0));
    let future = remaining_ticks(next, expiry, fresh.period_ms);
    if future > 0 {
        fresh.amount = (new_budget + old_budget) / future as f32;
    }
}

fn unpaid_barbed_damage(aura: &AuraInstance, index: u8) -> Option<(f64, f32)> {
    let effect = aura.effects.iter().find(|effect| effect.index == index)?;
    let unpaid = remaining_ticks(effect.next_tick_at(), aura.expires_at?, effect.period_ms);
    let budget = effect.amount * aura.stack_multiplier() * unpaid as f32;
    Some((effect.next_tick_at(), budget))
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
