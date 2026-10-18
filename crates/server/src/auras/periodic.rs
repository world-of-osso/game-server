//! Periodic aura effects. Each effect with a period ticks at
//! `applied + k × period` while `≤ expiry`, so a 16 s aura with a 2 s period
//! ticks 8 times; a refresh keeps the tick phase. Amounts are the per-stack
//! amount × stacks.
//!
//! Tick sources: the aura caster deals the tick. PERIODIC_TRIGGER_SPELL casts
//! `EffectTriggerSpell` from the caster at the aura's unit, as a triggered
//! spell (no procs from it).

use bevy::prelude::*;
use shared::casting::{CastState, CastType};
use shared::components::{Health, PowerType};

use super::handlers::*;
use super::{ActiveAuras, scripts};
use crate::spell_cast::effects::{self, EffectCtx};
use crate::spell_info::{SpellInfo, SpellStore};

/// Triggered spells cast by ticks resolve at this depth (no proc events).
const TICK_TRIGGER_DEPTH: u8 = 1;

/// World of Osso user decision (2026-10-05): Recuperate ends at full health.
/// Not TC behavior: SpellAuraEffects.cpp:5879-5881 skips only permanent auras.
const END_AT_FULL_HEALTH: &[u32] = &[1_231_418];

struct Tick {
    instance_id: u32,
    caster: Entity,
    spell_id: u32,
    effect_index: u8,
    aura_type: u32,
    amount: f32,
    misc: i32,
    trigger_spell: u32,
}

/// Ticks due by `now` of every aura on `unit`, or of aura instance `only`.
fn due_ticks(world: &mut World, unit: Entity, now: f64, only: Option<u32>) -> Vec<Tick> {
    let Some(mut auras) = world.get_mut::<ActiveAuras>(unit) else {
        return Vec::new();
    };
    let mut ticks = Vec::new();
    for aura in auras
        .bypass_change_detection()
        .auras
        .iter_mut()
        .filter(|aura| only.is_none_or(|instance| aura.instance_id == instance))
    {
        let expiry = aura.expires_at.unwrap_or(f64::INFINITY) + 1e-6;
        let stacks = aura.stack_multiplier();
        for effect in aura
            .effects
            .iter_mut()
            .filter(|effect| effect.period_ms > 0)
        {
            while effect.next_tick_at() <= now && effect.next_tick_at() <= expiry {
                effect.next_tick_index += 1;
                ticks.push(Tick {
                    instance_id: aura.instance_id,
                    caster: aura.caster,
                    spell_id: aura.spell_id,
                    effect_index: effect.index,
                    aura_type: effect.aura_type,
                    amount: effect.amount * stacks,
                    misc: effect.misc_values[0],
                    trigger_spell: effect.trigger_spell,
                });
            }
        }
    }
    ticks
}

/// Run every tick due on `unit` up to `now`.
pub(super) fn run_due_ticks(world: &mut World, unit: Entity, now: f64) {
    let ticks = due_ticks(world, unit, now, None);
    run_ticks(world, unit, ticks);
}

/// Run the ticks of aura `instance_id` on `unit` due by `until`.
pub(super) fn run_ticks_until(world: &mut World, unit: Entity, instance_id: u32, until: f64) {
    let ticks = due_ticks(world, unit, until, Some(instance_id));
    run_ticks(world, unit, ticks);
}

fn run_ticks(world: &mut World, unit: Entity, ticks: Vec<Tick>) {
    for tick in ticks {
        let alive = world
            .get::<ActiveAuras>(unit)
            .is_some_and(|auras| auras.get(tick.instance_id).is_some());
        if !alive || !crate::spell_cast::targets::is_alive(world, unit) {
            continue;
        }
        if end_aura_at_full_health(world, unit, &tick) {
            continue;
        }
        let Some(spell) = world
            .get_resource::<SpellStore>()
            .and_then(|store| store.get(tick.spell_id))
            .cloned()
        else {
            continue;
        };
        run_tick(world, unit, &spell, &tick);
        end_aura_at_full_health(world, unit, &tick);
    }
}

fn end_aura_at_full_health(world: &mut World, unit: Entity, tick: &Tick) -> bool {
    let full_health = world
        .get::<Health>(unit)
        .is_some_and(|health| health.current >= health.max);
    if !END_AT_FULL_HEALTH.contains(&tick.spell_id) || !full_health {
        return false;
    }
    super::remove_instance(world, unit, tick.instance_id, super::RemoveReason::Consumed);
    let channeling = world
        .get::<CastState>(tick.caster)
        .is_some_and(|cast| cast.cast_type == CastType::Channel && cast.spell_id == tick.spell_id);
    if channeling {
        crate::spell_cast::end_cast(world, tick.caster);
    }
    true
}

fn run_tick(world: &mut World, unit: Entity, spell: &SpellInfo, tick: &Tick) {
    let ctx = EffectCtx {
        caster: tick.caster,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    let max_health = world.get::<Health>(unit).map_or(0.0, |health| health.max);
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == tick.effect_index)
        .expect("periodic effect belongs to its spell");
    match tick.aura_type {
        PERIODIC_DAMAGE => {
            let agony_tick = spell.id == super::agony::AGONY && tick.effect_index == 0;
            let advance_agony = agony_tick && !super::is_immune(world, unit, spell.school_mask());
            effects::deal_effect_damage(world, &ctx, effect, unit, tick.amount, false, true);
            if advance_agony {
                super::agony::advance_stack(world, unit, tick.instance_id);
            }
            if spell.id == crate::spell_cast::scripts::DEATHMARK && tick.effect_index == 0 {
                crate::spell_cast::scripts::deathmark_energy(world, tick.caster, spell);
            }
        }
        PERIODIC_LEECH => effects::deal_leech(world, &ctx, effect, unit, tick.amount, false, true),
        PERIODIC_DAMAGE_PERCENT => effects::deal_effect_damage(
            world,
            &ctx,
            effect,
            unit,
            max_health * tick.amount / 100.0,
            false,
            true,
        ),
        PERIODIC_HEAL => effects::apply_heal(world, &ctx, unit, tick.amount, false, true),
        OBS_MOD_HEALTH => effects::apply_heal(
            world,
            &ctx,
            unit,
            max_health * tick.amount / 100.0,
            false,
            true,
        ),
        PERIODIC_ENERGIZE => energize(world, &ctx, unit, tick),
        OBS_MOD_POWER => obs_mod_power(world, &ctx, unit, tick),
        PERIODIC_TRIGGER_SPELL
            if spell.id == crate::spell_cast::scripts::EYE_BEAM && tick.effect_index == 0 =>
        {
            crate::spell_cast::scripts::cast_eye_beam_tick(world, tick.caster);
        }
        PERIODIC_TRIGGER_SPELL
            if spell.id == crate::spell_cast::omor_orbital::ORBITAL_WHIP
                && tick.effect_index == 0 =>
        {
            crate::spell_cast::omor_orbital::cast_whip_tick(world, unit, tick.trigger_spell);
        }
        PERIODIC_TRIGGER_SPELL => {
            // TC OnEffectPeriodic runs before the default trigger, even when
            // DB2 has no trigger (the script supplies Mirror Image1264098).
            run_scripted_tick(world, unit, spell, tick);
            if tick.trigger_spell != 0 {
                crate::spell_cast::cast_triggered(
                    world,
                    tick.caster,
                    tick.trigger_spell,
                    Some(unit),
                    TICK_TRIGGER_DEPTH,
                );
            }
        }
        PERIODIC_DUMMY => run_dummy_tick(world, unit, spell, tick),
        other => debug!(
            "Spell {}: periodic aura type {other} has no tick handler",
            spell.id
        ),
    }
}

fn run_scripted_tick(world: &mut World, unit: Entity, spell: &SpellInfo, tick: &Tick) -> bool {
    let scripted = world
        .get_resource::<scripts::AuraScripts>()
        .and_then(|scripts| scripts.0.get(&spell.id))
        .is_some_and(|script| script.on_periodic.is_some());
    if scripted {
        let aura = world
            .get::<ActiveAuras>(unit)
            .and_then(|auras| auras.get(tick.instance_id))
            .cloned();
        if let Some(aura) = aura {
            scripts::run_periodic(world, unit, &aura, tick.effect_index);
        }
    }
    scripted
}

fn run_dummy_tick(world: &mut World, unit: Entity, spell: &SpellInfo, tick: &Tick) {
    if run_scripted_tick(world, unit, spell, tick) {
        return;
    }
    let Some(effect) = spell
        .effects
        .iter()
        .find(|effect| effect.index == tick.effect_index)
    else {
        return;
    };
    let Ok(trigger) = super::periodic_dummy_trigger(world, effect) else {
        return;
    };
    // Self-applied channel auras retain the channel's original unit target.
    // Non-channel auras trigger at their unit, as periodic aura23 does.
    let target = world
        .get::<CastState>(tick.caster)
        .filter(|cast| cast.spell_id == spell.id && cast.cast_type == CastType::Channel)
        .and_then(|cast| Entity::try_from_bits(cast.target))
        .unwrap_or(unit);
    crate::spell_cast::cast_triggered(
        world,
        tick.caster,
        trigger,
        Some(target),
        TICK_TRIGGER_DEPTH,
    );
}

/// `AuraEffect::HandleObsModPowerAuraTick` (SpellAuraEffects.cpp): the amount percent
/// of the MiscValue power's max (`CalculatePct`, truncated); nothing without that pool.
fn obs_mod_power(world: &mut World, ctx: &EffectCtx, unit: Entity, tick: &Tick) {
    let Some(power) = PowerType::from_db(tick.misc) else {
        debug!(
            "Spell {}: periodic power of unknown power {}",
            ctx.spell.id, tick.misc
        );
        return;
    };
    let max =
        crate::spell_cast::powers::power_value(world, unit, power).map_or(0.0, |(_, max)| max);
    if max == 0.0 {
        return;
    }
    effects::energize_unit(
        world,
        ctx,
        unit,
        power,
        (max * tick.amount / 100.0).trunc(),
        true,
    );
}

fn energize(world: &mut World, ctx: &EffectCtx, unit: Entity, tick: &Tick) {
    let Some(power) = PowerType::from_db(tick.misc) else {
        debug!(
            "Spell {}: periodic energize of unknown power {}",
            ctx.spell.id, tick.misc
        );
        return;
    };
    effects::energize_unit(world, ctx, unit, power, tick.amount, true);
}
