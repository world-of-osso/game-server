//! NALORAKK_PYRE_TELEPORT: source pyres and menus, inferred pairing and party travel.
use bevy::prelude::*;
use shared::protocol::GameObjectInfo;

use super::gossip::{GossipAction, Opening, gossip_opening};
use crate::maps::{InMap, TeleportOptions, WorldLocation};

const MAP: u32 = 2825;

pub(super) fn menu(entry: u32) -> Option<u32> {
    match entry {
        616377 => Some(40087),
        613266 => Some(40088),
        _ => None,
    }
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

fn destination(entry: u32) -> Option<WorldLocation> {
    let (position, orientation) = match entry {
        616377 => (Vec3::new(4339.33, -4947.34, 104.31152), 4.3481),
        613266 => (Vec3::new(4480.69, -4912.04, 13.435155), 3.26377),
        _ => return None,
    };
    Some(WorldLocation {
        map_id: MAP,
        position,
        orientation,
    })
}

pub(super) fn traverse(world: &mut World, player: Entity, pyre: Entity) -> Result<(), String> {
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
    if !crate::maps::teleport_to(world, player, destination, TeleportOptions::default()) {
        return Err("pyre user near teleport refused".into());
    }
    for follower in followers {
        if let Some(mut navigation) = world.get_resource_mut::<crate::navigation::Navigation>() {
            navigation.cancel_route(follower);
        }
        world
            .entity_mut(follower)
            .insert(destination.bevy_position());
        crate::interest::mark_interest_position_changed(
            world.get_resource::<crate::interest::InterestDirtyQueue>(),
            follower,
        );
        crate::spell_cast::mark_dirty(world, follower);
    }
    info!(?player, ?pyre, "NALORAKK_PYRE_TELEPORT");
    Ok(())
}
