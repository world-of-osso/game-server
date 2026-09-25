//! Spell effect handlers, keyed by `SpellEffect.effect` (SpellEffects ID).
//!
//! Amounts: `BasePoints + BonusCoefficient × spell power +
//! BonusCoefficientFromAP × attack power`. Spell power, attack power and crit
//! chance come from the caster's `CharacterStats` (`stats.rs`, aura stat
//! modifiers included); units without it (NPCs) have 0.
//! Amounts take the caster's spell modifiers (Points/PointsIndexN on base
//! points, BonusCoefficient on coefficients); damage and healing then take
//! HealingAndDamage (or PeriodicHealingAndDamage) mods and the aura damage and
//! healing multipliers (`AuraStatMods`), then target immunity and absorbs.
//! CritChance mods add to the crit chance; a crit adds 100% of the amount
//! (retail 200%), scaled by CritDamageAndHealing mods.
//!
//! Weapon hits (58, 121, 31) roll the main hand's `[min, max]` and add
//! `AP / 6 × speed`; 121 NORMALIZED_WEAPON_DMG uses the normalized speed of the
//! weapon type (SimC `action_t::calculate_weapon_damage`). Main-hand damage is
//! `CharacterStats.main_hand` (`item_stats.rs`), unarmed 1-2 at 2.0 s.

use std::collections::HashSet;

use bevy::prelude::*;
use lightyear::prelude::ReplicationDirtyQueue;
use shared::casting::CastState;
use shared::components::{Health, Npc, PowerType};
use shared::protocol::{CombatLogEvent, CombatLogKind, MissKind};

use super::{SpellHistory, log_event, now};
use crate::auras::procs::{self, AuraEvent};
use crate::auras::{AuraStatMods, SpellModOp, spell_mods};
use crate::creature_scripts::{TrainingDummy, health_loss};
use crate::item_stats::WeaponDamage;
use crate::spell_info::{SpellEffect, SpellInfo};
use crate::stats::CharacterStats;

pub const EFFECT_SCHOOL_DAMAGE: u32 = 2;
pub const EFFECT_HEAL: u32 = 10;
pub const EFFECT_ENERGIZE: u32 = 30;
pub const EFFECT_WEAPON_PERCENT_DAMAGE: u32 = 31;
pub const EFFECT_DISPEL: u32 = 38;
pub const EFFECT_WEAPON_DAMAGE: u32 = 58;
pub const EFFECT_TRIGGER_SPELL: u32 = 64;
pub const EFFECT_INTERRUPT_CAST: u32 = 68;
pub const EFFECT_NORMALIZED_WEAPON_DMG: u32 = 121;
pub const EFFECT_HEAL_PCT: u32 = 136;

/// Triggered spells may trigger further spells up to this depth.
pub const MAX_TRIGGER_DEPTH: u8 = 3;

/// One resolving spell.
pub struct EffectCtx<'a> {
    pub caster: Entity,
    pub spell: &'a SpellInfo,
    /// 0 for the requested spell, +1 per `TRIGGER_SPELL` hop.
    pub depth: u8,
}

pub type EffectHandler = fn(&mut World, &EffectCtx, &SpellEffect, Entity);

/// The effect registry. Aura effects (`crate::auras::AURA_EFFECTS`, e.g. 6
/// APPLY_AURA) are resolved per target by `resolve_effects`, not here.
pub fn handler(effect_id: u32) -> Option<EffectHandler> {
    Some(match effect_id {
        EFFECT_SCHOOL_DAMAGE => school_damage,
        EFFECT_HEAL => heal,
        EFFECT_ENERGIZE => energize,
        EFFECT_WEAPON_PERCENT_DAMAGE => weapon_percent_damage,
        EFFECT_DISPEL => dispel,
        EFFECT_WEAPON_DAMAGE | EFFECT_NORMALIZED_WEAPON_DMG => weapon_damage,
        EFFECT_TRIGGER_SPELL => trigger_spell,
        EFFECT_INTERRUPT_CAST => interrupt_cast,
        EFFECT_HEAL_PCT => heal_pct,
        _ => return None,
    })
}

/// Effects that select targets and resolve: registry handlers and aura effects.
pub fn is_handled(effect: &SpellEffect) -> bool {
    handler(effect.effect).is_some() || crate::auras::is_aura_effect(effect)
}

/// `(spell id, effect id)` pairs already logged as unhandled.
#[derive(Resource, Debug, Default)]
pub struct UnhandledEffects(pub HashSet<(u32, u32)>);

pub fn log_unhandled(world: &mut World, spell_id: u32, effect_id: u32) {
    let mut logged = world.resource_mut::<UnhandledEffects>();
    if logged.0.insert((spell_id, effect_id)) {
        warn!("Spell {spell_id}: effect {effect_id} has no handler, skipped");
    }
}

/// `(spell power, attack power)` of `unit`.
pub(crate) fn power_stats(world: &World, unit: Entity) -> (f32, f32) {
    world
        .get::<CharacterStats>(unit)
        .map_or((0.0, 0.0), |stats| (stats.spell_power, stats.attack_power))
}

/// Base points with the caster's Points and PointsIndexN mods.
pub(crate) fn modded_base_points(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
) -> f32 {
    let base = spell_mods::modify(world, caster, spell, SpellModOp::Points, effect.base_points);
    match SpellModOp::points_index(effect.index) {
        Some(op) => spell_mods::modify(world, caster, spell, op, base),
        None => base,
    }
}

/// Effect amount before crit: `modded_base_points` plus SP/AP coefficients
/// with BonusCoefficient mods.
pub(crate) fn scaled_amount(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
) -> f32 {
    let (spell_power, attack_power) = power_stats(world, caster);
    let modify = |op, value| spell_mods::modify(world, caster, spell, op, value);
    let base = modded_base_points(world, caster, spell, effect);
    let coefficient = modify(SpellModOp::BonusCoefficient, effect.bonus_coefficient);
    let ap_coefficient = modify(
        SpellModOp::BonusCoefficient,
        effect.bonus_coefficient_from_ap,
    );
    base + coefficient * spell_power + ap_coefficient * attack_power
}

/// One main-hand weapon hit; units without `CharacterStats` hit unarmed with 0 AP.
fn weapon_hit(world: &World, caster: Entity, normalized: bool) -> f32 {
    let (_, attack_power) = power_stats(world, caster);
    let weapon = world
        .get::<CharacterStats>(caster)
        .map_or(WeaponDamage::UNARMED, |stats| stats.main_hand);
    let speed = if normalized {
        weapon.kind.normalized_speed()
    } else {
        weapon.speed
    };
    let roll = weapon.min + rand::random::<f32>() * (weapon.max - weapon.min);
    shared::formulas::auto_attack_damage(roll, attack_power, speed)
}

/// Crit chance in percent: `CharacterStats.crit_pct` plus CritChance mods.
fn crit_chance(world: &World, caster: Entity, spell: &SpellInfo) -> f32 {
    let base = world
        .get::<CharacterStats>(caster)
        .map_or(0.0, |stats| stats.crit_pct);
    spell_mods::modify(world, caster, spell, SpellModOp::CritChance, base)
}

fn with_crit(world: &World, caster: Entity, spell: &SpellInfo, amount: f32) -> (f32, bool) {
    let crit = rand::random::<f32>() * 100.0 < crit_chance(world, caster, spell);
    if !crit {
        return (amount, false);
    }
    let bonus = spell_mods::modify(
        world,
        caster,
        spell,
        SpellModOp::CritDamageAndHealing,
        amount,
    );
    (amount + bonus, true)
}

fn school_damage(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let amount = scaled_amount(world, ctx.caster, ctx.spell, effect);
    let (amount, crit) = with_crit(world, ctx.caster, ctx.spell, amount);
    deal_damage(world, ctx, target, amount, crit, false);
}

/// 58 WEAPON_DAMAGE and 121 NORMALIZED_WEAPON_DMG: weapon hit + BasePoints.
fn weapon_damage(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let normalized = effect.effect == EFFECT_NORMALIZED_WEAPON_DMG;
    let base = weapon_hit(world, ctx.caster, normalized) + effect.base_points;
    let (amount, crit) = with_crit(world, ctx.caster, ctx.spell, base);
    deal_damage(world, ctx, target, amount, crit, false);
}

/// 31 WEAPON_PERCENT_DAMAGE: BasePoints% of a weapon hit.
fn weapon_percent_damage(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let base = weapon_hit(world, ctx.caster, false) * effect.base_points / 100.0;
    let (amount, crit) = with_crit(world, ctx.caster, ctx.spell, base);
    deal_damage(world, ctx, target, amount, crit, false);
}

fn log_amount(ctx: &EffectCtx, target: Entity, kind: CombatLogKind) -> CombatLogEvent {
    CombatLogEvent {
        source: Some(ctx.caster.to_bits()),
        target: Some(target.to_bits()),
        spell_id: Some(ctx.spell.id),
        school_mask: ctx.spell.school_mask(),
        amount: 0,
        overflow: 0,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit: false,
        periodic: false,
        kind,
    }
}

fn mark_dirty(world: &World, entity: Entity) {
    if let Some(dirty) = world.get_resource::<ReplicationDirtyQueue>() {
        dirty.mark(entity);
    }
}

/// Spell mods (HealingAndDamage or PeriodicHealingAndDamage) on the base amount.
fn modded_amount(world: &World, ctx: &EffectCtx, amount: f32, periodic: bool) -> f32 {
    let op = if periodic {
        SpellModOp::PeriodicHealingAndDamage
    } else {
        SpellModOp::HealingAndDamage
    };
    spell_mods::modify(world, ctx.caster, ctx.spell, op, amount)
}

fn stat_mods(world: &World, unit: Entity) -> Option<&AuraStatMods> {
    world.get::<AuraStatMods>(unit)
}

/// Apply damage through the shared death path after spell mods, damage
/// multipliers, immunity and absorbs; logs Damage (+ Death) and pushes
/// damage events for procs and interrupts.
pub fn deal_damage(
    world: &mut World,
    ctx: &EffectCtx,
    target: Entity,
    amount: f32,
    crit: bool,
    periodic: bool,
) {
    if !super::targets::is_alive(world, target) {
        return;
    }
    let school = ctx.spell.school_mask();
    if crate::auras::is_immune(world, target, school) {
        let mut event = log_amount(ctx, target, CombatLogKind::Miss(MissKind::Immune));
        event.periodic = periodic;
        log_event(world, event);
        return;
    }
    let done = stat_mods(world, ctx.caster).map_or(1.0, |mods| mods.damage_done(school));
    let taken = crate::auras::damage_taken_multiplier(world, target, ctx.caster, school);
    let amount = (modded_amount(world, ctx, amount, periodic) * done * taken)
        .max(0.0)
        .round();
    let (amount, absorbed) = crate::auras::absorb_damage(world, target, school, amount);
    let training_dummy = world.get::<TrainingDummy>(target).is_some();
    let Some(mut health) = world.get_mut::<Health>(target) else {
        return;
    };
    let before = health.current;
    let lethal =
        crate::death::apply_health_damage(&mut health, health_loss(training_dummy, amount));
    let overkill = (amount - before).max(0.0);
    mark_dirty(world, target);
    world
        .entity_mut(target)
        .insert(crate::combat::LastDamager(ctx.caster));
    crate::combat::enter_combat(world, ctx.caster);
    crate::combat::enter_combat(world, target);
    let mut event = log_amount(ctx, target, CombatLogKind::Damage);
    event.amount = amount as i32;
    event.overflow = overkill as i32;
    event.absorbed = absorbed as i32;
    event.crit = crit;
    event.periodic = periodic;
    log_event(world, event);
    if amount > 0.0 {
        push_damage_event(world, ctx, target, periodic);
    }
    if lethal {
        kill(world, ctx.caster, target, ctx.depth == 0);
    }
}

fn push_damage_event(world: &mut World, ctx: &EffectCtx, target: Entity, periodic: bool) {
    let (actor, taken) = if periodic {
        (
            procs::PROC_DEAL_HARMFUL_PERIODIC,
            procs::PROC_TAKE_HARMFUL_PERIODIC,
        )
    } else {
        (0, 0)
    };
    procs::push_event(
        world,
        AuraEvent {
            actor: ctx.caster,
            target: Some(target),
            actor_mask: [actor, 0],
            target_mask: [taken | procs::PROC_TAKE_ANY_DAMAGE, 0],
            spell_id: Some(ctx.spell.id),
            can_proc: ctx.depth == 0,
            periodic,
        },
    );
}

fn kill(world: &mut World, killer: Entity, target: Entity, can_proc: bool) {
    if world.get::<Npc>(target).is_some() {
        crate::combat::mark_npc_dead_world(world, target);
    } else {
        world.write_message(crate::death::PlayerDeathTransition { player: target });
    }
    log_event(world, crate::combat::death_log_event(killer, target));
    procs::push_event(
        world,
        AuraEvent {
            actor: killer,
            target: Some(target),
            actor_mask: [procs::PROC_KILL, 0],
            target_mask: [procs::PROC_DEATH, 0],
            spell_id: None,
            can_proc,
            periodic: false,
        },
    );
}

fn heal(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let amount = scaled_amount(world, ctx.caster, ctx.spell, effect);
    let (amount, crit) = with_crit(world, ctx.caster, ctx.spell, amount);
    apply_heal(world, ctx, target, amount, crit, false);
}

/// 136 HEAL_PCT: BasePoints% of the target's max health.
fn heal_pct(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let max = world.get::<Health>(target).map_or(0.0, |health| health.max);
    apply_heal(
        world,
        ctx,
        target,
        max * effect.base_points / 100.0,
        false,
        false,
    );
}

/// Heal a living target after spell mods and healing multipliers, clamped
/// to max; logs Heal with overheal. Periodic heals push helpful-periodic events.
pub fn apply_heal(
    world: &mut World,
    ctx: &EffectCtx,
    target: Entity,
    amount: f32,
    crit: bool,
    periodic: bool,
) {
    let done = stat_mods(world, ctx.caster).map_or(1.0, |mods| mods.healing_done_mult);
    let taken = stat_mods(world, target).map_or(1.0, |mods| mods.healing_taken_mult);
    let amount = (modded_amount(world, ctx, amount, periodic) * done * taken)
        .max(0.0)
        .round();
    let Some(mut health) = world.get_mut::<Health>(target) else {
        return;
    };
    if health.current <= 0.0 {
        return;
    }
    let effective = amount.min(health.max - health.current).max(0.0);
    health.current += effective;
    mark_dirty(world, target);
    let mut event = log_amount(ctx, target, CombatLogKind::Heal);
    event.amount = amount as i32;
    event.overflow = (amount - effective) as i32;
    event.crit = crit;
    event.periodic = periodic;
    log_event(world, event);
    if periodic {
        procs::push_event(
            world,
            AuraEvent {
                actor: ctx.caster,
                target: Some(target),
                actor_mask: [procs::PROC_DEAL_HELPFUL_PERIODIC, 0],
                target_mask: [procs::PROC_TAKE_HELPFUL_PERIODIC, 0],
                spell_id: Some(ctx.spell.id),
                can_proc: ctx.depth == 0,
                periodic: true,
            },
        );
    }
}

/// 30 ENERGIZE: BasePoints raw units of power `EffectMiscValue_0`.
fn energize(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let Some(power) = PowerType::from_db(effect.misc_values[0]) else {
        debug!(
            "Spell {}: energize of unknown power {}",
            ctx.spell.id, effect.misc_values[0]
        );
        return;
    };
    energize_unit(world, ctx, target, power, effect.base_points, false);
}

/// Add `amount` raw units of `power`, clamped; logs Energize with the excess.
pub fn energize_unit(
    world: &mut World,
    ctx: &EffectCtx,
    target: Entity,
    power: PowerType,
    amount: f32,
    periodic: bool,
) {
    let amount = amount.max(0.0);
    let Some(gained) = super::powers::change_power(world, target, power, amount) else {
        debug!("Spell {}: target has no {power:?} pool", ctx.spell.id);
        return;
    };
    let mut event = log_amount(ctx, target, CombatLogKind::Energize);
    event.amount = amount as i32;
    event.overflow = (amount - gained) as i32;
    event.periodic = periodic;
    log_event(world, event);
}

/// 38 DISPEL: remove `BasePoints` (at least 1) auras of dispel type
/// `EffectMiscValue_0` from the target.
fn dispel(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let Ok(dispel_type) = u32::try_from(effect.misc_values[0]) else {
        return;
    };
    let count = (effect.base_points.max(1.0)) as usize;
    crate::auras::dispel(world, ctx.caster, target, dispel_type, count);
}

/// 64 TRIGGER_SPELL: cast `EffectTriggerSpell` at this target.
fn trigger_spell(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    super::cast_triggered(
        world,
        ctx.caster,
        effect.trigger_spell,
        Some(target),
        ctx.depth + 1,
    );
}

/// 68 INTERRUPT_CAST: cancel an interruptible cast and lock its school for
/// this spell's duration.
fn interrupt_cast(world: &mut World, ctx: &EffectCtx, _effect: &SpellEffect, target: Entity) {
    let Some(cast) = world.get::<CastState>(target).cloned() else {
        return;
    };
    if !cast.interruptible {
        return;
    }
    let school_mask = world
        .get_resource::<crate::spell_info::SpellStore>()
        .and_then(|store| store.get(cast.spell_id))
        .map_or(0, SpellInfo::school_mask);
    let lockout_ms = ctx
        .spell
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .map_or(0, |duration| duration.duration_ms.max(0));
    super::cancel_cast(world, target);
    let until = now(world) + lockout_ms as f64 / 1000.0;
    world
        .entity_mut(target)
        .entry::<SpellHistory>()
        .or_default()
        .get_mut()
        .lock_schools(school_mask, until);
    let mut event = log_amount(ctx, target, CombatLogKind::Interrupt);
    event.school_mask = school_mask;
    log_event(world, event);
}
