//! User-approved portal at a log-correlated position; the flight is not authored.
use bevy::prelude::*;
use shared::components::{MovementControl, Player, Position};

use super::jump_movement::{JumpMovement, move_jump};
use crate::maps::InMap;

#[cfg(test)]
#[path = "voidscar_portal_tests.rs"]
mod tests;

pub(super) const SCRIPT: &str = "VOIDSCAR_ATROXUS_PORTAL_POSITION_LOG_CORRELATED";
const MAP: u32 = 2923;
const LANDING: Vec3 = Vec3::new(4761.38, -62.1763, 451.23);
const FLIGHT_SPEED: f32 = 50.0;

#[derive(Component)]
pub(super) struct PortalFlight {
    epoch: Option<u32>,
}

/// The interaction layer validates reach and player health before dispatch.
pub(super) fn use_portal(world: &mut World, owner: Entity, object: Entity) {
    let Some(map) = world.get::<InMap>(object).copied() else {
        return;
    };
    if map.map_id != MAP
        || world.get::<InMap>(owner) != Some(&map)
        || world.get::<Player>(owner).is_none()
    {
        return;
    }
    let followers: Vec<Entity> = world
        .query::<(Entity, &crate::followers::Follower, &InMap)>()
        .iter(world)
        .filter_map(|(entity, follower, current)| {
            (follower.owner == owner && *current == map).then_some(entity)
        })
        .collect();
    let mut riders = vec![owner];
    riders.extend(followers);
    if riders.iter().any(|&rider| {
        world.get::<Position>(rider).is_none()
            || world.get::<JumpMovement>(rider).is_some()
            || world.get::<crate::combat::DeathState>(rider).is_some()
            || world
                .get::<MovementControl>(rider)
                .is_some_and(|control| control.controlled)
    }) {
        return;
    }
    for rider in riders {
        let epoch = world.get_mut::<MovementControl>(rider).map(|mut control| {
            control.epoch = control.epoch.wrapping_add(1);
            control.controlled = true;
            control.epoch
        });
        world.entity_mut(rider).insert(PortalFlight { epoch });
        move_jump(
            world,
            rider,
            0,
            Position {
                x: LANDING.x,
                y: LANDING.y,
                z: LANDING.z,
            },
            FLIGHT_SPEED,
            0.0,
        );
        crate::spell_cast::mark_dirty(world, rider);
    }
}

pub(super) fn finish_portal_flight(
    mut commands: Commands,
    landed: Query<(Entity, &Position, &PortalFlight), Without<JumpMovement>>,
) {
    for (entity, position, flight) in &landed {
        if Vec3::new(position.x, position.y, position.z).distance(LANDING) > 0.1 {
            error!("Voidscar portal flight ended away from its log-correlated destination");
            continue;
        }
        let epoch = flight.epoch;
        commands.entity(entity).remove::<PortalFlight>();
        commands.queue(move |world: &mut World| {
            if let Some(mut control) = world.get_mut::<MovementControl>(entity)
                && Some(control.epoch) == epoch
            {
                control.controlled = false;
            }
            crate::spell_cast::mark_dirty(world, entity);
        });
    }
}
