//! Trusted administrative equipment metadata and read-only ownership records.
use serde::{Deserialize, Serialize};
use shared::components::EquipmentVisualSlot;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EquipmentGrantItem {
    pub item_id: u32,
    pub name: String,
    pub quality: u8,
    pub required_level: u16,
    pub inventory_type: u8,
    pub vendor_sell_price: u32,
    pub allowable_class: i64,
    pub allowable_race: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EquipmentOwnership {
    pub slot: EquipmentVisualSlot,
    pub item_guid: u64,
    pub item_id: u32,
    pub name: String,
    pub quality: u8,
    pub required_level: u16,
    pub stack_count: u32,
    pub vendor_sell_price: u32,
    pub unique_auction: bool,
}
