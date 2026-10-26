//! SimC76f8118 unique_gear_midnight.cpp3803–3859: native Font values and channel ticks.
use crate::auras::{ActiveAuras, AuraInstance, scripts::AuraScripts};
use crate::spell_cast::effects::EffectCtx;
use crate::spell_info::{SpellInfo, SpellStore};
use bevy::prelude::*;

pub(crate) const CHANNEL: u32 = 1297908;
pub(crate) const SPLATTER: u32 = 1307222;
const VALUES: u32 = 1297911;
const EXTRA_INITIAL_PERIOD: u32 = 0x200;
const REDUCED_TARGETS: f32 = 8.0;
const MAX_AOE_ENEMIES: usize = 20;

pub(super) fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(CHANNEL).or_default().on_apply = Some(apply_channel);
}

fn native_tick_count(world: &World) -> f32 {
    let spell = world
        .resource::<SpellStore>()
        .get(CHANNEL)
        .expect("native Font channel");
    let duration = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .expect("native Font duration")
        .duration_ms;
    let period = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Font periodic effect")
        .aura_period_ms;
    assert!(
        duration > 0 && period > 0,
        "native Font requires a finite tick schedule"
    );
    let initial = crate::spell_cast::attributes(spell, 5) & EXTRA_INITIAL_PERIOD != 0;
    (duration as u32 / period + u32::from(initial)) as f32
}

fn tick_amount(world: &World, caster: Entity, index: u8, level: Option<u16>) -> f32 {
    super::native_amount(world, caster, VALUES, index, level) * super::role(world, caster, CHANNEL)
        / native_tick_count(world)
}

fn apply_channel(world: &mut World, target: Entity, aura: &AuraInstance) {
    let amount = tick_amount(world, aura.caster, 0, aura.cast_item_level);
    let mut auras = world
        .get_mut::<ActiveAuras>(target)
        .expect("applied Font aura");
    let live = auras
        .get_mut(aura.instance_id)
        .expect("applied Font instance");
    live.effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("native Font periodic effect")
        .amount = amount;
}

pub(crate) fn after_periodic_damage(world: &mut World, ctx: &EffectCtx, target: Entity) {
    if ctx.spell.id != CHANNEL {
        return;
    }
    let aura = world
        .get::<ActiveAuras>(target)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == CHANNEL && aura.caster == ctx.caster)
        })
        .expect("ticking Font aura");
    let level = aura.cast_item_level;
    let amount = tick_amount(world, ctx.caster, 1, level);
    super::cast_amount(world, ctx.caster, target, SPLATTER, amount, level, None);
}

pub(crate) fn filter_targets(spell: &SpellInfo, primary: Option<Entity>, units: &mut Vec<Entity>) {
    if spell.id == SPLATTER {
        units.retain(|unit| Some(*unit) != primary);
    }
}

pub(crate) fn splatter_damage(world: &World, caster: Entity, _target: Entity, amount: f32) -> f32 {
    let frame = world
        .resource::<crate::spell_cast::ResolvingHits>()
        .0
        .last()
        .expect("native Splatter hit frame");
    assert_eq!((frame.caster, frame.spell_id), (caster, SPLATTER));
    let count = frame.selected.len().min(MAX_AOE_ENEMIES) as f32;
    if count > REDUCED_TARGETS {
        amount * (REDUCED_TARGETS / count).sqrt()
    } else {
        amount
    }
}
