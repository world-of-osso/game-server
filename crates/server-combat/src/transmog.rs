//! Retail application at an authenticated Transmogrifier. See specs/transmog-apply.md.
mod custom_sets;
mod data;
mod outfits;
use bevy::prelude::*;
use lightyear::prelude::*;
use rusqlite::Connection;
use shared::components::{EquipmentAppearance, Health, Position, UnitFactionTemplate};
use shared::item_data::ItemDefinitionSource;
use shared::protocol::*;
use std::collections::{HashMap, HashSet};

use crate::auth::{Authenticated, InWorld};
use crate::faction::{FactionTemplates, Reaction};
use crate::gold::GoldUpdateQueue;
use crate::item_templates::{ItemTemplate, ItemTemplates};
use crate::networking::PlayerLink;
use crate::npc_interaction::{ActiveInteraction, INTERACTION_DISTANCE};
use crate::persistence::{
    GameDatabase,
    inventory_store::{InventoryRecord, load_inventory_record, save_inventory_record},
};
use crate::transmog_store::ItemTransmog;

pub(crate) fn authored_set_sources(set_id: u32) -> Result<Vec<u32>, String> {
    let wardrobe = data::Wardrobe::load_embedded();
    let set = wardrobe
        .sets()
        .iter()
        .find(|set| set.set_id == set_id)
        .ok_or_else(|| format!("TransmogSet{set_id} missing from pinned catalog"))?;
    if set.condition_id != 0 {
        return Err(format!(
            "TransmogSet{set_id} PlayerCondition{} unavailable",
            set.condition_id
        ));
    }
    if set.source_ids.is_empty() {
        return Err(format!("TransmogSet{set_id} has no readable members"));
    }
    Ok(set.source_ids.clone())
}

const OVERRIDE_GOLD_COST: i64 = 0x4000;
const NO_ALTER_VISUAL: i64 = 0x200000;
const NO_SOURCE_VISUAL: i64 = 0x400000;
const ARMOR: u8 = 4;
const WEAPON: u8 = 2;
const COSMETIC_ARMOR: u8 = 5;
const FISHING_POLE: u8 = 20;

#[derive(Clone)]
struct Source {
    view: TransmogSource,
    template: ItemTemplate,
    collectable: bool,
    offer: bool,
}

#[derive(Resource, Default)]
pub(crate) struct TransmogCatalog {
    sources: HashMap<u32, Source>,
    by_appearance: HashMap<u32, Vec<u32>>,
    prices: HashMap<(ItemDefinitionSource, u32), u64>,
    catalogue: Vec<Source>,
    /// ItemModifiedAppearance.ID -> index into `catalogue`.
    catalogue_index: HashMap<u32, usize>,
    wardrobe: data::Wardrobe,
}

impl TransmogCatalog {
    pub(crate) fn load(conn: &Connection) -> rusqlite::Result<Self> {
        let mut catalog = Self {
            wardrobe: data::Wardrobe::load_embedded(),
            ..Default::default()
        };
        let mut stmt = conn.prepare(
            "SELECT m.ID, m.ItemAppearanceID, i.ID, a.ItemDisplayInfoID, i.Name,
             i.InventoryType, i.ClassID, i.SubclassID, i.RequiredLevel, i.AllowableClass,
             i.AllowableRaces_0, i.AllowableRaces_1, i.Flags_1, a.DefaultIconFileDataID,
             i.Quality, m.TransmogSourceTypeEnum, a.TransmogPlayerConditionID
             FROM item_modified_appearance m JOIN item_appearance a ON a.ID=m.ItemAppearanceID
             JOIN content_item i ON i.ID=m.ItemID
             WHERE a.ItemDisplayInfoID>0 ORDER BY m.ID",
        )?;
        for row in stmt.query_map([], source_from_row)? {
            let source = row?;
            catalog
                .catalogue_index
                .insert(source.view.source_id, catalog.catalogue.len());
            catalog.catalogue.push(source.clone());
            if !source.offer {
                continue;
            }
            catalog
                .by_appearance
                .entry(source.view.appearance_id)
                .or_default()
                .push(source.view.source_id);
            catalog.sources.insert(source.view.source_id, source);
        }
        for (table, source) in [
            ("content_item", ItemDefinitionSource::Retail),
            ("skyborne_content_item", ItemDefinitionSource::Forever70205),
        ] {
            let mut stmt = conn.prepare(&format!(
                "SELECT ID, SellPrice FROM {table} WHERE (Flags_1 & {OVERRIDE_GOLD_COST})<>0"
            ))?;
            for row in
                stmt.query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u64>(1)?)))?
            {
                let (id, price) = row?;
                catalog.prices.insert((source, id), price);
            }
        }
        Ok(catalog)
    }

    fn eligible_sources(
        &self,
        target: &ItemTemplate,
        character: &crate::persistence::CharacterData,
        collection: &shared::transmog::AppearanceCollection,
    ) -> Vec<TransmogSource> {
        let mut sources: Vec<_> = collection
            .all()
            .filter_map(|id| self.by_appearance.get(&id))
            .flatten()
            .filter_map(|id| self.sources.get(id))
            .filter(|source| {
                source.template.allows(character.class, character.race)
                    && source.template.required_level <= character.level
                    && compatible(target, &source.template)
            })
            .map(|source| source.view.clone())
            .collect();
        sources.sort_by(|a, b| a.name.cmp(&b.name).then(a.source_id.cmp(&b.source_id)));
        sources
    }

    /// Sets the account has started. TrinityCore a352b1fa CollectionMgr::IsSetCompleted:
    /// members group by transmog slot and a slot is known when any member is collected.
    /// Members absent from world.db content_item are unknown here and omitted.
    fn available_sets(
        &self,
        class: u8,
        collection: &shared::transmog::AppearanceCollection,
    ) -> Vec<TransmogSet> {
        self.wardrobe
            .sets()
            .iter()
            .filter(|set| set.usable_by(class))
            .filter_map(|set| {
                let mut pieces = std::collections::BTreeMap::new();
                for id in &set.source_ids {
                    let Some(source) = self.catalogue_index.get(id).map(|&i| &self.catalogue[i])
                    else {
                        continue;
                    };
                    let Some(slot) = EquipmentSlot::for_inventory_type(source.view.inventory_type)
                        .iter()
                        .copied()
                        .find(|slot| slot.visual_slot().is_some())
                    else {
                        continue;
                    };
                    let state = collection_state(source, collection);
                    let known = pieces
                        .get(&slot)
                        .is_some_and(|piece: &TransmogCatalogueSource| {
                            piece.collection == TransmogCollectionState::Collected
                                || state != TransmogCollectionState::Collected
                        });
                    if !known {
                        pieces.insert(
                            slot,
                            TransmogCatalogueSource {
                                source: source.view.clone(),
                                collection: state,
                                slots: vec![slot],
                            },
                        );
                    }
                }
                let started = pieces
                    .values()
                    .any(|piece| piece.collection == TransmogCollectionState::Collected);
                started.then(|| TransmogSet {
                    set_id: set.set_id,
                    parent_set_id: set.parent_set_id,
                    name: set.name.clone(),
                    source_ids: set.source_ids.clone(),
                    primary_appearances: pieces.into_values().collect(),
                })
            })
            .collect()
    }

    fn state(
        &self,
        npc: u64,
        record: &InventoryRecord,
        templates: &ItemTemplates,
        collection: &shared::transmog::AppearanceCollection,
    ) -> TransmogState {
        let slots = record
            .equipped
            .iter()
            .filter_map(|equipped| {
                let definition = record.definition_sources[&equipped.item.item_guid];
                let target = templates.get(definition, equipped.item.item_id)?;
                if !valid_target(target) {
                    return None;
                }
                let slot = EquipmentSlot::from_visual_slot(equipped.slot);
                Some(TransmogSlot {
                    slot,
                    item_guid: equipped.item.item_guid,
                    item_id: equipped.item.item_id,
                    definition_source: definition,
                    inventory_type: target.inventory_type,
                    applied: record
                        .transmogs
                        .get(&equipped.item.item_guid)
                        .map_or(TransmogChoice::Original, |value| value.choice()),
                    cost: self
                        .prices
                        .get(&(definition, equipped.item.item_id))
                        .copied(),
                    can_hide: can_hide(slot),
                    sources: if definition == ItemDefinitionSource::Retail {
                        self.eligible_sources(target, &record.character, collection)
                    } else {
                        // Retail display IDs cannot be interpreted in a different item namespace.
                        Vec::new()
                    },
                })
            })
            .collect();
        let mut state = TransmogState {
            npc,
            slots,
            ..Default::default()
        };
        state.sets = self.available_sets(record.character.class, collection);
        self.wardrobe.populate(&mut state);
        state
    }
}

fn collection_state(
    source: &Source,
    collection: &shared::transmog::AppearanceCollection,
) -> TransmogCollectionState {
    if !source.collectable {
        TransmogCollectionState::NonCollectable
    } else if collection.has(source.view.appearance_id) {
        TransmogCollectionState::Collected
    } else {
        TransmogCollectionState::Uncollected
    }
}

fn source_from_row(row: &rusqlite::Row) -> rusqlite::Result<Source> {
    let low: i64 = row.get(10)?;
    let high: i64 = row.get(11)?;
    let template = ItemTemplate {
        inventory_type: row.get(5)?,
        required_level: row.get(8)?,
        allowable_class: row.get(9)?,
        allowable_race: (u64::from(high as u32) << 32) | u64::from(low as u32),
        scaling: crate::item_stats::ItemScaling {
            class_id: row.get(6)?,
            subclass_id: row.get(7)?,
            flags2: row.get(12)?,
            ..Default::default()
        },
        ..Default::default()
    };
    let quality: u8 = row.get(14)?;
    let source_type: u8 = row.get(15)?;
    let condition: u32 = row.get(16)?;
    let source_visual = template.scaling.flags2 & NO_SOURCE_VISUAL == 0 && quality != 6;
    Ok(Source {
        collectable: source_visual && !matches!(source_type, 6 | 9),
        offer: source_visual && condition == 0,
        view: TransmogSource {
            source_id: row.get(0)?,
            source_type,
            appearance_id: row.get(1)?,
            item_id: row.get(2)?,
            display_id: row.get(3)?,
            icon_fdid: row.get(13)?,
            name: row.get(4)?,
            inventory_type: template.inventory_type,
        },
        template,
    })
}

fn valid_target(target: &ItemTemplate) -> bool {
    let supported_class = matches!(target.scaling.class_id, ARMOR | WEAPON);
    let fishing = target.scaling.class_id == WEAPON && target.scaling.subclass_id == FISHING_POLE;
    supported_class
        && !fishing
        && target.scaling.flags2 & NO_ALTER_VISUAL == 0
        && EquipmentSlot::for_inventory_type(target.inventory_type)
            .iter()
            .any(|slot| slot.visual_slot().is_some())
}

fn compatible(target: &ItemTemplate, source: &ItemTemplate) -> bool {
    if target.scaling.class_id != source.scaling.class_id
        || source.scaling.flags2 & NO_SOURCE_VISUAL != 0
    {
        return false;
    }
    if target.scaling.class_id == WEAPON {
        return target.scaling.subclass_id == source.scaling.subclass_id
            || weapon_group(target.scaling.subclass_id)
                .is_some_and(|group| Some(group) == weapon_group(source.scaling.subclass_id));
    }
    compatible_slot_types(target, source)
}

fn compatible_slot_types(target: &ItemTemplate, source: &ItemTemplate) -> bool {
    let target_slots = EquipmentSlot::for_inventory_type(target.inventory_type);
    let source_slots = EquipmentSlot::for_inventory_type(source.inventory_type);
    target_slots == source_slots
        && !source_slots.is_empty()
        && (target.scaling.subclass_id == source.scaling.subclass_id
            || source.scaling.subclass_id == COSMETIC_ARMOR)
}

/// Pinned ItemTemplate::GetWeaponTransmogOutfitSlotOption, not strict subclass equality.
fn weapon_group(subclass: u8) -> Option<u8> {
    match subclass {
        0 | 4 | 7 | 9 | 13 | 15 => Some(1),
        1 | 5 | 6 | 8 | 10 => Some(2),
        2 | 3 | 18 => Some(3),
        _ => None,
    }
}

/// Pinned CollectionMgr hidden-item grants: helm, cloak, shoulders, chest, shirt,
/// tabard, bracers, gloves, belt, boots. Neither legs nor weapons has a grant.
fn can_hide(slot: EquipmentSlot) -> bool {
    matches!(
        slot,
        EquipmentSlot::Head
            | EquipmentSlot::Shoulder
            | EquipmentSlot::Back
            | EquipmentSlot::Chest
            | EquipmentSlot::Shirt
            | EquipmentSlot::Tabard
            | EquipmentSlot::Wrist
            | EquipmentSlot::Hands
            | EquipmentSlot::Waist
            | EquipmentSlot::Feet
    )
}

pub struct TransmogPlugin;
impl Plugin for TransmogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                send_opened_states,
                process_requests,
                outfits::process_saves,
                custom_sets::process_saves,
            )
                .chain()
                .after(crate::npc_interaction::NpcInteractionsProcessed),
        );
    }
}

fn collect_requests<M: lightyear::prelude::Message>(world: &mut World) -> Vec<(Entity, M)> {
    let mut query = world.query::<(Entity, &mut MessageReceiver<M>)>();
    query
        .iter_mut(world)
        .flat_map(|(entity, mut receiver)| {
            receiver
                .receive()
                .map(move |message| (entity, message))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn process_requests(world: &mut World) {
    for (connection, request) in collect_requests::<RequestTransmog>(world) {
        respond(world, connection, request.npc, None);
    }
    for (connection, request) in collect_requests::<ApplyTransmog>(world) {
        respond(world, connection, request.npc, Some(request.changes));
    }
}

fn send_opened_states(world: &mut World) {
    let mut query =
        world.query_filtered::<(Entity, &ActiveInteraction), Changed<ActiveInteraction>>();
    let opened: Vec<_> = query
        .iter(world)
        .filter(|(_, active)| active.role == Some(NpcRole::Transmogrifier))
        .map(|(entity, active)| (entity, active.npc.to_bits()))
        .collect();
    for (connection, npc) in opened {
        respond(world, connection, npc, None);
    }
}

fn respond(world: &mut World, connection: Entity, npc: u64, changes: Option<Vec<TransmogChange>>) {
    match handle(world, connection, npc, changes) {
        Ok(state) => {
            if let Some(mut sender) = world.get_mut::<MessageSender<TransmogState>>(connection) {
                sender.send::<TransmogChannel>(state);
            }
        }
        Err(error) => {
            if let Some(mut sender) = world.get_mut::<MessageSender<TransmogFailed>>(connection) {
                sender.send::<TransmogChannel>(TransmogFailed { npc, error });
            }
        }
    }
}

fn service_character(
    world: &World,
    connection: Entity,
    npc: u64,
) -> Result<(u64, u64, Entity), TransmogError> {
    let active = world
        .get::<ActiveInteraction>(connection)
        .ok_or(TransmogError::NotInteracting)?;
    if active.role != Some(NpcRole::Transmogrifier) || active.npc.to_bits() != npc {
        return Err(TransmogError::NotInteracting);
    }
    let player = world
        .get::<PlayerLink>(connection)
        .ok_or(TransmogError::NotInteracting)?
        .0;
    let player_position = world
        .get::<Position>(player)
        .ok_or(TransmogError::NotInteracting)?;
    let npc_position = world
        .get::<Position>(active.npc)
        .ok_or(TransmogError::NotInteracting)?;
    let distance = Vec3::new(
        player_position.x - npc_position.x,
        player_position.y - npc_position.y,
        player_position.z - npc_position.z,
    )
    .length();
    let alive = world
        .get::<Health>(player)
        .is_some_and(|health| health.current > 0.0)
        && world
            .get::<Health>(active.npc)
            .is_some_and(|health| health.current > 0.0);
    if !alive || distance > INTERACTION_DISTANCE {
        return Err(TransmogError::NotInteracting);
    }
    let flags = world
        .get::<NpcFlags>(active.npc)
        .ok_or(TransmogError::NotInteracting)?;
    if !flags.contains(NpcFlags::TRANSMOGRIFIER) {
        return Err(TransmogError::NotInteracting);
    }
    let faction = world
        .get::<UnitFactionTemplate>(active.npc)
        .ok_or(TransmogError::NotInteracting)?
        .0;
    let player_faction = world
        .get::<UnitFactionTemplate>(player)
        .ok_or(TransmogError::NotInteracting)?
        .0;
    let hostile = world
        .resource::<FactionTemplates>()
        .reaction(faction, player_faction)
        == Reaction::Hostile;
    if hostile {
        return Err(TransmogError::NotInteracting);
    }
    let character = world
        .get::<InWorld>(connection)
        .ok_or(TransmogError::NotInteracting)?
        .character_id;
    let account = world
        .get::<Authenticated>(connection)
        .ok_or(TransmogError::NotInteracting)?
        .account_id;
    Ok((character, account, player))
}

fn handle(
    world: &mut World,
    connection: Entity,
    npc: u64,
    changes: Option<Vec<TransmogChange>>,
) -> Result<TransmogState, TransmogError> {
    let (character, account, player) = service_character(world, connection, npc)?;
    let db = &world.resource::<GameDatabase>().db;
    let mut record = load_inventory_record(db, character).ok_or(TransmogError::ItemChanged)?;
    record.appearance_account = Some(account);
    let collection = crate::persistence::appearance_store::load_account_appearances(db, account);
    let catalog = world.resource::<TransmogCatalog>();
    let templates = world.resource::<ItemTemplates>();
    let mut state = catalog.state(npc, &record, templates, &collection);
    state.outfits = outfits::load_outfits(db, account);
    state.custom_sets = custom_sets::load_sets(db, account);
    let Some(changes) = changes else {
        return Ok(state);
    };
    apply_changes(&mut record, &state, &changes)?;
    let model = crate::inventory::inventory_model::InventoryModel::from_record(
        record,
        templates,
        &mut || crate::persistence::next_id(db, "item_instance"),
    );
    let appearance = model.appearance().clone();
    record = model.into_record();
    save_inventory_record(db, character, &record);
    let mut state = catalog.state(npc, &record, templates, &collection);
    state.outfits = outfits::load_outfits(db, account);
    state.custom_sets = custom_sets::load_sets(db, account);
    if let Some(mut current) = world.get_mut::<EquipmentAppearance>(player) {
        current.set_if_neq(appearance);
    }
    if let Some(dirty) = world.get_resource::<ReplicationDirtyQueue>() {
        dirty.mark(player);
    }
    world
        .resource_mut::<GoldUpdateQueue>()
        .record(character, record.inventory.gold);
    info!(
        "transmog applied character={character} changes={changes:?} gold={}",
        record.inventory.gold
    );
    Ok(state)
}

fn apply_changes(
    record: &mut InventoryRecord,
    state: &TransmogState,
    changes: &[TransmogChange],
) -> Result<(), TransmogError> {
    let mut seen = HashSet::new();
    let mut planned = Vec::new();
    let mut cost = 0u64;
    for change in changes {
        if !seen.insert(change.slot) {
            return Err(TransmogError::InvalidSlot);
        }
        let slot = state
            .slots
            .iter()
            .find(|slot| slot.slot == change.slot)
            .ok_or(TransmogError::InvalidSlot)?;
        if slot.item_guid != change.item_guid {
            return Err(TransmogError::ItemChanged);
        }
        if slot.applied == change.choice {
            continue;
        }
        let modifier = choice_modifier(slot, change.choice)?;
        let price = if modifier.is_some() {
            slot.cost.ok_or(TransmogError::CostUnavailable)?
        } else {
            0
        };
        cost = cost
            .checked_add(price)
            .ok_or(TransmogError::CostUnavailable)?;
        planned.push((change.item_guid, modifier, slot));
    }
    if record.inventory.gold < cost {
        return Err(TransmogError::NotEnoughMoney);
    }
    for (guid, modifier, slot) in planned {
        match modifier {
            Some(modifier) => {
                record.transmogs.insert(guid, modifier);
                record.instances.entry(guid).or_default().soulbound = true;
            }
            None => {
                record.transmogs.remove(&guid);
                restore_original_entry(record, slot);
            }
        }
    }
    record.inventory.gold -= cost;
    Ok(())
}

fn restore_original_entry(record: &mut InventoryRecord, slot: &TransmogSlot) {
    let visual = slot.slot.visual_slot().expect("validated visual slot");
    let entries = &mut record.character.equipment_appearance.entries;
    entries.retain(|entry| entry.slot != visual);
    entries.push(shared::components::EquippedAppearanceEntry {
        slot: visual,
        item_id: Some(slot.item_id),
        definition_source: Some(slot.definition_source),
        display_info_id: None,
        inventory_type: slot.inventory_type,
        hidden: false,
    });
}

fn choice_modifier(
    slot: &TransmogSlot,
    choice: TransmogChoice,
) -> Result<Option<ItemTransmog>, TransmogError> {
    match choice {
        TransmogChoice::Original => Ok(None),
        TransmogChoice::Hidden if slot.can_hide => Ok(Some(ItemTransmog {
            source_id: 0,
            display_id: 0,
            inventory_type: 0,
            hidden: true,
        })),
        TransmogChoice::Hidden => Err(TransmogError::AppearanceNotUsable),
        TransmogChoice::Source(id) => {
            let source = slot
                .sources
                .iter()
                .find(|source| source.source_id == id)
                .ok_or(TransmogError::AppearanceNotUsable)?;
            Ok(Some(ItemTransmog {
                source_id: id,
                display_id: source.display_id,
                inventory_type: source.inventory_type,
                hidden: false,
            }))
        }
    }
}
