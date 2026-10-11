//! Retail application at an authenticated Transmogrifier. See specs/transmog-apply.md.
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
}

#[derive(Resource, Default)]
pub(crate) struct TransmogCatalog {
    sources: HashMap<u32, Source>,
    by_appearance: HashMap<u32, Vec<u32>>,
    prices: HashMap<(ItemDefinitionSource, u32), u64>,
}

impl TransmogCatalog {
    pub(crate) fn load(conn: &Connection) -> rusqlite::Result<Self> {
        let mut catalog = Self::default();
        let mut stmt = conn.prepare(
            "SELECT m.ID, m.ItemAppearanceID, i.ID, a.ItemDisplayInfoID, i.Name,
             i.InventoryType, i.ClassID, i.SubclassID, i.RequiredLevel, i.AllowableClass,
             i.AllowableRaces_0, i.AllowableRaces_1, i.Flags_1
             FROM item_modified_appearance m JOIN item_appearance a ON a.ID=m.ItemAppearanceID
             JOIN content_item i ON i.ID=m.ItemID
             WHERE a.ItemDisplayInfoID>0 AND a.TransmogPlayerConditionID=0
             AND (i.Flags_1 & 4194304)=0 AND i.Quality<>6 ORDER BY m.ID",
        )?;
        for row in stmt.query_map([], source_from_row)? {
            let source = row?;
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
                    applied: record
                        .transmogs
                        .get(&equipped.item.item_guid)
                        .map_or(TransmogChoice::Original, |value| value.choice()),
                    cost: self
                        .prices
                        .get(&(definition, equipped.item.item_id))
                        .copied(),
                    can_hide: can_hide(slot),
                    sources: self.eligible_sources(target, &record.character, collection),
                })
            })
            .collect();
        TransmogState { npc, slots }
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
    Ok(Source {
        view: TransmogSource {
            source_id: row.get(0)?,
            appearance_id: row.get(1)?,
            item_id: row.get(2)?,
            display_id: row.get(3)?,
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

pub(crate) struct TransmogPlugin;
impl Plugin for TransmogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (send_opened_states, process_requests)
                .chain()
                .after(crate::npc_interaction::InteractionValidation),
        );
    }
}

fn collect_requests<M: Message>(world: &mut World) -> Vec<(Entity, M)> {
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
    let state = catalog.state(npc, &record, templates, &collection);
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
    let state = catalog.state(npc, &record, templates, &collection);
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
        planned.push((change.item_guid, modifier));
    }
    if record.inventory.gold < cost {
        return Err(TransmogError::NotEnoughMoney);
    }
    for (guid, modifier) in planned {
        match modifier {
            Some(modifier) => {
                record.transmogs.insert(guid, modifier);
                record.instances.entry(guid).or_default().soulbound = true;
            }
            None => {
                record.transmogs.remove(&guid);
            }
        }
    }
    record.inventory.gold -= cost;
    Ok(())
}

fn choice_modifier(
    slot: &TransmogSlot,
    choice: TransmogChoice,
) -> Result<Option<ItemTransmog>, TransmogError> {
    match choice {
        TransmogChoice::Original => Ok(None),
        TransmogChoice::Hidden if slot.can_hide => Ok(Some(ItemTransmog {
            source_id: 0,
            item_id: slot.item_id,
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
                item_id: source.item_id,
                display_id: source.display_id,
                inventory_type: source.inventory_type,
                hidden: false,
            }))
        }
    }
}
