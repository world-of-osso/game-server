//! Atomic account housing: plot payment, owned decor and durable basic layouts.
//! Never grants StartingQuantity, exterior components or rooms from catalog membership.
use std::collections::BTreeMap;

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition, WriteTransaction};
use serde::{Deserialize, Serialize};
use shared::item_data::ItemDefinitionSource;
use shared::protocol::{
    HouseAccess, HouseLayout, HousingCommand, HousingHouse, HousingPlot, HousingState, PlacedDecor,
    StoredDecor,
};

use super::{INVENTORIES, InventoryData, inventory_store};

pub(crate) const BUILD_KEY: &str = "dcfc90fffd79ba00406ae46f5f657592";
const STATE: TableDefinition<u8, &[u8]> = TableDefinition::new("housing_state");
const STATE_ROW: u8 = 0;
#[path = "persistence_housing_access.rs"]
mod access;
#[path = "persistence_housing_migration.rs"]
mod migration;
use crate::group::GroupRegistry;
#[path = "persistence_housing_dyes.rs"]
mod dyes;
pub(crate) use dyes::{commit_dyes, preview_dyes};
#[path = "persistence_housing_variants.rs"]
mod variants;
use access::require_visit;
const NEIGHBORHOOD_LOCATIONS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("housing_neighborhood_locations");

const HOUSE_LOCATIONS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("housing_house_locations");

#[derive(Serialize, Deserialize)]
pub(crate) struct HouseLocation {
    pub house_id: u64,
    pub account_id: u64,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct NeighborhoodLocation {
    pub map_id: u32,
    pub neighborhood_id: u32,
}

/// Trusted identity/map context, constructed from the authenticated server world.
#[derive(Clone, Copy)]
pub(crate) struct Actor {
    pub account_id: u64,
    pub character_id: u64,
    pub neighborhood_id: Option<u32>,
    pub inside_house: Option<u64>,
}

/// Server catalog evidence, never a client-supplied reward definition.
#[derive(Clone, Copy)]
pub(crate) struct RedemptionRule {
    pub item_id: u32,
    pub decor_id: u32,
}

#[derive(Serialize, Deserialize)]
struct OwnedHouse {
    house_id: u64,
    account_id: u64,
    owner_character_id: u64,
    neighborhood_id: u32,
    plot_id: u32,
    house_access: Option<HouseAccess>,
    decor: Vec<PlacedDecor>,
}

#[derive(Serialize, Deserialize)]
struct State {
    schema_version: u64,
    build_key: String,
    next_house_id: u64,
    next_instance_id: u64,
    houses: BTreeMap<u64, OwnedHouse>,
    storage: BTreeMap<u64, BTreeMap<u32, u32>>,
    storage_variants: BTreeMap<u64, Vec<shared::protocol::StoredDecorVariant>>,
}

impl State {
    fn new() -> Self {
        Self {
            schema_version: migration::SCHEMA_VERSION,
            build_key: BUILD_KEY.into(),
            next_house_id: 1,
            next_instance_id: 1,
            houses: BTreeMap::new(),
            storage: BTreeMap::new(),
            storage_variants: BTreeMap::new(),
        }
    }
}

pub(crate) fn create_tables(txn: &WriteTransaction) {
    txn.open_table(HOUSE_LOCATIONS)
        .expect("create housing house locations");
    txn.open_table(NEIGHBORHOOD_LOCATIONS)
        .expect("create housing neighborhood locations");
    let mut table = txn.open_table(STATE).expect("create housing state table");
    let existing = table
        .get(STATE_ROW)
        .expect("read housing initialization")
        .map(|row| row.value().to_vec());
    if existing.is_none() {
        let bytes = serde_json::to_vec(&State::new()).expect("encode new housing state");
        table
            .insert(STATE_ROW, bytes.as_slice())
            .expect("initialize empty housing state");
    }
    drop(table);
    if let Some(bytes) = existing {
        let version = migration::version(&bytes).expect("read housing schema version");
        if version != migration::SCHEMA_VERSION {
            let source = rusqlite::Connection::open_with_flags(
                crate::spell_info::WORLD_DB_PATH,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .expect("open housing migration source");
            migration::migrate(txn, &source)
                .expect("migrate housing transforms from authenticated source");
        }
    }
}

pub(crate) fn save_house_location(
    db: &Database,
    actor: Actor,
    house_id: u64,
    groups: Option<&GroupRegistry>,
) -> Result<(), String> {
    let txn = db.begin_write().map_err(|error| error.to_string())?;
    let state = read_in(&txn)?;
    require_visit(db, &state, actor, house_id, groups)?;
    let bytes = serde_json::to_vec(&HouseLocation {
        house_id,
        account_id: actor.account_id,
    })
    .map_err(|error| format!("Encode house admission: {error}"))?;
    txn.open_table(HOUSE_LOCATIONS)
        .map_err(|error| error.to_string())?
        .insert(actor.character_id, bytes.as_slice())
        .map_err(|error| error.to_string())?;
    crate::persistence_commit::commit(txn)
        .map_err(|error| format!("Persist visited house: {error}"))
}

pub(crate) fn load_house_location(
    db: &Database,
    character_id: u64,
    groups: Option<&GroupRegistry>,
) -> Result<HouseLocation, String> {
    let txn = db.begin_read().map_err(|error| error.to_string())?;
    let state = read_state(&txn.open_table(STATE).map_err(|error| error.to_string())?)?;
    let table = txn
        .open_table(HOUSE_LOCATIONS)
        .map_err(|error| error.to_string())?;
    let row = table
        .get(character_id)
        .map_err(|error| error.to_string())?
        .ok_or("No persisted visited-house identity exists.")?;
    let location: HouseLocation = serde_json::from_slice(row.value())
        .map_err(|error| format!("Decode visited house: {error}"))?;
    require_visit(
        db,
        &state,
        Actor {
            account_id: location.account_id,
            character_id,
            neighborhood_id: None,
            inside_house: None,
        },
        location.house_id,
        groups,
    )?;
    Ok(location)
}

pub(crate) fn save_neighborhood_location(
    db: &Database,
    character_id: u64,
    location: &NeighborhoodLocation,
) -> Result<(), String> {
    let txn = db.begin_write().map_err(|error| error.to_string())?;
    read_in(&txn)?;
    let bytes = serde_json::to_vec(location)
        .map_err(|error| format!("Encode housing admission: {error}"))?;
    txn.open_table(NEIGHBORHOOD_LOCATIONS)
        .map_err(|error| error.to_string())?
        .insert(character_id, bytes.as_slice())
        .map_err(|error| error.to_string())?;
    crate::persistence_commit::commit(txn).map_err(|error| {
        format!("Persist neighborhood admission for character {character_id}: {error}")
    })
}

pub(crate) fn load_neighborhood_location(
    db: &Database,
    character_id: u64,
) -> Result<Option<NeighborhoodLocation>, String> {
    let txn = db.begin_read().map_err(|error| error.to_string())?;
    read_state(&txn.open_table(STATE).map_err(|error| error.to_string())?)?;
    let table = txn
        .open_table(NEIGHBORHOOD_LOCATIONS)
        .map_err(|error| error.to_string())?;
    let row = table.get(character_id).map_err(|error| error.to_string())?;
    row.map(|row| {
        serde_json::from_slice(row.value()).map_err(|error| {
            format!("Decode character {character_id} neighborhood admission: {error}")
        })
    })
    .transpose()
}

fn read_state(table: &impl ReadableTable<u8, &'static [u8]>) -> Result<State, String> {
    let row = table
        .get(STATE_ROW)
        .map_err(|error| error.to_string())?
        .ok_or("Housing state has not been initialized.")?;
    let state: State = serde_json::from_slice(row.value())
        .map_err(|error| format!("Decode housing state: {error}"))?;
    if state.schema_version != migration::SCHEMA_VERSION {
        return Err(format!(
            "Unsupported housing schema {}.",
            state.schema_version
        ));
    }
    if state.build_key != BUILD_KEY {
        return Err(format!(
            "Housing state build {} does not match {BUILD_KEY}.",
            state.build_key
        ));
    }
    Ok(state)
}

fn read_in(txn: &WriteTransaction) -> Result<State, String> {
    read_state(&txn.open_table(STATE).map_err(|error| error.to_string())?)
}

fn commit(txn: WriteTransaction, state: &State) -> Result<(), String> {
    let bytes =
        serde_json::to_vec(state).map_err(|error| format!("Encode housing state: {error}"))?;
    txn.open_table(STATE)
        .map_err(|error| error.to_string())?
        .insert(STATE_ROW, bytes.as_slice())
        .map_err(|error| error.to_string())?;
    crate::persistence_commit::commit(txn).map_err(|error| format!("Commit housing state: {error}"))
}

fn read_inventory(txn: &WriteTransaction, character_id: u64) -> Result<InventoryData, String> {
    let table = txn
        .open_table(INVENTORIES)
        .map_err(|error| error.to_string())?;
    let row = table
        .get(character_id)
        .map_err(|error| error.to_string())?
        .ok_or("Character inventory is unavailable.")?;
    bitcode::decode(row.value())
        .map_err(|error| format!("Decode character {character_id} inventory: {error}"))
}

fn write_inventory(
    txn: &WriteTransaction,
    character_id: u64,
    inventory: &InventoryData,
) -> Result<(), String> {
    let bytes = bitcode::encode(inventory);
    txn.open_table(INVENTORIES)
        .map_err(|error| error.to_string())?
        .insert(character_id, bytes.as_slice())
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn house_view(house: &OwnedHouse, actor: Actor) -> HousingHouse {
    HousingHouse {
        house_id: house.house_id,
        neighborhood_id: house.neighborhood_id,
        plot_id: house.plot_id,
        owner_character_id: house.owner_character_id,
        can_edit: house.account_id == actor.account_id,
        house_access: house.house_access,
        exterior_grant_available: false,
        interior_grant_available: false,
    }
}

pub(crate) fn purchase(
    db: &Database,
    actor: Actor,
    plot: &HousingPlot,
) -> Result<HousingHouse, String> {
    if actor.neighborhood_id != Some(plot.neighborhood_id) {
        return Err("Enter this neighborhood before purchasing its plot.".into());
    }
    let txn = db.begin_write().map_err(|error| error.to_string())?;
    let mut state = read_in(&txn)?;
    if state
        .houses
        .values()
        .any(|house| house.plot_id == plot.plot_id && house.neighborhood_id == plot.neighborhood_id)
    {
        return Err("This plot is already owned.".into());
    }
    if state.houses.values().any(|house| {
        house.account_id == actor.account_id && house.neighborhood_id == plot.neighborhood_id
    }) {
        return Err("Your account already owns a plot in this neighborhood.".into());
    }
    let mut inventory = read_inventory(&txn, actor.character_id)?;
    inventory.gold = inventory
        .gold
        .checked_sub(plot.cost)
        .ok_or("You cannot afford this plot.")?;
    let house_id = state.next_house_id;
    state.next_house_id = house_id
        .checked_add(1)
        .ok_or("House identity range exhausted.")?;
    let house = OwnedHouse {
        house_id,
        account_id: actor.account_id,
        owner_character_id: actor.character_id,
        neighborhood_id: plot.neighborhood_id,
        plot_id: plot.plot_id,
        house_access: None,
        decor: Vec::new(),
    };
    let reply = house_view(&house, actor);
    state.houses.insert(house_id, house);
    write_inventory(&txn, actor.character_id, &inventory)?;
    commit(txn, &state)?;
    Ok(reply)
}

fn require_owner(state: &State, actor: Actor, house_id: u64) -> Result<(), String> {
    let house = state.houses.get(&house_id).ok_or("House not found.")?;
    if house.account_id != actor.account_id {
        return Err("Only the owner may edit this house.".into());
    }
    Ok(())
}

pub(crate) fn set_access(
    db: &Database,
    actor: Actor,
    house_id: u64,
    access: HouseAccess,
) -> Result<(), String> {
    let txn = db.begin_write().map_err(|error| error.to_string())?;
    let mut state = read_in(&txn)?;
    require_owner(&state, actor, house_id)?;
    state
        .houses
        .get_mut(&house_id)
        .expect("validated house")
        .house_access = Some(access);
    commit(txn, &state)
}

pub(crate) fn snapshot(
    db: &Database,
    actor: Actor,
    house_id: Option<u64>,
    groups: Option<&GroupRegistry>,
) -> Result<HousingState, String> {
    let txn = db.begin_read().map_err(|error| error.to_string())?;
    let state = read_state(&txn.open_table(STATE).map_err(|error| error.to_string())?)?;
    let layout = if let Some(house_id) = house_id {
        require_visit(db, &state, actor, house_id, groups)?;
        let house = &state.houses[&house_id];
        Some(HouseLayout {
            house_id,
            can_edit: house.account_id == actor.account_id,
            decor: house.decor.clone(),
        })
    } else {
        None
    };
    let owned_houses = state
        .houses
        .values()
        .filter(|house| house.account_id == actor.account_id)
        .map(|house| house_view(house, actor))
        .collect();
    let stored = state
        .storage
        .get(&actor.account_id)
        .into_iter()
        .flat_map(|storage| storage.iter())
        .map(|(&decor_id, &quantity)| StoredDecor { decor_id, quantity })
        .collect();
    let mut placed = BTreeMap::<u32, u32>::new();
    for decor in state
        .houses
        .values()
        .filter(|house| house.account_id == actor.account_id)
        .flat_map(|house| &house.decor)
    {
        *placed.entry(decor.decor_id).or_default() += 1;
    }
    Ok(HousingState {
        build_key: state.build_key,
        neighborhood_id: actor.neighborhood_id,
        opened_cornerstone: None,
        bulletin_board: None,
        plots: Vec::new(),
        owned_houses,
        stored,
        stored_variants: state
            .storage_variants
            .get(&actor.account_id)
            .cloned()
            .unwrap_or_default(),
        dye_preview: None,
        placed: placed
            .into_iter()
            .map(|(decor_id, quantity)| StoredDecor { decor_id, quantity })
            .collect(),
        layout,
    })
}

pub(crate) fn populate_plots(db: &Database, plots: &mut [HousingPlot]) -> Result<(), String> {
    let txn = db.begin_read().map_err(|error| error.to_string())?;
    let state = read_state(&txn.open_table(STATE).map_err(|error| error.to_string())?)?;
    for plot in plots {
        if let Some(house) = state.houses.values().find(|house| {
            house.plot_id == plot.plot_id && house.neighborhood_id == plot.neighborhood_id
        }) {
            plot.house_id = Some(house.house_id);
            plot.owner_character_id = Some(house.owner_character_id);
        }
    }
    Ok(())
}

pub(crate) fn house(
    db: &Database,
    actor: Actor,
    house_id: u64,
    groups: Option<&GroupRegistry>,
) -> Result<HousingHouse, String> {
    let txn = db.begin_read().map_err(|error| error.to_string())?;
    let state = read_state(&txn.open_table(STATE).map_err(|error| error.to_string())?)?;
    require_visit(db, &state, actor, house_id, groups)?;
    Ok(house_view(&state.houses[&house_id], actor))
}

pub(crate) fn redeem(
    db: &Database,
    actor: Actor,
    item_guid: u64,
    rule: &RedemptionRule,
) -> Result<(), String> {
    let txn = db.begin_write().map_err(|error| error.to_string())?;
    let mut state = read_in(&txn)?;
    let mut inventory = read_inventory(&txn, actor.character_id)?;
    let index = inventory
        .items
        .iter()
        .position(|item| item.item_guid == item_guid)
        .ok_or("You do not own this decor item in your bags.")?;
    let item = &mut inventory.items[index];
    if item.item_id != rule.item_id || item.stack_count == 0 {
        return Err("This item does not match the authenticated decor redemption rule.".into());
    }
    let sources = txn
        .open_table(inventory_store::ITEM_DEFINITION_SOURCES)
        .map_err(|error| error.to_string())?;
    if inventory_store::read_item_definition_source(&sources, item_guid)
        != ItemDefinitionSource::Retail
    {
        return Err("Housing redemption requires the authenticated Retail item definition.".into());
    }
    drop(sources);
    item.stack_count -= 1;
    if item.stack_count == 0 {
        inventory.items.remove(index);
        remove_placement(&txn, actor.character_id, item_guid)?;
    }
    let quantity = state
        .storage
        .entry(actor.account_id)
        .or_default()
        .entry(rule.decor_id)
        .or_default();
    *quantity = quantity
        .checked_add(1)
        .ok_or("Decor storage quantity overflow.")?;
    write_inventory(&txn, actor.character_id, &inventory)?;
    commit(txn, &state)
}

fn remove_placement(
    txn: &WriteTransaction,
    character_id: u64,
    item_guid: u64,
) -> Result<(), String> {
    let mut table = txn
        .open_table(inventory_store::INVENTORY_LAYOUTS)
        .map_err(|error| error.to_string())?;
    let row = table
        .get(character_id)
        .map_err(|error| error.to_string())?
        .map(|row| row.value().to_vec());
    if let Some(bytes) = row {
        let mut placements: Vec<inventory_store::ItemPlacementData> =
            bitcode::decode(&bytes).map_err(|error| error.to_string())?;
        placements.retain(|placement| placement.item_guid != item_guid);
        table
            .insert(character_id, bitcode::encode(&placements).as_slice())
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn edit_house_id(command: &HousingCommand) -> Result<u64, String> {
    match command {
        HousingCommand::PlaceDecor { house_id, .. }
        | HousingCommand::MoveDecor { house_id, .. }
        | HousingCommand::RotateDecor { house_id, .. }
        | HousingCommand::TransformDecor { house_id, .. }
        | HousingCommand::RemoveDecor { house_id, .. } => Ok(*house_id),
        _ => Err("Not a basic decor edit.".into()),
    }
}

fn validate_position(position: &[f32; 3]) -> Result<(), String> {
    if position.iter().all(|coordinate| coordinate.is_finite()) {
        Ok(())
    } else {
        Err("Decor position must be finite.".into())
    }
}

fn validate_yaw(yaw: f32) -> Result<(), String> {
    if yaw.is_finite() {
        Ok(())
    } else {
        Err("Decor rotation must be finite.".into())
    }
}

fn validate_transform(transform: &shared::protocol::DecorTransform) -> Result<(), String> {
    validate_position(&transform.position)?;
    for radians in [transform.yaw, transform.pitch, transform.roll] {
        validate_yaw(radians)?;
    }
    if !transform.scale.is_finite() || transform.scale <= 0.0 {
        return Err("Decor scale must be finite and positive.".into());
    }
    Ok(())
}

pub(crate) fn edit(db: &Database, actor: Actor, command: &HousingCommand) -> Result<(), String> {
    let house_id = edit_house_id(command)?;
    let txn = db.begin_write().map_err(|error| error.to_string())?;
    let mut state = read_in(&txn)?;
    require_house_editor(&state, actor, house_id)?;
    apply_edit(&mut state, actor.account_id, house_id, command)?;
    commit(txn, &state)
}

fn require_house_editor(state: &State, actor: Actor, house_id: u64) -> Result<(), String> {
    require_owner(state, actor, house_id)?;
    if actor.inside_house != Some(house_id) {
        return Err("Enter your house before editing decor.".into());
    }
    Ok(())
}

fn find_decor(
    state: &mut State,
    house_id: u64,
    instance_id: u64,
) -> Result<&mut PlacedDecor, String> {
    state
        .houses
        .get_mut(&house_id)
        .expect("validated house")
        .decor
        .iter_mut()
        .find(|decor| decor.instance_id == instance_id)
        .ok_or_else(|| "Placed decor not found in this house.".into())
}

fn apply_edit(
    state: &mut State,
    account: u64,
    house_id: u64,
    command: &HousingCommand,
) -> Result<(), String> {
    match command {
        HousingCommand::PlaceDecor {
            decor_id,
            variant_id,
            transform,
            ..
        } => {
            validate_transform(transform)?;
            let dyes = variants::take_variant(state, account, *decor_id, *variant_id)?;
            let storage = state
                .storage
                .get_mut(&account)
                .ok_or("You do not own this decor.")?;
            let quantity = storage
                .get_mut(decor_id)
                .ok_or("No stored copies of this decor remain.")?;
            *quantity = quantity
                .checked_sub(1)
                .ok_or("No stored copies of this decor remain.")?;
            if *quantity == 0 {
                storage.remove(decor_id);
            }
            let instance_id = state.next_instance_id;
            state.next_instance_id = instance_id
                .checked_add(1)
                .ok_or("Decor identity range exhausted.")?;
            state
                .houses
                .get_mut(&house_id)
                .expect("validated house")
                .decor
                .push(PlacedDecor {
                    instance_id,
                    decor_id: *decor_id,
                    transform: *transform,
                    dyes,
                });
        }
        HousingCommand::MoveDecor {
            instance_id,
            position,
            ..
        } => {
            validate_position(position)?;
            find_decor(state, house_id, *instance_id)?
                .transform
                .position = *position;
        }
        HousingCommand::RotateDecor {
            instance_id, yaw, ..
        } => {
            validate_yaw(*yaw)?;
            find_decor(state, house_id, *instance_id)?.transform.yaw = *yaw;
        }
        HousingCommand::TransformDecor {
            instance_id,
            transform,
            ..
        } => {
            validate_transform(transform)?;
            find_decor(state, house_id, *instance_id)?.transform = *transform;
        }
        HousingCommand::RemoveDecor { instance_id, .. } => {
            let house = state.houses.get_mut(&house_id).expect("validated house");
            let index = house
                .decor
                .iter()
                .position(|decor| decor.instance_id == *instance_id)
                .ok_or("Placed decor not found in this house.")?;
            let decor = house.decor.remove(index);
            variants::store_variant(state, account, &decor)?;
            let quantity = state
                .storage
                .entry(account)
                .or_default()
                .entry(decor.decor_id)
                .or_default();
            *quantity = quantity
                .checked_add(1)
                .ok_or("Decor storage quantity overflow.")?;
        }
        _ => return Err("Not a basic decor edit.".into()),
    }
    Ok(())
}

#[cfg(test)]
#[path = "persistence_housing_tests.rs"]
mod tests;
