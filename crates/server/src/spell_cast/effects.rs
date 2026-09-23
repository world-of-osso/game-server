//! Spell effect handlers, keyed by `SpellEffect.effect` (SpellEffects ID).
//!
//! Amounts: `BasePoints + BonusCoefficient × spell power +
//! BonusCoefficientFromAP × attack power`. Spell power, attack power and crit
//! chance come from the caster's `CharacterStats` (`stats.rs`); units without
//! it (NPCs) have 0. A crit doubles the amount (retail 200%).
//!
//! Weapon model (58, 121, 31): no server-side weapon data exists, so a
//! weapon hit is the auto-attack's flat 10 plus `AP / 6 × 2.0` (the 2 s
//! auto-attack interval as weapon speed).

use std::collections::HashSet;

use bevy::prelude::*;
use lightyear::prelude::ReplicationDirtyQueue;
use shared::casting::CastState;
use shared::components::{Health, Npc, PowerType};
use shared::protocol::{CombatLogEvent, CombatLogKind};

use super::{SpellHistory, log_event, now};
use crate::spell_info::{SpellEffect, SpellInfo};
use crate::stats::CharacterStats;

pub const EFFECT_SCHOOL_DAMAGE: u32 = 2;
pub const EFFECT_APPLY_AURA: u32 = 6;
pub const EFFECT_HEAL: u32 = 10;
pub const EFFECT_ENERGIZE: u32 = 30;
pub const EFFECT_WEAPON_PERCENT_DAMAGE: u32 = 31;
pub const EFFECT_WEAPON_DAMAGE: u32 = 58;
pub const EFFECT_TRIGGER_SPELL: u32 = 64;
pub const EFFECT_INTERRUPT_CAST: u32 = 68;
pub const EFFECT_NORMALIZED_WEAPON_DMG: u32 = 121;
pub const EFFECT_HEAL_PCT: u32 = 136;

/// Triggered spells may trigger further spells up to this depth.
pub const MAX_TRIGGER_DEPTH: u8 = 3;
const WEAPON_BASE_DAMAGE: f32 = 10.0;
const WEAPON_SPEED: f32 = 2.0;
const CRIT_MULTIPLIER: f32 = 2.0;

/// One resolving spell.
pub struct EffectCtx<'a> {
    pub caster: Entity,
    pub spell: &'a SpellInfo,
    /// 0 for the requested spell, +1 per `TRIGGER_SPELL` hop.
    pub depth: u8,
}

pub type EffectHandler = fn(&mut World, &EffectCtx, &SpellEffect, Entity);

/// The effect registry.
pub fn handler(effect_id: u32) -> Option<EffectHandler> {
    Some(match effect_id {
        EFFECT_SCHOOL_DAMAGE => school_damage,
        EFFECT_APPLY_AURA => apply_aura,
        EFFECT_HEAL => heal,
        EFFECT_ENERGIZE => energize,
        EFFECT_WEAPON_PERCENT_DAMAGE => weapon_percent_damage,
        EFFECT_WEAPON_DAMAGE | EFFECT_NORMALIZED_WEAPON_DMG => weapon_damage,
        EFFECT_TRIGGER_SPELL => trigger_spell,
        EFFECT_INTERRUPT_CAST => interrupt_cast,
        EFFECT_HEAL_PCT => heal_pct,
        _ => return None,
    })
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

/// One aura application request from effect 6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuraApplication {
    pub caster: Entity,
    pub target: Entity,
    pub spell_id: u32,
    pub effect_index: u8,
}

/// Hook for the aura runtime. Called with the world; must not re-enter
/// `AuraApplierHook` synchronously (it is taken out while running).
pub trait AuraApplier: Send + Sync + 'static {
    fn apply_aura(&mut self, _world: &mut World, _application: &AuraApplication) {}
    /// A channel reached its tick period (shortest effect aura period).
    fn channel_tick(&mut self, _world: &mut World, _caster: Entity, _spell_id: u32) {}
    /// A channel completed or was cancelled/interrupted.
    fn channel_ended(&mut self, _world: &mut World, _caster: Entity, _spell_id: u32) {}
}

/// Default: auras are not implemented.
pub struct NoopAuraApplier;
impl AuraApplier for NoopAuraApplier {}

#[derive(Resource)]
pub struct AuraApplierHook(pub Box<dyn AuraApplier>);

impl Default for AuraApplierHook {
    fn default() -> Self {
        Self(Box::new(NoopAuraApplier))
    }
}

pub fn with_aura_hook(world: &mut World, call: impl FnOnce(&mut dyn AuraApplier, &mut World)) {
    world.resource_scope(|world, mut hook: Mut<AuraApplierHook>| call(hook.0.as_mut(), world));
}

fn apply_aura(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let application = AuraApplication {
        caster: ctx.caster,
        target,
        spell_id: ctx.spell.id,
        effect_index: effect.index,
    };
    with_aura_hook(world, |hook, world| hook.apply_aura(world, &application));
}

/// `(spell power, attack power)` of `unit`.
fn power_stats(world: &World, unit: Entity) -> (f32, f32) {
    world
        .get::<CharacterStats>(unit)
        .map_or((0.0, 0.0), |stats| (stats.spell_power, stats.attack_power))
}

fn scaled_amount(world: &World, caster: Entity, effect: &SpellEffect) -> f32 {
    let (spell_power, attack_power) = power_stats(world, caster);
    effect.base_points
        + effect.bonus_coefficient * spell_power
        + effect.bonus_coefficient_from_ap * attack_power
}

fn weapon_hit(world: &World, caster: Entity) -> f32 {
    let (_, attack_power) = power_stats(world, caster);
    WEAPON_BASE_DAMAGE + shared::formulas::ap_bonus_damage(attack_power, WEAPON_SPEED)
}

fn roll_crit(world: &World, caster: Entity) -> bool {
    let chance = world
        .get::<CharacterStats>(caster)
        .map_or(0.0, |stats| stats.crit_pct);
    rand::random::<f32>() * 100.0 < chance
}

fn with_crit(world: &World, caster: Entity, amount: f32) -> (f32, bool) {
    let crit = roll_crit(world, caster);
    (
        if crit {
            amount * CRIT_MULTIPLIER
        } else {
            amount
        },
        crit,
    )
}

fn school_damage(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let (amount, crit) = with_crit(world, ctx.caster, scaled_amount(world, ctx.caster, effect));
    deal_damage(world, ctx, target, amount, crit);
}

/// 58 WEAPON_DAMAGE and 121 NORMALIZED_WEAPON_DMG: weapon hit + BasePoints.
fn weapon_damage(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let base = weapon_hit(world, ctx.caster) + effect.base_points;
    let (amount, crit) = with_crit(world, ctx.caster, base);
    deal_damage(world, ctx, target, amount, crit);
}

/// 31 WEAPON_PERCENT_DAMAGE: BasePoints% of a weapon hit.
fn weapon_percent_damage(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let base = weapon_hit(world, ctx.caster) * effect.base_points / 100.0;
    let (amount, crit) = with_crit(world, ctx.caster, base);
    deal_damage(world, ctx, target, amount, crit);
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

/// Apply damage through the shared death path; logs Damage (+ Death).
pub fn deal_damage(world: &mut World, ctx: &EffectCtx, target: Entity, amount: f32, crit: bool) {
    let amount = amount.max(0.0).round();
    let Some(mut health) = world.get_mut::<Health>(target) else {
        return;
    };
    let before = health.current;
    if before <= 0.0 {
        return;
    }
    let lethal = crate::death::apply_health_damage(&mut health, amount);
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
    event.crit = crit;
    log_event(world, event);
    if lethal {
        kill(world, ctx.caster, target);
    }
}

fn kill(world: &mut World, killer: Entity, target: Entity) {
    if world.get::<Npc>(target).is_some() {
        crate::combat::mark_npc_dead_world(world, target);
    } else {
        world.write_message(crate::death::PlayerDeathTransition { player: target });
    }
    log_event(
        world,
        CombatLogEvent {
            source: Some(killer.to_bits()),
            target: Some(target.to_bits()),
            spell_id: None,
            school_mask: 0,
            amount: 0,
            overflow: 0,
            absorbed: 0,
            resisted: 0,
            blocked: 0,
            crit: false,
            periodic: false,
            kind: CombatLogKind::Death,
        },
    );
}

fn heal(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let (amount, crit) = with_crit(world, ctx.caster, scaled_amount(world, ctx.caster, effect));
    apply_heal(world, ctx, target, amount, crit);
}

/// 136 HEAL_PCT: BasePoints% of the target's max health.
fn heal_pct(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let max = world.get::<Health>(target).map_or(0.0, |health| health.max);
    apply_heal(world, ctx, target, max * effect.base_points / 100.0, false);
}

/// Heal a living target, clamped to max; logs Heal with overheal.
pub fn apply_heal(world: &mut World, ctx: &EffectCtx, target: Entity, amount: f32, crit: bool) {
    let amount = amount.max(0.0).round();
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
    log_event(world, event);
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
    let amount = effect.base_points.max(0.0);
    let Some(gained) = super::powers::change_power(world, target, power, amount) else {
        debug!("Spell {}: target has no {power:?} pool", ctx.spell.id);
        return;
    };
    let mut event = log_amount(ctx, target, CombatLogKind::Energize);
    event.amount = amount as i32;
    event.overflow = (amount - gained) as i32;
    log_event(world, event);
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
