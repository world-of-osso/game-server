//! Owner-only bag and equipment networking.
//!
//! Enter world sends `InventorySnapshot` + `EquipmentSnapshot`. Item requests run
//! against `InventoryModel`, persist, and record the character in
//! `GoldUpdateQueue`; `send_inventory_deltas` then diffs the persisted state
//! against the last view sent to the owner. Trade, auction mail and profession
//! paths record the same queue, so their item changes reach the owner as deltas.
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use redb::Database;
use shared::components::{EquipmentAppearance, Player};
use shared::protocol::{
    DestroyItem, EquipItem, EquipmentSnapshot, InventoryChannel, InventoryDelta, InventoryError,
    InventoryErrorReason, InventorySnapshot, SortBags, SplitItem, SwapItem,
};

use crate::appearance_collection::AppearanceSourceItems;
use crate::auth::{Authenticated, InWorld};
use crate::gold::GoldUpdateQueue;
use crate::item_templates::ItemTemplates;
use crate::networking::PlayerLink;
use crate::persistence::inventory_store::{load_inventory_record, save_inventory_record};
use crate::persistence::{self, GameDatabase};
use crate::stats::EquippedItems;

#[path = "inventory_model.rs"]
pub(crate) mod inventory_model;

use inventory_model::{InventoryModel, InventoryView, diff_views, error};

/// Last inventory view sent to this connection; deltas diff against it.
#[derive(Component, Debug, Default)]
pub(crate) struct SentInventoryView(InventoryView);

/// `send_inventory_deltas`, which drains `GoldUpdateQueue`'s changed inventories.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct InventoryDeltas;

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                send_initial_inventory,
                process_swap_requests,
                process_split_requests,
                process_destroy_requests,
                process_equip_requests,
                process_sort_requests,
            ),
        );
        app.add_systems(
            PostUpdate,
            send_inventory_deltas
                .in_set(InventoryDeltas)
                .before(ReplicationSystems::Send),
        );
    }
}

type Requests<'w, 's, M> =
    Query<'w, 's, (Entity, &'static InWorld, &'static mut MessageReceiver<M>), With<Authenticated>>;
type ErrorSenders<'w, 's> = Query<'w, 's, &'static mut MessageSender<InventoryError>>;
type OpResult = Result<(), InventoryError>;

/// Persistence plus item templates: everything `load_model` reads.
#[derive(SystemParam)]
struct InventoryStore<'w> {
    db: Option<Res<'w, GameDatabase>>,
    templates: Option<Res<'w, ItemTemplates>>,
}

impl InventoryStore<'_> {
    fn get(&self) -> Option<(&Database, &ItemTemplates)> {
        Some((&self.db.as_ref()?.db, self.templates.as_deref()?))
    }
}

pub(crate) fn load_model(
    db: &Database,
    templates: &ItemTemplates,
    character_id: u64,
) -> Result<InventoryModel, InventoryError> {
    let record = load_inventory_record(db, character_id).ok_or_else(|| {
        error(
            InventoryErrorReason::InternalBagError,
            format!("character {character_id} has no record"),
        )
    })?;
    let mut mint = || persistence::next_id(db, "item_instance");
    Ok(InventoryModel::from_record(record, templates, &mut mint))
}

/// Store `count` of `item_id` in the character's bags (AzerothCore `StoreNewItem`),
/// with its `content_item` name, quality and sell price; the item's name on success.
/// Nothing is saved when it does not fit. Callers record the change in
/// `GoldUpdateQueue` so the owner gets an `InventoryDelta`.
pub(crate) fn grant_item(
    db: &Database,
    templates: &ItemTemplates,
    catalog: &crate::merchant::VendorCatalog,
    character_id: u64,
    (item_id, count): (u32, u32),
) -> Result<String, String> {
    let goods = catalog
        .goods
        .get(&item_id)
        .ok_or_else(|| format!("item {item_id} is not in content_item"))?;
    let template = *templates
        .get(item_id)
        .ok_or_else(|| format!("item {item_id} has no template"))?;
    let mut model =
        load_model(db, templates, character_id).map_err(|error| format!("{error:?}"))?;
    let item = persistence::InventoryItemData {
        item_guid: persistence::next_id(db, "item_instance"),
        item_id,
        name: goods.name.clone(),
        quality: goods.quality,
        required_level: template.required_level,
        stack_count: count,
        vendor_sell_price: goods.sell_price,
        unique_auction: false,
    };
    let mut mint = || persistence::next_id(db, "item_instance");
    model
        .store_item(item, template, &mut mint)
        .map_err(|error| format!("{error:?}"))?;
    save_inventory_record(db, character_id, &model.into_record());
    Ok(goods.name.clone())
}

/// Empty bag cells once every owned item is placed; 0 when the record is missing.
pub(crate) fn free_bag_cells(db: &Database, templates: &ItemTemplates, character_id: u64) -> usize {
    load_model(db, templates, character_id).map_or(0, |model| model.free_cells())
}

fn send_error(connection: Entity, senders: &mut ErrorSenders, error: InventoryError) {
    if let Ok(mut sender) = senders.get_mut(connection) {
        sender.send::<InventoryChannel>(error);
    }
}

fn send_initial_inventory(
    mut commands: Commands,
    connections: Query<(Entity, &InWorld, &PlayerLink), (With<Authenticated>, Added<InWorld>)>,
    store: InventoryStore,
    mut inventory_senders: Query<&mut MessageSender<InventorySnapshot>>,
    mut equipment_senders: Query<&mut MessageSender<EquipmentSnapshot>>,
    mut errors: ErrorSenders,
) {
    let Some((db, templates)) = store.get() else {
        return;
    };
    for (connection, in_world, link) in &connections {
        let model = match load_model(db, templates, in_world.character_id) {
            Ok(model) => model,
            Err(error) => {
                send_error(connection, &mut errors, error);
                continue;
            }
        };
        if let Ok(mut sender) = inventory_senders.get_mut(connection) {
            sender.send::<InventoryChannel>(model.inventory_snapshot());
        }
        if let Ok(mut sender) = equipment_senders.get_mut(connection) {
            sender.send::<InventoryChannel>(model.equipment_snapshot());
        }
        commands.entity(connection).insert((
            SentInventoryView(model.view()),
            AppearanceSourceItems(model.appearance_sources()),
        ));
        commands.entity(link.0).insert(model.equipped_items());
        // Persist placements chosen for previously unplaced items.
        save_inventory_record(db, in_world.character_id, &model.into_record());
    }
}

/// Apply each queued request to the sender's own inventory and persist on success.
fn process_requests<M: lightyear::prelude::Message>(
    receivers: &mut Requests<M>,
    store: InventoryStore,
    updates: &mut GoldUpdateQueue,
    errors: &mut ErrorSenders,
    mut apply: impl FnMut(&mut InventoryModel, M, &Database) -> OpResult,
) {
    let Some((db, templates)) = store.get() else {
        return;
    };
    for (connection, in_world, mut receiver) in receivers.iter_mut() {
        let character_id = in_world.character_id;
        for request in receiver.receive() {
            let result = load_model(db, templates, character_id).and_then(|mut model| {
                apply(&mut model, request, db)?;
                save_inventory_record(db, character_id, &model.into_record());
                Ok(())
            });
            match result {
                Ok(()) => updates.record_items(character_id),
                Err(error) => send_error(connection, errors, error),
            }
        }
    }
}

fn process_swap_requests(
    mut receivers: Requests<SwapItem>,
    store: InventoryStore,
    mut updates: ResMut<GoldUpdateQueue>,
    mut errors: ErrorSenders,
) {
    process_requests(
        &mut receivers,
        store,
        &mut updates,
        &mut errors,
        |model, req, _| model.swap(req.from, req.to),
    );
}

fn process_split_requests(
    mut receivers: Requests<SplitItem>,
    store: InventoryStore,
    mut updates: ResMut<GoldUpdateQueue>,
    mut errors: ErrorSenders,
) {
    process_requests(
        &mut receivers,
        store,
        &mut updates,
        &mut errors,
        |model, req, db| {
            let new_guid = persistence::next_id(db, "item_instance");
            model.split(req.from, req.to, req.count, new_guid)
        },
    );
}

fn process_destroy_requests(
    mut receivers: Requests<DestroyItem>,
    store: InventoryStore,
    mut updates: ResMut<GoldUpdateQueue>,
    mut errors: ErrorSenders,
) {
    process_requests(
        &mut receivers,
        store,
        &mut updates,
        &mut errors,
        |model, req, _| model.destroy(req.location, req.count),
    );
}

fn process_equip_requests(
    mut receivers: Requests<EquipItem>,
    store: InventoryStore,
    mut updates: ResMut<GoldUpdateQueue>,
    mut errors: ErrorSenders,
) {
    process_requests(
        &mut receivers,
        store,
        &mut updates,
        &mut errors,
        |model, req, _| model.auto_equip(req.from),
    );
}

fn process_sort_requests(
    mut receivers: Requests<SortBags>,
    store: InventoryStore,
    mut updates: ResMut<GoldUpdateQueue>,
    mut errors: ErrorSenders,
) {
    process_requests(
        &mut receivers,
        store,
        &mut updates,
        &mut errors,
        |model, _, _| model.sort_bags(),
    );
}

type DeltaConnections<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static InWorld,
        &'static PlayerLink,
        &'static mut SentInventoryView,
    ),
>;

/// Send each changed character's owner the locations that differ from its last view,
/// and refresh the replicated `EquipmentAppearance` and the connection's
/// `AppearanceSourceItems`.
fn send_inventory_deltas(
    mut commands: Commands,
    mut updates: ResMut<GoldUpdateQueue>,
    store: InventoryStore,
    mut connections: DeltaConnections,
    mut players: Query<(&mut EquipmentAppearance, Option<&mut EquippedItems>), With<Player>>,
    dirty_queue: Option<Res<ReplicationDirtyQueue>>,
    mut senders: Query<&mut MessageSender<InventoryDelta>>,
) {
    let changed = updates.take_changed_inventories();
    let Some((db, templates)) = store.get() else {
        return;
    };
    if changed.is_empty() {
        return;
    }
    for (connection, in_world, link, mut sent) in &mut connections {
        if !changed.contains(&in_world.character_id) {
            continue;
        }
        let Ok(model) = load_model(db, templates, in_world.character_id) else {
            continue;
        };
        let view = model.view();
        commands
            .entity(connection)
            .insert(AppearanceSourceItems(model.appearance_sources()));
        let appearance = model.appearance().clone();
        let equipped = model.equipped_items();
        save_inventory_record(db, in_world.character_id, &model.into_record());
        let changes = diff_views(&sent.0, &view);
        if !changes.is_empty()
            && let Ok(mut sender) = senders.get_mut(connection)
        {
            sender.send::<InventoryChannel>(InventoryDelta { changes });
            sent.0 = view;
        }
        let Ok((mut current, current_equipped)) = players.get_mut(link.0) else {
            continue;
        };
        if let Some(mut current_equipped) = current_equipped {
            current_equipped.set_if_neq(equipped);
        }
        if *current != appearance {
            *current = appearance;
            if let Some(dirty_queue) = dirty_queue.as_deref() {
                dirty_queue.mark(link.0);
            }
        }
    }
}

#[cfg(test)]
#[path = "inventory_tests.rs"]
mod tests;
