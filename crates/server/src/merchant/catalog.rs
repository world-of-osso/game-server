//! Vendor lists from world.db `content_npc_vendor`, priced from `content_item`.
//!
//! Loading follows AzerothCore `ObjectMgr::LoadVendors` (ObjectMgr.cpp:9911):
//! rows in `slot, item` order, a negative `item` inlines that vendor's list
//! (`LoadReferenceVendor`), and rows with a `maxcount` but no `incrtime` (or the
//! reverse) are ignored. Rows with a Wrath `ExtendedCost` are skipped: Retail
//! `ItemExtendedCost` is not imported, so their honor/token prices are unknown.
use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use rusqlite::Connection;

/// AzerothCore `MAX_VENDOR_ITEMS` (Creature.h:40).
const MAX_VENDOR_ITEMS: usize = 150;

/// One vendor slot (AzerothCore `VendorItem`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VendorSlot {
    pub item_id: u32,
    /// Limited stock size; 0 = unlimited.
    pub maxcount: u32,
    /// Seconds per restock of one purchase while limited.
    pub incrtime: u32,
}

/// `content_item` prices and names of a sold item.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VendorGoods {
    pub name: String,
    pub quality: u8,
    /// Copper per purchase of `buy_count` items.
    pub buy_price: u32,
    /// Copper per item paid when selling it.
    pub sell_price: u32,
    /// `VendorStackCount`: items one purchase yields.
    pub buy_count: u32,
    /// `MaxCount`: most a player may own; 0 = no limit.
    pub max_count: u32,
}

#[derive(Resource, Debug, Default, Clone)]
pub struct VendorCatalog {
    /// Vendor lists by creature template entry.
    pub vendors: HashMap<u32, Vec<VendorSlot>>,
    /// Every `content_item` row, by item id.
    pub goods: HashMap<u32, VendorGoods>,
}

impl VendorCatalog {
    pub fn items(&self, entry: u32) -> &[VendorSlot] {
        self.vendors.get(&entry).map_or(&[], Vec::as_slice)
    }
}

type RawRows = HashMap<u32, Vec<(i64, u32, u32)>>;

pub fn load_vendor_catalog(conn: &Connection) -> rusqlite::Result<VendorCatalog> {
    let goods = load_goods(conn)?;
    let raw = load_raw_rows(conn)?;
    let vendors = raw
        .keys()
        .map(|&entry| {
            let mut slots = Vec::new();
            expand(&raw, entry, &mut HashSet::new(), &mut slots);
            slots.retain(|slot| goods.contains_key(&slot.item_id));
            slots.truncate(MAX_VENDOR_ITEMS);
            (entry, slots)
        })
        .collect();
    Ok(VendorCatalog { vendors, goods })
}

fn load_goods(conn: &Connection) -> rusqlite::Result<HashMap<u32, VendorGoods>> {
    let mut statement = conn.prepare(
        "SELECT ID, Name, Quality, BuyPrice, SellPrice, VendorStackCount, MaxCount \
         FROM content_item",
    )?;
    let rows = statement.query_map([], |row| {
        let amount = |column| -> rusqlite::Result<u32> {
            Ok(row.get::<_, i64>(column)?.clamp(0, i64::from(u32::MAX)) as u32)
        };
        let goods = VendorGoods {
            name: row.get(1)?,
            quality: row.get(2)?,
            buy_price: amount(3)?,
            sell_price: amount(4)?,
            buy_count: amount(5)?.max(1),
            max_count: amount(6)?,
        };
        Ok((row.get(0)?, goods))
    })?;
    rows.collect()
}

fn load_raw_rows(conn: &Connection) -> rusqlite::Result<RawRows> {
    let mut statement = conn.prepare(
        "SELECT entry, item, maxcount, incrtime FROM content_npc_vendor \
         WHERE ExtendedCost = 0 ORDER BY entry, slot, item",
    )?;
    let mut raw = RawRows::new();
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            (row.get(1)?, row.get(2)?, row.get(3)?),
        ))
    })?;
    for row in rows {
        let (entry, slot) = row?;
        raw.entry(entry).or_default().push(slot);
    }
    Ok(raw)
}

/// Append `entry`'s slots, inlining referenced vendors once each.
fn expand(raw: &RawRows, entry: u32, seen: &mut HashSet<u32>, out: &mut Vec<VendorSlot>) {
    if !seen.insert(entry) {
        return;
    }
    for &(item, maxcount, incrtime) in raw.get(&entry).into_iter().flatten() {
        if item < 0 {
            expand(raw, item.unsigned_abs() as u32, seen, out);
            continue;
        }
        if (maxcount == 0) != (incrtime == 0) {
            continue;
        }
        out.push(VendorSlot {
            item_id: item as u32,
            maxcount,
            incrtime,
        });
    }
}
