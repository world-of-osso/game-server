//! Data-backed living encounter completion at a sourced health boundary.
use bevy::prelude::*;
use shared::components::{Health, Npc};

use crate::maps::{InMap, MapStore};

use super::instance_script::{self, EncounterState};

fn threshold(world: &World, unit: Entity, health: &Health) -> Option<(f32, bool)> {
    let map = world.get::<InMap>(unit)?;
    if map.instance_id == 0 {
        return None;
    }
    let entry = world.get::<Npc>(unit)?.template_id;
    let percent = world
        .get_resource::<MapStore>()?
        .encounter_health_thresholds
        .get(&(map.map_id, entry))?;
    Some(((health.max * percent / 100.0).max(1.0), *percent == 0.0))
}

/// Clamp lethal or threshold-crossing damage before death processing.
pub(crate) fn limit_loss(world: &World, unit: Entity, health: &Health, loss: f32) -> f32 {
    let Some((floor, lethal_only)) = threshold(world, unit, health) else {
        return loss;
    };
    if lethal_only && loss < health.current {
        return loss;
    }
    let limit = (health.current - floor).max(0.0);
    let safe_limit = if floor == 1.0 && limit >= health.current {
        health.current.next_down()
    } else {
        limit
    };
    loss.min(safe_limit)
}

/// Complete only on an actual crossing while the bound encounter is in progress.
pub(crate) fn complete_crossing(world: &mut World, unit: Entity, before: f32, incoming: f32) {
    let Some(health) = world.get::<Health>(unit) else {
        return;
    };
    let Some((floor, lethal_only)) = threshold(world, unit, health) else {
        return;
    };
    if lethal_only && incoming < before {
        return;
    }
    let rounded_lethal_floor =
        lethal_only && before - floor == before && health.current <= before - before.next_down();
    if (!lethal_only && before <= floor)
        || health.current <= 0.0
        || (health.current > floor && !rounded_lethal_floor)
    {
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
        if lethal_only {
            world
                .get_mut::<Health>(unit)
                .expect("living encounter actor")
                .current = floor;
        }
        super::lost_dwarves::leave_combat_friendly(world, unit);
    }
}
