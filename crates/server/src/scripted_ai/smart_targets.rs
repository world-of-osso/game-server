//! TrinityCore a352b1fa GetTargets(SMART_TARGET_INVOKER_PARTY).
use bevy::prelude::*;
use shared::components::{Health, Npc, Player, Position};

const AURA_FEIGN_DEATH: u32 = 66;

pub(super) fn nearest_creature(
    world: &mut World,
    me: Entity,
    entry: u32,
    radius: f32,
    alive_state: u32,
) -> Option<Entity> {
    let origin = *world.get::<Position>(me)?;
    let map = world.get::<crate::maps::InMap>(me).copied();
    world
        .query::<(
            Entity,
            &Npc,
            &Position,
            &Health,
            Option<&crate::maps::InMap>,
            Option<&crate::combat::DeathState>,
            Option<&crate::auras::ActiveAuras>,
        )>()
        .iter(world)
        .filter(|(entity, npc, _, _, candidate_map, _, _)| {
            *entity != me
                && (entry == 0 || npc.template_id == entry)
                && candidate_map.copied() == map
        })
        .filter(|(_, _, _, health, _, death, auras)| {
            let alive = health.current > 0.0 && death.is_none();
            let feigning = auras.is_some_and(|auras| auras.has_type(AURA_FEIGN_DEATH));
            match alive_state {
                0 => alive,
                1 => !alive,
                2 => alive && !feigning,
                3 => !alive || feigning,
                _ => false,
            }
        })
        .map(|(entity, _, position, ..)| {
            (
                entity,
                Vec3::new(
                    position.x - origin.x,
                    position.y - origin.y,
                    position.z - origin.z,
                )
                .length_squared(),
            )
        })
        .filter(|(_, distance)| *distance <= radius * radius)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(entity, _)| entity)
}

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
