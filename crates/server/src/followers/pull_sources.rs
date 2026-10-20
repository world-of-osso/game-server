//! Source presence and formation identity for pull admission, independent of visibility.
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

use crate::maps::InMap;
use crate::world_data::{CreatureSpawn, WorldData};
use crate::zones::CreatureOrigin;

#[derive(Debug)]
pub(super) struct Source {
    pub guid: u32,
    pub entry: u32,
    pub pack_guid: u32,
    pub position: Vec3,
    pub actor: Option<Entity>,
}

pub(super) fn read_present_sources(
    world: &mut World,
    map: InMap,
    difficulty: u32,
    cleared: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, Source>, String> {
    let actors = read_source_actors(world, map)?;
    let data = world
        .get_resource::<WorldData>()
        .ok_or("Pull admission requires the source world catalog")?;
    let map_id = u16::try_from(map.map_id).map_err(|_| "Source map ID exceeds catalog range")?;
    let indices = data
        .spawns_by_map
        .get(&map_id)
        .ok_or_else(|| format!("Pull admission lacks source spawns for map {}", map.map_id))?;
    let present = indices
        .iter()
        .map(|&index| &data.spawns[index])
        .filter(|spawn| spawn.spawns_at(difficulty) && !cleared.contains(&spawn.guid));
    present
        .map(|spawn| read_source(data, spawn, &actors).map(|source| (source.guid, source)))
        .collect()
}

fn read_source_actors(
    world: &mut World,
    map: InMap,
) -> Result<BTreeMap<u32, (Entity, u32)>, String> {
    let mut actors = BTreeMap::new();
    let units = world.resource::<crate::maps::units::MapUnits>();
    for unit in units.in_map(map) {
        let Some(origin) = world.get::<CreatureOrigin>(unit) else {
            continue;
        };
        if origin.guid == 0 {
            continue;
        }
        if actors
            .insert(origin.guid, (unit, origin.original_entry))
            .is_some()
        {
            return Err(format!(
                "Ambiguous source GUID {} in copy {map:?}",
                origin.guid
            ));
        }
    }
    Ok(actors)
}

fn read_source(
    data: &WorldData,
    spawn: &CreatureSpawn,
    actors: &BTreeMap<u32, (Entity, u32)>,
) -> Result<Source, String> {
    let actor = actors.get(&spawn.guid).copied();
    if actor.is_some_and(|(_, entry)| entry != spawn.template_entry) {
        return Err(format!(
            "Source GUID {} has a mismatched original template",
            spawn.guid
        ));
    }
    let (x, y, z) =
        crate::terrain_height::wow_to_bevy(spawn.position_x, spawn.position_y, spawn.position_z);
    let position = Vec3::new(x, y, z);
    if !position.is_finite() {
        return Err(format!(
            "Source GUID {} lacks a finite position",
            spawn.guid
        ));
    }
    Ok(Source {
        guid: spawn.guid,
        entry: spawn.template_entry,
        pack_guid: super::pack_identity::pack_guid(
            spawn.guid,
            data.formations
                .get(&spawn.guid)
                .map(|formation| formation.leader_guid),
        ),
        position,
        actor: actor.map(|(unit, _)| unit),
    })
}
