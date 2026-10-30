//! World-static AreaTriggers, separate from DB2 portals and spell-owned fields.
//! TC a352b1fa AreaTrigger.cpp363–376: authored spawns have duration -1 and no caster.
pub(crate) use server_map_rules::world_area_triggers::WorldAreaTrigger;

use std::collections::HashSet;

use bevy::prelude::*;
use shared::components::{Player, Position, Rotation};

use crate::maps::{InMap, MapStore, store::AreaTriggerShape};

const CANDLELIGHT_SCRIPT: &str = "at_the_candle_king_candlelight";
const CANDLELIGHT: u32 = 426125;
const DARKLIGHT: u32 = 426127;

#[derive(Component, Default)]
struct CandlelightOccupants(HashSet<Entity>);

/// TC area trigger boxes use the authored extents, not DB2 teleport box lengths.
pub(crate) fn candlelight_contains(
    area: &WorldAreaTrigger,
    center: Position,
    rotation: Rotation,
    point: Position,
) -> bool {
    if area.script_name != CANDLELIGHT_SCRIPT || area.shape != 1 {
        return false;
    }
    let to_wow = |p: Position| Vec3::new(p.x, -p.z, p.y);
    let shape = AreaTriggerShape::Box {
        half_length: area.shape_data[0],
        half_width: area.shape_data[1],
        half_height: area.shape_data[2],
        yaw: rotation.y - std::f32::consts::FRAC_PI_2,
    };
    shape.contains(to_wow(center), to_wow(point))
}

/// TC at_the_candle_king_candlelight: enter applies candlelight and removes darklight;
/// exit removes candlelight. Followers participate as player-party companions.
pub(crate) fn update_candlelight(world: &mut World) {
    let areas: Vec<_> = world
        .query::<(
            Entity,
            &WorldAreaTrigger,
            &Position,
            &Rotation,
            &InMap,
            &CandlelightOccupants,
        )>()
        .iter(world)
        .map(|(entity, area, position, rotation, map, old)| {
            (
                entity,
                area.clone(),
                *position,
                *rotation,
                *map,
                old.0.clone(),
            )
        })
        .collect();
    for (entity, area, center, rotation, map, previous) in areas {
        let inside: HashSet<_> = world
            .query::<(
                Entity,
                &Position,
                &InMap,
                Has<Player>,
                Has<crate::followers::Follower>,
            )>()
            .iter(world)
            .filter(|(_, position, unit_map, player, follower)| {
                **unit_map == map
                    && (*player || *follower)
                    && candlelight_contains(&area, center, rotation, **position)
            })
            .map(|(unit, ..)| unit)
            .collect();
        for &unit in inside.difference(&previous) {
            crate::spell_cast::cast_triggered(world, unit, CANDLELIGHT, Some(unit), 0);
            crate::auras::remove_auras(world, unit, crate::auras::RemoveReason::Expired, |aura| {
                aura.spell_id == DARKLIGHT
            });
        }
        for &unit in previous.difference(&inside) {
            if world.get_entity(unit).is_ok() {
                crate::auras::remove_auras(
                    world,
                    unit,
                    crate::auras::RemoveReason::Expired,
                    |aura| aura.spell_id == CANDLELIGHT,
                );
            }
        }
        world
            .entity_mut(entity)
            .insert(CandlelightOccupants(inside));
    }
}

/// All created instance entry paths queue this once; there is no caster/aura lifetime.
/// The realm's existing Follower205→Normal1 population policy leaves source rows intact.
pub(crate) fn spawn_map(world: &mut World, map: InMap, difficulty: u32) {
    let population = crate::followers::population_difficulty(difficulty);
    let spawns = world
        .resource::<MapStore>()
        .world_area_triggers
        .spawns(map.map_id, population);
    for spawn in spawns {
        let area: WorldAreaTrigger = spawn.area;
        trace!(spawn = area.spawn_id, properties = area.create_properties_id,
            custom = area.create_properties_custom, entry = area.template_id,
            template_custom = area.template_custom, visual = ?area.visual_spell,
            shape = area.shape, data = ?area.shape_data, script = area.script_name,
            phase_flags = area.phase_use_flags, phase = area.phase_id, phase_group = area.phase_group,
            ?map, "Spawn world-static AreaTrigger");
        let candlelight = area.script_name == CANDLELIGHT_SCRIPT;
        let mut entity = world.spawn((
            area,
            spawn.location.bevy_position(),
            Rotation {
                x: 0.0,
                y: spawn.location.orientation + std::f32::consts::FRAC_PI_2,
                z: 0.0,
            },
            map,
        ));
        if candlelight {
            entity.insert(CandlelightOccupants::default());
        }
    }
}

pub(crate) fn spawn_hosted_continents(world: &mut World) {
    for map_id in crate::maps::HOSTED_CONTINENTS {
        spawn_map(world, InMap::continent(map_id), 0);
    }
}
