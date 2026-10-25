//! Test-only native state reads for the frozen engine AssistedCombat evaluator.
use super::super::{CastFailure, SpellHistory, charge_mods, history_now};
use crate::class_progression::KnownSpells;
use crate::spell_info::{SpellInfo, SpellStore};
use bevy::prelude::*;
use game_engine_core::assisted_combat::AssistedFacts;
use shared::components::{Health, Player, Position, ThreatTable};
use std::collections::BTreeSet;

pub fn read_facts(
    world: &mut World,
    actor: Entity,
    target: Entity,
    needed: &BTreeSet<u32>,
) -> Result<AssistedFacts, String> {
    let mut facts = read_actor_facts(world, actor, target)?;
    facts.player_combat_enemies_within_8yd = Some(count_nearby_enemies(world, actor)?);
    for &requested in needed {
        let effective = crate::auras::overrides::resolve_spell(world, actor, requested);
        if facts.known.contains(&effective) {
            facts.known.insert(requested);
        }
        let Some(spell) = world.resource::<SpellStore>().get(effective) else {
            let error = format!("spell{effective}: native metadata absent");
            facts.ready.insert(requested, Err(error.clone()));
            facts.affordable.insert(requested, Err(error.clone()));
            facts.in_range.insert(requested, Err(error));
            continue;
        };
        read_spell_facts(world, actor, target, requested, spell, &mut facts);
    }
    Ok(facts)
}

pub fn read_request_window(world: &World, actor: Entity, spell_id: u32) -> Result<bool, String> {
    let spell = world
        .resource::<SpellStore>()
        .get(spell_id)
        .ok_or_else(|| format!("native player request spell{spell_id} absent"))?;
    Ok(super::super::cast_queue_delay(world, actor, spell) as f32
        <= super::super::SPELL_QUEUE_WINDOW_SECS)
}

fn read_actor_facts(world: &World, actor: Entity, target: Entity) -> Result<AssistedFacts, String> {
    if world.get::<Player>(actor).is_none() {
        return Err("AssistedCombat measured actor is not a native Player".into());
    }
    let known = world
        .get::<KnownSpells>(actor)
        .ok_or("native KnownSpells absent")?;
    let pools = world
        .get::<super::super::powers::PowerPools>(actor)
        .ok_or("native player PowerPools absent")?;
    let powers = pools
        .order
        .iter()
        .map(|&power| {
            super::super::powers::power_value(world, actor, power)
                .map(|(current, _)| (power as u8, f64::from(current)))
                .ok_or_else(|| format!("native raw power{power:?} absent"))
        })
        .collect::<Result<_, _>>()?;
    let mut facts = AssistedFacts {
        known: known.iter().collect(),
        powers,
        player_auras: aura_ids(world, actor),
        target_auras: aura_ids(world, target),
        player_health_pct: health_pct(world, actor),
        target_health_pct: health_pct(world, target),
        target_distance_yd: Some(f64::from(distance(world, actor, target)?)),
        target_hostile: Some(super::super::targets::relation_matches(
            world,
            actor,
            target,
            super::super::targets::Relation::Enemy,
        )),
        has_pet: read_pet_presence(world, actor),
        ..Default::default()
    };
    if let Some(runes) = world.get::<super::super::runes::Runes>(actor) {
        facts.powers.insert(5, runes.ready() as f64);
    }
    Ok(facts)
}

fn aura_ids(world: &World, unit: Entity) -> BTreeSet<u32> {
    world
        .get::<crate::auras::ActiveAuras>(unit)
        .into_iter()
        .flat_map(|auras| auras.iter().map(|aura| aura.spell_id))
        .collect()
}

fn health_pct(world: &World, unit: Entity) -> Option<f64> {
    world
        .get::<Health>(unit)
        .filter(|health| health.max > 0.0)
        .map(|health| f64::from(health.current) * 100.0 / f64::from(health.max))
}

fn distance(world: &World, actor: Entity, target: Entity) -> Result<f32, String> {
    let origin =
        super::super::targets::position(world, actor).ok_or("native actor position absent")?;
    let point =
        super::super::targets::position(world, target).ok_or("native target position absent")?;
    Ok(origin.distance(point))
}

fn read_pet_presence(world: &World, actor: Entity) -> Option<bool> {
    match crate::hunter_pet::summoned_pet(world, actor) {
        None => Some(false),
        Some(pet) => world
            .get::<Health>(pet)
            .filter(|health| health.current > 0.0)
            .map(|_| true),
    }
}

fn count_nearby_enemies(world: &mut World, actor: Entity) -> Result<u32, String> {
    let origin =
        super::super::targets::position(world, actor).ok_or("native actor position absent")?;
    let copy = *world
        .get::<crate::maps::InMap>(actor)
        .ok_or("native actor map absent")?;
    let mut query = world.query::<(
        Entity,
        &Position,
        &Health,
        &crate::maps::InMap,
        &ThreatTable,
    )>();
    let candidates: Vec<_> = query
        .iter(world)
        .filter(|(_, position, health, map, threat)| {
            let point = Vec3::new(position.x, position.y, position.z);
            **map == copy
                && health.current > 0.0
                && origin.distance(point) <= 8.0
                && threat
                    .entries()
                    .iter()
                    .any(|entry| entry.entity == actor.to_bits())
        })
        .map(|(unit, ..)| unit)
        .collect();
    Ok(candidates
        .into_iter()
        .filter(|&unit| {
            super::super::targets::relation_matches(
                world,
                actor,
                unit,
                super::super::targets::Relation::Enemy,
            )
        })
        .count() as u32)
}

fn read_spell_facts(
    world: &World,
    actor: Entity,
    target: Entity,
    requested: u32,
    spell: &SpellInfo,
    facts: &mut AssistedFacts,
) {
    facts
        .ready
        .insert(requested, read_readiness(world, actor, spell));
    let affordable = super::super::applicable_costs(world, actor, spell)
        .map(|costs| costs.iter().all(|(_, amount, current)| current >= amount))
        .map_err(|error| format!("spell{}: native cost {error:?}", spell.id));
    facts.affordable.insert(requested, affordable);
    let in_range = super::super::validate_target(
        world,
        actor,
        spell,
        Some(target),
        super::super::Sight::Start,
    )
    .map(|_| true)
    .or_else(|error| match error {
        CastFailure::OutOfRange | CastFailure::TooClose => Ok(false),
        other => Err(format!(
            "spell{}: native target admission {other:?}",
            spell.id
        )),
    });
    facts.in_range.insert(requested, in_range);
    read_recovery_facts(world, actor, requested, spell, facts);
}

fn read_readiness(world: &World, actor: Entity, spell: &SpellInfo) -> Result<bool, String> {
    match super::super::read_healer_recovery(world, actor, spell) {
        Ok(()) => Ok(true),
        Err(CastFailure::OnCooldown | CastFailure::NoCharges | CastFailure::SchoolLockedOut) => {
            Ok(false)
        }
        Err(other) => Err(format!("spell{}: native recovery {other:?}", spell.id)),
    }
}

fn read_recovery_facts(
    world: &World,
    actor: Entity,
    requested: u32,
    spell: &SpellInfo,
    facts: &mut AssistedFacts,
) {
    let now = history_now(world);
    let mut history = world
        .get::<SpellHistory>(actor)
        .cloned()
        .unwrap_or_default();
    let category = spell.categories.as_ref().map_or(0, |row| row.category);
    let ready_at = history
        .spell_ready_at
        .get(&spell.id)
        .copied()
        .unwrap_or(0.0)
        .max(
            history
                .category_ready_at
                .get(&category)
                .copied()
                .unwrap_or(0.0),
        );
    facts
        .cooldowns_ms
        .insert(requested, (ready_at - now).max(0.0) * 1000.0);
    if let Some(charges) = history.charges_mut(spell, now, charge_mods(world, actor, spell)) {
        facts
            .charges
            .insert(requested, charges.snapshot(now).current);
    }
}
