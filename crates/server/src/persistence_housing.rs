//! Atomic account housing: plot payment, owned decor and durable basic layouts.
//! Never grants StartingQuantity, exterior components or rooms from catalog membership.
use std::collections::BTreeMap;

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition, WriteTransaction};
use serde::{Deserialize, Serialize};
use shared::item_data::ItemDefinitionSource;
use shared::protocol::{
    HouseLayout, HousingCommand, HousingHouse, HousingPlot, HousingState, PlacedDecor, StoredDecor,
};

use super::{INVENTORIES, InventoryData, inventory_store};

pub(crate) const BUILD_KEY: &str = "dcfc90fffd79ba00406ae46f5f657592";
const STATE: TableDefinition<u8, &[u8]> = TableDefinition::new("housing_state");
const STATE_ROW: u8 = 0;
const NEIGHBORHOOD_LOCATIONS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("housing_neighborhood_locations");

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
    access_anyone: Option<bool>,
    decor: Vec<PlacedDecor>,
}

#[derive(Serialize, Deserialize)]
struct State {
    build_key: String,
    next_house_id: u64,
    next_instance_id: u64,
    houses: BTreeMap<u64, OwnedHouse>,
    storage: BTreeMap<u64, BTreeMap<u32, u32>>,
}

impl State {
    fn new() -> Self {
        Self {
            build_key: BUILD_KEY.into(),
            next_house_id: 1,
            next_instance_id: 1,
            houses: BTreeMap::new(),
            storage: BTreeMap::new(),
        }
    }
}

pub(crate) fn create_tables(txn: &WriteTransaction) {
    txn.open_table(NEIGHBORHOOD_LOCATIONS)
        .expect("create housing neighborhood locations");
    let mut table = txn.open_table(STATE).expect("create housing state table");
    let exists = table
        .get(STATE_ROW)
        .expect("read housing initialization")
        .is_some();
    if !exists {
        let bytes = serde_json::to_vec(&State::new()).expect("encode new housing state");
        table
            .insert(STATE_ROW, bytes.as_slice())
            .expect("initialize empty housing state");
    }
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
    txn.commit().map_err(|error| {
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
    txn.commit()
        .map_err(|error| format!("Commit housing state: {error}"))
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
        house_access_anyone: house.access_anyone,
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
        access_anyone: None,
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

fn require_visit(state: &State, actor: Actor, house_id: u64) -> Result<(), String> {
    let house = state.houses.get(&house_id).ok_or("House not found.")?;
    if house.account_id != actor.account_id && house.access_anyone != Some(true) {
        return Err("The owner has not allowed you to visit this house.".into());
    }
    Ok(())
}

pub(crate) fn set_access(
    db: &Database,
    actor: Actor,
    house_id: u64,
    allowed: bool,
) -> Result<(), String> {
    let txn = db.begin_write().map_err(|error| error.to_string())?;
    let mut state = read_in(&txn)?;
    require_owner(&state, actor, house_id)?;
    state
        .houses
        .get_mut(&house_id)
        .expect("validated house")
        .access_anyone = Some(allowed);
    commit(txn, &state)
}

pub(crate) fn snapshot(
    db: &Database,
    actor: Actor,
    house_id: Option<u64>,
) -> Result<HousingState, String> {
    let txn = db.begin_read().map_err(|error| error.to_string())?;
    let state = read_state(&txn.open_table(STATE).map_err(|error| error.to_string())?)?;
    let layout = if let Some(house_id) = house_id {
        require_visit(&state, actor, house_id)?;
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
    Ok(HousingState {
        build_key: state.build_key,
        neighborhood_id: actor.neighborhood_id,
        plots: Vec::new(),
        owned_houses,
        stored,
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

pub(crate) fn house(db: &Database, actor: Actor, house_id: u64) -> Result<HousingHouse, String> {
    let txn = db.begin_read().map_err(|error| error.to_string())?;
    let state = read_state(&txn.open_table(STATE).map_err(|error| error.to_string())?)?;
    require_visit(&state, actor, house_id)?;
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

pub(crate) fn edit(db: &Database, actor: Actor, command: &HousingCommand) -> Result<(), String> {
    let house_id = edit_house_id(command)?;
    let txn = db.begin_write().map_err(|error| error.to_string())?;
    let mut state = read_in(&txn)?;
    require_owner(&state, actor, house_id)?;
    if actor.inside_house != Some(house_id) {
        return Err("Enter your house before editing decor.".into());
    }
    apply_edit(&mut state, actor.account_id, house_id, command)?;
    commit(txn, &state)
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
            transform,
            ..
        } => {
            validate_position(&transform.position)?;
            validate_yaw(transform.yaw)?;
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
        HousingCommand::RemoveDecor { instance_id, .. } => {
            let house = state.houses.get_mut(&house_id).expect("validated house");
            let index = house
                .decor
                .iter()
                .position(|decor| decor.instance_id == *instance_id)
                .ok_or("Placed decor not found in this house.")?;
            let decor = house.decor.remove(index);
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
