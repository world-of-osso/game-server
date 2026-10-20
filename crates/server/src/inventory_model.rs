//! Pure bag/equipment model: builds a slot grid from `InventoryRecord`, applies
//! Retail item moves, and writes the result back.
//!
//! Item rules come from `content_item` (`ItemTemplates`): inventory type, stack
//! size, required level, class/race masks, bonding and container size/type.
//! Containers 1-4 and the reagent bag (5) are as large as the bag equipped in
//! their bag slot. Learned dual-wield and Titan's Grip effects govern hand slots;
//! weapon/armor proficiencies remain untracked. Non-visual paperdoll slots are stored
//! separately from visual equipment to preserve existing character encodings.
use std::collections::{BTreeMap, HashMap, HashSet};

use shared::components::EquippedAppearanceEntry;
use shared::item_data::ItemDefinitionSource;
use shared::protocol::{
    BACKPACK_SLOTS, BAG_COUNT, BagContents, BagSlotItem, CraftingOrigin, EquipmentSlot,
    EquipmentSnapshot, EquippedItem as EquippedItemView, InventoryError, InventoryErrorReason,
    InventorySlotChange, InventorySnapshot, ItemDurability, ItemLocation, ItemStack, REAGENT_BAG,
};

use crate::item_stats::ItemScaling;
use crate::item_templates::{BIND_ON_EQUIP, ItemTemplate, ItemTemplates, OnUseSpell};
use crate::persistence::equipment::EquippedItem;
use crate::persistence::inventory_store::{
    EquippedBagData, InventoryRecord, ItemInstanceData, ItemPlacementData,
};
use crate::persistence::{CharacterData, DurabilityEntryData, InventoryItemData};
use crate::stats::EquippedItems;

pub(crate) type InventoryView = BTreeMap<ItemLocation, ItemStack>;
type OpResult = Result<(), InventoryError>;

const TWO_HAND_TYPES: [u8; 4] = [15, 17, 25, 26];
const TWO_HAND_WEAPON: u8 = 17;
const EFFECT_TITAN_GRIP: u32 = 155;

/// TC Player::SetCanTitanGrip keeps independent weapon/armor subclass masks.
#[derive(Default)]
struct TitanGrip {
    weapon_subclasses: u32,
    armor_subclasses: u32,
}

pub(crate) struct InventoryModel {
    bags: Vec<Vec<Option<InventoryItemData>>>,
    /// Attached only for an authorized character bank move; persisted separately.
    bank: Option<crate::persistence::banks::BankData>,
    /// Passive personal-bank ownership; never permits bank-location operations.
    bank_owned: Vec<(ItemDefinitionSource, InventoryItemData)>,
    /// Owned items with no free bag slot; kept so saving never drops them.
    overflow: Vec<InventoryItemData>,
    equipped: BTreeMap<EquipmentSlot, InventoryItemData>,
    instances: HashMap<u64, ItemInstanceData>,
    crafting_origins: HashMap<u64, CraftingOrigin>,
    item_durability: HashMap<u64, (u32, u32)>,
    definition_sources: HashMap<u64, ItemDefinitionSource>,
    /// Source-qualified definitions of owned items.
    templates: HashMap<(ItemDefinitionSource, u32), ItemTemplate>,
    character: CharacterData,
    gold: u64,
    appearance_account: Option<u64>,
    appearance_grants: Vec<u32>,
    transmogs: HashMap<u64, crate::transmog_store::ItemTransmog>,
    /// `Player::CanDualWield`: the player knows a SPELL_EFFECT_DUAL_WIELD spell.
    can_dual_wield: bool,
    titan_grip: Option<TitanGrip>,
}

pub(crate) fn error(reason: InventoryErrorReason, detail: impl Into<String>) -> InventoryError {
    InventoryError {
        reason,
        detail: Some(detail.into()),
    }
}

impl InventoryModel {
    /// `mint` allocates a fresh item guid (for duplicate guids and splits).
    pub(crate) fn from_record(
        record: InventoryRecord,
        templates: &ItemTemplates,
        mint: &mut impl FnMut() -> u64,
    ) -> Self {
        let owned: Vec<_> = record
            .inventory
            .items
            .iter()
            .chain(record.equipped.iter().map(|entry| &entry.item))
            .chain(record.nonvisual_equipped.iter().map(|(_, item)| item))
            .chain(record.bag_slots.iter().map(|entry| &entry.item))
            .filter_map(|item| {
                let source = record.definition_sources[&item.item_guid];
                Some((
                    item.item_guid,
                    (source, item.item_id),
                    *templates.get(source, item.item_id)?,
                ))
            })
            .collect();
        let mut crafting_origins = record.crafting_origins;
        crafting_origins.retain(|guid, _| {
            owned
                .iter()
                .any(|(owned, _, template)| owned == guid && template.records_crafting_origin())
        });
        let owned_templates = owned
            .into_iter()
            .map(|(_, key, template)| (key, template))
            .collect();
        let mut model = Self {
            bags: vec![Vec::new(); usize::from(BAG_COUNT)],
            bank: None,
            bank_owned: Vec::new(),
            overflow: Vec::new(),
            equipped: BTreeMap::new(),
            instances: record.instances,
            crafting_origins,
            item_durability: record.item_durability,
            definition_sources: record.definition_sources,
            templates: owned_templates,
            character: record.character,
            gold: record.inventory.gold,
            appearance_account: record.appearance_account,
            appearance_grants: record.appearance_grants,
            transmogs: record.transmogs,
            can_dual_wield: false,
            titan_grip: None,
        };
        for entry in record.equipped {
            model
                .equipped
                .insert(EquipmentSlot::from_visual_slot(entry.slot), entry.item);
        }
        for (id, item) in record.nonvisual_equipped {
            let slot = EquipmentSlot::ALL
                .into_iter()
                .find(|slot| slot.inv_slot_id() == id && slot.visual_slot().is_none())
                .expect("saved nonvisual equipment slot id");
            model.equipped.insert(slot, item);
        }
        for entry in record.bag_slots {
            let slot = EquipmentSlot::from_bag_index(entry.bag).expect("saved bag slot");
            model.equipped.insert(slot, entry.item);
        }
        model.sync_containers();
        model.load_bag_items(record.inventory.items, &record.placements, mint);
        model.bind_owned_pickup_items();
        let equipped_slots: Vec<_> = model
            .equipped
            .iter()
            .filter(|(_, item)| model.transmogs.contains_key(&item.item_guid))
            .map(|(slot, _)| *slot)
            .collect();
        for slot in equipped_slots {
            model.sync_appearance(slot);
        }
        model
    }

    pub(crate) fn load_bank_ownership(&mut self, db: &redb::Database, character_id: u64) {
        let bank = crate::persistence::banks::load_character_bank(db, character_id);
        self.bank_owned = bank
            .tabs
            .into_iter()
            .flat_map(|tab| tab.slots)
            .flatten()
            .map(|item| {
                let source = crate::persistence::inventory_store::load_item_definition_source(
                    db,
                    item.item_guid,
                );
                (source, item)
            })
            .collect();
    }

    pub(crate) fn attach_bank(
        &mut self,
        db: &redb::Database,
        templates: &ItemTemplates,
        bank: crate::persistence::banks::BankData,
    ) -> OpResult {
        let guids = bank
            .tabs
            .iter()
            .flat_map(|tab| tab.slots.iter().flatten())
            .map(|item| item.item_guid);
        let bound = crate::persistence::inventory_store::soulbound_guids(db, guids);
        for item in bank.tabs.iter().flat_map(|tab| tab.slots.iter().flatten()) {
            let source = crate::persistence::inventory_store::load_item_definition_source(
                db,
                item.item_guid,
            );
            let template = templates
                .get(source, item.item_id)
                .copied()
                .ok_or_else(|| {
                    error(
                        InventoryErrorReason::ItemNotFound,
                        format!("bank GUID {} has no definition", item.item_guid),
                    )
                })?;
            self.register_definition(item, source, template);
            if let Some(origin) =
                crate::persistence::inventory_store::load_item_crafting_origin(db, item.item_guid)
                    .filter(|_| template.records_crafting_origin())
            {
                self.crafting_origins.insert(item.item_guid, origin);
            }
            self.instances.insert(
                item.item_guid,
                ItemInstanceData {
                    soulbound: bound.contains(&item.item_guid),
                },
            );
        }
        self.bank = Some(bank);
        Ok(())
    }

    pub(crate) fn take_bank(&mut self) -> Option<crate::persistence::banks::BankData> {
        self.bank.take()
    }

    /// Bind-on-pickup and quest items are soulbound once owned.
    fn bind_owned_pickup_items(&mut self) {
        let guids: Vec<u64> = self
            .bags
            .iter()
            .flatten()
            .flatten()
            .chain(self.equipped.values())
            .chain(&self.overflow)
            .filter(|item| {
                self.template(item)
                    .is_some_and(ItemTemplate::binds_on_pickup)
            })
            .map(|item| item.item_guid)
            .collect();
        for guid in guids {
            self.instances.entry(guid).or_default().soulbound = true;
        }
    }

    fn load_bag_items(
        &mut self,
        items: Vec<InventoryItemData>,
        placements: &[ItemPlacementData],
        mint: &mut impl FnMut() -> u64,
    ) {
        let placed: HashMap<u64, (u8, u8)> = placements
            .iter()
            .map(|p| (p.item_guid, (p.bag, p.slot)))
            .collect();
        let mut seen: HashSet<u64> = self.equipped.values().map(|i| i.item_guid).collect();
        let mut unplaced = Vec::new();
        for mut item in items {
            if !seen.insert(item.item_guid) {
                let source_guid = item.item_guid;
                let meta = self.instances.get(&source_guid).copied();
                item.item_guid = mint();
                self.copy_instance_identity(source_guid, item.item_guid);
                seen.insert(item.item_guid);
                if let Some(meta) = meta {
                    self.instances.insert(item.item_guid, meta);
                }
                unplaced.push(item);
                continue;
            }
            match placed.get(&item.item_guid) {
                Some(&(bag, slot))
                    if self.cell_is_free(bag, slot) && self.accepts(usize::from(bag), &item) =>
                {
                    self.bags[usize::from(bag)][usize::from(slot)] = Some(item);
                }
                _ => unplaced.push(item),
            }
        }
        for item in unplaced {
            match self.first_free_cell(&item) {
                Some((bag, slot)) => self.bags[bag][slot] = Some(item),
                None => self.overflow.push(item),
            }
        }
    }

    fn cell_is_free(&self, bag: u8, slot: u8) -> bool {
        self.bags
            .get(usize::from(bag))
            .and_then(|cells| cells.get(usize::from(slot)))
            .is_some_and(Option::is_none)
    }

    /// Empty cells of the backpack and bags 1-4 (Retail
    /// `CalculateTotalNumberOfFreeBagSlots`); the reagent bag holds only reagents.
    pub(crate) fn free_cells(&self) -> usize {
        self.bags[..usize::from(REAGENT_BAG)]
            .iter()
            .flatten()
            .filter(|cell| cell.is_none())
            .count()
    }

    /// First empty cell of a container `item` may go into: profession and reagent
    /// bags before the containers that take anything (TrinityCore
    /// `Player::CanStoreItem`, Player.cpp:10433-10491), each in bag order.
    fn first_free_cell(&self, item: &InventoryItemData) -> Option<(usize, usize)> {
        let (specialty, general): (Vec<usize>, Vec<usize>) = (0..self.bags.len())
            .filter(|&bag| self.accepts(bag, item))
            .partition(|&bag| !self.is_general_container(bag));
        specialty.into_iter().chain(general).find_map(|bag| {
            self.bags[bag]
                .iter()
                .position(Option::is_none)
                .map(|slot| (bag, slot))
        })
    }

    /// Container `bag`'s capacity: the backpack's, else its equipped bag's
    /// `ContainerSlots` (0 when the bag slot is empty).
    fn container_size(&self, bag: u8) -> usize {
        let Some(slot) = EquipmentSlot::from_bag_index(bag) else {
            return usize::from(BACKPACK_SLOTS);
        };
        self.equipped
            .get(&slot)
            .and_then(|item| self.template(item))
            .map_or(0, |template| usize::from(template.container_slots))
    }

    /// Resize containers whose bag slot changed. Callers move a bag only when it is
    /// empty (or move its items along), so a resized container never holds items.
    fn sync_containers(&mut self) {
        for bag in 0..BAG_COUNT {
            let size = self.container_size(bag);
            let cells = &mut self.bags[usize::from(bag)];
            if cells.len() != size {
                assert!(
                    cells.iter().all(Option::is_none),
                    "container {bag} resized while holding items"
                );
                *cells = vec![None; size];
            }
        }
    }

    /// TrinityCore `ItemCanGoIntoBag`: the backpack takes anything; a bag neither
    /// itself nor items its type refuses (a reagent bag holds only reagents).
    fn accepts(&self, bag: usize, item: &InventoryItemData) -> bool {
        let Some(slot) = EquipmentSlot::from_bag_index(bag as u8) else {
            return true;
        };
        self.equipped.get(&slot).is_some_and(|container| {
            container.item_guid != item.item_guid && self.fits_container(item, container)
        })
    }

    fn fits_container(&self, item: &InventoryItemData, container: &InventoryItemData) -> bool {
        let Some(container) = self.template(container) else {
            return false;
        };
        self.template(item)
            .is_some_and(|item| item.can_go_into_bag(container))
    }

    /// `EQUIP_ERR_WRONG_BAG_TYPE` when `item` may not be put at `location`.
    fn check_bag_type(&self, location: ItemLocation, item: &InventoryItemData) -> OpResult {
        match location {
            ItemLocation::Bag { bag, .. } if !self.accepts(usize::from(bag), item) => Err(error(
                InventoryErrorReason::WrongBagType,
                format!("item {} does not go in container {bag}", item.item_id),
            )),
            _ => Ok(()),
        }
    }

    pub(crate) fn into_record(mut self) -> InventoryRecord {
        let appearances: Vec<u32> = self
            .appearance_sources()
            .into_iter()
            .filter_map(|(source, id)| self.templates.get(&(source, id))?.appearance_id)
            .collect();
        self.appearance_grants.extend(appearances);
        let mut items = Vec::new();
        let mut placements = Vec::new();
        for (bag, cells) in self.bags.into_iter().enumerate() {
            for (slot, item) in cells.into_iter().enumerate() {
                let Some(item) = item else { continue };
                placements.push(ItemPlacementData {
                    item_guid: item.item_guid,
                    bag: bag as u8,
                    slot: slot as u8,
                });
                items.push(item);
            }
        }
        items.extend(self.overflow);
        let mut equipped = Vec::new();
        let mut nonvisual_equipped = Vec::new();
        let mut bag_slots = Vec::new();
        for (slot, item) in self.equipped {
            if let Some(bag) = slot.bag_index() {
                bag_slots.push(EquippedBagData { bag, item });
            } else if let Some(slot) = slot.visual_slot() {
                equipped.push(EquippedItem { slot, item });
            } else {
                nonvisual_equipped.push((slot.inv_slot_id(), item));
            }
        }
        InventoryRecord {
            character: self.character,
            inventory: crate::persistence::InventoryData {
                gold: self.gold,
                items,
            },
            placements,
            equipped,
            nonvisual_equipped,
            bag_slots,
            instances: self.instances,
            crafting_origins: self.crafting_origins,
            item_durability: self.item_durability,
            definition_sources: self.definition_sources,
            appearance_account: self.appearance_account,
            appearance_grants: self.appearance_grants,
            transmogs: self.transmogs,
        }
    }

    fn copy_instance_identity(&mut self, source_guid: u64, new_guid: u64) {
        if let Some(source) = self.definition_sources.get(&source_guid).copied() {
            self.definition_sources.insert(new_guid, source);
        }
    }

    pub(crate) fn record_crafting_origin(&mut self, guid: u64, origin: CraftingOrigin) {
        self.crafting_origins.insert(guid, origin);
    }

    pub(crate) fn remember_definition_sources(
        &mut self,
        sources: HashMap<u64, ItemDefinitionSource>,
    ) {
        for (guid, source) in sources {
            if let Some(previous) = self.definition_sources.insert(guid, source) {
                assert_eq!(
                    previous, source,
                    "cannot change definition source for GUID {guid}"
                );
            }
        }
    }

    fn register_definition(
        &mut self,
        item: &InventoryItemData,
        source: ItemDefinitionSource,
        template: ItemTemplate,
    ) {
        if let Some(previous) = self.definition_sources.insert(item.item_guid, source) {
            assert_eq!(
                previous, source,
                "cannot change definition source for GUID {}",
                item.item_guid
            );
        }
        self.templates.insert((source, item.item_id), template);
    }

    fn same_definition(&self, left: &InventoryItemData, right: &InventoryItemData) -> bool {
        left.item_id == right.item_id
            && self.definition_source(left.item_guid) == self.definition_source(right.item_guid)
    }

    // --- Views -----------------------------------------------------------

    pub(crate) fn appearance(&self) -> &shared::components::EquipmentAppearance {
        &self.character.equipment_appearance
    }

    /// Physically equipped item ids, the input of character stats.
    pub(crate) fn equipped_items(&self) -> EquippedItems {
        EquippedItems {
            items: self
                .equipped
                .iter()
                .map(|(slot, item)| (*slot, self.definition_source(item.item_guid), item.item_id))
                .collect(),
        }
    }

    pub(crate) fn definition_source(&self, guid: u64) -> ItemDefinitionSource {
        self.definition_sources[&guid]
    }

    fn stack(&self, item: &InventoryItemData, durability: Option<ItemDurability>) -> ItemStack {
        ItemStack {
            item_guid: item.item_guid,
            item_id: item.item_id,
            definition_source: self.definition_source(item.item_guid),
            count: item.stack_count,
            durability,
            soulbound: self
                .instances
                .get(&item.item_guid)
                .is_some_and(|meta| meta.soulbound),
            crafting_origin: self.crafting_origins.get(&item.item_guid).copied(),
        }
    }

    /// Saved durability of the equipped slot (kept current by the durability plugin).
    fn equipped_durability(&self, slot: EquipmentSlot) -> Option<ItemDurability> {
        let visual = slot.visual_slot()?;
        self.character
            .durability
            .iter()
            .find(|entry| entry.slot == visual)
            .map(|entry| ItemDurability {
                current: entry.current,
                max: entry.max,
            })
    }

    /// Every occupied bag and equipment location.
    pub(crate) fn view(&self) -> InventoryView {
        let bag_items = self.bags.iter().enumerate().flat_map(|(bag, cells)| {
            cells.iter().enumerate().filter_map(move |(slot, item)| {
                let location = ItemLocation::Bag {
                    bag: bag as u8,
                    slot: slot as u8,
                };
                item.as_ref().map(|item| {
                    (
                        location,
                        self.stack(item, self.bag_durability(item.item_guid)),
                    )
                })
            })
        });
        let equipped = self.equipped.iter().map(|(slot, item)| {
            let stack = self.stack(item, self.equipped_durability(*slot));
            (ItemLocation::Equipment(*slot), stack)
        });
        bag_items.chain(equipped).collect()
    }

    pub(crate) fn inventory_snapshot(&self) -> InventorySnapshot {
        let view = self.view();
        let bags = (0..BAG_COUNT)
            .map(|bag| BagContents {
                bag,
                size: self.bags[usize::from(bag)].len() as u8,
                items: view
                    .iter()
                    .filter_map(|(location, item)| match location {
                        ItemLocation::Bag { bag: b, slot } if *b == bag => Some(BagSlotItem {
                            slot: *slot,
                            item: item.clone(),
                        }),
                        _ => None,
                    })
                    .collect(),
            })
            .collect();
        InventorySnapshot { bags }
    }

    pub(crate) fn equipment_snapshot(&self) -> EquipmentSnapshot {
        let items = self
            .view()
            .into_iter()
            .filter_map(|(location, item)| match location {
                ItemLocation::Equipment(slot) => Some(EquippedItemView { slot, item }),
                ItemLocation::Bag { .. } | ItemLocation::Bank { .. } => None,
            })
            .collect();
        EquipmentSnapshot { items }
    }

    // --- Slot access -----------------------------------------------------

    fn check_location(&self, location: ItemLocation) -> OpResult {
        match location {
            ItemLocation::Bag { bag, slot } => {
                let exists = self
                    .bags
                    .get(usize::from(bag))
                    .is_some_and(|cells| usize::from(slot) < cells.len());
                if exists {
                    Ok(())
                } else {
                    Err(error(
                        InventoryErrorReason::InternalBagError,
                        format!("bag {bag} slot {slot} does not exist"),
                    ))
                }
            }
            ItemLocation::Equipment(_) => Ok(()),
            ItemLocation::Bank { tab, slot } => {
                let exists = self
                    .bank
                    .as_ref()
                    .and_then(|bank| bank.tabs.get(usize::from(tab)))
                    .is_some_and(|tab| usize::from(slot) < tab.slots.len());
                if exists {
                    Ok(())
                } else {
                    Err(error(
                        InventoryErrorReason::WrongSlot,
                        format!("bank tab {tab} slot {slot} does not exist"),
                    ))
                }
            }
        }
    }

    fn get(&self, location: ItemLocation) -> Option<&InventoryItemData> {
        match location {
            ItemLocation::Bag { bag, slot } => {
                self.bags[usize::from(bag)][usize::from(slot)].as_ref()
            }
            ItemLocation::Equipment(slot) => self.equipped.get(&slot),
            ItemLocation::Bank { tab, slot } => {
                self.bank.as_ref()?.tabs[usize::from(tab)].slots[usize::from(slot)].as_ref()
            }
        }
    }

    fn take(&mut self, location: ItemLocation) -> Option<InventoryItemData> {
        match location {
            ItemLocation::Bag { bag, slot } => {
                self.bags[usize::from(bag)][usize::from(slot)].take()
            }
            ItemLocation::Equipment(slot) => self.equipped.remove(&slot),
            ItemLocation::Bank { tab, slot } => {
                self.bank.as_mut()?.tabs[usize::from(tab)].slots[usize::from(slot)].take()
            }
        }
    }

    fn put(&mut self, location: ItemLocation, item: Option<InventoryItemData>) {
        match location {
            ItemLocation::Bag { bag, slot } => {
                self.bags[usize::from(bag)][usize::from(slot)] = item;
            }
            ItemLocation::Equipment(slot) => {
                match item {
                    Some(item) => self.equipped.insert(slot, item),
                    None => self.equipped.remove(&slot),
                };
            }
            ItemLocation::Bank { tab, slot } => {
                self.bank.as_mut().expect("validated bank").tabs[usize::from(tab)].slots
                    [usize::from(slot)] = item;
            }
        }
    }

    pub(crate) fn existing(
        &self,
        location: ItemLocation,
    ) -> Result<&InventoryItemData, InventoryError> {
        self.check_location(location)?;
        self.get(location).ok_or_else(|| {
            error(
                InventoryErrorReason::ItemNotFound,
                format!("no item at {location:?}"),
            )
        })
    }

    fn template(&self, item: &InventoryItemData) -> Option<&ItemTemplate> {
        self.templates
            .get(&(self.definition_source(item.item_guid), item.item_id))
    }

    fn inventory_type(&self, item: &InventoryItemData) -> u8 {
        self.template(item)
            .map_or(0, |template| template.inventory_type)
    }

    /// Slots `item` may occupy, preferred first: a reagent bag goes only in the
    /// reagent bag slot (TrinityCore `Player::FindEquipSlot`, Player.cpp:9296-9302).
    fn equip_slots(&self, item: &InventoryItemData) -> &'static [EquipmentSlot] {
        if self
            .template(item)
            .is_some_and(ItemTemplate::is_reagent_bag)
        {
            return &[EquipmentSlot::ReagentBag];
        }
        if self.inventory_type(item) == TWO_HAND_WEAPON
            && self.can_dual_wield
            && self.can_titan_grip(item)
        {
            return &[EquipmentSlot::MainHand, EquipmentSlot::OffHand];
        }
        EquipmentSlot::for_inventory_type(self.inventory_type(item))
    }

    /// Items without a template never stack.
    fn max_stack(&self, item: &InventoryItemData) -> u32 {
        self.template(item).map_or(1, |template| template.max_stack)
    }

    /// Run `op`, then update appearance entries of the slots whose item changed.
    /// Untouched slots keep their entries (appearance-only overrides included).
    fn with_appearance_sync(&mut self, op: impl FnOnce(&mut Self) -> OpResult) -> OpResult {
        let before = self.equipped.clone();
        let result = op(self);
        for slot in EquipmentSlot::ALL {
            if before.get(&slot) != self.equipped.get(&slot) {
                self.sync_item_durability(slot, before.get(&slot).map(|item| item.item_guid));
                self.sync_appearance(slot);
            }
        }
        result
    }

    /// Preserve damage on the item leaving a slot and restore the incoming item's state.
    fn sync_item_durability(&mut self, slot: EquipmentSlot, previous_guid: Option<u64>) {
        let Some(visual) = slot.visual_slot() else {
            return;
        };
        if let (Some(guid), Some(state)) = (previous_guid, self.equipped_durability(slot)) {
            self.item_durability
                .insert(guid, (state.current, state.max));
        }
        self.character
            .durability
            .retain(|entry| entry.slot != visual);
        let incoming = self
            .equipped
            .get(&slot)
            .and_then(|item| self.item_durability.get(&item.item_guid));
        if let Some(&(current, max)) = incoming {
            self.character.durability.push(DurabilityEntryData {
                slot: visual,
                current,
                max,
            });
        }
    }

    /// Replace the appearance entry for `slot` with the physical item, keeping `hidden`.
    fn sync_appearance(&mut self, slot: EquipmentSlot) {
        let Some(visual) = slot.visual_slot() else {
            return;
        };
        let entry =
            self.equipped
                .get(&slot)
                .map(|item| match self.transmogs.get(&item.item_guid) {
                    Some(modifier) => {
                        modifier.entry(visual, item.item_id, self.definition_source(item.item_guid))
                    }
                    None => EquippedAppearanceEntry {
                        slot: visual,
                        item_id: Some(item.item_id),
                        definition_source: Some(self.definition_source(item.item_guid)),
                        display_info_id: None,
                        inventory_type: self.inventory_type(item),
                        hidden: false,
                    },
                });
        let entries = &mut self.character.equipment_appearance.entries;
        entries.retain(|entry| entry.slot != visual);
        if let Some(entry) = entry {
            entries.push(entry);
        }
    }

    // --- Operations ------------------------------------------------------

    /// Retail `CMSG_SWAP_ITEM` / `SWAP_INV_ITEM`: move, equip, unequip or swap.
    pub(crate) fn swap(&mut self, from: ItemLocation, to: ItemLocation) -> OpResult {
        let result = self.with_appearance_sync(|model| model.move_item(from, to));
        self.sync_containers();
        result
    }

    fn move_item(&mut self, from: ItemLocation, to: ItemLocation) -> OpResult {
        self.existing(from)?;
        self.check_location(to)?;
        if from == to {
            return Ok(());
        }
        if let ItemLocation::Equipment(slot) = from
            && self.get(to).is_none()
        {
            match to {
                ItemLocation::Bag { bag, .. } => self.check_unequip_bag(slot, bag)?,
                ItemLocation::Bank { .. } => {
                    if let Some(bag) = slot.bag_index() {
                        self.check_bag_empty(bag)?;
                    }
                }
                ItemLocation::Equipment(_) => {}
            }
        }
        match (from, to) {
            (_, ItemLocation::Equipment(slot)) => self.equip_into(from, slot),
            (
                ItemLocation::Equipment(slot),
                ItemLocation::Bag { .. } | ItemLocation::Bank { .. },
            ) if self.get(to).is_some() => self.equip_into(to, slot),
            _ if self.can_merge(from, to) => {
                self.merge_stacks(from, to, u32::MAX);
                Ok(())
            }
            _ => {
                self.check_bag_type(to, self.get(from).expect("validated source"))?;
                if let Some(displaced) = self.get(to) {
                    self.check_bag_type(from, displaced)?;
                }
                let moving = self.take(from);
                let displaced = self.take(to);
                self.put(to, moving);
                self.put(from, displaced);
                Ok(())
            }
        }
    }

    /// Same definition in two storage slots with room left in the target stack.
    fn can_merge(&self, from: ItemLocation, to: ItemLocation) -> bool {
        let (Some(source), Some(target)) = (self.get(from), self.get(to)) else {
            return false;
        };
        matches!(
            (from, to),
            (
                ItemLocation::Bag { .. } | ItemLocation::Bank { .. },
                ItemLocation::Bag { .. } | ItemLocation::Bank { .. }
            )
        ) && self.same_definition(source, target)
            && target.stack_count < self.max_stack(target)
    }

    /// Move up to `count` items from `from` onto the stack at `to`, capped at the
    /// stack size; an emptied source stack is removed.
    /// TrinityCore `Player::SwapItem` (Player.cpp:12641-12655) moving the bag in
    /// `slot` to an empty cell of container `bag`: not into itself
    /// (`EQUIP_ERR_BAG_IN_BAG`), and only when empty (`CanUnequipItem`,
    /// `EQUIP_ERR_DESTROY_NONEMPTY_BAG`).
    fn check_unequip_bag(&self, slot: EquipmentSlot, bag: u8) -> OpResult {
        let Some(own) = slot.bag_index() else {
            return Ok(());
        };
        if own == bag {
            return Err(error(
                InventoryErrorReason::BagInBag,
                format!("bag {own} cannot go inside itself"),
            ));
        }
        self.check_bag_empty(own)
    }

    fn check_bag_empty(&self, bag: u8) -> OpResult {
        if self.bags[usize::from(bag)].iter().any(Option::is_some) {
            return Err(error(
                InventoryErrorReason::DestroyNonemptyBag,
                format!("bag {bag} still holds items"),
            ));
        }
        Ok(())
    }

    fn merge_stacks(&mut self, from: ItemLocation, to: ItemLocation, count: u32) {
        let target = self.get(to).expect("merge target");
        let room = self.max_stack(target).saturating_sub(target.stack_count);
        let mut source = self.take(from).expect("merge source");
        let mut target = self.take(to).expect("merge target");
        let moved = count.min(room).min(source.stack_count);
        source.stack_count -= moved;
        target.stack_count += moved;
        if source.stack_count == 0 {
            self.instances.remove(&source.item_guid);
        } else {
            self.put(from, Some(source));
        }
        self.put(to, Some(target));
    }

    /// Retail `CMSG_AUTO_EQUIP_ITEM`: equip into the first allowed slot, or store
    /// an equipped item in the first free bag slot.
    pub(crate) fn auto_equip(&mut self, from: ItemLocation) -> OpResult {
        let item = self.existing(from)?;
        if let ItemLocation::Equipment(_) = from {
            let (bag, slot) = self.first_free_cell(item).ok_or_else(|| {
                error(
                    InventoryErrorReason::InvFull,
                    "no free bag slot to unequip into",
                )
            })?;
            let to = ItemLocation::Bag {
                bag: bag as u8,
                slot: slot as u8,
            };
            return self.swap(from, to);
        }
        let slots = self.equip_slots(item);
        let Some(first) = slots.first() else {
            return Err(not_equippable(self.inventory_type(item)));
        };
        let valid: Vec<EquipmentSlot> = slots
            .iter()
            .copied()
            .filter(|slot| self.validate_equip(item, *slot).is_ok())
            .collect();
        let target = valid
            .iter()
            .find(|slot| !self.equipped.contains_key(*slot))
            .or_else(|| {
                valid.iter().min_by_key(|slot| {
                    let equipped = self.equipped.get(*slot).expect("occupied candidate slot");
                    self.template(equipped)
                        .expect("equipped definition")
                        .scaling
                        .item_level
                })
            })
            .unwrap_or(first);
        self.swap(from, ItemLocation::Equipment(*target))
    }

    pub(crate) fn set_weapon_skills(
        &mut self,
        known: Option<&crate::class_progression::KnownSpells>,
        spells: Option<&crate::spell_info::SpellStore>,
    ) {
        self.can_dual_wield =
            crate::stats::knows_effect(known, spells, crate::stats::EFFECT_DUAL_WIELD);
        self.titan_grip = None;
        let (Some(known), Some(spells)) = (known, spells) else {
            return;
        };
        for spell in known.iter().filter_map(|id| spells.get(id)) {
            if !spell
                .effects
                .iter()
                .any(|effect| effect.effect == EFFECT_TITAN_GRIP)
            {
                continue;
            }
            let grip = self.titan_grip.get_or_insert_with(TitanGrip::default);
            if let Some(required) = spell.equipped_items.as_deref() {
                match required.item_class {
                    2 => grip.weapon_subclasses = required.item_subclass,
                    4 => grip.armor_subclasses = required.item_subclass,
                    _ => {}
                }
            }
        }
    }

    /// TC Player.cpp13142–13163: zero mask means unrestricted, not no subclasses.
    fn can_titan_grip(&self, item: &InventoryItemData) -> bool {
        let (Some(grip), Some(template)) = (&self.titan_grip, self.template(item)) else {
            return false;
        };
        let mask = match template.scaling.class_id {
            2 => grip.weapon_subclasses,
            4 => grip.armor_subclasses,
            _ => 0,
        };
        mask == 0 || mask & (1 << template.scaling.subclass_id) != 0
    }

    /// TC a352b1fa `Player::CanEquipItem` off-hand weapons (Player.cpp:10866-10886).
    /// No content off-hand weapon carries ITEM_FLAG3_ALWAYS_ALLOW_DUAL_WIELD.
    fn validate_off_hand_weapon(&self, item: &InventoryItemData, inventory_type: u8) -> OpResult {
        const WEAPON: u8 = 13;
        const WEAPON_OFF_HAND: u8 = 22;
        const ITEM_CLASS_WEAPON: u8 = 2;
        const SUBCLASS_POLEARM: u8 = 6;
        let scaling = self.template(item).map(|template| template.scaling);
        let polearm = scaling.is_some_and(|scaling| {
            scaling.class_id == ITEM_CLASS_WEAPON && scaling.subclass_id == SUBCLASS_POLEARM
        });
        if inventory_type == WEAPON && polearm {
            return Err(error(
                InventoryErrorReason::WrongSlot,
                "one-hand polearms cannot go in the off hand",
            ));
        }
        if matches!(inventory_type, WEAPON | WEAPON_OFF_HAND) && !self.can_dual_wield {
            return Err(error(
                InventoryErrorReason::CantDualWield,
                "the character cannot dual wield",
            ));
        }
        Ok(())
    }

    fn validate_equip(&self, item: &InventoryItemData, slot: EquipmentSlot) -> OpResult {
        let inventory_type = self.inventory_type(item);
        let allowed = self.equip_slots(item);
        if allowed.is_empty() {
            return Err(not_equippable(inventory_type));
        }
        if !allowed.contains(&slot) {
            return Err(error(
                InventoryErrorReason::WrongSlot,
                format!("inventory type {inventory_type} cannot go in {slot:?}"),
            ));
        }
        if slot == EquipmentSlot::OffHand {
            self.validate_off_hand_weapon(item, inventory_type)?;
        }
        let template = self
            .template(item)
            .expect("equippable items have templates");
        if template.required_level > self.character.level {
            return Err(error(
                InventoryErrorReason::CantEquipLevelI {
                    level: template.required_level,
                },
                format!("requires level {}", template.required_level),
            ));
        }
        if !template.allows(self.character.class, self.character.race) {
            return Err(error(
                InventoryErrorReason::CantEquipEver,
                format!(
                    "item {} is not usable by class {} race {}",
                    item.item_id, self.character.class, self.character.race
                ),
            ));
        }
        self.validate_unique_equipped(item, slot)
    }

    /// TrinityCore Player::CanEquipUniqueItem ignores the slot being replaced.
    /// The same GUID may move between paired slots without counting itself.
    fn validate_unique_equipped(&self, item: &InventoryItemData, slot: EquipmentSlot) -> OpResult {
        let template = self.template(item).expect("equippable item template");
        let duplicate = self.equipped.iter().any(|(occupied_slot, equipped)| {
            *occupied_slot != slot
                && equipped.item_guid != item.item_guid
                && self.same_definition(item, equipped)
        });
        if template.unique_equipped && duplicate {
            return Err(error(
                InventoryErrorReason::ItemUniqueEquippable,
                format!("item {} is unique-equipped", item.item_id),
            ));
        }
        self.validate_category_limit(item, slot)
    }

    /// Player::CanEquipUniqueItem also applies category quantities, regardless of mode.
    fn validate_category_limit(&self, item: &InventoryItemData, slot: EquipmentSlot) -> OpResult {
        let Some(limit) = self
            .template(item)
            .and_then(|template| template.equip_limit)
        else {
            return Ok(());
        };
        let source = self.definition_source(item.item_guid);
        let occupied = self
            .equipped
            .iter()
            .filter(|(occupied_slot, equipped)| {
                **occupied_slot != slot && equipped.item_guid != item.item_guid
            })
            .filter(|(_, equipped)| {
                self.definition_source(equipped.item_guid) == source
                    && self
                        .template(equipped)
                        .and_then(|template| template.equip_limit)
                        .is_some_and(|other| other.category == limit.category)
            })
            .count();
        if occupied >= usize::from(limit.quantity) {
            return Err(error(
                InventoryErrorReason::ItemMaxLimitCategoryEquippedExceeded {
                    quantity: limit.quantity,
                },
                format!(
                    "ItemLimitCategory {} allows {} equipped items",
                    limit.category, limit.quantity
                ),
            ));
        }
        Ok(())
    }

    /// Equip the item at `source` into `slot`. The previous occupant returns to
    /// `source`; a conflicting other-hand item goes to a free bag slot.
    fn equip_into(&mut self, source: ItemLocation, slot: EquipmentSlot) -> OpResult {
        if let Some(bag) = slot.bag_index() {
            return self.equip_bag(source, bag);
        }
        let item = self.existing(source)?;
        self.validate_equip(item, slot)?;
        if let (Some(previous), ItemLocation::Equipment(source_slot)) =
            (self.equipped.get(&slot), source)
        {
            self.validate_equip(previous, source_slot)?;
        }
        self.finish_equip(source, slot)
    }

    /// TrinityCore `Player::SwapItem` (Player.cpp:12589-12900) into bag slot `bag`.
    /// Between bag slots the bags swap with their contents. From a cell, the bag
    /// replaced goes to that cell and its items move into the new bag, which must
    /// take every one (`EQUIP_ERR_BAG_IN_BAG`) and have room (`EQUIP_ERR_CANT_SWAP`);
    /// a bag cannot replace the bag it sits in (`EQUIP_ERR_CANT_SWAP`).
    fn equip_bag(&mut self, source: ItemLocation, bag: u8) -> OpResult {
        let slot = EquipmentSlot::from_bag_index(bag).expect("bag slot");
        let item = self.existing(source)?;
        self.validate_equip(item, slot)?;
        match source {
            ItemLocation::Equipment(from_slot) => {
                let from_bag = from_slot.bag_index().expect("only bag slots hold bags");
                if let Some(previous) = self.equipped.get(&slot) {
                    self.validate_equip(previous, from_slot)?;
                }
                let moving = self.equipped.remove(&from_slot);
                let previous = self.take(ItemLocation::Equipment(slot));
                self.put(ItemLocation::Equipment(slot), moving);
                self.put(source, previous);
                self.bags.swap(usize::from(bag), usize::from(from_bag));
                return Ok(());
            }
            ItemLocation::Bag { .. } | ItemLocation::Bank { .. } => {}
        }
        if matches!(source, ItemLocation::Bag { bag: source_bag, .. } if source_bag == bag) {
            return Err(error(
                InventoryErrorReason::CantSwap,
                format!("the bag in container {bag} cannot replace it"),
            ));
        }
        let contents: Vec<InventoryItemData> = self.bags[usize::from(bag)]
            .iter()
            .flatten()
            .cloned()
            .collect();
        if let Some(previous) = self.equipped.get(&slot) {
            self.check_bag_type(source, previous)?;
            if contents
                .iter()
                .any(|content| !self.fits_container(content, item))
            {
                return Err(error(
                    InventoryErrorReason::BagInBag,
                    format!("bag {} cannot hold the items of bag {bag}", item.item_id),
                ));
            }
            let size = self
                .template(item)
                .map_or(0, |t| usize::from(t.container_slots));
            if contents.len() > size {
                return Err(error(
                    InventoryErrorReason::CantSwap,
                    format!("{} items do not fit a {size}-slot bag", contents.len()),
                ));
            }
        }
        let moving = self.take(source);
        let previous = self
            .equipped
            .insert(slot, moving.expect("validated source"));
        self.put(source, previous);
        self.bags[usize::from(bag)] = Vec::new();
        self.sync_containers();
        for (cell, content) in self.bags[usize::from(bag)].iter_mut().zip(contents) {
            *cell = Some(content);
        }
        self.bind_on_equip(slot);
        Ok(())
    }

    fn finish_equip(&mut self, source: ItemLocation, slot: EquipmentSlot) -> OpResult {
        let conflict = self.hand_conflict(self.get(source).expect("validated source"), slot);
        let moving = self.take(source);
        let previous = self.take(ItemLocation::Equipment(slot));
        self.put(ItemLocation::Equipment(slot), moving);
        self.put(source, previous);
        let Some(conflict) = conflict else {
            self.bind_on_equip(slot);
            return Ok(());
        };
        let other = self.equipped.get(&conflict).expect("conflicting item");
        match self.first_free_cell(other) {
            Some((bag, cell)) => {
                let other = self.take(ItemLocation::Equipment(conflict));
                let to = ItemLocation::Bag {
                    bag: bag as u8,
                    slot: cell as u8,
                };
                self.put(to, other);
                self.bind_on_equip(slot);
                Ok(())
            }
            None => {
                // Undo: nothing was persisted yet, restore the original placement.
                let previous = self.take(source);
                let moving = self.take(ItemLocation::Equipment(slot));
                self.put(ItemLocation::Equipment(slot), previous);
                self.put(source, moving);
                Err(error(
                    InventoryErrorReason::InvFull,
                    format!("no free bag slot for the item in {conflict:?}"),
                ))
            }
        }
    }

    fn bind_on_equip(&mut self, slot: EquipmentSlot) {
        let Some(item) = self.equipped.get(&slot) else {
            return;
        };
        if self
            .template(item)
            .is_some_and(|t| t.bonding == BIND_ON_EQUIP)
        {
            let guid = item.item_guid;
            self.instances.entry(guid).or_default().soulbound = true;
        }
    }

    /// Other-hand slot that must be emptied when `item` goes into `slot`.
    fn hand_conflict(
        &self,
        item: &InventoryItemData,
        slot: EquipmentSlot,
    ) -> Option<EquipmentSlot> {
        let is_two_hand = |item: &InventoryItemData| self.requires_both_hands(item);
        match slot {
            EquipmentSlot::MainHand
                if is_two_hand(item) && self.equipped.contains_key(&EquipmentSlot::OffHand) =>
            {
                Some(EquipmentSlot::OffHand)
            }
            EquipmentSlot::OffHand
                if self
                    .equipped
                    .get(&EquipmentSlot::MainHand)
                    .is_some_and(is_two_hand) =>
            {
                Some(EquipmentSlot::MainHand)
            }
            _ => None,
        }
    }

    /// Retail `CMSG_SPLIT_ITEM`: move `count` of a bag stack into an empty bag slot
    /// or onto a stack of the same item with room for all of them.
    pub(crate) fn split(
        &mut self,
        from: ItemLocation,
        to: ItemLocation,
        count: u32,
        new_guid: u64,
    ) -> OpResult {
        let source = self.existing(from)?;
        self.check_location(to)?;
        if !matches!(
            (from, to),
            (
                ItemLocation::Bag { .. } | ItemLocation::Bank { .. },
                ItemLocation::Bag { .. } | ItemLocation::Bank { .. }
            )
        ) {
            return Err(error(
                InventoryErrorReason::SplitFailed,
                "stacks can only be split between bag or bank slots",
            ));
        }
        if count == 0 || count >= source.stack_count {
            return Err(error(
                InventoryErrorReason::TooFewToSplit,
                format!(
                    "cannot split {count} from a stack of {}",
                    source.stack_count
                ),
            ));
        }
        self.check_bag_type(to, source)?;
        if let Some(target) = self.get(to) {
            let fits = self.same_definition(source, target)
                && target.stack_count + count <= self.max_stack(target);
            if !fits {
                return Err(error(
                    InventoryErrorReason::CantStack,
                    format!("{count} more do not stack onto the item at {to:?}"),
                ));
            }
            self.merge_stacks(from, to, count);
            return Ok(());
        }
        let mut split = source.clone();
        let source_guid = split.item_guid;
        split.item_guid = new_guid;
        split.stack_count = count;
        self.copy_instance_identity(source_guid, new_guid);
        if let Some(meta) = self.instances.get(&source_guid).copied() {
            self.instances.insert(new_guid, meta);
        }
        let mut remaining = self.take(from).expect("validated source");
        remaining.stack_count -= count;
        self.put(from, Some(remaining));
        self.put(to, Some(split));
        Ok(())
    }

    /// Retail `CMSG_DESTROY_ITEM`: 0 or at least the stack size destroys the stack.
    pub(crate) fn destroy(&mut self, location: ItemLocation, count: u32) -> OpResult {
        let result = self.with_appearance_sync(|model| model.destroy_count(location, count));
        self.sync_containers();
        result
    }

    fn destroy_count(&mut self, location: ItemLocation, count: u32) -> OpResult {
        let item = self.existing(location)?;
        if let ItemLocation::Equipment(slot) = location
            && let Some(bag) = slot.bag_index()
        {
            self.check_bag_empty(bag)?;
        }
        let item = item.clone();
        self.collect_disposed_appearance(&item);
        if count == 0 || count >= item.stack_count {
            let removed = self.take(location).expect("validated item");
            self.instances.remove(&removed.item_guid);
            return Ok(());
        }
        if let ItemLocation::Bag { bag, slot } = location {
            let item = self.bags[usize::from(bag)][usize::from(slot)]
                .as_mut()
                .expect("validated item");
            item.stack_count -= count;
        }
        Ok(())
    }

    /// The spell using the item at `location` casts (`Player::ProcessItemCast`,
    /// Player.cpp:31665-31691): an equippable item is used only when equipped.
    pub(crate) fn use_spell(
        &self,
        location: ItemLocation,
    ) -> Result<(OnUseSpell, ItemScaling), InventoryError> {
        let item = self.existing(location)?;
        let template = self.template(item);
        if template.is_some_and(|template| template.inventory_type != 0)
            && !matches!(location, ItemLocation::Equipment(_))
        {
            return Err(error(
                InventoryErrorReason::ItemNotFound,
                format!("item {} is used only when equipped", item.item_id),
            ));
        }
        template
            .and_then(|template| template.on_use.map(|spell| (spell, template.scaling)))
            .ok_or_else(|| {
                error(
                    InventoryErrorReason::CantDoRightNow,
                    format!("item {} has no on-use spell", item.item_id),
                )
            })
    }

    pub(crate) fn gold(&self) -> u64 {
        self.gold
    }

    pub(crate) fn set_gold(&mut self, gold: u64) {
        self.gold = gold;
    }

    /// TC Player.cpp:9490–9512,9928–9954 counts equipment, bags and personal bank.
    pub(crate) fn owned_count(&self, source: ItemDefinitionSource, item_id: u32) -> u32 {
        self.owned_count_excluding(source, item_id, None)
    }

    fn owned_count_excluding(
        &self,
        source: ItemDefinitionSource,
        item_id: u32,
        skip: Option<u64>,
    ) -> u32 {
        self.carried_count_excluding(source, item_id, skip)
            .saturating_add(self.bank_count_excluding(source, item_id, skip))
    }

    fn carried_count_excluding(
        &self,
        source: ItemDefinitionSource,
        item_id: u32,
        skip: Option<u64>,
    ) -> u32 {
        self.carried_items()
            .filter(|item| {
                item.item_id == item_id
                    && Some(item.item_guid) != skip
                    && self.definition_source(item.item_guid) == source
            })
            .map(|item| item.stack_count)
            .fold(0, u32::saturating_add)
    }

    fn carried_items(&self) -> impl Iterator<Item = &InventoryItemData> {
        self.bags
            .iter()
            .flatten()
            .flatten()
            .chain(self.equipped.values())
            .chain(&self.overflow)
    }

    fn bank_count_excluding(
        &self,
        source: ItemDefinitionSource,
        item_id: u32,
        skip: Option<u64>,
    ) -> u32 {
        if let Some(bank) = &self.bank {
            let items = bank.tabs.iter().flat_map(|tab| tab.slots.iter().flatten());
            return items
                .filter(|item| {
                    item.item_id == item_id
                        && Some(item.item_guid) != skip
                        && self.definition_source(item.item_guid) == source
                })
                .map(|item| item.stack_count)
                .fold(0, u32::saturating_add);
        }
        let carried_guids: HashSet<_> = self.carried_items().map(|item| item.item_guid).collect();
        self.bank_owned
            .iter()
            .filter(|(owned_source, item)| {
                let same_definition = *owned_source == source && item.item_id == item_id;
                let excluded =
                    Some(item.item_guid) == skip || carried_guids.contains(&item.item_guid);
                same_definition && !excluded
            })
            .map(|(_, item)| item.stack_count)
            .fold(0, u32::saturating_add)
    }

    fn check_max_count(
        &self,
        item: &InventoryItemData,
        source: ItemDefinitionSource,
        template: ItemTemplate,
    ) -> OpResult {
        if template.max_count <= 0 {
            return Ok(());
        }
        let owned = self.owned_count_excluding(source, item.item_id, Some(item.item_guid));
        if u64::from(owned) + u64::from(item.stack_count) > template.max_count as u64 {
            return Err(error(
                InventoryErrorReason::ItemMaxCount,
                format!("item {} ownership cap {}", item.item_id, template.max_count),
            ));
        }
        Ok(())
    }

    /// Items of `(source, item_id)` in bags (not equipment or overflow).
    pub(crate) fn bag_count(&self, source: ItemDefinitionSource, item_id: u32) -> u32 {
        self.bag_items()
            .filter(|item| {
                item.item_id == item_id && self.definition_source(item.item_guid) == source
            })
            .map(|item| item.stack_count)
            .sum()
    }

    /// Item ids of bag stacks belonging to the selected source.
    pub(crate) fn bag_item_ids(
        &self,
        source: ItemDefinitionSource,
    ) -> impl Iterator<Item = u32> + '_ {
        self.bag_items()
            .filter(move |item| self.definition_source(item.item_guid) == source)
            .map(|item| item.item_id)
    }

    /// AzerothCore `DestroyItemCount`: remove `count` of `(source, item_id)`
    /// in bag order; nothing changes when that source holds fewer.
    pub(crate) fn destroy_bag_items(
        &mut self,
        source: ItemDefinitionSource,
        item_id: u32,
        count: u32,
    ) -> Result<(), String> {
        if self.bag_count(source, item_id) < count {
            return Err(format!(
                "bags hold fewer than {count} of {source:?} item {item_id}"
            ));
        }
        let sources = &self.definition_sources;
        let mut left = count;
        for cell in self.bags.iter_mut().flatten() {
            if left == 0 {
                break;
            }
            let Some(item) = cell
                .as_mut()
                .filter(|item| item.item_id == item_id && sources[&item.item_guid] == source)
            else {
                continue;
            };
            let taken = left.min(item.stack_count);
            item.stack_count -= taken;
            left -= taken;
            if item.stack_count == 0 {
                let guid = item.item_guid;
                *cell = None;
                self.instances.remove(&guid);
            }
        }
        Ok(())
    }

    /// Visual slot of the equipped item `item_guid`.
    pub(crate) fn equipped_visual_slot(
        &self,
        item_guid: u64,
    ) -> Option<shared::components::EquipmentVisualSlot> {
        self.equipped
            .iter()
            .find(|(_, item)| item.item_guid == item_guid)
            .and_then(|(slot, _)| slot.visual_slot())
    }

    /// Saved per-item state; callers validate the item's location separately.
    pub(crate) fn bag_durability(&self, guid: u64) -> Option<ItemDurability> {
        self.item_durability
            .get(&guid)
            .map(|&(current, max)| ItemDurability { current, max })
    }

    pub(crate) fn restore_bag_durability(&mut self, guid: u64) {
        let state = self
            .item_durability
            .get_mut(&guid)
            .expect("validated bag durability");
        state.0 = state.1;
    }

    /// Every item in the bags, in bag and slot order.
    pub(crate) fn bag_items(&self) -> impl Iterator<Item = &InventoryItemData> {
        self.bags.iter().flatten().flatten()
    }

    /// Item ids whose appearance the account learns (Retail transmog): every
    /// equipped item and every soulbound one.
    pub(crate) fn appearance_sources(&self) -> Vec<(ItemDefinitionSource, u32)> {
        let equipped = self.equipped.values();
        let soulbound = self
            .bags
            .iter()
            .flatten()
            .flatten()
            .chain(&self.overflow)
            .filter(|item| self.is_soulbound(item.item_guid));
        equipped
            .chain(soulbound)
            .map(|item| (self.definition_source(item.item_guid), item.item_id))
            .collect()
    }

    /// Retail Warbands: selling/deleting/disenchanting also learns an unbound item.
    /// Transfer helpers intentionally do not call this.
    pub(crate) fn collect_disposed_appearance(&mut self, item: &InventoryItemData) {
        if let Some(appearance) = self
            .template(item)
            .and_then(|template| template.appearance_id)
        {
            self.appearance_grants.push(appearance);
        }
    }

    pub(crate) fn is_soulbound(&self, item_guid: u64) -> bool {
        self.instances
            .get(&item_guid)
            .is_some_and(|meta| meta.soulbound)
    }

    /// The bag item with `item_guid`.
    pub(crate) fn bag_item(&self, item_guid: u64) -> Option<&InventoryItemData> {
        self.bags
            .iter()
            .flatten()
            .flatten()
            .find(|item| item.item_guid == item_guid)
    }

    /// Remove `count` of the bag stack `item_guid` (0 or the stack size takes all).
    /// A partial take is a new item with guid `mint()` (AzerothCore `CloneItem`).
    pub(crate) fn take_bag_item(
        &mut self,
        item_guid: u64,
        count: u32,
        mint: &mut impl FnMut() -> u64,
    ) -> Result<InventoryItemData, InventoryError> {
        let cell = self
            .bags
            .iter_mut()
            .flatten()
            .find(|cell| {
                cell.as_ref()
                    .is_some_and(|item| item.item_guid == item_guid)
            })
            .ok_or_else(|| {
                error(
                    InventoryErrorReason::ItemNotFound,
                    format!("no bag item {item_guid}"),
                )
            })?;
        let item = cell.as_mut().expect("found item");
        if count == 0 || count == item.stack_count {
            let taken = cell.take().expect("found item");
            return Ok(taken);
        }
        if count > item.stack_count {
            return Err(error(
                InventoryErrorReason::TooFewToSplit,
                format!("cannot take {count} from a stack of {}", item.stack_count),
            ));
        }
        item.stack_count -= count;
        let mut taken = item.clone();
        taken.item_guid = mint();
        taken.stack_count = count;
        self.copy_instance_identity(item_guid, taken.item_guid);
        if let Some(meta) = self.instances.get(&item_guid).copied() {
            self.instances.insert(taken.item_guid, meta);
        }
        Ok(taken)
    }

    /// AzerothCore `CanStoreNewItem` + `StoreNewItem`: top up existing stacks of the
    /// item, then fill empty bag slots with full stacks. The first new stack keeps
    /// `item`'s guid, later ones get `mint()`. Nothing changes when it doesn't fit.
    pub(crate) fn store_item(
        &mut self,
        item: InventoryItemData,
        source: ItemDefinitionSource,
        template: ItemTemplate,
        mint: &mut impl FnMut() -> u64,
    ) -> OpResult {
        self.check_max_count(&item, source, template)?;
        self.register_definition(&item, source, template);
        let max_stack = template.max_stack.max(1);
        if self.room_for(&item, max_stack) < item.stack_count {
            return Err(error(
                InventoryErrorReason::InvFull,
                format!("no room for {} of item {}", item.stack_count, item.item_id),
            ));
        }
        let mut left = item.stack_count;
        let sources = &self.definition_sources;
        for other in self.bags.iter_mut().flatten().flatten() {
            if left == 0 {
                break;
            }
            if other.item_id == item.item_id
                && sources[&other.item_guid] == source
                && other.stack_count < max_stack
            {
                let moved = left.min(max_stack - other.stack_count);
                other.stack_count += moved;
                left -= moved;
            }
        }
        let mut guid = Some(item.item_guid);
        while left > 0 {
            let (bag, slot) = self.first_free_cell(&item).expect("room was counted");
            let count = left.min(max_stack);
            let mut stack = item.clone();
            stack.item_guid = guid.take().unwrap_or_else(&mut *mint);
            stack.stack_count = count;
            self.copy_instance_identity(item.item_guid, stack.item_guid);
            self.bags[bag][slot] = Some(stack);
            left -= count;
        }
        self.bind_owned_pickup_items();
        Ok(())
    }

    /// TrinityCore `Player::StoreNewItemInBestSlots` (Player.cpp:561-596): equip the
    /// items one at a time into free slots they may occupy, then store the rest in
    /// the bags like `store_item`.
    pub(crate) fn store_new_item_in_best_slots(
        &mut self,
        item: InventoryItemData,
        source: ItemDefinitionSource,
        template: ItemTemplate,
        mint: &mut impl FnMut() -> u64,
    ) -> OpResult {
        self.check_max_count(&item, source, template)?;
        self.register_definition(&item, source, template);
        let mut guid = Some(item.item_guid);
        let mut left = item.stack_count;
        while left > 0 {
            let Some(slot) = self.free_equip_slot(&item) else {
                break;
            };
            let item_guid = guid.take().unwrap_or_else(&mut *mint);
            self.copy_instance_identity(item.item_guid, item_guid);
            let copy = InventoryItemData {
                item_guid,
                stack_count: 1,
                ..item.clone()
            };
            self.equip_new(copy, slot)?;
            left -= 1;
        }
        if left == 0 {
            return Ok(());
        }
        let item_guid = guid.unwrap_or_else(&mut *mint);
        let rest = InventoryItemData {
            item_guid,
            stack_count: left,
            ..item
        };
        self.copy_instance_identity(item.item_guid, item_guid);
        self.store_item(rest, source, template, mint)
    }

    /// `Player::Create` second pass (Player.cpp:525-554): each backpack item in slot
    /// order is equipped when it now has a free slot (an off hand stored before its
    /// main hand was equipped); the others move to the first free backpack slot up to
    /// their own (`CanStoreItem` counts the moving item's slot as free).
    pub(crate) fn equip_backpack_items(&mut self) -> OpResult {
        let backpack = usize::from(shared::protocol::BACKPACK_BAG);
        for cell in 0..self.bags[backpack].len() {
            let Some(item) = self.bags[backpack][cell].take() else {
                continue;
            };
            if let Some(slot) = self.free_equip_slot(&item) {
                self.equip_new(item, slot)?;
                continue;
            }
            let target = self.bags[backpack]
                .iter()
                .position(Option::is_none)
                .expect("the item's own slot is free");
            self.bags[backpack][target] = Some(item);
        }
        Ok(())
    }

    /// `Player::FindEquipSlot` with NULL_SLOT and no swap (Player.cpp:9211-9411): the
    /// first empty slot the item may occupy. The off hand is not free under a
    /// two-hander (`IsTwoHandUsed`), and a one-hander gets no off hand without dual
    /// wield (`validate_equip`).
    fn free_equip_slot(&self, item: &InventoryItemData) -> Option<EquipmentSlot> {
        self.equip_slots(item).iter().copied().find(|&slot| {
            !self.equipped.contains_key(&slot)
                && !(slot == EquipmentSlot::OffHand && self.two_hand_used())
                && self.validate_equip(item, slot).is_ok()
        })
    }

    fn two_hand_used(&self) -> bool {
        self.equipped
            .get(&EquipmentSlot::MainHand)
            .is_some_and(|item| self.requires_both_hands(item))
    }

    fn requires_both_hands(&self, item: &InventoryItemData) -> bool {
        let inventory_type = self.inventory_type(item);
        TWO_HAND_TYPES.contains(&inventory_type)
            && (inventory_type != TWO_HAND_WEAPON || !self.can_titan_grip(item))
    }

    /// `Player::EquipNewItem` then `AutoUnequipOffhandIfNeed` (Player.cpp:26123-26167):
    /// a two-hander moves the off-hand item to a free bag slot.
    fn equip_new(&mut self, item: InventoryItemData, slot: EquipmentSlot) -> OpResult {
        self.with_appearance_sync(|model| {
            model.equipped.insert(slot, item);
            model.sync_containers();
            model.bind_on_equip(slot);
            if !(model.two_hand_used() && model.equipped.contains_key(&EquipmentSlot::OffHand)) {
                return Ok(());
            }
            let off_hand = &model.equipped[&EquipmentSlot::OffHand];
            let (bag, cell) = model.first_free_cell(off_hand).ok_or_else(|| {
                error(
                    InventoryErrorReason::InvFull,
                    "no free bag slot for the off-hand item",
                )
            })?;
            model.bags[bag][cell] = model.equipped.remove(&EquipmentSlot::OffHand);
            Ok(())
        })
    }

    /// TrinityCore `CanStoreNewItem(bag, slot)` + `StoreNewItem` for a purchase
    /// dropped on a bag slot: it goes to `location` when that is empty or holds the
    /// same item with room; what does not fit there is stored like `store_item`
    /// (`CanStoreItem` spills to other stacks and free slots). Another item or a
    /// full stack at `location` refuses with `CantStack`; nothing changes on failure.
    pub(crate) fn store_item_at(
        &mut self,
        mut item: InventoryItemData,
        source: ItemDefinitionSource,
        template: ItemTemplate,
        location: ItemLocation,
        mint: &mut impl FnMut() -> u64,
    ) -> OpResult {
        let ItemLocation::Bag { bag, slot } = location else {
            return Err(error(
                InventoryErrorReason::WrongSlot,
                format!("purchases go to bag slots, not {location:?}"),
            ));
        };
        self.check_location(location)?;
        self.check_max_count(&item, source, template)?;
        self.register_definition(&item, source, template);
        self.check_bag_type(location, &item)?;
        let max_stack = template.max_stack.max(1);
        let room_here = match self.get(location) {
            None => max_stack,
            Some(other) if self.same_definition(other, &item) && other.stack_count < max_stack => {
                max_stack - other.stack_count
            }
            Some(_) => {
                return Err(error(
                    InventoryErrorReason::CantStack,
                    format!("item {} does not stack onto {location:?}", item.item_id),
                ));
            }
        };
        if self.room_for(&item, max_stack) < item.stack_count {
            return Err(error(
                InventoryErrorReason::InvFull,
                format!("no room for {} of item {}", item.stack_count, item.item_id),
            ));
        }
        let here = room_here.min(item.stack_count);
        let rest = item.stack_count - here;
        let cell = &mut self.bags[usize::from(bag)][usize::from(slot)];
        let guid_taken = match cell {
            Some(other) => {
                other.stack_count += here;
                false
            }
            None => {
                *cell = Some(InventoryItemData {
                    stack_count: here,
                    ..item.clone()
                });
                true
            }
        };
        if rest == 0 {
            self.bind_owned_pickup_items();
            return Ok(());
        }
        if guid_taken {
            let source_guid = item.item_guid;
            item.item_guid = mint();
            self.copy_instance_identity(source_guid, item.item_guid);
        }
        item.stack_count = rest;
        self.store_item(item, source, template, mint)
    }

    /// Items like `item` the bags can still take: free slots of containers it may go
    /// into plus room on its stacks.
    fn room_for(&self, item: &InventoryItemData, max_stack: u32) -> u32 {
        self.bags
            .iter()
            .enumerate()
            .filter(|(bag, _)| self.accepts(*bag, item))
            .flat_map(|(_, cells)| cells)
            .map(|cell| match cell {
                None => max_stack,
                Some(other) if self.same_definition(other, item) => {
                    max_stack.saturating_sub(other.stack_count)
                }
                Some(_) => 0,
            })
            .fold(0u32, u32::saturating_add)
    }

    /// Compact bag items: quality descending, then item id, then larger stacks first.
    /// Specialty bags (reagent, profession) fill first with the items they take,
    /// then the rest go to the containers that take anything.
    pub(crate) fn sort_bags(&mut self) -> OpResult {
        let mut items: Vec<_> = self
            .bags
            .iter_mut()
            .flatten()
            .filter_map(Option::take)
            .collect();
        items.sort_by(|a, b| {
            b.quality
                .cmp(&a.quality)
                .then(a.item_id.cmp(&b.item_id))
                .then(b.stack_count.cmp(&a.stack_count))
        });
        let specialty: Vec<usize> = (1..self.bags.len())
            .filter(|&bag| !self.is_general_container(bag))
            .collect();
        let mut rest = Vec::new();
        for item in items {
            let cell = specialty
                .iter()
                .filter(|&&bag| self.accepts(bag, &item))
                .find_map(|&bag| {
                    let slot = self.bags[bag].iter().position(Option::is_none)?;
                    Some((bag, slot))
                });
            match cell {
                Some((bag, slot)) => self.bags[bag][slot] = Some(item),
                None => rest.push(item),
            }
        }
        let general: Vec<usize> = (0..self.bags.len())
            .filter(|&bag| self.is_general_container(bag))
            .collect();
        let mut cells = general
            .into_iter()
            .flat_map(|bag| (0..self.bags[bag].len()).map(move |slot| (bag, slot)))
            .collect::<Vec<_>>()
            .into_iter();
        for item in rest {
            match cells.next() {
                Some((bag, slot)) => self.bags[bag][slot] = Some(item),
                None => self.overflow.push(item),
            }
        }
        Ok(())
    }

    /// The backpack or a bag of `ITEM_SUBCLASS_CONTAINER` (takes any item).
    fn is_general_container(&self, bag: usize) -> bool {
        let Some(slot) = EquipmentSlot::from_bag_index(bag as u8) else {
            return true;
        };
        self.equipped
            .get(&slot)
            .and_then(|item| self.template(item))
            .is_some_and(|template| template.scaling.subclass_id == 0)
    }
}

fn not_equippable(inventory_type: u8) -> InventoryError {
    error(
        InventoryErrorReason::NotEquippable,
        format!("inventory type {inventory_type} is not equippable"),
    )
}

/// Locations whose content differs between two views.
pub(crate) fn diff_views(old: &InventoryView, new: &InventoryView) -> Vec<InventorySlotChange> {
    let removed = old
        .keys()
        .filter(|location| !new.contains_key(location))
        .map(|location| InventorySlotChange {
            location: *location,
            item: None,
        });
    let changed = new
        .iter()
        .filter(|(location, item)| old.get(location) != Some(item))
        .map(|(location, item)| InventorySlotChange {
            location: *location,
            item: Some(item.clone()),
        });
    let mut changes: Vec<_> = removed.chain(changed).collect();
    changes.sort_by_key(|change| change.location);
    changes
}
