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
        // NALORAKK_PYRE_REVERSE_ARRIVAL_LOG_CORRELATED: WoWCombatLog-102026_223255.txt:136181.
        Vec3::new(4484.93, -4906.93, 13.4853),
        0.0689,
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
    if !crate::maps::teleport_to(world, player, destination, TeleportOptions::default()) {
        return Err("pyre user near teleport refused".into());
    }
    for unit in followers {
        crate::maps::transfer::teleport_creature_with_companions(world, unit, &destination);
    }
    info!(?player, ?pyre, "NALORAKK_PYRE_TELEPORT");
    Ok(())
}
