//! Pure bank tab model shared by the character bank, the Warband bank and the
//! guild bank: purchased tabs of `BANK_TAB_SLOTS` item slots.
//!
//! A deposit is AzerothCore `Player::CanBankItem` / `BankItem` (and
//! `Guild::_DoItemsMove` for the guild bank): stacks of the same item are topped up
//! first, then empty slots are filled, tab by tab in the order given. Nothing
//! changes when the whole stack doesn't fit.
use shared::item_data::ItemDefinitionSource;
use shared::protocol::{BANK_TAB_SLOTS, ItemStack};
use std::collections::HashMap;

use crate::persistence::InventoryItemData;

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct BankTabData {
    pub name: String,
    /// Icon FileDataID.
    pub icon: u32,
    /// Retail `Enum.BagSlotFlags` deposit assignments.
    pub deposit_flags: u32,
    /// Guild bank tab info text.
    pub text: String,
    pub slots: Vec<Option<InventoryItemData>>,
}

impl BankTabData {
    pub fn new(name: String) -> Self {
        Self {
            name,
            icon: 0,
            deposit_flags: 0,
            text: String::new(),
            slots: vec![None; BANK_TAB_SLOTS],
        }
    }
}

/// The deposit didn't fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TabsFull;

/// Wire view of a tab's slots; `soulbound` tells whether an item guid is bound.
pub fn tab_slots(
    tab: &BankTabData,
    soulbound: impl Fn(u64) -> bool,
    source: impl Fn(u64) -> shared::item_data::ItemDefinitionSource,
    origin: impl Fn(u64) -> Option<shared::protocol::CraftingOrigin>,
) -> Vec<Option<ItemStack>> {
    tab.slots
        .iter()
        .map(|slot| {
            slot.as_ref().map(|item| ItemStack {
                item_guid: item.item_guid,
                item_id: item.item_id,
                definition_source: source(item.item_guid),
                count: item.stack_count,
                durability: None,
                soulbound: soulbound(item.item_guid),
                crafting_origin: origin(item.item_guid),
            })
        })
        .collect()
}

/// Store `item` in `tabs`, visiting tabs in `order`. The first new stack keeps
/// `item`'s guid, later ones get `mint()`.
pub fn deposit(
    tabs: &mut [BankTabData],
    order: &[usize],
    item: InventoryItemData,
    max_stack: u32,
    sources: &mut HashMap<u64, ItemDefinitionSource>,
    mint: &mut impl FnMut() -> u64,
) -> Result<(), TabsFull> {
    let source = sources[&item.item_guid];
    let max_stack = max_stack.max(1);
    let room: u32 = order
        .iter()
        .filter_map(|&tab| tabs.get(tab))
        .flat_map(|tab| &tab.slots)
        .map(|slot| match slot {
            None => max_stack,
            Some(other) if other.item_id == item.item_id && sources[&other.item_guid] == source => {
                max_stack.saturating_sub(other.stack_count)
            }
            Some(_) => 0,
        })
        .fold(0u32, u32::saturating_add);
    if room < item.stack_count {
        return Err(TabsFull);
    }
    let mut left = item.stack_count;
    let mut guid = Some(item.item_guid);
    for &index in order {
        let Some(tab) = tabs.get_mut(index) else {
            continue;
        };
        for other in tab.slots.iter_mut().flatten() {
            if left > 0
                && other.item_id == item.item_id
                && sources[&other.item_guid] == source
                && other.stack_count < max_stack
            {
                let moved = left.min(max_stack - other.stack_count);
                other.stack_count += moved;
                left -= moved;
            }
        }
        for slot in tab.slots.iter_mut().filter(|slot| slot.is_none()) {
            if left == 0 {
                break;
            }
            let count = left.min(max_stack);
            let mut stack = item.clone();
            stack.item_guid = guid.take().unwrap_or_else(&mut *mint);
            stack.stack_count = count;
            sources.insert(stack.item_guid, source);
            *slot = Some(stack);
            left -= count;
        }
    }
    Ok(())
}

/// Read GUID identity separately from the unchanged bank row encoding.
pub(crate) fn load_tab_definition_sources(
    db: &redb::Database,
    tabs: &[BankTabData],
) -> HashMap<u64, ItemDefinitionSource> {
    tabs.iter()
        .flat_map(|tab| tab.slots.iter().flatten())
        .map(|item| {
            (
                item.item_guid,
                crate::persistence::inventory_store::load_item_definition_source(
                    db,
                    item.item_guid,
                ),
            )
        })
        .collect()
}

/// Take the whole stack in `tab`/`slot`.
pub fn take(tabs: &mut [BankTabData], tab: u8, slot: u8) -> Option<InventoryItemData> {
    tabs.get_mut(usize::from(tab))?
        .slots
        .get_mut(usize::from(slot))?
        .take()
}

/// `preferred` first, then every other tab in order.
pub fn tab_order(tab_count: usize, preferred: usize) -> Vec<usize> {
    std::iter::once(preferred)
        .filter(|&tab| tab < tab_count)
        .chain((0..tab_count).filter(|&tab| tab != preferred))
        .collect()
}

#[cfg(test)]
#[path = "bank_storage_tests.rs"]
mod tests;
