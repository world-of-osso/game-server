//! Owner-only bag and equipment networking.
//!
//! Enter world sends `InventorySnapshot` + `EquipmentSnapshot`. Item requests run
//! against `InventoryModel`, persist, and record the character in
//! `GoldUpdateQueue`; `send_inventory_deltas` then diffs the persisted state
//! against the last view sent to the owner; a changed bag slot also resends
//! `InventorySnapshot`, whose container sizes follow the equipped bags. Trade, auction mail and profession
//! paths record the same queue, so their item changes reach the owner as deltas.
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use redb::Database;
use shared::components::{EquipmentAppearance, Player};
use shared::protocol::{
    DestroyItem, EquipItem, EquipmentSnapshot, InventoryChannel, InventoryDelta, InventoryError,
    InventoryErrorReason, InventorySlotChange, InventorySnapshot, ItemLocation, SortBags,
    SplitItem, SwapItem, UseItem,
};

use crate::appearance_collection::AppearanceSourceItems;
use crate::auth::{Authenticated, InWorld};
use crate::gold::GoldUpdateQueue;
use crate::item_templates::ItemTemplates;
use crate::networking::PlayerLink;
use crate::persistence::inventory_store::{
    InventoryRecord, load_inventory_record, save_inventory_record,
};
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
        app.init_resource::<crate::bank::ChangedCharacterBanks>()
            .add_systems(
                Update,
                (
                    send_initial_inventory,
                    process_swap_requests,
                    process_split_requests,
                    process_destroy_requests,
                    process_use_requests,
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

/// Persistence plus item templates: everything `load_model` reads, and the
/// requesting player's learned spells for `Player::CanDualWield`.
#[derive(SystemParam)]
struct InventoryStore<'w, 's> {
    db: Option<Res<'w, GameDatabase>>,
    templates: Option<Res<'w, ItemTemplates>>,
    links: Query<'w, 's, &'static PlayerLink>,
    known: Query<'w, 's, &'static crate::class_progression::KnownSpells>,
    spells: Option<Res<'w, crate::spell_info::SpellStore>>,
}

impl InventoryStore<'_, '_> {
    fn get(&self) -> Option<(&Database, &ItemTemplates)> {
        Some((&self.db.as_ref()?.db, self.templates.as_deref()?))
    }

    /// Learned effect40/155 spells supply TC's per-player weapon capabilities.
    fn configure_weapon_skills(&self, connection: Entity, model: &mut InventoryModel) {
        let known = self
            .links
            .get(connection)
            .ok()
            .and_then(|link| self.known.get(link.0).ok());
        model.set_weapon_skills(known, self.spells.as_deref());
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
    load_model_from_record(db, templates, character_id, record)
}

pub(crate) fn load_model_from_record(
    db: &Database,
    templates: &ItemTemplates,
    character_id: u64,
    record: InventoryRecord,
) -> Result<InventoryModel, InventoryError> {
    for item in record
        .inventory
        .items
        .iter()
        .chain(record.equipped.iter().map(|entry| &entry.item))
        .chain(record.nonvisual_equipped.iter().map(|(_, item)| item))
        .chain(record.bag_slots.iter().map(|entry| &entry.item))
    {
        let source = record.definition_sources[&item.item_guid];
        if templates.get(source, item.item_id).is_none() {
            return Err(error(
                InventoryErrorReason::ItemNotFound,
                format!(
                    "GUID {} has no {:?} definition for item {}",
                    item.item_guid, source, item.item_id
                ),
            ));
        }
    }
    let mut mint = || persistence::next_id(db, "item_instance");
    let mut model = InventoryModel::from_record(record, templates, &mut mint);
    model.load_bank_ownership(db, character_id);
    Ok(model)
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
    item: (u32, u32),
) -> Result<String, String> {
    grant_item_with_origin(db, templates, catalog, character_id, item, None)
}

pub(crate) fn grant_item_with_origin(
    db: &Database,
    templates: &ItemTemplates,
    catalog: &crate::merchant::VendorCatalog,
    character_id: u64,
    (item_id, count): (u32, u32),
    origin: Option<shared::protocol::CraftingOrigin>,
) -> Result<String, String> {
    let goods = catalog
        .goods
        .get(&item_id)
        .ok_or_else(|| format!("item {item_id} is not in content_item"))?;
    let template = *templates
        .get(shared::item_data::ItemDefinitionSource::Retail, item_id)
        .ok_or_else(|| format!("item {item_id} has no Retail template"))?;
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
    if let Some(origin) = origin {
        model.record_crafting_origin(item.item_guid, origin);
    }
    let mut mint = || persistence::next_id(db, "item_instance");
    model
        .store_item(
            item,
            shared::item_data::ItemDefinitionSource::Retail,
            template,
            &mut mint,
        )
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
                store.configure_weapon_skills(connection, &mut model);
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

type MoveRequests<'w, 's, M> = Query<
    'w,
    's,
    (
        Entity,
        &'static InWorld,
        Option<&'static crate::npc_interaction::ActiveInteraction>,
        &'static mut MessageReceiver<M>,
    ),
    With<Authenticated>,
>;

#[derive(SystemParam)]
struct MoveStore<'w, 's> {
    store: InventoryStore<'w, 's>,
    updates: ResMut<'w, GoldUpdateQueue>,
    errors: ErrorSenders<'w, 's>,
    banks: ResMut<'w, crate::bank::ChangedCharacterBanks>,
}

/// TC a352b1fa ItemHandler.cpp `HandleSwapItem`: both bank positions require
/// `CanUseBank`. Bank and inventory rows are saved together only on success.
fn process_item_moves<M: lightyear::prelude::Message>(
    receivers: &mut MoveRequests<M>,
    store: &mut MoveStore,
    locations: impl Fn(&M) -> (ItemLocation, ItemLocation),
    apply: impl Fn(&mut InventoryModel, M, &Database) -> OpResult,
) {
    let Some((db, templates)) = store.store.get() else {
        return;
    };
    for (connection, in_world, interaction, mut receiver) in receivers.iter_mut() {
        let character_id = in_world.character_id;
        for request in receiver.receive() {
            let (from, to) = locations(&request);
            let bank_move = matches!(from, ItemLocation::Bank { .. })
                || matches!(to, ItemLocation::Bank { .. });
            let banker_open =
                interaction.is_some_and(|i| i.role == Some(shared::protocol::NpcRole::Banker));
            let result = apply_item_move(
                db,
                templates,
                character_id,
                bank_move,
                banker_open,
                |model| {
                    store.store.configure_weapon_skills(connection, model);
                    apply(model, request, db)
                },
            );
            match result {
                Ok(()) => {
                    store.updates.record_items(character_id);
                    if bank_move {
                        store.banks.0.insert(connection);
                    }
                }
                Err(error) => send_error(connection, &mut store.errors, error),
            }
        }
    }
}

fn apply_item_move(
    db: &Database,
    templates: &ItemTemplates,
    character_id: u64,
    bank_move: bool,
    banker_open: bool,
    apply: impl FnOnce(&mut InventoryModel) -> OpResult,
) -> OpResult {
    if bank_move && !banker_open {
        return Err(error(
            InventoryErrorReason::CantDoRightNow,
            "no character bank open",
        ));
    }
    let mut model = load_model(db, templates, character_id)?;
    if bank_move {
        model.attach_bank(
            db,
            templates,
            persistence::banks::load_character_bank(db, character_id),
        )?;
    }
    apply(&mut model)?;
    let bank = model.take_bank();
    let record = model.into_record();
    match bank {
        Some(bank) => persistence::banks::save_bank_move(
            db,
            character_id,
            persistence::banks::BankOwner::Character(character_id, &bank),
            Some(&record),
        ),
        None => save_inventory_record(db, character_id, &record),
    }
    Ok(())
}

fn process_swap_requests(mut receivers: MoveRequests<SwapItem>, mut store: MoveStore) {
    process_item_moves(
        &mut receivers,
        &mut store,
        |req| (req.from, req.to),
        |model, req, _| model.swap(req.from, req.to),
    );
}

fn process_split_requests(mut receivers: MoveRequests<SplitItem>, mut store: MoveStore) {
    process_item_moves(
        &mut receivers,
        &mut store,
        |req| (req.from, req.to),
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

/// `WorldSession::HandleUseItemOpcode` → `Player::ProcessItemCast` → `CastItemUseSpell`
/// (SpellHandler.cpp:49-63, Player.cpp:31665-31740, 8790-8840): the item's on-use spell
/// is cast as a spell the item grants (no spellbook check) at the requested target,
/// else the player's current target. Refusals answer `InventoryError`; the cast path
/// answers its own `CastFailed`.
pub(crate) fn process_use_requests(world: &mut World) {
    let mut receivers = world.query_filtered::<(
        Entity,
        &InWorld,
        &crate::networking::PlayerLink,
        &mut MessageReceiver<UseItem>,
    ), With<Authenticated>>();
    let requests: Vec<_> = receivers
        .iter_mut(world)
        .flat_map(|(connection, in_world, link, mut receiver)| {
            let (character_id, player) = (in_world.character_id, link.0);
            receiver
                .receive()
                .map(move |request| (connection, character_id, player, request))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, character_id, player, request) in requests {
        let result =
            crate::guild::charters::use_item(world, connection, character_id, request.location)
                .and_then(|handled| {
                    if handled {
                        Ok(())
                    } else {
                        use_item(world, character_id, player, &request)
                    }
                });
        if let Err(error) = result {
            if let Some(mut sender) = world.get_mut::<MessageSender<InventoryError>>(connection) {
                sender.send::<InventoryChannel>(error);
            }
        }
    }
}

/// `SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL` (SharedDefines.h:464).
const SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL: u32 = 0x1000_0000;

/// One use: the item must be there (equipped, if equippable) with an on-use spell
/// that, in combat, allows combat (`SpellInfo::CanBeUsedInCombat`,
/// `EQUIP_ERR_NOT_IN_COMBAT`). Once the spell starts, an expendable item loses one from
/// its stack (`Spell::TakeCastItem`).
fn use_item(
    world: &mut World,
    character_id: u64,
    player: Entity,
    request: &UseItem,
) -> Result<(), InventoryError> {
    if crate::housing_acquisition::collect_item(world, character_id, player, request.location)? {
        return Ok(());
    }
    if crate::mount_journal::learn_item(world, character_id, player, request.location)? {
        return Ok(());
    }
    if crate::toys::learn_item(world, character_id, player, request.location)? {
        return Ok(());
    }
    let (db, templates) = use_store(world)?;
    let model = load_model(db, templates, character_id)?;
    let (on_use, item_scaling) = model.use_spell(request.location)?;
    let item = model.existing(request.location)?;
    let retail_item_id = (model.definition_source(item.item_guid)
        == shared::item_data::ItemDefinitionSource::Retail)
        .then_some(item.item_id);
    let item_level = world
        .resource::<crate::item_stats::ItemScalingTables>()
        .item_level(&item_scaling);
    let in_combat = world
        .get::<shared::components::CombatStatus>(player)
        .is_some_and(|status| status.0);
    let peaceful_only = world
        .get_resource::<crate::spell_info::SpellStore>()
        .and_then(|store| store.get(on_use.spell_id))
        .and_then(|spell| spell.misc.as_ref())
        .is_some_and(|misc| misc.attributes[0] & SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL != 0);
    if in_combat && peaceful_only {
        return Err(error(
            InventoryErrorReason::NotInCombat,
            format!("spell {} cannot be used in combat", on_use.spell_id),
        ));
    }
    let target = request
        .target
        .and_then(Entity::try_from_bits)
        .filter(|&target| world.get_entity(target).is_ok())
        .or_else(|| {
            world
                .get::<crate::networking::CombatTarget>(player)
                .and_then(|target| target.0)
        });
    let spell = on_use.spell_id;
    let learned_pet = crate::battle_pets::learn_companion_from_item(world, player, spell, 1)
        .map_err(|message| error(InventoryErrorReason::CantDoRightNow, message))?;
    if learned_pet {
        let (db, templates) = use_store(world)?;
        let mut model = load_model(db, templates, character_id)?;
        model.destroy(request.location, 1)?;
        save_inventory_record(db, character_id, &model.into_record());
        world
            .resource_mut::<GoldUpdateQueue>()
            .record_items(character_id);
        if let Some(item) = retail_item_id {
            crate::achievement::record_item_use(world, player, item);
        }
        return Ok(());
    }
    if let Err(failure) = crate::spell_cast::cast_item_spell(
        world,
        player,
        spell,
        target,
        on_use.cooldown,
        item_level,
        retail_item_id,
    ) {
        debug!("Item use spell {spell} by {player} failed: {failure:?}");
        return Ok(());
    }
    if on_use.charges < 0 {
        let (db, templates) = use_store(world)?;
        let mut model = load_model(db, templates, character_id)?;
        model.destroy(request.location, 1)?;
        save_inventory_record(db, character_id, &model.into_record());
        world
            .resource_mut::<GoldUpdateQueue>()
            .record_items(character_id);
    }
    Ok(())
}

pub(crate) fn use_store(world: &World) -> Result<(&Database, &ItemTemplates), InventoryError> {
    match (
        world.get_resource::<GameDatabase>(),
        world.get_resource::<ItemTemplates>(),
    ) {
        (Some(db), Some(templates)) => Ok((&db.db, templates)),
        _ => Err(error(
            InventoryErrorReason::InternalBagError,
            "no inventory database or item templates",
        )),
    }
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
    mut senders: Query<(
        &mut MessageSender<InventoryDelta>,
        &mut MessageSender<InventorySnapshot>,
    )>,
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
        let snapshot = model.inventory_snapshot();
        commands
            .entity(connection)
            .insert(AppearanceSourceItems(model.appearance_sources()));
        let appearance = model.appearance().clone();
        let equipped = model.equipped_items();
        save_inventory_record(db, in_world.character_id, &model.into_record());
        let changes = diff_views(&sent.0, &view);
        if !changes.is_empty()
            && let Ok((mut sender, mut snapshot_sender)) = senders.get_mut(connection)
        {
            if changes.iter().any(changes_bag_slot) {
                snapshot_sender.send::<InventoryChannel>(snapshot);
            }
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

/// A bag slot's item changed, so its container's size may have too.
fn changes_bag_slot(change: &InventorySlotChange) -> bool {
    matches!(change.location, ItemLocation::Equipment(slot) if slot.bag_index().is_some())
}

#[cfg(test)]
#[path = "inventory_max_count_tests.rs"]
mod max_count_tests;

#[cfg(test)]
#[path = "item_use_tests.rs"]
mod item_use_tests;

#[cfg(test)]
#[path = "inventory_tests.rs"]
mod tests;
