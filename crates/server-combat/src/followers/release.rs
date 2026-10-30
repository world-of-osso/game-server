//! Release dead dungeon followers only after a complete native Follow route is proven.
use bevy::prelude::*;
use shared::components::{CombatStatus, Health, Mana, Position};

use super::{Follower, party};
use crate::maps::{InMap, MapStore, WorldLocation};
use crate::navigation::{Goal, Navigation};
use ::navigation::worker::{Output, Query};

const FOLLOWER_RELEASE_MAX_WALK_SECONDS: f32 = 120.0;

pub(super) fn update(world: &mut World) {
    let dead: Vec<_> = world
        .query::<(Entity, &Follower, &crate::combat::Corpse)>()
        .iter(world)
        .map(|(unit, follower, _)| (unit, follower.owner))
        .collect();
    for (unit, owner) in dead {
        let (decision, reason, walk_seconds, error) = release_if_reachable(world, unit, owner);
        info!(follower = ?unit, owner = ?owner, decision, reason, walk_seconds = ?walk_seconds, error = ?error, "Follower release evaluation");
    }
}

type Evaluation = (&'static str, &'static str, Option<f32>, Option<String>);

fn wait(reason: &'static str) -> Evaluation {
    ("wait", reason, None, None)
}

fn release_if_reachable(world: &mut World, unit: Entity, owner: Entity) -> Evaluation {
    let Some((map, members, _)) = party::read_admission(world, owner) else {
        return wait("no_admitted_party");
    };
    if !members.contains(&unit) || world.get::<InMap>(unit) != Some(&map) {
        return wait("not_in_admitted_copy");
    }
    let in_combat = std::iter::once(owner)
        .chain(members.iter().copied())
        .any(|member| {
            world
                .get::<CombatStatus>(member)
                .is_some_and(|status| status.0)
        });
    if in_combat {
        if let Some(mut nav) = world.get_resource_mut::<Navigation>() {
            nav.cancel_query(unit, Goal::FollowerRelease);
        }
        return wait("party_in_combat");
    }
    let Some(destination) = release_location(world, unit, map) else {
        return wait("no_release_location");
    };
    if destination.map_id != map.map_id {
        return wait("release_outside_instance");
    }
    let Some(owner_position) = world.get::<Position>(owner).copied() else {
        return wait("owner_position_missing");
    };
    let start = destination.bevy_position();
    let point = |position: Position| Vec3::new(position.x, position.y, position.z);
    let Some(mut nav) = world.get_resource_mut::<Navigation>() else {
        return wait("navigation_unavailable");
    };
    let result = nav.query_deferred(
        unit,
        map.map_id,
        Goal::FollowerRelease,
        Query::FollowPath {
            from: point(start),
            to: point(owner_position),
        },
    );
    let path = match result {
        None => return wait("route_pending"),
        Some(Err(error)) => return ("wait", "route_error", None, Some(error)),
        Some(Ok(None)) => return wait("navigation_unbaked"),
        Some(Ok(Some(Output::Path(path)))) => path,
        Some(Ok(Some(_))) => return wait("unexpected_route_result"),
    };
    if !path.complete {
        return wait("route_incomplete");
    }
    let speed = crate::creature_ai::BASE_RUN_SPEED
        * world
            .get::<crate::auras::AuraStatMods>(unit)
            .map_or(1.0, |mods| mods.movement_speed);
    let distance: f32 = path
        .points
        .windows(2)
        .map(|pair| pair[0].distance(pair[1]))
        .sum();
    if speed <= 0.0 {
        return wait("run_speed_zero");
    }
    let walk_seconds = distance / speed;
    if walk_seconds > FOLLOWER_RELEASE_MAX_WALK_SECONDS {
        return ("wait", "route_too_long", Some(walk_seconds), None);
    }
    world
        .resource_mut::<Navigation>()
        .cancel_query(unit, Goal::FollowerRelease);
    revive_at_release(world, unit, owner, &destination);
    ("release", "route_reachable", Some(walk_seconds), None)
}

fn release_location(world: &World, unit: Entity, map: InMap) -> Option<WorldLocation> {
    let position = world.get::<Position>(unit)?;
    let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
    let location = WorldLocation {
        map_id: map.map_id,
        position: Vec3::new(x, y, z),
        orientation: 0.0,
    };
    let maps = world.get_resource::<MapStore>()?;
    crate::death::dungeon_release_location(&location, maps.map(map.map_id)?, maps)
}

fn revive_at_release(world: &mut World, unit: Entity, owner: Entity, destination: &WorldLocation) {
    crate::maps::transfer::teleport_creature_with_companions(world, unit, destination);
    if let Some(mut health) = world.get_mut::<Health>(unit) {
        health.current = (health.max * crate::death::GHOST_RESURRECT_HEALTH_PCT).max(1.0);
    }
    if let Some(mut mana) = world.get_mut::<Mana>(unit) {
        mana.current = mana.max * crate::death::GHOST_RESURRECT_MANA_PCT;
    }
    world
        .entity_mut(unit)
        .remove::<(crate::combat::DeathState, crate::combat::Corpse)>()
        .insert((
            crate::creature_ai::CreatureState::Idle,
            crate::creature_ai::FollowOwner::new(owner),
        ));
    crate::spell_cast::mark_dirty(world, unit);
}
