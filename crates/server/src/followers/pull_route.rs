//! Complete baked route and native source aggro exposure for admission.
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use shared::components::UnitLevel;
use shared::level_scaling::{LevelScaling, level_for_viewer};
use std::collections::{BTreeMap, BTreeSet};

use super::pull_sources::{self, Source};
use crate::creature_aggro::AggroDistance;
use crate::maps::InMap;
#[cfg(test)]
use crate::navigation::Navigation;
#[path = "pull_route_paths.rs"]
mod paths;

type ReadPath =
    fn(&mut World, Entity, Vec3, InMap, paths::Key) -> Result<Option<Vec<Vec3>>, String>;

pub(super) struct Request<'a> {
    pub tank: Entity,
    pub target_guid: u32,
    pub difficulty: u32,
    pub cleared: &'a BTreeSet<u32>,
}

#[derive(Debug)]
pub(super) struct PullRoute {
    pub points: Vec<Vec3>,
    pub spawns: BTreeSet<u32>,
    pub pack_count: usize,
}

pub(super) fn read_route(
    world: &mut World,
    request: &Request<'_>,
) -> Result<Option<PullRoute>, String> {
    read_route_with_destination(world, request, None, read_complete_route)
}

pub(super) fn read_point_route(
    world: &mut World,
    request: &Request<'_>,
    destination: Vec3,
) -> Result<Option<PullRoute>, String> {
    read_route_with_destination(world, request, Some(destination), read_complete_route)
}

fn read_route_with_destination(
    world: &mut World,
    request: &Request<'_>,
    destination: Option<Vec3>,
    read_path: ReadPath,
) -> Result<Option<PullRoute>, String> {
    let map = *world
        .get::<InMap>(request.tank)
        .ok_or("Tank route lacks a map copy")?;
    let sources =
        pull_sources::read_present_sources(world, map, request.difficulty, request.cleared)?;
    let target = sources
        .get(&request.target_guid)
        .ok_or("Pull target is not a present source spawn")?;
    let actor = read_source_actor(target)?;
    if !crate::spell_cast::targets::is_alive(world, actor)
        || !crate::spell_cast::targets::relation_matches(
            world,
            request.tank,
            actor,
            crate::spell_cast::targets::Relation::Enemy,
        )
        || read_source_aggro_mode(world, actor)? == crate::creature_aggro::AggroMode::Passive
    {
        return Err("Pull target must be a living attackable non-passive source actor".into());
    }
    let key = destination.map_or(paths::Key::Target(request.target_guid), |_| {
        paths::Key::Point
    });
    let destination = match destination {
        Some(point) => point,
        None => crate::spell_cast::targets::position(world, actor)
            .ok_or("Source route target lacks a position")?,
    };
    let Some(points) = read_path(world, request.tank, destination, map, key)? else {
        return Ok(None);
    };
    if points.is_empty() {
        return Err("Pull route is empty".into());
    }
    let from = crate::spell_cast::targets::position(world, request.tank)
        .ok_or("Tank route lacks a position")?;
    let mut points = points;
    points.insert(0, from);
    let packs = read_exposed_packs(world, request.tank, &sources, &points, target.pack_guid)?;
    let spawns = sources
        .values()
        .filter(|source| packs.contains(&source.pack_guid))
        .map(|source| source.guid)
        .collect();
    Ok(Some(PullRoute {
        points,
        spawns,
        pack_count: packs.len(),
    }))
}

fn read_source_aggro_mode(
    world: &World,
    actor: Entity,
) -> Result<crate::creature_aggro::AggroMode, String> {
    let entry = world
        .get::<shared::components::Npc>(actor)
        .ok_or("Source actor lacks native template identity")?
        .template_id;
    // Difficulty0 is represented by the absence of CreatureDifficulty at native spawn.
    let difficulty = world
        .get::<crate::zones::CreatureDifficulty>(actor)
        .map_or(0, |difficulty| difficulty.0);
    let data = world
        .get_resource::<crate::world_data::WorldData>()
        .ok_or("Source actor lacks native world catalog")?;
    let template = data
        .template_for(entry, difficulty)
        .ok_or("Source actor lacks its native difficulty template")?;
    // Cached by the ordinary loader using the shared AggroMode::for_template rule.
    Ok(template.aggro_mode)
}

fn read_source_actor(source: &Source) -> Result<Entity, String> {
    source.actor.ok_or_else(|| format!("Source GUID {} entry {} at {:?} remains present but has no actor for native admission checks", source.guid, source.entry, source.position))
}

fn read_exposed_packs(
    world: &mut World,
    tank: Entity,
    sources: &BTreeMap<u32, Source>,
    points: &[Vec3],
    target_pack: u32,
) -> Result<BTreeSet<u32>, String> {
    let mut packs = BTreeSet::from([target_pack]);
    let mut geometry = SystemState::<AggroDistance>::new(world);
    for source in sources.values() {
        if route_enters_source(world, &mut geometry, tank, source, points)? {
            packs.insert(source.pack_guid);
        }
    }
    Ok(packs)
}

fn read_complete_route(
    world: &mut World,
    tank: Entity,
    to: Vec3,
    map: InMap,
    key: paths::Key,
) -> Result<Option<Vec<Vec3>>, String> {
    let from =
        crate::spell_cast::targets::position(world, tank).ok_or("Tank route lacks a position")?;
    if !from.is_finite() || !to.is_finite() {
        return Err("Pull route endpoints must be finite".into());
    }
    paths::read(world, tank, map, key, from, to)
}

#[cfg(test)]
pub(super) fn inspect_route(world: &mut World, request: &Request<'_>) -> Result<PullRoute, String> {
    read_route_with_destination(world, request, None, inspect_complete_route)?
        .ok_or_else(|| "Fixture route inspection unexpectedly pending".into())
}

#[cfg(test)]
pub(super) fn inspect_point_route(
    world: &mut World,
    request: &Request<'_>,
    point: Vec3,
) -> Result<PullRoute, String> {
    read_route_with_destination(world, request, Some(point), inspect_complete_route)?
        .ok_or_else(|| "Fixture point inspection unexpectedly pending".into())
}

#[cfg(test)]
fn inspect_complete_route(
    world: &mut World,
    tank: Entity,
    to: Vec3,
    map: InMap,
    _: paths::Key,
) -> Result<Option<Vec<Vec3>>, String> {
    let from =
        crate::spell_cast::targets::position(world, tank).ok_or("Tank route lacks a position")?;
    world
        .get_resource_mut::<Navigation>()
        .ok_or("Pull route requires authored navigation")?
        .query_path(map.map_id, from, to)?
        .map(Some)
        .ok_or_else(|| "Pull route has no authored navigation bake".into())
}

fn route_enters_source(
    world: &World,
    geometry: &mut SystemState<AggroDistance>,
    tank: Entity,
    source: &Source,
    route: &[Vec3],
) -> Result<bool, String> {
    let actor = read_source_actor(source)?;
    if !crate::spell_cast::targets::relation_matches(
        world,
        tank,
        actor,
        crate::spell_cast::targets::Relation::Enemy,
    ) || read_source_aggro_mode(world, actor)? != crate::creature_aggro::AggroMode::Aggressive
    {
        // Passive and reactive sources do not acquire proximity victims. A deliberately
        // attacked reactive target's whole formation is still included by target_pack.
        return Ok(false);
    }
    let radius = read_native_radius(world, geometry, tank, actor)
        .map_err(|error| format!("Source GUID {}: {error}", source.guid))?;
    let center = crate::spell_cast::targets::position(world, actor)
        .ok_or_else(|| format!("Source GUID {} lacks a position", source.guid))?;
    if !center.is_finite() || !radius.is_finite() || radius <= 0.0 {
        return Err(format!(
            "Source GUID {} has invalid native aggro geometry",
            source.guid
        ));
    }
    Ok(route
        .windows(2)
        .any(|leg| segment_distance_squared(center, leg[0], leg[1]) < radius * radius))
}

fn read_native_radius(
    world: &World,
    geometry: &mut SystemState<AggroDistance>,
    tank: Entity,
    actor: Entity,
) -> Result<f32, String> {
    let target_level = world
        .get::<UnitLevel>(tank)
        .ok_or("Tank lacks native level")?
        .0;
    let level = *world
        .get::<UnitLevel>(actor)
        .ok_or("Source actor lacks native level")?;
    let creature_level = level_for_viewer(level, world.get::<LevelScaling>(actor), target_level);
    geometry
        .get(world)
        .map_err(|error| format!("Native aggro geometry inputs are unavailable: {error:?}"))?
        .center_radius(actor, tank, creature_level, target_level)
        .ok_or_else(|| "Source actor lacks native aggro radius".into())
}

fn segment_distance_squared(center: Vec3, from: Vec3, to: Vec3) -> f32 {
    let delta = to - from;
    let length_squared = delta.length_squared();
    let fraction = if length_squared > 0.0 {
        ((center - from).dot(delta) / length_squared).clamp(0.0, 1.0)
    } else {
        0.0
    };
    center.distance_squared(from + delta * fraction)
}
