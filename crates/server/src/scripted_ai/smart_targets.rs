//! TrinityCore a352b1fa GetTargets(SMART_TARGET_INVOKER_PARTY).
use bevy::prelude::*;
use shared::components::Player;

pub(super) fn party_targets(world: &mut World, player: Entity) -> Vec<Entity> {
    if world.get::<Player>(player).is_none() {
        return Vec::new();
    }
    let members = world
        .get::<crate::persistence::ClientId>(player)
        .and_then(|id| {
            world
                .get_resource::<crate::group::GroupRegistry>()
                .map(|groups| groups.group_members(id.0))
        })
        .unwrap_or_default();
    if members.is_empty() {
        return vec![player];
    }
    let map = world.get::<crate::maps::InMap>(player).copied();
    world
        .query_filtered::<(
            Entity,
            &crate::persistence::ClientId,
            Option<&crate::maps::InMap>,
        ), With<Player>>()
        .iter(world)
        .filter(|(_, id, member_map)| members.contains(&id.0) && member_map.copied() == map)
        .map(|(entity, _, _)| entity)
        .collect()
}
