//! Vengeance lesser soul fragments heal from maximum health or recent damage.
use std::collections::VecDeque;

use bevy::prelude::*;
use shared::components::{ActiveSpec, Health};
use shared::protocol::{CombatLogEvent, CombatLogKind};

use super::combat;
use combat::auras::ActiveAuras;
use combat::spell_cast::effects::{self, EffectCtx, EffectHandler};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::{SpellEffect, SpellStore};

const PARAMETERS: u32 = 203783;
const LESSER_HEAL: u32 = 203794;
const SHATTERED_RESTORATION: u32 = 389824;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::HealthChangeObserved(record_damage),
    SpellScriptHook::EffectHandler(effect_handler),
];

#[derive(Component, Default)]
struct RecentDamage(VecDeque<(f64, f32)>);

fn parameter(world: &World, index: u8) -> f32 {
    world
        .resource::<SpellStore>()
        .get(PARAMETERS)
        .expect("Consume Soul parameters")
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .expect("Consume Soul parameter effect")
        .base_points
}

// SimC76f8118 player.cpp8324–8331: record damage after mitigation and absorbs.
fn record_damage(world: &mut World, event: &CombatLogEvent) {
    if event.kind != CombatLogKind::Damage {
        return;
    }
    let Some(target) = event.target.and_then(Entity::try_from_bits) else {
        return;
    };
    if !world
        .get::<ActiveSpec>(target)
        .is_some_and(|spec| spec.0 == 581)
    {
        return;
    }
    let amount = (event.amount - event.overflow.max(0)).max(0) as f32;
    if amount == 0.0 {
        return;
    }
    let now = combat::spell_cast::now(world);
    let cutoff = now - f64::from(parameter(world, 3));
    let mut entity = world.entity_mut(target);
    let mut history = entity.entry::<RecentDamage>().or_default();
    let mut history = history.get_mut();
    while history.0.front().is_some_and(|(time, _)| *time < cutoff) {
        history.0.pop_front();
    }
    history.0.push_back((now, amount));
}

fn effect_handler(spell: u32, index: u8) -> Option<EffectHandler> {
    (spell == LESSER_HEAL && index == 0).then_some(heal_lesser)
}

// SimC76f8118 sc_demon_hunter.cpp3617–3668: only lesser Vengeance souls
// use the larger of recent-damage and max-health percentages.
fn heal_lesser(world: &mut World, ctx: &EffectCtx<'_>, effect: &SpellEffect, target: Entity) {
    if !world
        .get::<ActiveSpec>(ctx.caster)
        .is_some_and(|spec| spec.0 == 581)
    {
        return;
    }
    let max_health = world
        .get::<Health>(target)
        .expect("Consume Soul target health")
        .max;
    let cutoff = combat::spell_cast::now(world) - f64::from(parameter(world, 3));
    let recent = world.get::<RecentDamage>(target).map_or(0.0, |history| {
        history
            .0
            .iter()
            .filter(|(time, _)| *time >= cutoff)
            .map(|(_, amount)| amount)
            .sum::<f32>()
    });
    let restoration = world
        .get::<ActiveAuras>(ctx.caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == SHATTERED_RESTORATION && aura.caster == ctx.caster)
                .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
                .map(|effect| effect.amount)
        })
        .unwrap_or(0.0);
    let mut resolved = effect.clone();
    resolved.base_points = (max_health * parameter(world, 2) / 100.0)
        .max(recent * parameter(world, 1) / 100.0)
        * (1.0 + restoration / 100.0);
    effects::handler(effects::EFFECT_HEAL).expect("native heal handler")(
        world, ctx, &resolved, target,
    );
}
