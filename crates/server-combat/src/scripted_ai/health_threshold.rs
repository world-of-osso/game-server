//! Data-backed living encounter completion at a sourced health boundary.
use bevy::prelude::*;
use shared::components::{Health, Npc};

use crate::maps::{InMap, MapStore};

use super::instance_script::{self, EncounterState};

fn threshold(world: &World, unit: Entity, health: &Health) -> Option<f32> {
    let map = world.get::<InMap>(unit)?;
    if map.instance_id == 0 {
        return None;
    }
    let entry = world.get::<Npc>(unit)?.template_id;
    let percent = world
        .get_resource::<MapStore>()?
        .encounter_health_thresholds
        .get(&(map.map_id, entry))?;
    Some((health.max * percent / 100.0).max(1.0))
}

/// Clamp lethal or threshold-crossing damage before death processing.
pub(crate) fn limit_loss(world: &World, unit: Entity, health: &Health, loss: f32) -> f32 {
    let Some(floor) = threshold(world, unit, health) else {
        return loss;
    };
    loss.min((health.current - floor).max(0.0))
}

/// Complete only on an actual crossing while the bound encounter is in progress.
pub(crate) fn complete_crossing(world: &mut World, unit: Entity, before: f32) {
    let Some(health) = world.get::<Health>(unit) else {
        return;
    };
    let Some(floor) = threshold(world, unit, health) else {
        return;
    };
    if before <= floor || health.current > floor || health.current <= 0.0 {
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
    if instance_script::complete_bound_entry(world, map, entry, unit) {
        super::lost_dwarves::leave_combat_friendly(world, unit);
    }
}
