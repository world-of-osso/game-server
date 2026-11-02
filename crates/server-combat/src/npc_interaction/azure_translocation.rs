//! AZURE_TRANSLOCATION_BOOK_ENDPOINTS; ActionSet bindings/endpoints remain unsourced.
use crate::maps::{InMap, WorldLocation};
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
        .find(|spawn| spawn.guid == u64::from(destination) && spawn.map == 2515)
        .ok_or_else(|| format!("Azure receiving book source{destination} absent"))?;
    Ok(WorldLocation {
        map_id: 2515,
        position: Vec3::new(spawn.position_x, spawn.position_y, spawn.position_z),
        orientation: spawn.orientation,
    })
}

pub fn traverse(world: &mut World, player: Entity, book: Entity) -> Result<usize, String> {
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
    crate::maps::transfer::teleport_transport_user(world, player, destination)?;
    if world.get::<shared::components::Player>(player).is_some() {
        crate::followers::transport_follow::record_use(
            world,
            player,
            book,
            crate::followers::transport_follow::TransportKind::AzureBook,
        );
    }
    info!(
        ?player,
        ?book,
        endpoint = link.destination,
        "AZURE_TRANSLOCATION_BOOK_ENDPOINTS"
    );
    Ok(1)
}
