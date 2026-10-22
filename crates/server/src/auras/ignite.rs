//! TC a352b1fa spell_mage.cpp1292–1328:12846 positive-hit damage ->12654.
//! Percent and tick count are native; generic ROLLING_PERIODIC owns unpaid carry.
use bevy::prelude::*;

use super::AuraInstance;
use super::procs::AuraEvent;
use crate::spell_info::SpellStore;

pub(super) const MASTERY_IGNITE: u32 = 12846;
const IGNITE: u32 = 12654;
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
