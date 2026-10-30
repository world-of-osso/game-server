//! Recent owner positions for followers to retrace safe turns.
use std::collections::VecDeque;

use crate::maps::InMap;
use crate::spell_cast::targets;
use bevy::prelude::*;

const SAMPLE_DISTANCE: f32 = 1.0;
const MAX_SAMPLES: usize = 256;
const TELEPORT_DISTANCE: f32 = 20.0;

#[derive(Component)]
struct OwnerTrail {
    map: InMap,
    points: VecDeque<Vec3>,
}

pub(super) fn record(world: &mut World, owner: Entity) {
    if world
        .get::<shared::components::CombatStatus>(owner)
        .is_some_and(|status| status.0)
    {
        world.entity_mut(owner).remove::<OwnerTrail>();
        return;
    }
    let Some(map) = world.get::<InMap>(owner).copied() else {
        return;
    };
    let Some(point) = targets::position(world, owner) else {
        return;
    };
    let mut actor = world.entity_mut(owner);
    let mut trail = actor
        .entry::<OwnerTrail>()
        .or_insert(OwnerTrail {
            map,
            points: VecDeque::new(),
        })
        .into_mut();
    if trail.map != map
        || trail
            .points
            .back()
            .is_some_and(|previous| previous.distance(point) > TELEPORT_DISTANCE)
    {
        trail.map = map;
        trail.points.clear();
    }
    if trail
        .points
        .back()
        .is_none_or(|previous| previous.distance(point) >= SAMPLE_DISTANCE)
    {
        trail.points.push_back(point);
        if trail.points.len() > MAX_SAMPLES {
            trail.points.pop_front();
        }
    }
}

pub(super) fn points(world: &World, owner: Entity, map: InMap) -> Vec<Vec3> {
    world
        .get::<OwnerTrail>(owner)
        .filter(|trail| trail.map == map)
        .map(|trail| trail.points.iter().copied().collect())
        .unwrap_or_default()
}
