//! World-static AreaTriggers, separate from DB2 portals and spell-owned fields.
//! TC a352b1fa AreaTrigger.cpp363–376: authored spawns have duration -1 and no caster.
mod catalog;

use bevy::prelude::*;
pub(crate) use catalog::{Catalog, load_catalog};
use shared::components::Rotation;

use crate::maps::{InMap, MapStore};

/// All created instance entry paths queue this once; there is no caster/aura lifetime.
/// The realm's existing Follower205→Normal1 population policy leaves source rows intact.
pub(crate) fn spawn_map(world: &mut World, map: InMap, difficulty: u32) {
    let population = crate::followers::population_difficulty(difficulty);
    let spawns = world
        .resource::<MapStore>()
        .world_area_triggers
        .spawns(map.map_id, population);
    for spawn in spawns {
        let area = spawn.area;
        trace!(spawn = area.spawn_id, properties = area.create_properties_id,
            custom = area.create_properties_custom, entry = area.template_id,
            template_custom = area.template_custom, visual = ?area.visual_spell,
            shape = area.shape, data = ?area.shape_data, script = area.script_name,
            phase_flags = area.phase_use_flags, phase = area.phase_id, phase_group = area.phase_group,
            ?map, "Spawn world-static AreaTrigger");
        world.spawn((
            area,
            spawn.location.bevy_position(),
            Rotation {
                x: 0.0,
                y: spawn.location.orientation + std::f32::consts::FRAC_PI_2,
                z: 0.0,
            },
            map,
        ));
    }
}

pub(crate) fn spawn_hosted_continents(world: &mut World) {
    for map_id in crate::maps::HOSTED_CONTINENTS {
        spawn_map(world, InMap::continent(map_id), 0);
    }
}
