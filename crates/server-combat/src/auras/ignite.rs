//! TC a352b1fa spell_mage.cpp1292–1328:12846 positive-hit damage ->12654.
//! Percent and tick count are native; generic ROLLING_PERIODIC owns unpaid carry.
use bevy::prelude::*;

use super::procs::AuraEvent;
use super::{ActiveAuras, AuraInstance};
use crate::spell_info::{SpellInfo, SpellStore};

pub(super) const MASTERY_IGNITE: u32 = 12846;
pub(super) const IGNITE: u32 = 12654;
const INTENSIFYING_FLAME: u32 = 416714;
const INTENSIFYING_FLAME_DAMAGE: u32 = 419800;
const PYROBLAST: u32 = 11366;
const FLAMESTRIKE: u32 = 2120;
const NATIVE_FLAMESTRIKE: u32 = 1254851;
const IGNITION: u32 = 1254756;

pub(super) fn check_damage(_world: &World, _aura: &AuraInstance, event: &AuraEvent) -> bool {
    // TC IsTriggeredByAura excludes the producer's own child. This scripted
    // child is not in EffectTriggerSpell, so retain that relationship here.
    event.damage > 0.0 && event.target.is_some() && event.spell_id != Some(IGNITE)
}

/// SimC76f8118 sc_mage.cpp2216–2279/4586–4590: Fire Blast spreads
/// the old owned bank before its own damage contributes another Ignite.
pub fn spread_ignite(world: &mut World, caster: Entity, primary: Entity, spell: &SpellInfo) {
    let Some(source) = owned_ignite(world, caster, primary).cloned() else {
        return;
    };
    let cap =
        crate::spell_cast::scripts::script_effect_value(world, caster, MASTERY_IGNITE, 4) as usize;
    if cap == 0 {
        return;
    }
    let percent = crate::spell_cast::scripts::script_effect_value(world, caster, MASTERY_IGNITE, 1);
    let source_bank = ignite_bank(&source) * f64::from(percent) / 100.0;
    for (bank, target) in find_spread_targets(world, caster, primary, spell)
        .into_iter()
        .take(cap)
    {
        if bank < source_bank {
            copy_ignite(world, caster, target, &source);
        }
    }
}

fn owned_ignite(world: &World, caster: Entity, target: Entity) -> Option<&AuraInstance> {
    let now = super::aura_now(world);
    world.get::<ActiveAuras>(target)?.iter().find(|aura| {
        aura.spell_id == IGNITE
            && aura.caster == caster
            && aura.expires_at.is_some_and(|expiry| expiry > now)
    })
}

fn ignite_effect(aura: &AuraInstance) -> &super::AuraEffectState {
    aura.effects
        .iter()
        .find(|effect| effect.index == 0 && effect.period_ms > 0)
        .expect("native Ignite requires periodic effect1")
}

fn fractional_ticks(aura: &AuraInstance) -> f64 {
    // SimC76f8118 dot.cpp672–678: bank includes the unpaid final fraction,
    // not the rounded integer exposed by the APL ticks_remaining expression.
    let effect = ignite_effect(aura);
    let expiry = aura
        .expires_at
        .expect("native Ignite requires finite expiry");
    let period = f64::from(effect.period_ms) / 1000.0;
    (1.0 + (expiry - effect.next_tick_at()) / period).max(0.0)
}

fn ignite_bank(aura: &AuraInstance) -> f64 {
    f64::from(ignite_effect(aura).amount) * fractional_ticks(aura)
}

fn find_spread_targets(
    world: &mut World,
    caster: Entity,
    primary: Entity,
    spell: &SpellInfo,
) -> Vec<(f64, Entity)> {
    use crate::spell_cast::targets::{self, Relation};
    let position = world
        .get::<shared::components::Position>(caster)
        .expect("native Fire Blast requires caster position");
    let center = Vec3::new(position.x, position.y, position.z);
    let radius = targets::spell_range(spell, true).1;
    let mut targets: Vec<_> =
        targets::area_units(world, caster, center, radius, Relation::Enemy, 0)
            .into_iter()
            .filter(|target| *target != primary)
            .map(|target| {
                let bank = owned_ignite(world, caster, target).map_or(0.0, ignite_bank);
                (bank, target)
            })
            .collect();
    targets.sort_by(|a, b| a.0.total_cmp(&b.0));
    targets
}

fn copy_ignite(world: &mut World, caster: Entity, target: Entity, source: &AuraInstance) {
    let now = super::aura_now(world);
    let previous = owned_ignite(world, caster, target).cloned();
    // Keep native aura admission, logging and application callbacks. Copying
    // below replaces the rolling deposit/phase rather than adding another bank.
    crate::spell_cast::cast_triggered_with_effect0_amount(
        world,
        caster,
        IGNITE,
        Some(target),
        1,
        ignite_effect(source).amount,
    );
    let Some(mut auras) = world.get_mut::<ActiveAuras>(target) else {
        return;
    };
    let Some(copy) = auras
        .auras
        .iter_mut()
        .find(|aura| aura.spell_id == IGNITE && aura.caster == caster)
    else {
        return;
    };
    overwrite_spread_state(copy, source, previous.as_ref(), now);
    auras.dirty = true;
    drop(auras);
    super::auras_changed(world, target);
}

fn overwrite_spread_state(
    copy: &mut AuraInstance,
    source: &AuraInstance,
    previous: Option<&AuraInstance>,
    now: f64,
) {
    // SimC76f8118 dot.cpp286–332/action.cpp4614–4620: a new dot starts
    // its own tick clock; a running dot clones source progress, with the
    // normal rolling refresh duration computed from its old pending tick.
    let remaining = source.expires_at.expect("native Ignite expiry") - now;
    let carry = previous
        .filter(|prior| fractional_ticks(prior) >= 1.0)
        .map_or(0.0, |prior| {
            (ignite_effect(prior).next_tick_at() - now).max(0.0)
        });
    let duration = remaining + carry;
    copy.duration_ms = (duration * 1000.0).round() as u32;
    copy.expires_at = Some(now + duration);
    copy.effects.clone_from(&source.effects);
    if previous.is_none() {
        for effect in copy
            .effects
            .iter_mut()
            .filter(|effect| effect.period_ms > 0)
        {
            effect.periodic_started_at = now;
            effect.next_tick_index = 1;
            effect.periodic_ticks = 0;
            effect.initial_period_pending = false;
        }
    }
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
    let inflame = world
        .get::<ActiveAuras>(unit)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == 417467))
        .map_or(0.0, |aura| {
            aura.effects
                .iter()
                .find(|effect| effect.index == 0)
                .expect("native Inflame requires effect1")
                .amount
        });
    let target_multiplier = event.target.map_or(1.0, |target| {
        controlled_destruction_multiplier(world, unit, target)
    });
    let ignition = flamestrike_ignite_multiplier(world, unit, event.spell_id);
    let amount =
        calculate_tick_amount(effect.amount, event, ticks, inflame) * target_multiplier * ignition;
    crate::spell_cast::cast_triggered_with_effect0_amount(
        world,
        unit,
        IGNITE,
        event.target,
        1,
        amount,
    );
}

fn flamestrike_ignite_multiplier(world: &World, caster: Entity, spell: Option<u32>) -> f32 {
    let flamestrike = matches!(spell, Some(FLAMESTRIKE | NATIVE_FLAMESTRIKE));
    let selected = world
        .get::<crate::class_progression::KnownSpells>(caster)
        .is_some_and(|known| known.contains(IGNITION));
    if !flamestrike || !selected {
        return 1.0;
    }
    // SimC76f8118 sc_mage.cpp3896–3898: only Flamestrike's new
    // Ignite contribution receives selected Ignition's native effect2.
    let percent = crate::spell_cast::scripts::script_effect_value(world, caster, IGNITION, 1);
    1.0 + percent / 100.0
}

fn controlled_destruction_multiplier(world: &World, caster: Entity, target: Entity) -> f32 {
    // SimC76f8118 sc_mage.cpp1945–1946: only this caster's earned target
    // debuff scales new Ignite contributions; the unpaid old bank is not rescaled.
    let percent = world
        .get::<ActiveAuras>(target)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 453268 && aura.caster == caster)
        })
        .map_or(0.0, |aura| {
            let amount = aura
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .expect("native Controlled Destruction requires effect1")
                .amount;
            amount * f32::from(aura.stacks)
        });
    1.0 + percent / 100.0
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

fn calculate_tick_amount(percent: f32, event: &AuraEvent, ticks: u32, inflame: f32) -> f32 {
    // snapshot_proc_modifiers admits only HS/LavaSurge. The latter's native
    // cast-time mask does not affect Pyroblast/Flamestrike, so this is HS's
    // actual launch marker, even after the aura was spent or a missile flew.
    let hot_streak = event.cast_used_proc_modifier
        && matches!(
            event.spell_id,
            Some(PYROBLAST | FLAMESTRIKE | NATIVE_FLAMESTRIKE)
        );
    // SimC76f8118 sc_mage.cpp2397–2399: Inflame multiplies only an
    // actual admitted Hot Streak contribution, not ordinary hardcasts.
    let percent = percent
        * if hot_streak {
            2.0 * (1.0 + inflame / 100.0)
        } else {
            1.0
        };
    // TC DamageInfo::GetDamage is uint32 before CalculatePct.
    let damage = event.damage as u32;
    damage as f32 * percent / 100.0 / ticks as f32
}
