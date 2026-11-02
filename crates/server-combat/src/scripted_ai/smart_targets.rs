//! TrinityCore a352b1fa GetTargets(SMART_TARGET_INVOKER_PARTY).
use crate::random as rand;
use bevy::prelude::*;
use shared::components::{Health, Npc, Player, Position};

const AURA_FEIGN_DEATH: u32 = 66;

#[cfg(test)]
mod tests {
    use super::*;
    use shared::threat::ThreatTable;

    #[test]
    fn random_not_top_skips_victim_before_applying_range_filter() {
        let mut world = World::new();
        let origin = Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let alive = Health {
            current: 100.0,
            max: 100.0,
        };
        let victim = world.spawn((origin, alive)).id();
        let outside = world.spawn((Position { x: 20.0, ..origin }, alive)).id();
        let inside = world.spawn((Position { x: 3.0, ..origin }, alive)).id();
        let mut threat = ThreatTable::default();
        threat.add_threat(victim.to_bits(), 90.0);
        threat.add_threat(outside.to_bits(), 100.0);
        threat.add_threat(inside.to_bits(), 30.0);
        let caster = world.spawn((origin, threat)).id();

        for _ in 0..20 {
            assert_eq!(
                random_hostile(&world, caster, [10, 0, 0, 0], true, Some(victim)),
                Some(inside)
            );
        }
        assert_eq!(
            random_hostile(&world, caster, [1, 0, 0, 0], true, Some(victim)),
            None
        );
        assert_eq!(
            random_hostile(&world, caster, [1, 0, 0, 0], false, None),
            Some(victim)
        );
    }
}

/// TrinityCore a352b1fa GetTargets(SMART_TARGET_HOSTILE_RANDOM): threat-list
/// selection, including the victim, with distance/player/primary-power filters.
pub(super) fn random_hostile(
    world: &World,
    me: Entity,
    params: [u32; 4],
    skip_top: bool,
    victim: Option<Entity>,
) -> Option<Entity> {
    use rand::seq::SliceRandom;
    let [distance, player_only, power, _] = params;
    let threat = world.get::<shared::threat::ThreatTable>(me)?;
    // TC UnitAI::PrepareTargetListSelection skips the current victim, or the
    // highest threat entry if there is no victim, before target filtering.
    let excluded = if skip_top {
        victim.or_else(|| {
            threat
                .top_threat()
                .and_then(|entry| Entity::try_from_bits(entry.entity))
        })
    } else {
        None
    };
    let origin = crate::spell_cast::targets::position(world, me)?;
    let map = world.get::<crate::maps::InMap>(me);
    let candidates: Vec<Entity> = threat
        .entries()
        .iter()
        .filter_map(|entry| Entity::try_from_bits(entry.entity))
        .filter(|&unit| {
            Some(unit) != excluded
                && crate::spell_cast::targets::is_alive(world, unit)
                && world.get::<crate::maps::InMap>(unit) == map
                && (player_only == 0 || world.get::<Player>(unit).is_some())
                && (distance == 0
                    || crate::spell_cast::targets::position(world, unit)
                        .is_some_and(|position| position.distance(origin) <= distance as f32))
                && (power == 0
                    || world
                        .get::<shared::components::UnitPowers>(unit)
                        .and_then(|powers| powers.entries.first())
                        .is_some_and(|entry| entry.power as u32 + 1 == power))
        })
        .collect();
    candidates.choose(&mut rand::thread_rng()).copied()
}

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
