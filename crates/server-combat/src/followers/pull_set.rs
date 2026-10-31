//! The native formation and immediate assistance snapshot of one pull.
use bevy::prelude::*;
use std::collections::HashSet;

#[derive(Component, PartialEq, Eq)]
struct LoggedPull {
    target: Entity,
    count: usize,
}

pub(super) fn log_admission(world: &mut World, tank: Entity, target: Entity) {
    let count = members(world, target, tank).len();
    let current = LoggedPull { target, count };
    if count > 1 && world.get::<LoggedPull>(tank) != Some(&current) {
        info!(target: "game_server::follower_pull", ?tank, ?target, count, "planned pull includes formation or assistance");
    }
    world.entity_mut(tank).insert(current);
}

pub(super) fn members(world: &World, target: Entity, victim: Entity) -> HashSet<Entity> {
    let mut result: HashSet<_> = if let Some(snapshot) =
        world.get::<crate::creature_assistance::AssistanceSnapshot>(target)
    {
        snapshot.0.iter().copied().collect()
    } else if world
        .get::<crate::creature_assistance::AssistanceCalled>(target)
        .is_some()
    {
        HashSet::new()
    } else {
        crate::creature_assistance::assistance_snapshot(world, target, victim)
            .into_iter()
            .collect()
    };
    result.insert(target);
    if let (Some(map), Some(units)) = (
        world.get::<crate::maps::InMap>(target),
        world.get_resource::<crate::maps::units::MapUnits>(),
    ) {
        result.extend(units.in_map(*map).filter(|&candidate| {
            super::positioning::in_target_formation(world, target, candidate)
        }));
    }
    result
}
