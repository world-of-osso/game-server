//! Pure bag/equipment model: builds a slot grid from `InventoryRecord`, applies
//! Retail item moves, and writes the result back.
//!
//! Item rules come from `content_item` (`ItemTemplates`): inventory type, stack
//! size, required level, class/race masks and bonding. Gaps: bags 1-4 have no
//! container support (size 0), dual wield and weapon/armor skills are not tracked
//! (one-handers go to the main hand only), and neck, finger, trinket and ranged
//! slots have no server storage.
use std::collections::{BTreeMap, HashMap, HashSet};

use shared::components::EquippedAppearanceEntry;
use shared::protocol::{
    BACKPACK_SLOTS, BAG_COUNT, BagContents, BagSlotItem, EquipmentSlot, EquipmentSnapshot,
    EquippedItem as EquippedItemView, InventoryError, InventoryErrorReason, InventorySlotChange,
    InventorySnapshot, ItemDurability, ItemLocation, ItemStack,
};

use crate::item_templates::{BIND_ON_EQUIP, ItemTemplate, ItemTemplates};
use crate::persistence::equipment::EquippedItem;
use crate::persistence::inventory_store::{InventoryRecord, ItemInstanceData, ItemPlacementData};
use crate::persistence::{CharacterData, InventoryItemData};
use crate::stats::EquippedItems;

pub(crate) type InventoryView = BTreeMap<ItemLocation, ItemStack>;
type OpResult = Result<(), InventoryError>;

const TWO_HAND_TYPES: [u8; 4] = [15, 17, 25, 26];

pub(crate) struct InventoryModel {
    bags: Vec<Vec<Option<InventoryItemData>>>,
    /// Owned items with no free bag slot; kept so saving never drops them.
    overflow: Vec<InventoryItemData>,
    equipped: BTreeMap<EquipmentSlot, InventoryItemData>,
    instances: HashMap<u64, ItemInstanceData>,
    /// Templates of the item ids this character owns.
    templates: HashMap<u32, ItemTemplate>,
    character: CharacterData,
    gold: u64,
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
    pub(crate) fn from_record(
        record: InventoryRecord,
        templates: &ItemTemplates,
        mint: &mut impl FnMut() -> u64,
    ) -> Self {
        let owned_templates = record
            .inventory
            .items
            .iter()
            .chain(record.equipped.iter().map(|entry| &entry.item))
            .filter_map(|item| Some((item.item_id, *templates.get(item.item_id)?)))
            .collect();
        let mut model = Self {
            bags: (0..BAG_COUNT)
                .map(|bag| vec![None; bag_size(bag)])
                .collect(),
            overflow: Vec::new(),
            equipped: BTreeMap::new(),
            instances: record.instances,
            templates: owned_templates,
            character: record.character,
            gold: record.inventory.gold,
        };
        for entry in record.equipped {
            model
                .equipped
                .insert(EquipmentSlot::from_visual_slot(entry.slot), entry.item);
        }
        model.load_bag_items(record.inventory.items, &record.placements, mint);
        model.bind_owned_pickup_items();
        model
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

    pub(crate) fn free_cells(&self) -> usize {
        self.bags
            .iter()
            .flatten()
            .filter(|cell| cell.is_none())
            .count()
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

    /// Physically equipped item ids, the input of character stats.
    pub(crate) fn equipped_items(&self) -> EquippedItems {
        EquippedItems {
            items: self
                .equipped
                .iter()
                .map(|(slot, item)| (*slot, item.item_id))
                .collect(),
        }
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

    fn template(&self, item: &InventoryItemData) -> Option<&ItemTemplate> {
        self.templates.get(&item.item_id)
    }

    fn inventory_type(&self, item: &InventoryItemData) -> u8 {
        self.template(item)
            .map_or(0, |template| template.inventory_type)
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
            _ if self.can_merge(from, to) => {
                self.merge_stacks(from, to, u32::MAX);
                Ok(())
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

    /// Same item in two bag slots with room left in the target stack.
    fn can_merge(&self, from: ItemLocation, to: ItemLocation) -> bool {
        let (Some(source), Some(target)) = (self.get(from), self.get(to)) else {
            return false;
        };
        matches!(
            (from, to),
            (ItemLocation::Bag { .. }, ItemLocation::Bag { .. })
        ) && source.item_id == target.item_id
            && target.stack_count < self.max_stack(target)
    }

    /// Move up to `count` items from `from` onto the stack at `to`, capped at the
    /// stack size; an emptied source stack is removed.
    fn merge_stacks(&mut self, from: ItemLocation, to: ItemLocation, count: u32) {
        let (
            ItemLocation::Bag { bag, slot },
            ItemLocation::Bag {
                bag: to_bag,
                slot: to_slot,
            },
        ) = (from, to)
        else {
            return;
        };
        let target = self.get(to).expect("merge target");
        let room = self.max_stack(target).saturating_sub(target.stack_count);
        let source = self.bags[usize::from(bag)][usize::from(slot)]
            .as_mut()
            .expect("merge source");
        let moved = count.min(room).min(source.stack_count);
        source.stack_count -= moved;
        if source.stack_count == 0 {
            let removed = self.take(from).expect("merge source");
            self.instances.remove(&removed.item_guid);
        }
        let target = self.bags[usize::from(to_bag)][usize::from(to_slot)]
            .as_mut()
            .expect("merge target");
        target.stack_count += moved;
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
            self.bind_on_equip(slot);
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
        if let Some(target) = self.get(to) {
            let fits = target.item_id == source.item_id
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

    pub(crate) fn gold(&self) -> u64 {
        self.gold
    }

    pub(crate) fn set_gold(&mut self, gold: u64) {
        self.gold = gold;
    }

    /// Items of `item_id` in bags and equipment.
    pub(crate) fn owned_count(&self, item_id: u32) -> u32 {
        self.bags
            .iter()
            .flatten()
            .flatten()
            .chain(self.equipped.values())
            .chain(&self.overflow)
            .filter(|item| item.item_id == item_id)
            .map(|item| item.stack_count)
            .sum()
    }

    /// Items of `item_id` in bags (reagents and tools never come from equipment).
    pub(crate) fn bag_count(&self, item_id: u32) -> u32 {
        self.bag_items()
            .filter(|item| item.item_id == item_id)
            .map(|item| item.stack_count)
            .sum()
    }

    /// Item ids of every bag stack.
    pub(crate) fn bag_item_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.bag_items().map(|item| item.item_id)
    }

    /// AzerothCore `DestroyItemCount`: remove `count` of `item_id` from bag
    /// stacks in bag order; nothing changes when the bags hold fewer.
    pub(crate) fn destroy_bag_items(&mut self, item_id: u32, count: u32) -> Result<(), String> {
        if self.bag_count(item_id) < count {
            return Err(format!("bags hold fewer than {count} of item {item_id}"));
        }
        let mut left = count;
        for cell in self.bags.iter_mut().flatten() {
            if left == 0 {
                break;
            }
            let Some(item) = cell.as_mut().filter(|item| item.item_id == item_id) else {
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

    /// Every item in the bags, in bag and slot order.
    pub(crate) fn bag_items(&self) -> impl Iterator<Item = &InventoryItemData> {
        self.bags.iter().flatten().flatten()
    }

    /// Item ids whose appearance the account learns (Retail transmog): every
    /// equipped item and every soulbound one.
    pub(crate) fn appearance_sources(&self) -> Vec<u32> {
        let equipped = self.equipped.values();
        let soulbound = self
            .bags
            .iter()
            .flatten()
            .flatten()
            .chain(&self.overflow)
            .filter(|item| self.is_soulbound(item.item_guid));
        equipped.chain(soulbound).map(|item| item.item_id).collect()
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
        template: ItemTemplate,
        mint: &mut impl FnMut() -> u64,
    ) -> OpResult {
        self.templates.insert(item.item_id, template);
        let max_stack = template.max_stack.max(1);
        if self.room_for(item.item_id, max_stack) < item.stack_count {
            return Err(error(
                InventoryErrorReason::InvFull,
                format!("no room for {} of item {}", item.stack_count, item.item_id),
            ));
        }
        let mut left = item.stack_count;
        for other in self.bags.iter_mut().flatten().flatten() {
            if left == 0 {
                break;
            }
            if other.item_id == item.item_id && other.stack_count < max_stack {
                let moved = left.min(max_stack - other.stack_count);
                other.stack_count += moved;
                left -= moved;
            }
        }
        let mut guid = Some(item.item_guid);
        while left > 0 {
            let (bag, slot) = self.first_free_cell().expect("room was counted");
            let count = left.min(max_stack);
            let mut stack = item.clone();
            stack.item_guid = guid.take().unwrap_or_else(&mut *mint);
            stack.stack_count = count;
            self.bags[bag][slot] = Some(stack);
            left -= count;
        }
        Ok(())
    }

    /// TrinityCore `Player::StoreNewItemInBestSlots` (Player.cpp:561-596): equip the
    /// items one at a time into free slots they may occupy, then store the rest in
    /// the bags like `store_item`.
    pub(crate) fn store_new_item_in_best_slots(
        &mut self,
        item: InventoryItemData,
        template: ItemTemplate,
        mint: &mut impl FnMut() -> u64,
    ) -> OpResult {
        self.templates.insert(item.item_id, template);
        let mut guid = Some(item.item_guid);
        let mut left = item.stack_count;
        while left > 0 {
            let Some(slot) = self.free_equip_slot(&item) else {
                break;
            };
            let item_guid = guid.take().unwrap_or_else(&mut *mint);
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
        self.store_item(rest, template, mint)
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
        EquipmentSlot::for_inventory_type(self.inventory_type(item))
            .iter()
            .copied()
            .find(|&slot| {
                slot.visual_slot().is_some()
                    && !self.equipped.contains_key(&slot)
                    && !(slot == EquipmentSlot::OffHand && self.two_hand_used())
                    && self.validate_equip(item, slot).is_ok()
            })
    }

    fn two_hand_used(&self) -> bool {
        self.equipped
            .get(&EquipmentSlot::MainHand)
            .is_some_and(|item| TWO_HAND_TYPES.contains(&self.inventory_type(item)))
    }

    /// `Player::EquipNewItem` then `AutoUnequipOffhandIfNeed` (Player.cpp:26123-26167):
    /// a two-hander moves the off-hand item to a free bag slot.
    fn equip_new(&mut self, item: InventoryItemData, slot: EquipmentSlot) -> OpResult {
        self.with_appearance_sync(|model| {
            model.equipped.insert(slot, item);
            model.bind_on_equip(slot);
            if !(model.two_hand_used() && model.equipped.contains_key(&EquipmentSlot::OffHand)) {
                return Ok(());
            }
            let (bag, cell) = model.first_free_cell().ok_or_else(|| {
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
        self.templates.insert(item.item_id, template);
        let max_stack = template.max_stack.max(1);
        let room_here = match self.get(location) {
            None => max_stack,
            Some(other) if other.item_id == item.item_id && other.stack_count < max_stack => {
                max_stack - other.stack_count
            }
            Some(_) => {
                return Err(error(
                    InventoryErrorReason::CantStack,
                    format!("item {} does not stack onto {location:?}", item.item_id),
                ));
            }
        };
        if self.room_for(item.item_id, max_stack) < item.stack_count {
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
            return Ok(());
        }
        if guid_taken {
            item.item_guid = mint();
        }
        item.stack_count = rest;
        self.store_item(item, template, mint)
    }

    /// Items of `item_id` the bags can still take: free slots plus room on its stacks.
    fn room_for(&self, item_id: u32, max_stack: u32) -> u32 {
        self.bags
            .iter()
            .flatten()
            .map(|cell| match cell {
                None => max_stack,
                Some(other) if other.item_id == item_id => {
                    max_stack.saturating_sub(other.stack_count)
                }
                Some(_) => 0,
            })
            .fold(0u32, u32::saturating_add)
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
