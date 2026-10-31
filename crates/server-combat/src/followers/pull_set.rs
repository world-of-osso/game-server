//! Formation and transitive native assistance snapshots for a pull.
use bevy::prelude::*;
use std::collections::HashSet;

#[derive(Component, PartialEq, Eq)]
struct LoggedPull {
    target: Entity,
    count: usize,
}

pub(super) fn log_admission(world: &mut World, tank: Entity, target: Entity) {
    let planned = members(world, target, tank);
    let count = planned.len();
    let current = LoggedPull { target, count };
    if world
        .get::<LoggedPull>(tank)
        .is_none_or(|previous| previous.target != target)
    {
        let mut member_ids: Vec<_> = planned.into_iter().map(Entity::to_bits).collect();
        member_ids.sort_unstable();
        debug!(target: "game_server::follower_pull", tank = tank.to_bits(), target = target.to_bits(), ?member_ids, "planned pull closure");
    }
    if count > 1 && world.get::<LoggedPull>(tank) != Some(&current) {
        info!(target: "game_server::follower_pull", ?tank, ?target, count, "planned pull includes formation or assistance");
    }
    world.entity_mut(tank).insert(current);
}

fn assistants(world: &World, target: Entity, victim: Entity) -> Vec<Entity> {
    world
        .get::<crate::creature_assistance::AssistanceSnapshot>(target)
        .map(|snapshot| snapshot.0.clone())
        .unwrap_or_else(|| crate::creature_assistance::assistance_snapshot(world, target, victim))
}

pub(super) fn first_wave(world: &World, target: Entity, victim: Entity) -> HashSet<Entity> {
    let mut result: HashSet<_> = assistants(world, target, victim).into_iter().collect();
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

pub(super) fn risky_pending_helpers(
    world: &World,
    target: Entity,
    victim: Entity,
) -> HashSet<Entity> {
    let uncalled = world
        .get::<crate::creature_assistance::AssistanceCalled>(target)
        .is_none()
        .then_some(target);
    crate::creature_assistance::pending_callers(world)
        .chain(uncalled)
        .flat_map(|caller| {
            let first = first_wave(world, caller, victim);
            assistants(world, caller, victim)
                .into_iter()
                .filter(move |helper| {
                    world
                        .get::<crate::creature_assistance::AssistanceCalled>(*helper)
                        .is_none()
                        && first_wave(world, *helper, victim)
                            .iter()
                            .any(|member| !first.contains(member))
                })
        })
        .collect()
}

pub(super) fn members(world: &World, target: Entity, victim: Entity) -> HashSet<Entity> {
    let mut result = HashSet::from([target]);
    let mut pending = vec![target];
    while let Some(caller) = pending.pop() {
        for member in first_wave(world, caller, victim) {
            if result.insert(member) {
                pending.push(member);
            }
        }
    }
    result
}
