//! Frozen pre-source equipment ABI, shared by all historical character layouts.
use shared::components::{EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry};
use shared::item_data::ItemDefinitionSource;

#[derive(bitcode::Encode, bitcode::Decode, Clone, Default)]
pub(super) struct StoredEquipmentAppearance {
    entries: Vec<StoredEquippedAppearanceEntry>,
}

#[derive(bitcode::Encode, bitcode::Decode, Clone)]
struct StoredEquippedAppearanceEntry {
    slot: StoredEquipmentVisualSlot,
    item_id: Option<u32>,
    display_info_id: Option<u32>,
    inventory_type: u8,
    hidden: bool,
}

// Order is part of the disk contract; never import the mutable wire enum here.
#[derive(bitcode::Encode, bitcode::Decode, Clone, Copy)]
enum StoredEquipmentVisualSlot {
    Head,
    Shoulder,
    Back,
    Chest,
    Shirt,
    Tabard,
    Wrist,
    Hands,
    Waist,
    Legs,
    Feet,
    MainHand,
    OffHand,
    Ranged,
}

impl From<EquipmentVisualSlot> for StoredEquipmentVisualSlot {
    fn from(slot: EquipmentVisualSlot) -> Self {
        match slot {
            EquipmentVisualSlot::Head => Self::Head,
            EquipmentVisualSlot::Shoulder => Self::Shoulder,
            EquipmentVisualSlot::Back => Self::Back,
            EquipmentVisualSlot::Chest => Self::Chest,
            EquipmentVisualSlot::Shirt => Self::Shirt,
            EquipmentVisualSlot::Tabard => Self::Tabard,
            EquipmentVisualSlot::Wrist => Self::Wrist,
            EquipmentVisualSlot::Hands => Self::Hands,
            EquipmentVisualSlot::Waist => Self::Waist,
            EquipmentVisualSlot::Legs => Self::Legs,
            EquipmentVisualSlot::Feet => Self::Feet,
            EquipmentVisualSlot::MainHand => Self::MainHand,
            EquipmentVisualSlot::OffHand => Self::OffHand,
            EquipmentVisualSlot::Ranged => Self::Ranged,
        }
    }
}

impl From<StoredEquipmentVisualSlot> for EquipmentVisualSlot {
    fn from(slot: StoredEquipmentVisualSlot) -> Self {
        match slot {
            StoredEquipmentVisualSlot::Head => Self::Head,
            StoredEquipmentVisualSlot::Shoulder => Self::Shoulder,
            StoredEquipmentVisualSlot::Back => Self::Back,
            StoredEquipmentVisualSlot::Chest => Self::Chest,
            StoredEquipmentVisualSlot::Shirt => Self::Shirt,
            StoredEquipmentVisualSlot::Tabard => Self::Tabard,
            StoredEquipmentVisualSlot::Wrist => Self::Wrist,
            StoredEquipmentVisualSlot::Hands => Self::Hands,
            StoredEquipmentVisualSlot::Waist => Self::Waist,
            StoredEquipmentVisualSlot::Legs => Self::Legs,
            StoredEquipmentVisualSlot::Feet => Self::Feet,
            StoredEquipmentVisualSlot::MainHand => Self::MainHand,
            StoredEquipmentVisualSlot::OffHand => Self::OffHand,
            StoredEquipmentVisualSlot::Ranged => Self::Ranged,
        }
    }
}

impl From<&EquipmentAppearance> for StoredEquipmentAppearance {
    fn from(appearance: &EquipmentAppearance) -> Self {
        Self {
            entries: appearance
                .entries
                .iter()
                .map(|entry| StoredEquippedAppearanceEntry {
                    slot: entry.slot.into(),
                    item_id: entry.item_id,
                    display_info_id: entry.display_info_id,
                    inventory_type: entry.inventory_type,
                    hidden: entry.hidden,
                })
                .collect(),
        }
    }
}

impl From<StoredEquipmentAppearance> for EquipmentAppearance {
    fn from(appearance: StoredEquipmentAppearance) -> Self {
        Self {
            entries: appearance
                .entries
                .into_iter()
                .map(|entry| EquippedAppearanceEntry {
                    slot: entry.slot.into(),
                    item_id: entry.item_id,
                    // Historical item caches are Retail. DB loads replace this using owned GUIDs.
                    definition_source: entry.item_id.map(|_| ItemDefinitionSource::Retail),
                    display_info_id: entry.display_info_id,
                    inventory_type: entry.inventory_type,
                    hidden: entry.hidden,
                })
                .collect(),
        }
    }
}
