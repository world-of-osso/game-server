//! TC a352b1fa spell_mage.cpp1292–1328:12846 positive-hit damage ->12654.
//! Percent and tick count are native; generic ROLLING_PERIODIC owns unpaid carry.
use bevy::prelude::*;

use super::procs::AuraEvent;
use super::{ActiveAuras, AuraInstance};
use crate::spell_info::SpellStore;

pub(super) const MASTERY_IGNITE: u32 = 12846;
pub(super) const IGNITE: u32 = 12654;
const INTENSIFYING_FLAME: u32 = 416714;
const INTENSIFYING_FLAME_DAMAGE: u32 = 419800;
const PYROBLAST: u32 = 11366;
const FLAMESTRIKE: u32 = 2120;
const NATIVE_FLAMESTRIKE: u32 = 1254851;

pub(super) fn check_damage(_world: &World, _aura: &AuraInstance, event: &AuraEvent) -> bool {
    // TC IsTriggeredByAura excludes the producer's own child. This scripted
    // child is not in EffectTriggerSpell, so retain that relationship here.
    event.damage > 0.0 && event.target.is_some() && event.spell_id != Some(IGNITE)
}

fn native_tick_count(world: &World) -> Option<u32> {
    let dot = world.resource::<SpellStore>().get(IGNITE)?;
    let duration = dot.misc.as_ref()?.duration?.duration_ms;
    let period = dot
        .effects
        .iter()
        .find(|effect| effect.index == 0)?
        .aura_period_ms;
    if duration <= 0 || period == 0 {
        return None;
    }
    let ticks = duration as u32 / period;
    (ticks > 0).then_some(ticks)
}

pub(super) fn apply_damage(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    event: &AuraEvent,
) {
    let Some(effect) = aura.effects.iter().find(|effect| effect.index == 0) else {
        error!("Native Ignite12846 has no effect0 percentage");
        return;
    };
    let Some(ticks) = native_tick_count(world) else {
        error!("Native Ignite12654 has no positive periodic tick count");
        return;
    };
    let amount = calculate_tick_amount(effect.amount, event, ticks);
    crate::spell_cast::cast_triggered_with_effect0_amount(
        world,
        unit,
        IGNITE,
        event.target,
        1,
        amount,
    );
}

/// FIRE_INTENSIFYING_FLAME_SIMC_TICK_BRIDGE, SimC76f8118 sc_mage.cpp2502–2567.
/// Run at the real tick boundary: deferred proc handling follows aura expiry,
/// when the final tick's base amount and active-dot identity are already gone.
/// Native416714 owns the cap, percentage, periodic mask and chance;419800 owns damage.
pub(super) fn trigger_intensifying_flame(
    world: &mut World,
    caster: Entity,
    target: Entity,
    base_tick: f32,
) {
    let Some((max_targets, percent, chance)) = read_intensifying_flame_proc(world, caster) else {
        return;
    };
    let active_dots = count_living_ignite_targets(world, caster);
    if active_dots as f32 > max_targets {
        return;
    }
    if chance < 100.0 && crate::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    crate::spell_cast::cast_triggered_with_effect0_amount(
        world,
        caster,
        INTENSIFYING_FLAME_DAMAGE,
        Some(target),
        1,
        base_tick * percent / 100.0,
    );
}

fn count_living_ignite_targets(world: &mut World, caster: Entity) -> usize {
    world
        .query::<(&ActiveAuras, &shared::components::Health)>()
        .iter(world)
        .filter(|(auras, health)| {
            health.current > 0.0
                && auras
                    .iter()
                    .any(|aura| aura.spell_id == IGNITE && aura.caster == caster)
        })
        .count()
}

fn read_intensifying_flame_proc(world: &World, caster: Entity) -> Option<(f32, f32, f32)> {
    let aura = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == INTENSIFYING_FLAME)?;
    let max_targets = aura.effects.iter().find(|effect| effect.index == 0)?.amount;
    let percent = aura.effects.iter().find(|effect| effect.index == 1)?.amount;
    let spell = world.resource::<SpellStore>().get(INTENSIFYING_FLAME)?;
    let options = spell.aura_options.as_ref()?;
    if options.proc_type_mask[0] & super::procs::PROC_DEAL_HARMFUL_PERIODIC == 0 {
        return None;
    }
    let chance = super::spell_mods::modify(
        world,
        caster,
        spell,
        super::SpellModOp::ProcChance,
        options.proc_chance as f32,
    );
    Some((max_targets, percent, chance))
}

fn calculate_tick_amount(percent: f32, event: &AuraEvent, ticks: u32) -> f32 {
    // snapshot_proc_modifiers admits only HS/LavaSurge. The latter's native
    // cast-time mask does not affect Pyroblast/Flamestrike, so this is HS's
    // actual launch marker, even after the aura was spent or a missile flew.
    let hot_streak = event.cast_used_proc_modifier
        && matches!(
            event.spell_id,
            Some(PYROBLAST | FLAMESTRIKE | NATIVE_FLAMESTRIKE)
        );
    let percent = percent * if hot_streak { 2.0 } else { 1.0 };
    // TC DamageInfo::GetDamage is uint32 before CalculatePct.
    let damage = event.damage as u32;
    damage as f32 * percent / 100.0 / ticks as f32
}
