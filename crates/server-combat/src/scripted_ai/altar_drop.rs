//! INFERRED_WCL_POSITIONS_SNAPPED_TO_LEDGE: Liadrin protects the one-way Altar drop.
use bevy::prelude::*;
use shared::components::{Npc, Player, Position};

use crate::maps::InMap;

pub(crate) const MAP: u32 = 2993;
pub(crate) const LIADRIN: u32 = 263790;
const BLESSING: u32 = 1_309_794;

#[cfg(test)]
#[path = "altar_drop_tests.rs"]
mod tests;
// WCL pPFDbkfQK7MACtmY/41, xG9MB3Z8AhfQF7zR/3, 4QGkmdM9aHhT2Pxj/10:
// centroid snapped to the nearest upper/lower Detour components. Retire when
// authenticated trigger and landing locations are captured.
pub(crate) const SOURCE: Vec3 = Vec3::new(1605.9896, 15.828735, -2127.083);
// The native collision floor is 0.277 yd below the lower Detour polygon.
pub(crate) const LANDING: Vec3 = Vec3::new(1607.8125, -67.84819, -2117.8264);

#[derive(Component)]
struct AltarDrop {
    liadrin: Entity,
    blessed: bool,
}

pub(crate) fn traverse(world: &mut World, unit: Entity, liadrin: Entity) -> Result<(), String> {
    let map = *world
        .get::<InMap>(unit)
        .ok_or("Altar drop user lacks a map")?;
    if map.map_id != MAP || world.get::<InMap>(liadrin) != Some(&map) {
        return Err("Altar drop requires Liadrin and user in the same copy".into());
    }
    if world
        .get::<Npc>(liadrin)
        .is_none_or(|npc| npc.template_id != LIADRIN)
    {
        return Err("Altar drop source is not Liadrin".into());
    }
    let position = world
        .get::<Position>(unit)
        .ok_or("Altar drop user lacks position")?;
    if Vec3::new(position.x, position.y, position.z).distance(SOURCE)
        > crate::npc_interaction::INTERACTION_DISTANCE
    {
        return Err("Altar drop user is outside the ledge interaction range".into());
    }
    if world.get::<AltarDrop>(unit).is_some() {
        return Err("Altar drop is already in progress".into());
    }
    world.entity_mut(unit).insert(AltarDrop {
        liadrin,
        blessed: false,
    });
    crate::navigation::collision_drop::start_passage_drop(world, unit, map, SOURCE, LANDING);
    Ok(())
}

/// Cast only when the unit actually starts falling, so the real four-second aura
/// covers the landing even when it had to walk to the lip first.
pub(crate) fn advance(world: &mut World) {
    let units: Vec<_> = world
        .query::<(Entity, &AltarDrop)>()
        .iter(world)
        .map(|(unit, drop)| (unit, drop.liadrin, drop.blessed))
        .collect();
    for (unit, liadrin, blessed) in units {
        let falling = world
            .get::<super::fall_movement::FallMovement>(unit)
            .is_some();
        let moving = world
            .get::<crate::navigation::collision_drop::CollisionDropMovement>(unit)
            .is_some();
        if falling && !blessed {
            crate::spell_cast::cast_triggered(world, liadrin, BLESSING, Some(unit), 0);
            if !crate::auras::is_immune(world, unit, 1) {
                error!(?unit, ?liadrin, "Altar Liadrin protection failed");
            }
            world
                .get_mut::<AltarDrop>(unit)
                .expect("drop exists")
                .blessed = true;
        } else if blessed && !moving && !falling {
            world.entity_mut(unit).remove::<AltarDrop>();
            if world.get::<Player>(unit).is_some() {
                crate::followers::transport_follow::record_use(
                    world,
                    unit,
                    liadrin,
                    crate::followers::transport_follow::TransportKind::AltarDrop,
                );
            }
        }
    }
}

pub(crate) fn start_nearby(world: &mut World) {
    let mut players: Vec<_> = crate::system_states::with_query::<
        (Entity, &Position, &InMap),
        With<Player>,
        _,
    >(world, |world, players| {
        players
            .iter(world)
            .filter_map(|(unit, position, map)| {
                (map.map_id == MAP
                    && world.get::<AltarDrop>(unit).is_none()
                    && crate::spell_cast::targets::is_alive(world, unit)
                    && Vec3::new(position.x, position.y, position.z).distance(SOURCE)
                        <= crate::npc_interaction::INTERACTION_DISTANCE)
                    .then_some((unit, *map))
            })
            .collect()
    });
    players.sort_unstable_by_key(|(unit, _)| *unit);
    for (unit, map) in players {
        let caster = crate::system_states::with_query::<(Entity, &Npc, &InMap), (), _>(
            world,
            |world, units| {
                units
                    .iter(world)
                    .filter_map(|(entity, npc, copy)| {
                        (npc.template_id == LIADRIN && *copy == map).then_some(entity)
                    })
                    .min()
            },
        );
        if let Some(caster) = caster {
            if let Err(error) = traverse(world, unit, caster) {
                error!(?unit, %error, "Altar drop failed");
            }
        }
    }
}
