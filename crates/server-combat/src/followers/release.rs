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
        release_if_reachable(world, unit, owner);
    }
}

fn release_if_reachable(world: &mut World, unit: Entity, owner: Entity) {
    let Some((map, members, _)) = party::read_admission(world, owner) else {
        return;
    };
    if !members.contains(&unit) || world.get::<InMap>(unit) != Some(&map) {
        return;
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
        return;
    }
    let Some(destination) = release_location(world, unit, map) else {
        return;
    };
    if destination.map_id != map.map_id {
        return;
    }
    let Some(owner_position) = world.get::<Position>(owner).copied() else {
        return;
    };
    let start = destination.bevy_position();
    let point = |position: Position| Vec3::new(position.x, position.y, position.z);
    let Some(mut nav) = world.get_resource_mut::<Navigation>() else {
        return;
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
    let Some(Ok(Some(Output::Path(path)))) = result else {
        return;
    };
    if !path.complete {
        return;
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
    if speed <= 0.0 || distance > speed * FOLLOWER_RELEASE_MAX_WALK_SECONDS {
        return;
    }
    world
        .resource_mut::<Navigation>()
        .cancel_query(unit, Goal::FollowerRelease);
    revive_at_release(world, unit, owner, &destination);
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
