//! AZURE_TRANSLOCATION_BOOK_ENDPOINTS; ActionSet bindings/endpoints remain unsourced.
use crate::maps::{InMap, TeleportOptions, WorldLocation};
use bevy::prelude::*;
use shared::components::Npc;

pub(crate) const FINAL_POI_WOW: Vec3 = Vec3::new(-5364.33008, 1035.76001, 157.716995);

pub(crate) struct BookLink {
    pub entry: u32,
    pub menu: u32,
    pub destination: u32,
}

// Source text/component derivation: docs/wiki/systems/azure-translocation.md.
// Destination7116 is the authenticated AreaPOI inside Umbrelskul's component.
pub(crate) const LINKS: &[BookLink] = &[
    BookLink {
        entry: 194602,
        menu: 28961,
        destination: 29001870,
    },
    BookLink {
        entry: 194712,
        menu: 29009,
        destination: 29001886,
    },
    BookLink {
        entry: 194713,
        menu: 29011,
        destination: 29001915,
    },
    BookLink {
        entry: 194714,
        menu: 29012,
        destination: 29002013,
    },
    BookLink {
        entry: 194715,
        menu: 29013,
        destination: 7116,
    },
];

pub(crate) fn read_endpoint(world: &World, destination: u32) -> Result<WorldLocation, String> {
    if destination == 7116 {
        return Ok(WorldLocation {
            map_id: 2515,
            position: FINAL_POI_WOW,
            orientation: 0.0,
        });
    }
    let spawn = world
        .resource::<crate::world_data::WorldData>()
        .spawns
        .iter()
        .find(|spawn| spawn.guid == destination && spawn.map == 2515)
        .ok_or_else(|| format!("Azure receiving book source{destination} absent"))?;
    Ok(WorldLocation {
        map_id: 2515,
        position: Vec3::new(spawn.position_x, spawn.position_y, spawn.position_z),
        orientation: spawn.orientation,
    })
}

pub(crate) fn traverse(world: &mut World, player: Entity, book: Entity) -> Result<usize, String> {
    let copy = *world.get::<InMap>(player).ok_or("Azure reader lacks map")?;
    if copy.map_id != 2515 || world.get::<InMap>(book) != Some(&copy) {
        return Err("Azure book and reader must share the Azure copy".into());
    }
    let entry = world
        .get::<Npc>(book)
        .ok_or("Azure source is not an NPC")?
        .template_id;
    let link = LINKS
        .iter()
        .find(|link| link.entry == entry)
        .ok_or("Azure book has no qualified endpoint")?;
    let destination = read_endpoint(world, link.destination)?;
    let recipients = read_party(world, player, copy);
    if !crate::maps::teleport_to(world, player, destination, TeleportOptions::default()) {
        return Err("Azure reader near teleport refused".into());
    }
    for unit in recipients.iter().copied().filter(|unit| *unit != player) {
        teleport_member(world, unit, destination)?;
    }
    info!(
        ?player,
        ?book,
        endpoint = link.destination,
        "AZURE_TRANSLOCATION_BOOK_ENDPOINTS"
    );
    Ok(recipients.len())
}

fn teleport_member(
    world: &mut World,
    unit: Entity,
    destination: WorldLocation,
) -> Result<(), String> {
    if world.get::<shared::components::Player>(unit).is_some() {
        if !crate::maps::teleport_to(world, unit, destination, TeleportOptions::default()) {
            return Err(format!("Azure party member{unit:?} near teleport refused"));
        }
        return Ok(());
    }
    if let Some(mut navigation) = world.get_resource_mut::<crate::navigation::Navigation>() {
        navigation.cancel_route(unit);
    }
    world.entity_mut(unit).insert(destination.bevy_position());
    crate::interest::mark_interest_position_changed(
        world.get_resource::<crate::interest::InterestDirtyQueue>(),
        unit,
    );
    Ok(())
}

fn read_party(world: &mut World, player: Entity, copy: InMap) -> Vec<Entity> {
    let members = world
        .get::<crate::persistence::ClientId>(player)
        .and_then(|id| {
            world
                .get_resource::<crate::group::GroupRegistry>()
                .map(|groups| groups.group_members(id.0))
        })
        .unwrap_or_default();
    let mut owners: Vec<_> = world
        .query::<(Entity, &crate::persistence::ClientId, &InMap)>()
        .iter(world)
        .filter(|(_, id, map)| **map == copy && members.contains(&id.0))
        .map(|(unit, _, _)| unit)
        .collect();
    if !owners.contains(&player) {
        owners.push(player);
    }
    let followers: Vec<_> = world
        .query::<(Entity, &crate::followers::Follower, &InMap)>()
        .iter(world)
        .filter(|(_, follower, map)| **map == copy && owners.contains(&follower.owner))
        .map(|(unit, _, _)| unit)
        .collect();
    owners.extend(followers);
    let pets: Vec<_> = owners
        .iter()
        .filter_map(|unit| crate::hunter_pet::summoned_pet(world, *unit))
        .filter(|pet| world.get::<InMap>(*pet) == Some(&copy))
        .collect();
    owners.extend(pets);
    owners.retain(|unit| crate::spell_cast::targets::is_alive(world, *unit));
    owners.sort();
    owners.dedup();
    owners
}
