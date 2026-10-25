//! Native30743 template action and TC a352b1fa at_telash_greywing_vault_rune.
use super::{SpellAreaTrigger, aura_owned, targets};
use crate::auras::{ActiveAuras, RemoveReason};
use crate::maps::{InMap, InstanceManager};
use bevy::prelude::*;
use shared::components::{Health, Player, Position, UnitFlags};
use std::collections::HashSet;

pub(super) const SCRIPT_NAME: &str = "at_telash_greywing_vault_rune";
pub(super) const CREATE_PROPERTIES: u32 = 26586;
pub(super) const TEMPLATE: u32 = 30743;
pub(super) const PROTECTION: u32 = 388072;
const SHIELD: u32 = 390201;
const INACTIVE: u32 = 390238;

pub(super) fn update(world: &mut World, field: Entity, previous: &SpellAreaTrigger) {
    let inside = collect_members(world, field);
    // AreaTrigger.cpp982–1003 publishes the whole new set before Enter then Exit.
    world.get_mut::<SpellAreaTrigger>(field).unwrap().inside = inside.clone();
    for &unit in inside.difference(&previous.inside) {
        enter(world, previous, unit);
    }
    for &unit in previous.inside.difference(&inside) {
        exit(world, field, previous, unit);
    }
}

fn collect_members(world: &mut World, field: Entity) -> HashSet<Entity> {
    let center = *world.get::<Position>(field).expect("Rune field position");
    let map = *world.get::<InMap>(field).expect("Rune field copy");
    // Native SearchUnits sets reqAlive=false; dead NPCs are not Player corpses.
    let units: Vec<_> = world
        .query_filtered::<(Entity, &Position, &InMap), With<Health>>()
        .iter(world)
        .filter(|(_, _, copy)| **copy == map)
        .map(|(unit, position, _)| (unit, *position))
        .collect();
    units
        .into_iter()
        .filter_map(|(unit, position)| {
            if !eligible_member(world, unit) {
                return None;
            }
            let Some(radius) = aura_owned::collision_radius(world, field, unit) else {
                warn!(
                    ?field,
                    ?unit,
                    "Rune collision requires source unit combat reach"
                );
                return None;
            };
            let origin = Vec3::new(center.x, center.y, center.z);
            let point = Vec3::new(position.x, position.y, position.z);
            (origin.distance_squared(point) < radius * radius).then_some(unit)
        })
        .collect()
}

fn eligible_member(world: &World, unit: Entity) -> bool {
    // ActionSetFlags0 does not allow uninteractible units or dead/ghost Players.
    if world
        .get::<UnitFlags>(unit)
        .is_some_and(|flags| !flags.is_selectable())
    {
        return false;
    }
    world.get::<Player>(unit).is_none() || living_player(world, unit)
}

fn living_player(world: &World, unit: Entity) -> bool {
    world.get::<Player>(unit).is_some()
        && world
            .get::<Health>(unit)
            .is_some_and(|health| health.current > 0.0)
        && world
            .get::<crate::death::PlayerDeathState>(unit)
            .is_none_or(|state| state.state == shared::death::DeathState::Alive)
}

fn enter(world: &mut World, trigger: &SpellAreaTrigger, unit: Entity) {
    // AreaTrigger.cpp1023–1025: template DoActions precedes script OnUnitEnter.
    if targets::relation_matches(world, trigger.caster, unit, trigger.relation) {
        cast_source(world, trigger.caster, trigger.trigger_spell, Some(unit));
    }
    if world.get::<Player>(unit).is_some() && world.get_entity(trigger.caster).is_ok() {
        let shielded = world
            .get::<ActiveAuras>(trigger.caster)
            .is_some_and(|auras| auras.iter().any(|aura| aura.spell_id == SHIELD));
        if !shielded {
            cast_source(world, trigger.caster, SHIELD, None);
        }
    }
}

fn exit(world: &mut World, field: Entity, trigger: &SpellAreaTrigger, unit: Entity) {
    // AreaTrigger.cpp1055–1061: caster-scoped UndoActions precedes OnUnitExit.
    aura_owned::undo_action(world, trigger, unit);
    if world.get::<Player>(unit).is_none() || world.get_entity(trigger.caster).is_err() {
        return;
    }
    let occupied = world
        .get::<SpellAreaTrigger>(field)
        .is_some_and(|stored| stored.inside.iter().any(|unit| living_player(world, *unit)));
    if !occupied {
        crate::auras::remove_auras(world, trigger.caster, RemoveReason::Cancelled, |aura| {
            aura.spell_id == SHIELD
        });
    }
}

pub(super) fn remove(world: &mut World, field: Entity, previous: &SpellAreaTrigger) {
    // AreaTrigger.cpp121–124: OnRemove sees original membership before all exits.
    deactivate(world, field, previous);
    world
        .get_mut::<SpellAreaTrigger>(field)
        .unwrap()
        .inside
        .clear();
    for &unit in &previous.inside {
        exit(world, field, previous, unit);
    }
}

fn deactivate(world: &mut World, field: Entity, previous: &SpellAreaTrigger) {
    let map = *world.get::<InMap>(field).expect("Rune field copy");
    let difficulty = world
        .get_resource::<InstanceManager>()
        .and_then(|instances| instances.copy_difficulty(map))
        .map(|difficulty| difficulty.difficulty_id);
    let Some(difficulty) = difficulty else {
        warn!(
            ?field,
            ?map,
            "Rune removal requires its native copy difficulty"
        );
        return;
    };
    if !matches!(difficulty, 23 | 8) || world.get_entity(previous.caster).is_err() {
        return;
    }
    if previous
        .inside
        .iter()
        .any(|unit| living_player(world, *unit))
    {
        cast_source(world, previous.caster, INACTIVE, Some(previous.caster));
    }
}

fn cast_source(world: &mut World, caster: Entity, spell: u32, target: Option<Entity>) {
    if let Err(error) = crate::spell_cast::creature_cast(world, caster, spell, target, true) {
        warn!(?caster, spell, ?error, "Native Rune cast failed");
    }
}
