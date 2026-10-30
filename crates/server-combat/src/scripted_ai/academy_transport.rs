//! Academy pad NPCs and log-derived destinations; arcs are project-authored.
use bevy::prelude::*;
use shared::components::{MovementControl, Npc, Player, Position};

use super::jump_movement::{JumpMovement, move_jump};
use crate::maps::InMap;

#[cfg(test)]
#[path = "academy_transport_tests.rs"]
mod tests;

pub(crate) const MAP: u32 = 2526;
const PAD: Vec3 = Vec3::new(1440.6, 951.742, 2827.3);
const LANDING: Vec3 = Vec3::new(1511.15, 900.57, 2926.35);
pub(crate) const PADS: &[(u32, Vec3, Vec3, f32)] = &[
    (
        195564,
        PAD,
        LANDING,
        crate::npc_interaction::INTERACTION_DISTANCE,
    ),
    (
        195564,
        Vec3::new(1881.92, 914.693, 3133.49),
        Vec3::new(2054.35, 925.0429, 3122.63),
        crate::npc_interaction::INTERACTION_DISTANCE,
    ),
    (
        192864,
        Vec3::new(2116.46, 928.646, 3105.17),
        Vec3::new(2306.25, 907.3357, 3052.91),
        crate::npc_interaction::INTERACTION_DISTANCE,
    ),
    (
        192864,
        Vec3::new(2318.13, 910.02, 3054.92),
        Vec3::new(2126.83, 925.12317, 3102.39),
        2.0,
    ),
    (
        195564,
        Vec3::new(2078.2, 925.406, 3062.87),
        Vec3::new(2028.9, 896.8801, 2931.39),
        crate::npc_interaction::INTERACTION_DISTANCE,
    ),
];

#[derive(Component)]
pub(crate) struct AcademyPadFlight {
    epoch: Option<u32>,
    target: Vec3,
}

/// Use each real pad NPC and its project-authored proximity boundary as the trigger.
pub(super) fn launch_entrance_pad(world: &mut World) {
    let pads: Vec<_> = world
        .query::<(&Npc, &Position, &InMap)>()
        .iter(world)
        .filter_map(|(npc, pos, map)| {
            if map.map_id != MAP {
                return None;
            }
            PADS.iter()
                .find(|(entry, start, _, _)| {
                    npc.template_id == *entry
                        && Vec3::new(pos.x, pos.y, pos.z).distance(*start) < 1.0
                })
                .map(|(_, _, target, radius)| (*pos, *map, *target, *radius))
        })
        .collect();
    let mut owners = Vec::new();
    for (pad, map, target, radius) in pads {
        owners.extend(world.query_filtered::<(Entity, &Position, &InMap, &MovementControl), (With<Player>, Without<AcademyPadFlight>)>()
            .iter(world).filter_map(|(entity, pos, current, control)| {
                (*current == map && !control.controlled
                    && Vec3::new(pos.x, pos.y, pos.z).distance(Vec3::new(pad.x, pad.y, pad.z)) <= radius)
                    .then_some((entity, map, target))
            }));
    }
    for (owner, map, target) in owners {
        let followers: Vec<Entity> = world
            .query::<(Entity, &crate::followers::Follower, &InMap)>()
            .iter(world)
            .filter_map(|(entity, follower, current)| {
                (follower.owner == owner && *current == map).then_some(entity)
            })
            .collect();
        let mut riders = vec![owner];
        riders.extend(followers);
        let companions: Vec<_> = riders
            .iter()
            .flat_map(|&rider| crate::maps::transfer::owned_companions(world, rider))
            .collect();
        riders.extend(companions);
        if riders.iter().any(|&rider| {
            world.get::<Position>(rider).is_none()
                || world.get::<JumpMovement>(rider).is_some()
                || world.get::<crate::combat::DeathState>(rider).is_some()
                || world
                    .get::<MovementControl>(rider)
                    .is_some_and(|control| control.controlled)
        }) {
            continue;
        }
        for rider in riders {
            let epoch = world.get_mut::<MovementControl>(rider).map(|mut control| {
                control.epoch = control.epoch.wrapping_add(1);
                control.controlled = true;
                control.epoch
            });
            world
                .entity_mut(rider)
                .insert(AcademyPadFlight { epoch, target });
            move_jump(
                world,
                rider,
                0,
                Position {
                    x: target.x,
                    y: target.y,
                    z: target.z,
                },
                50.0,
                0.0,
            );
            crate::spell_cast::mark_dirty(world, rider);
        }
    }
}

pub(super) fn finish_entrance_pad(
    mut commands: Commands,
    landed: Query<(Entity, &Position, &AcademyPadFlight), Without<JumpMovement>>,
) {
    for (entity, position, flight) in &landed {
        if Vec3::new(position.x, position.y, position.z).distance(flight.target) > 0.1 {
            error!("Academy pad flight ended away from the log-derived destination");
            continue;
        }
        let epoch = flight.epoch;
        commands.entity(entity).remove::<AcademyPadFlight>();
        commands.queue(move |world: &mut World| {
            if let Some(mut control) = world.get_mut::<MovementControl>(entity)
                && Some(control.epoch) == epoch
            {
                control.controlled = false;
                control.epoch = control.epoch.wrapping_add(1);
            }
            crate::spell_cast::mark_dirty(world, entity);
        });
    }
}
