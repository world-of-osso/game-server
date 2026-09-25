//! Limited vendor stock and the per-session buyback slots.
use std::collections::HashMap;

use bevy::prelude::*;
use shared::protocol::BUYBACK_SLOTS;

use super::catalog::VendorSlot;
use crate::persistence::InventoryItemData;

/// Stock left of one limited slot (AzerothCore `VendorItemCount`).
#[derive(Debug, Clone, Copy, PartialEq)]
struct StockCount {
    count: u32,
    last_increment: f64,
}

/// Limited stock per vendor creature and slot, in seconds of server time.
#[derive(Resource, Debug, Default)]
pub struct VendorStock(HashMap<(Entity, u32), StockCount>);

impl VendorStock {
    /// AzerothCore `Creature::GetVendorItemCurrentCount` (Creature.cpp:3026):
    /// `None` for unlimited slots; one purchase restocks per `incrtime` since the
    /// last purchase, capped at `maxcount`.
    pub fn available(
        &mut self,
        vendor: Entity,
        index: u32,
        slot: VendorSlot,
        buy_count: u32,
        now: f64,
    ) -> Option<u32> {
        if slot.maxcount == 0 {
            return None;
        }
        let key = (vendor, index);
        let Some(stock) = self.0.get_mut(&key) else {
            return Some(slot.maxcount);
        };
        if restock(stock, slot, buy_count, now) >= slot.maxcount {
            self.0.remove(&key);
            return Some(slot.maxcount);
        }
        Some(stock.count)
    }

    /// AzerothCore `Creature::UpdateVendorItemCurrentCount` (Creature.cpp:3061).
    pub fn consume(
        &mut self,
        vendor: Entity,
        index: u32,
        slot: VendorSlot,
        buy_count: u32,
        used: u32,
        now: f64,
    ) {
        if slot.maxcount == 0 {
            return;
        }
        let stock = self.0.entry((vendor, index)).or_insert(StockCount {
            count: slot.maxcount,
            last_increment: now,
        });
        stock.count = restock(stock, slot, buy_count, now).min(slot.maxcount);
        stock.count = stock.count.saturating_sub(used);
        stock.last_increment = now;
    }
}

/// Apply whole restock periods; returns the uncapped count.
fn restock(stock: &mut StockCount, slot: VendorSlot, buy_count: u32, now: f64) -> u32 {
    let incrtime = f64::from(slot.incrtime);
    if stock.last_increment + incrtime > now {
        return stock.count;
    }
    let periods = ((now - stock.last_increment) / incrtime) as u32;
    let count = stock
        .count
        .saturating_add(periods.saturating_mul(buy_count));
    stock.count = count.min(slot.maxcount);
    stock.last_increment = now;
    count
}

/// A sold item held for buyback.
#[derive(Debug, Clone, PartialEq)]
pub struct BuybackEntry {
    pub item: InventoryItemData,
    /// Copper the sale paid and the buyback costs.
    pub price: u32,
    pub sold_at: f64,
}

/// The session's 12 buyback slots (AzerothCore `BUYBACK_SLOT_START..END`),
/// dropped with the connection like AzerothCore drops them on logout.
#[derive(Component, Debug, Clone, PartialEq, Default)]
pub struct BuybackSlots {
    slots: [Option<BuybackEntry>; BUYBACK_SLOTS],
    current: usize,
}

impl BuybackSlots {
    /// AzerothCore `Player::AddItemToBuyBackSlot` (PlayerStorage.cpp:4003): the
    /// current slot if free, else the oldest sale is replaced.
    pub fn add(&mut self, entry: BuybackEntry) {
        let slot = if self.slots[self.current].is_none() {
            self.current
        } else {
            self.oldest()
        };
        self.slots[slot] = Some(entry);
        if self.current < BUYBACK_SLOTS - 1 {
            self.current += 1;
        }
    }

    fn oldest(&self) -> usize {
        (0..BUYBACK_SLOTS)
            .min_by(|&a, &b| {
                let time = |slot: usize| self.slots[slot].as_ref().map_or(0.0, |e| e.sold_at);
                time(a).total_cmp(&time(b))
            })
            .unwrap_or(0)
    }

    pub fn get(&self, slot: u8) -> Option<&BuybackEntry> {
        self.slots.get(usize::from(slot))?.as_ref()
    }

    /// AzerothCore `RemoveItemFromBuyBackSlot` (PlayerStorage.cpp:4062).
    pub fn remove(&mut self, slot: u8) -> Option<BuybackEntry> {
        let entry = self.slots.get_mut(usize::from(slot))?.take();
        if self.slots[self.current].is_some() {
            self.current = usize::from(slot);
        }
        entry
    }

    /// Filled slots in slot order.
    pub fn entries(&self) -> impl Iterator<Item = (u8, &BuybackEntry)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(slot, entry)| Some((slot as u8, entry.as_ref()?)))
    }
}
