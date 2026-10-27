//! Test-only native state reads for the frozen engine AssistedCombat evaluator.
use super::super::{CastFailure, SpellHistory, charge_mods, history_now};
use crate::class_progression::KnownSpells;
use crate::spell_info::{SpellInfo, SpellStore};
use bevy::prelude::*;
use game_engine_core::assisted_combat::AssistedFacts;
use shared::components::{Health, Player, ThreatTable};
use std::collections::{BTreeMap, BTreeSet};

pub fn read_facts(
    world: &mut World,
    actor: Entity,
    target: Entity,
    needed: &BTreeSet<u32>,
) -> Result<AssistedFacts, String> {
    read_facts_with_dots(world, actor, target, needed, &BTreeSet::new())
}

pub fn read_facts_with_dots(
    world: &mut World,
    actor: Entity,
    target: Entity,
    needed: &BTreeSet<u32>,
    dot_spells: &BTreeSet<u32>,
) -> Result<AssistedFacts, String> {
    let mut facts = read_actor_facts(world, actor, target)?;
    let enemies = combat_enemies(world, actor)?;
    let origin =
        super::super::targets::position(world, actor).ok_or("native actor position absent")?;
    facts.active_enemies = Some(enemies.len() as u32);
    facts.player_combat_enemies_within_8yd = Some(
        enemies
            .iter()
            .filter(|&&unit| {
                super::super::targets::position(world, unit)
                    .is_some_and(|point| origin.distance(point) <= 8.0)
            })
            .count() as u32,
    );
    facts.active_dot_counts = Some(read_active_dot_counts(world, actor, &enemies, dot_spells));
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
        player_aura_stacks: Some(aura_stacks(world, actor)),
        target_aura_stacks: Some(aura_stacks(world, target)),
        player_aura_remaining_ms: Some(aura_remaining_ms(world, actor)),
        target_aura_remaining_ms: Some(aura_remaining_ms(world, target)),
        mana_pct: super::super::powers::power_value(
            world,
            actor,
            shared::components::PowerType::Mana,
        )
        .filter(|(_, max)| *max > 0.0)
        .map(|(current, max)| f64::from(current) * 100.0 / f64::from(max)),
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

fn aura_stacks(world: &World, unit: Entity) -> std::collections::BTreeMap<u32, u8> {
    let mut stacks = std::collections::BTreeMap::new();
    for aura in world
        .get::<crate::auras::ActiveAuras>(unit)
        .into_iter()
        .flat_map(|auras| auras.iter())
    {
        stacks
            .entry(aura.spell_id)
            .and_modify(|count: &mut u8| *count = (*count).max(aura.stacks))
            .or_insert(aura.stacks);
    }
    stacks
}

fn aura_remaining_ms(world: &World, unit: Entity) -> BTreeMap<u32, f64> {
    let now = crate::auras::aura_now(world);
    let mut values = BTreeMap::new();
    for aura in world
        .get::<crate::auras::ActiveAuras>(unit)
        .into_iter()
        .flat_map(|auras| auras.iter())
    {
        let remaining = aura
            .expires_at
            .map_or(f64::MAX, |expiry| ((expiry - now) * 1000.0).max(0.0));
        values
            .entry(aura.spell_id)
            .and_modify(|old: &mut f64| *old = old.max(remaining))
            .or_insert(remaining);
    }
    values
}

fn read_active_dot_counts(
    world: &World,
    actor: Entity,
    enemies: &[Entity],
    dot_spells: &BTreeSet<u32>,
) -> BTreeMap<u32, u32> {
    let store = world.resource::<SpellStore>();
    dot_spells
        .iter()
        .filter_map(|&spell| {
            let info = store.get(spell)?;
            if !info
                .effects
                .iter()
                .any(|effect| effect.aura == 3 && effect.aura_period_ms > 0)
            {
                return None;
            }
            let count = enemies
                .iter()
                .filter(|&&enemy| {
                    world
                        .get::<crate::auras::ActiveAuras>(enemy)
                        .is_some_and(|auras| {
                            auras
                                .iter()
                                .any(|aura| aura.spell_id == spell && aura.caster == actor)
                        })
                })
                .count();
            Some((spell, count as u32))
        })
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

fn combat_enemies(world: &mut World, actor: Entity) -> Result<Vec<Entity>, String> {
    let copy = *world
        .get::<crate::maps::InMap>(actor)
        .ok_or("native actor map absent")?;
    let mut query = world.query::<(Entity, &Health, &crate::maps::InMap, &ThreatTable)>();
    let candidates: Vec<_> = query
        .iter(world)
        .filter(|(_, health, map, threat)| {
            **map == copy
                && health.current > 0.0
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
        .collect())
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
        // SimC76f8118 monk/sc_monk.cpp2675–2689: an ineligible Touch of Death
        // target makes the action unavailable, not an evaluation error.
        CastFailure::OutOfRange | CastFailure::TooClose | CastFailure::InvalidTarget => Ok(false),
        other => Err(format!(
            "spell{}: native target admission {other:?}",
            spell.id
        )),
    });
    facts.in_range.insert(requested, in_range);
    read_recovery_facts(world, actor, requested, spell, facts);
}

fn read_readiness(world: &World, actor: Entity, spell: &SpellInfo) -> Result<bool, String> {
    // SimC76f8118 monk/sc_monk.cpp4550–4565: WDP's cooldown window gates readiness.
    // Native aura restrictions enforce the corresponding earned caster window.
    match super::super::aura_spell_restrictions::check_caster(world, actor, spell) {
        Ok(()) => {}
        Err(CastFailure::CasterAuraState) => return Ok(false),
        Err(other) => return Err(format!("spell{}: native caster aura {other:?}", spell.id)),
    }
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
