//! NALORAKK_PYRE_TELEPORT: source pyres and menus, inferred pairing and party travel.
use bevy::prelude::*;
use shared::protocol::GameObjectInfo;

use super::gossip::{GossipAction, Opening, gossip_opening};
use crate::maps::{InMap, TeleportOptions, WorldLocation};

pub(crate) const MAP: u32 = 2825;
pub(crate) const PYRES: &[(u32, u32, Vec3, f32)] = &[
    (
        616377,
        40087,
        Vec3::new(4339.33, -4947.34, 104.31152),
        4.3481,
    ),
    (
        613266,
        40088,
        Vec3::new(4480.69, -4912.04, 13.435155),
        3.26377,
    ),
];

pub(crate) fn menu(entry: u32) -> Option<u32> {
    PYRES.iter().find(|row| row.0 == entry).map(|row| row.1)
}

pub(super) fn opening(entry: u32) -> Opening {
    gossip_opening(
        menu(entry).expect("validated pyre"),
        "The bonfire dances with whispers of an unproven fate, beckoning you closer.".into(),
        vec![(
            0,
            0,
            "<Meditate on the sound of the flames.>".into(),
            GossipAction::NalorakkPyre,
        )],
    )
}

pub(crate) fn destination(entry: u32) -> Option<WorldLocation> {
    let (_, _, position, orientation) = *PYRES.iter().find(|row| row.0 == entry)?;
    Some(WorldLocation {
        map_id: MAP,
        position,
        orientation,
    })
}

pub(crate) fn traverse(world: &mut World, player: Entity, pyre: Entity) -> Result<(), String> {
    let copy = *world.get::<InMap>(player).ok_or("pyre user lacks a map")?;
    if copy.map_id != MAP || world.get::<InMap>(pyre) != Some(&copy) {
        return Err("pyre and user must share a Nalorakk instance".into());
    }
    let entry = world
        .get::<GameObjectInfo>(pyre)
        .ok_or("pyre lacks object info")?
        .entry;
    let destination = destination(entry).ok_or("unknown pyre")?;
    let followers: Vec<_> = world
        .query::<(Entity, &crate::followers::Follower, &InMap)>()
        .iter(world)
        .filter_map(|(entity, follower, map)| {
            (follower.owner == player
                && *map == copy
                && world.get::<crate::combat::DeathState>(entity).is_none())
            .then_some(entity)
        })
        .collect();
    // TC 15e0ed76 Player.cpp:1274-1278 temporarily unsummons distant pets;
    // MovementHandler.cpp:464-465 restores them after a near teleport.
    let owners: Vec<_> = std::iter::once(player)
        .chain(followers.iter().copied())
        .collect();
    let mut companions: std::collections::HashSet<_> = owners
        .iter()
        .flat_map(|&owner| crate::hunter_pet::active_owned_pets(world, owner))
        .collect();
    companions.extend(
        world
            .query::<(Entity, &crate::spell_cast::summon::SummonedCreature, &InMap)>()
            .iter(world)
            .filter_map(|(unit, summon, map)| {
                (owners.contains(&summon.owner)
                    && *map == copy
                    && summon.properties.is_guardian()
                    && crate::spell_cast::targets::is_alive(world, unit))
                .then_some(unit)
            }),
    );

    if !crate::maps::teleport_to(world, player, destination, TeleportOptions::default()) {
        return Err("pyre user near teleport refused".into());
    }
    for unit in followers.into_iter().chain(companions) {
        if let Some(mut navigation) = world.get_resource_mut::<crate::navigation::Navigation>() {
            navigation.cancel_route(unit);
        }
        world.entity_mut(unit).insert(destination.bevy_position());
        crate::interest::mark_interest_position_changed(
            world.get_resource::<crate::interest::InterestDirtyQueue>(),
            unit,
        );
        crate::spell_cast::mark_dirty(world, unit);
    }
    info!(?player, ?pyre, "NALORAKK_PYRE_TELEPORT");
    Ok(())
}
