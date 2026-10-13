//! Native Chrono Flame431442 fields0/2(percent),1(seconds), override431443.
//! DescriptionVariables852: cap=SP*2.5*(1+versadmg), Overclock1260647
//! adds its native40%, corroborated by cap2=SP*3.5*(1+versadmg).
use crate::spell_cast::{
    CastFailure,
    effects::{self, EffectCtx},
};
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use shared::components::ActiveSpec;
use shared::protocol::{CombatLogEvent, CombatLogKind};
use std::collections::VecDeque;

pub(super) const PASSIVE: u32 = 431442;
pub(super) const CHRONO_FLAMES: u32 = 431443;
const ARCANE_HEAL: u32 = 431483;
const ARCANE_DAMAGE: u32 = 431583;
const OVERCLOCK: u32 = 1260647;
const PRESERVATION: u32 = 1468;
const SPELLPOWER_CAP: f32 = 2.5;

struct HealthChange {
    target: u64,
    healing: bool,
    at: f64,
    amount: f32,
}

#[derive(Component, Default)]
struct RecentHealthChanges(VecDeque<HealthChange>);

fn history_seconds(world: &World, caster: Entity) -> f64 {
    f64::from(super::script_effect_value(world, caster, PASSIVE, 1))
}

/// Retain only this active learner's effective damage/healing, not overheal,
/// overkill, misses or another actor's events. State dies with the caster entity.
fn parse_health_change(event: &CombatLogEvent, at: f64) -> Option<(Entity, HealthChange)> {
    let healing = match event.kind {
        CombatLogKind::Damage => false,
        CombatLogKind::Heal => true,
        _ => return None,
    };
    let actor = event.source.and_then(Entity::try_from_bits)?;
    let target = event.target?;
    let amount = (event.amount - event.overflow).max(0) as f32;
    (amount > 0.0).then_some((
        actor,
        HealthChange {
            target,
            healing,
            at,
            amount,
        },
    ))
}

pub(crate) fn record_health_change(world: &mut World, event: &CombatLogEvent) {
    let at = crate::spell_cast::now(world);
    let Some((actor, change)) = parse_health_change(event, at) else {
        return;
    };
    if !crate::auras::has_aura(world, actor, PASSIVE) {
        return;
    }
    let cutoff = at - history_seconds(world, actor);
    if world.get::<RecentHealthChanges>(actor).is_none() {
        world
            .entity_mut(actor)
            .insert(RecentHealthChanges::default());
    }
    let mut history = world
        .get_mut::<RecentHealthChanges>(actor)
        .expect("learner history");
    while history.0.front().is_some_and(|entry| entry.at < cutoff) {
        history.0.pop_front();
    }
    history.0.push_back(change);
}

fn recent_amount(world: &World, caster: Entity, target: Entity, healing: bool) -> f32 {
    let cutoff = crate::spell_cast::now(world) - history_seconds(world, caster);
    world
        .get::<RecentHealthChanges>(caster)
        .map_or(0.0, |history| {
            history
                .0
                .iter()
                .filter(|entry| {
                    entry.target == target.to_bits()
                        && entry.healing == healing
                        && entry.at >= cutoff
                })
                .map(|entry| entry.amount)
                .sum()
        })
}

fn repeat_amount(world: &World, caster: Entity, target: Entity, healing: bool) -> f32 {
    let index = if world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == PRESERVATION)
    {
        0
    } else {
        2
    };
    let percent = super::script_effect_value(world, caster, PASSIVE, index);
    let bonus = if crate::auras::has_aura(world, caster, OVERCLOCK) {
        super::script_effect_value(world, caster, OVERCLOCK, 0) / 100.0
    } else {
        0.0
    };
    let (spellpower, _) = effects::power_stats(world, caster, healing);
    let cap = spellpower
        * SPELLPOWER_CAP
        * (1.0 + bonus)
        * effects::versatility_multiplier(world, caster);
    (recent_amount(world, caster, target, healing) * percent / 100.0)
        .min(cap)
        .max(0.0)
}

/// Validate the actual scripted children before payment; absent native records
/// remain an explicit catalog failure, not a missing-effect no-op or panic.
pub(super) fn check_children(world: &World) -> Result<(), CastFailure> {
    let store = world.resource::<SpellStore>();
    for id in [
        PASSIVE,
        super::LIVING_FLAME_DAMAGE,
        super::LIVING_FLAME_HEAL,
        ARCANE_DAMAGE,
        ARCANE_HEAL,
    ] {
        if store.get(id).is_none() {
            return Err(CastFailure::Unsupported(format!(
                "Chrono Flame missing native child {id}"
            )));
        }
    }
    Ok(())
}

fn resolve_repeat(
    world: &mut World,
    caster: Entity,
    target: Entity,
    depth: u8,
    healing: bool,
    amount: f32,
) {
    let id = if healing { ARCANE_HEAL } else { ARCANE_DAMAGE };
    let spell = world
        .resource::<SpellStore>()
        .get(id)
        .expect("validated Chrono child")
        .clone();
    let ctx = EffectCtx {
        caster,
        spell: &spell,
        depth: depth + 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    if healing {
        effects::apply_fixed_heal(world, &ctx, target, amount);
    } else {
        effects::deal_copied_damage(world, &ctx, target, amount);
    }
}

pub(super) fn repeat_landed_flame(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    let healing = crate::spell_cast::targets::relation_matches(
        world,
        caster,
        target,
        crate::spell_cast::targets::Relation::Ally,
    );
    if !crate::auras::has_aura(world, caster, PASSIVE) {
        return;
    }
    // Native Living Flame children can land after the root due to missiles.
    // Read history from the actual child hit, including that primary hit; Retail
    // historical inclusion/latency parity is not independently measured.
    let amount = repeat_amount(world, caster, target, healing);
    if amount <= 0.0 {
        return;
    }
    resolve_repeat(world, caster, target, depth, healing, amount);
}
