//! Server-only Scrapping Machine preflight. Loot execution is deliberately blocked:
//! no authentic scrapping loot source exists in the inspected local Retail data.
use std::collections::HashSet;

use bevy::prelude::*;
use shared::components::{Health, Position};
use shared::item_data::ItemDefinitionSource::Retail;

use crate::game_objects::{GAME_OBJECT_INTERACT_DISTANCE, GameObjectBounds, wow_position};
use crate::inventory::load_model;
use crate::item_templates::ItemTemplates;
use crate::maps::InMap;
use crate::persistence::{ClientId, GameDatabase};

/// Retail Blizzard_ScrappingMachineUI: three rows of three slots.
pub(crate) const SCRAP_SLOTS: usize = 9;
/// TC a352b1fa GameObjectData.h: UI_LINK with UILinkType 2.
pub(crate) const GAMEOBJECT_TYPE_UI_LINK: u8 = 48;
pub(crate) const UI_LINK_SCRAPPING_MACHINE: u8 = 2;

#[derive(Component)]
pub(crate) struct ScrappingMachine;

#[derive(Resource, Default)]
pub(crate) struct ScrappingData {
    items: HashSet<u32>,
}

pub(crate) fn load_scrapping_data(conn: &rusqlite::Connection) -> rusqlite::Result<ScrappingData> {
    // ItemTemplate.h ITEM_FLAG4_SCRAPABLE: Flags[3], NOT Flags[0].
    const ITEM_FLAG4_SCRAPABLE: u32 = 0x20;
    let mut statement = conn.prepare(
        "SELECT c.ID FROM content_item c JOIN item_sparse s ON s.ID=c.ID \
         WHERE (s.Flags_3 & ?1) != 0",
    )?;
    let rows = statement.query_map([ITEM_FLAG4_SCRAPABLE], |row| row.get(0))?;
    Ok(ScrappingData {
        items: rows.collect::<rusqlite::Result<_>>()?,
    })
}

/// Internal server request, not a shared-protocol message. The future wire adapter
/// must resolve `player` from its authenticated InWorld connection, never the client.
#[derive(Message, Clone)]
pub(crate) struct ScrapRequest {
    pub player: Entity,
    pub machine: Entity,
    pub items: Vec<u64>,
}

#[derive(Debug)]
pub(crate) enum ScrapFailure {
    InvalidRequest,
    InvalidMachine,
    PlayerDead,
    /// Retail ERR_CANT_BE_SCRAPPED (1049), SPELL_FAILED_CANT_BE_SCRAPPED (278).
    CantBeScrapped,
    Inventory(shared::protocol::InventoryError),
    /// Server content failure, NOT a Retail eligibility error.
    LootUnavailable,
}

impl std::fmt::Display for ScrapFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequest => formatter.write_str("invalid scrapping item list or player"),
            Self::InvalidMachine => {
                formatter.write_str("scrapping machine is not reachable in this map copy")
            }
            Self::PlayerDead => formatter.write_str("player is dead"),
            Self::CantBeScrapped => formatter.write_str("You can't scrap that item"),
            Self::Inventory(error) => write!(formatter, "scrapping inventory: {error:?}"),
            Self::LootUnavailable => {
                formatter.write_str("authentic scrapping loot is unavailable; no item consumed")
            }
        }
    }
}

#[derive(Message, Debug)]
pub(crate) struct ScrapResponse {
    pub player: Entity,
    pub result: Result<(), ScrapFailure>,
}

pub(crate) fn check_machine(world: &World, request: &ScrapRequest) -> Result<(), ScrapFailure> {
    let health = world
        .get::<Health>(request.player)
        .ok_or(ScrapFailure::InvalidRequest)?;
    if health.current <= 0.0 {
        return Err(ScrapFailure::PlayerDead);
    }
    if world.get::<ScrappingMachine>(request.machine).is_none() {
        return Err(ScrapFailure::InvalidMachine);
    }
    let player_map = world
        .get::<InMap>(request.player)
        .ok_or(ScrapFailure::InvalidMachine)?;
    if world.get::<InMap>(request.machine) != Some(player_map) {
        return Err(ScrapFailure::InvalidMachine);
    }
    check_reach(world, request)
}

fn check_reach(world: &World, request: &ScrapRequest) -> Result<(), ScrapFailure> {
    let player_position = world
        .get::<Position>(request.player)
        .ok_or(ScrapFailure::InvalidMachine)?;
    let machine_position = world
        .get::<Position>(request.machine)
        .ok_or(ScrapFailure::InvalidMachine)?;
    let bounds = world
        .get::<GameObjectBounds>(request.machine)
        .ok_or(ScrapFailure::InvalidMachine)?;
    let reaches = bounds.reaches(
        wow_position(machine_position),
        wow_position(player_position),
        GAME_OBJECT_INTERACT_DISTANCE,
    );
    if reaches {
        Ok(())
    } else {
        Err(ScrapFailure::InvalidMachine)
    }
}

fn check_selection(items: &[u64]) -> Result<(), ScrapFailure> {
    if items.is_empty() || items.len() > SCRAP_SLOTS {
        return Err(ScrapFailure::InvalidRequest);
    }
    let unique: HashSet<_> = items.iter().collect();
    if unique.len() != items.len() {
        return Err(ScrapFailure::InvalidRequest);
    }
    Ok(())
}

pub(crate) fn load_and_check_bag_items(
    world: &World,
    request: &ScrapRequest,
) -> Result<(), ScrapFailure> {
    check_selection(&request.items)?;
    let character = world
        .get::<ClientId>(request.player)
        .ok_or(ScrapFailure::InvalidRequest)?
        .0;
    let db = world.resource::<GameDatabase>();
    let templates = world.resource::<ItemTemplates>();
    let model = load_model(&db.db, templates, character).map_err(ScrapFailure::Inventory)?;
    let data = world
        .get_resource::<ScrappingData>()
        .ok_or(ScrapFailure::LootUnavailable)?;
    for &guid in &request.items {
        let item = model.bag_item(guid).ok_or_else(|| {
            ScrapFailure::Inventory(crate::inventory::inventory_model::error(
                shared::protocol::InventoryErrorReason::ItemNotFound,
                format!("no bag item {guid}"),
            ))
        })?;
        if model.definition_source(guid) != Retail || !data.items.contains(&item.item_id) {
            return Err(ScrapFailure::CantBeScrapped);
        }
    }
    Ok(())
}

fn load_startup(mut commands: Commands) {
    let result = rusqlite::Connection::open_with_flags(
        crate::spell_info::WORLD_DB_PATH,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .and_then(|conn| load_scrapping_data(&conn));
    match result {
        Ok(data) => {
            commands.insert_resource(data);
        }
        Err(error) => error!("Scrapping eligibility unavailable: {error}"),
    }
}

fn reject_unavailable_scrap_requests(world: &mut World) {
    let requests: Vec<_> = world
        .resource_mut::<Messages<ScrapRequest>>()
        .drain()
        .collect();
    for request in requests {
        let result = check_machine(world, &request)
            .and_then(|()| load_and_check_bag_items(world, &request))
            .and(Err(ScrapFailure::LootUnavailable));
        let response = ScrapResponse {
            player: request.player,
            result,
        };
        if let Err(failure) = &response.result {
            debug!("Scrapping player {:?}: {failure}", response.player);
        }
        world.write_message(response);
    }
}

pub(crate) struct ScrappingPlugin;
impl Plugin for ScrappingPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ScrapRequest>()
            .add_message::<ScrapResponse>()
            .add_systems(Startup, load_startup)
            .add_systems(Update, reject_unavailable_scrap_requests);
    }
}

#[cfg(test)]
#[path = "scrapping_tests.rs"]
mod tests;
