//! Data-backed encounter actors completed by healing to full.
use bevy::prelude::*;
use shared::components::{Health, Npc};

use super::instance_script::{self, EncounterState};
use crate::maps::{InMap, MapStore};

fn start_percent(world: &World, unit: Entity) -> Option<f32> {
    let map = world.get::<InMap>(unit)?;
    if map.instance_id == 0 {
        return None;
    }
    let entry = world.get::<Npc>(unit)?.template_id;
    world
        .get_resource::<MapStore>()?
        .encounter_heal_objectives
        .get(&(map.map_id, entry))
        .copied()
}

/// Apply the authored initial health after creature scaling.
pub(crate) fn initialize(world: &mut World, unit: Entity) {
    let Some(percent) = start_percent(world, unit) else {
        return;
    };
    if let Some(mut health) = world.get_mut::<Health>(unit) {
        health.current = (health.max * percent / 100.0).max(1.0);
    }
    world
        .entity_mut(unit)
        .remove::<crate::spell_cast::powers::CreatureHealthRegenTimer>();
}

/// Read the native state of a data-backed healing objective.
pub fn encounter_state(world: &mut World, unit: Entity) -> Option<EncounterState> {
    start_percent(world, unit)?;
    let map = *world.get::<InMap>(unit)?;
    let boss = instance_script::bound_boss(world, unit)?;
    instance_script::get_boss_state(world, map, boss.boss_id)
}

/// Start the bound objective after its encounter-specific interaction.
pub(crate) fn start(world: &mut World, unit: Entity) {
    if start_percent(world, unit).is_none() {
        return;
    }
    let Some(map) = world.get::<InMap>(unit).copied() else {
        return;
    };
    let Some(boss) = instance_script::bound_boss(world, unit) else {
        return;
    };
    if instance_script::get_boss_state(world, map, boss.boss_id) == Some(EncounterState::NotStarted)
        && instance_script::set_boss_state(world, map, boss.boss_id, EncounterState::InProgress)
    {
        instance_script::send_engage_unit(world, map, unit);
    }
}

/// Objectives cannot be healed before their encounter starts.
pub fn accepts_healing(world: &mut World, unit: Entity, caster: Entity) -> bool {
    if start_percent(world, unit).is_none() {
        return true;
    }
    if super::temple_avatar::healing_blocked(world, unit, caster) {
        return false;
    }
    let Some(map) = world.get::<InMap>(unit).copied() else {
        return false;
    };
    let Some(boss) = instance_script::bound_boss(world, unit) else {
        return false;
    };
    matches!(
        instance_script::get_boss_state(world, map, boss.boss_id),
        Some(EncounterState::InProgress | EncounterState::Done)
    )
}

/// Process actual healing, not overheal or changes to maximum health.
pub(crate) fn healed(world: &mut World, unit: Entity, effective: f32) {
    if effective <= 0.0 || start_percent(world, unit).is_none() {
        return;
    }
    let Some(map) = world.get::<InMap>(unit).copied() else {
        return;
    };
    let Some(entry) = world.get::<Npc>(unit).map(|npc| npc.template_id) else {
        return;
    };
    let Some(boss) = instance_script::bound_boss(world, unit) else {
        return;
    };
    if instance_script::get_boss_state(world, map, boss.boss_id) != Some(EncounterState::InProgress)
    {
        return;
    }
    if world
        .get::<Health>(unit)
        .is_some_and(|health| health.current >= health.max)
    {
        instance_script::complete_bound_entry(world, map, entry, unit);
    }
}
