//! Pure bag/equipment model: builds a slot grid from `InventoryRecord`, applies
//! Retail item moves, and writes the result back.
//!
//! Server data gaps (no item template table): inventory type comes from
//! `ItemInstanceData` or the equipped appearance entry, stack limits are unknown
//! (same-item stacks swap instead of merging), bags 1-4 have no container data
//! (size 0), dual wield is not tracked (one-handers go to the main hand only),
//! class/race proficiency and binding types are not stored, and neck, finger,
//! trinket and ranged slots have no server storage.
use std::collections::{BTreeMap, HashMap, HashSet};

use shared::components::EquippedAppearanceEntry;
use shared::protocol::{
    BACKPACK_SLOTS, BAG_COUNT, BagContents, BagSlotItem, EquipmentSlot, EquipmentSnapshot,
    EquippedItem as EquippedItemView, InventoryError, InventoryErrorReason, InventorySlotChange,
    InventorySnapshot, ItemDurability, ItemLocation, ItemStack,
};

use crate::persistence::equipment::EquippedItem;
use crate::persistence::inventory_store::{InventoryRecord, ItemInstanceData, ItemPlacementData};
use crate::persistence::{CharacterData, InventoryItemData};

pub(crate) type InventoryView = BTreeMap<ItemLocation, ItemStack>;
type OpResult = Result<(), InventoryError>;

const TWO_HAND_TYPES: [u8; 4] = [15, 17, 25, 26];

pub(crate) struct InventoryModel {
    bags: Vec<Vec<Option<InventoryItemData>>>,
    /// Owned items with no free bag slot; kept so saving never drops them.
    overflow: Vec<InventoryItemData>,
    equipped: BTreeMap<EquipmentSlot, InventoryItemData>,
    instances: HashMap<u64, ItemInstanceData>,
    character: CharacterData,
    gold: u32,
}

pub(crate) fn error(reason: InventoryErrorReason, detail: impl Into<String>) -> InventoryError {
    InventoryError {
        reason,
        detail: Some(detail.into()),
    }
}

fn bag_size(bag: u8) -> usize {
    if bag == 0 {
        usize::from(BACKPACK_SLOTS)
    } else {
        0
    }
}

impl InventoryModel {
    /// `mint` allocates a fresh item guid (for duplicate guids and splits).
    pub(crate) fn from_record(record: InventoryRecord, mint: &mut impl FnMut() -> u64) -> Self {
        let mut model = Self {
            bags: (0..BAG_COUNT)
                .map(|bag| vec![None; bag_size(bag)])
                .collect(),
            overflow: Vec::new(),
            equipped: BTreeMap::new(),
            instances: record.instances,
            character: record.character,
            gold: record.inventory.gold,
        };
        model.load_equipped(record.equipped);
        model.load_bag_items(record.inventory.items, &record.placements, mint);
        model
    }

    fn load_equipped(&mut self, equipped: Vec<EquippedItem>) {
        for entry in equipped {
            let inventory_type = self
                .character
                .equipment_appearance
                .entries
                .iter()
                .find(|appearance| appearance.slot == entry.slot)
                .map(|appearance| appearance.inventory_type)
                .unwrap_or(0);
            self.instances
                .entry(entry.item.item_guid)
                .or_insert(ItemInstanceData {
                    inventory_type,
                    soulbound: false,
                });
            self.equipped
                .insert(EquipmentSlot::from_visual_slot(entry.slot), entry.item);
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
                let meta = self.instances.get(&item.item_guid).copied();
                item.item_guid = mint();
                seen.insert(item.item_guid);
                if let Some(meta) = meta {
                    self.instances.insert(item.item_guid, meta);
                }
                unplaced.push(item);
                continue;
            }
            match placed.get(&item.item_guid) {
                Some(&(bag, slot)) if self.cell_is_free(bag, slot) => {
                    self.bags[usize::from(bag)][usize::from(slot)] = Some(item);
                }
                _ => unplaced.push(item),
            }
        }
        for item in unplaced {
            match self.first_free_cell() {
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

    fn first_free_cell(&self) -> Option<(usize, usize)> {
        self.bags.iter().enumerate().find_map(|(bag, cells)| {
            cells
                .iter()
                .position(Option::is_none)
                .map(|slot| (bag, slot))
        })
    }

    pub(crate) fn into_record(self) -> InventoryRecord {
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
        let equipped = self
            .equipped
            .into_iter()
            .filter_map(|(slot, item)| {
                Some(EquippedItem {
                    slot: slot.visual_slot()?,
                    item,
                })
            })
            .collect();
        InventoryRecord {
            character: self.character,
            inventory: crate::persistence::InventoryData {
                gold: self.gold,
                items,
            },
            placements,
            equipped,
            instances: self.instances,
        }
    }

    // --- Views -----------------------------------------------------------

    pub(crate) fn appearance(&self) -> &shared::components::EquipmentAppearance {
        &self.character.equipment_appearance
    }

    fn stack(&self, item: &InventoryItemData, durability: Option<ItemDurability>) -> ItemStack {
        ItemStack {
            item_guid: item.item_guid,
            item_id: item.item_id,
            count: item.stack_count,
            durability,
            soulbound: self
                .instances
                .get(&item.item_guid)
                .is_some_and(|meta| meta.soulbound),
        }
    }

    fn equipped_durability(&self, slot: EquipmentSlot) -> Option<ItemDurability> {
        let visual = slot.visual_slot()?;
        crate::durability::load_equipped_durability(&self.character)
            .slots
            .into_iter()
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
                item.as_ref().map(|item| (location, self.stack(item, None)))
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
                size: bag_size(bag) as u8,
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
                ItemLocation::Bag { .. } => None,
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
            ItemLocation::Equipment(slot) if slot.visual_slot().is_none() => Err(error(
                InventoryErrorReason::WrongSlot,
                format!("server has no storage for equipment slot {slot:?}"),
            )),
            ItemLocation::Equipment(_) => Ok(()),
        }
    }

    fn get(&self, location: ItemLocation) -> Option<&InventoryItemData> {
        match location {
            ItemLocation::Bag { bag, slot } => {
                self.bags[usize::from(bag)][usize::from(slot)].as_ref()
            }
            ItemLocation::Equipment(slot) => self.equipped.get(&slot),
        }
    }

    fn take(&mut self, location: ItemLocation) -> Option<InventoryItemData> {
        match location {
            ItemLocation::Bag { bag, slot } => {
                self.bags[usize::from(bag)][usize::from(slot)].take()
            }
            ItemLocation::Equipment(slot) => self.equipped.remove(&slot),
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
        }
    }

    fn existing(&self, location: ItemLocation) -> Result<&InventoryItemData, InventoryError> {
        self.check_location(location)?;
        self.get(location).ok_or_else(|| {
            error(
                InventoryErrorReason::ItemNotFound,
                format!("no item at {location:?}"),
            )
        })
    }

    fn inventory_type(&self, item: &InventoryItemData) -> u8 {
        self.instances
            .get(&item.item_guid)
            .map_or(0, |meta| meta.inventory_type)
    }

    /// Run `op`, then update appearance entries of the slots whose item changed.
    /// Untouched slots keep their entries (appearance-only overrides included).
    fn with_appearance_sync(&mut self, op: impl FnOnce(&mut Self) -> OpResult) -> OpResult {
        let before = self.equipped.clone();
        let result = op(self);
        for slot in EquipmentSlot::ALL {
            if before.get(&slot) != self.equipped.get(&slot) {
                self.sync_appearance(slot);
            }
        }
        result
    }

    /// Replace the appearance entry for `slot` with the physical item, keeping `hidden`.
    fn sync_appearance(&mut self, slot: EquipmentSlot) {
        let Some(visual) = slot.visual_slot() else {
            return;
        };
        let entry = self
            .equipped
            .get(&slot)
            .map(|item| (item.item_id, self.inventory_type(item)));
        let entries = &mut self.character.equipment_appearance.entries;
        let hidden = entries
            .iter()
            .find(|entry| entry.slot == visual)
            .is_some_and(|entry| entry.hidden);
        entries.retain(|entry| entry.slot != visual);
        if let Some((item_id, inventory_type)) = entry {
            entries.push(EquippedAppearanceEntry {
                slot: visual,
                item_id: Some(item_id),
                display_info_id: None,
                inventory_type,
                hidden,
            });
        }
    }

    // --- Operations ------------------------------------------------------

    /// Retail `CMSG_SWAP_ITEM` / `SWAP_INV_ITEM`: move, equip, unequip or swap.
    pub(crate) fn swap(&mut self, from: ItemLocation, to: ItemLocation) -> OpResult {
        self.with_appearance_sync(|model| model.move_item(from, to))
    }

    fn move_item(&mut self, from: ItemLocation, to: ItemLocation) -> OpResult {
        self.existing(from)?;
        self.check_location(to)?;
        if from == to {
            return Ok(());
        }
        match (from, to) {
            (_, ItemLocation::Equipment(slot)) => self.equip_into(from, slot),
            (ItemLocation::Equipment(slot), ItemLocation::Bag { .. }) if self.get(to).is_some() => {
                self.equip_into(to, slot)
            }
            _ => {
                let moving = self.take(from);
                let displaced = self.take(to);
                self.put(to, moving);
                self.put(from, displaced);
                Ok(())
            }
        }
    }

    /// Retail `CMSG_AUTO_EQUIP_ITEM`: equip into the first allowed slot, or store
    /// an equipped item in the first free bag slot.
    pub(crate) fn auto_equip(&mut self, from: ItemLocation) -> OpResult {
        let item = self.existing(from)?;
        if let ItemLocation::Equipment(_) = from {
            let (bag, slot) = self.first_free_cell().ok_or_else(|| {
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
        let slots = EquipmentSlot::for_inventory_type(self.inventory_type(item));
        let Some(first) = slots.first() else {
            return Err(not_equippable(self.inventory_type(item)));
        };
        let valid: Vec<EquipmentSlot> = slots
            .iter()
            .copied()
            .filter(|slot| slot.visual_slot().is_some() && self.validate_equip(item, *slot).is_ok())
            .collect();
        let target = valid
            .iter()
            .find(|slot| !self.equipped.contains_key(*slot))
            .or(valid.first())
            .unwrap_or(first);
        self.swap(from, ItemLocation::Equipment(*target))
    }

    fn validate_equip(&self, item: &InventoryItemData, slot: EquipmentSlot) -> OpResult {
        let inventory_type = self.inventory_type(item);
        let allowed = EquipmentSlot::for_inventory_type(inventory_type);
        if allowed.is_empty() {
            return Err(not_equippable(inventory_type));
        }
        if !allowed.contains(&slot) {
            return Err(error(
                InventoryErrorReason::WrongSlot,
                format!("inventory type {inventory_type} cannot go in {slot:?}"),
            ));
        }
        if inventory_type == 13 && slot == EquipmentSlot::OffHand {
            return Err(error(
                InventoryErrorReason::CantDualWield,
                "dual wield is not tracked by the server",
            ));
        }
        if item.required_level > self.character.level {
            return Err(error(
                InventoryErrorReason::CantEquipLevelI,
                format!("requires level {}", item.required_level),
            ));
        }
        Ok(())
    }

    /// Equip the item at `source` into `slot`. The previous occupant returns to
    /// `source`; a conflicting other-hand item goes to a free bag slot.
    fn equip_into(&mut self, source: ItemLocation, slot: EquipmentSlot) -> OpResult {
        let item = self.existing(source)?;
        self.validate_equip(item, slot)?;
        if let (Some(previous), ItemLocation::Equipment(source_slot)) =
            (self.equipped.get(&slot), source)
        {
            self.validate_equip(previous, source_slot)?;
        }
        self.finish_equip(source, slot)
    }

    fn finish_equip(&mut self, source: ItemLocation, slot: EquipmentSlot) -> OpResult {
        let conflict = self.hand_conflict(self.get(source).expect("validated source"), slot);
        let moving = self.take(source);
        let previous = self.take(ItemLocation::Equipment(slot));
        self.put(ItemLocation::Equipment(slot), moving);
        self.put(source, previous);
        let Some(conflict) = conflict else {
            return Ok(());
        };
        match self.first_free_cell() {
            Some((bag, cell)) => {
                let other = self.take(ItemLocation::Equipment(conflict));
                let to = ItemLocation::Bag {
                    bag: bag as u8,
                    slot: cell as u8,
                };
                self.put(to, other);
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

    /// Other-hand slot that must be emptied when `item` goes into `slot`.
    fn hand_conflict(
        &self,
        item: &InventoryItemData,
        slot: EquipmentSlot,
    ) -> Option<EquipmentSlot> {
        let is_two_hand =
            |item: &InventoryItemData| TWO_HAND_TYPES.contains(&self.inventory_type(item));
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

    /// Retail `CMSG_SPLIT_ITEM`: move `count` of a bag stack into an empty bag slot.
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
            (ItemLocation::Bag { .. }, ItemLocation::Bag { .. })
        ) {
            return Err(error(
                InventoryErrorReason::SplitFailed,
                "stacks can only be split between bag slots",
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
        if self.get(to).is_some() {
            return Err(error(
                InventoryErrorReason::CantStack,
                "split target must be empty: stack limits are unknown to the server",
            ));
        }
        let mut split = source.clone();
        let source_guid = split.item_guid;
        split.item_guid = new_guid;
        split.stack_count = count;
        if let Some(meta) = self.instances.get(&source_guid).copied() {
            self.instances.insert(new_guid, meta);
        }
        if let ItemLocation::Bag { bag, slot } = from {
            let remaining = self.bags[usize::from(bag)][usize::from(slot)]
                .as_mut()
                .expect("validated source");
            remaining.stack_count -= count;
        }
        self.put(to, Some(split));
        Ok(())
    }

    /// Retail `CMSG_DESTROY_ITEM`: 0 or at least the stack size destroys the stack.
    pub(crate) fn destroy(&mut self, location: ItemLocation, count: u32) -> OpResult {
        self.with_appearance_sync(|model| model.destroy_count(location, count))
    }

    fn destroy_count(&mut self, location: ItemLocation, count: u32) -> OpResult {
        let item = self.existing(location)?;
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

    /// Item on-use spells need item spell data, which the server does not load.
    pub(crate) fn use_item(&self, location: ItemLocation) -> OpResult {
        let item = self.existing(location)?;
        Err(error(
            InventoryErrorReason::CantDoRightNow,
            format!(
                "item {} has no on-use spell data on the server",
                item.item_id
            ),
        ))
    }

    /// Compact bag items: quality descending, then item id, then larger stacks first.
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
        for (cell, item) in self.bags.iter_mut().flatten().zip(items) {
            *cell = Some(item);
        }
        Ok(())
    }
}

fn not_equippable(inventory_type: u8) -> InventoryError {
    error(
        InventoryErrorReason::NotEquippable,
        format!("inventory type {inventory_type} is not equippable (0 = unknown to the server)"),
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
