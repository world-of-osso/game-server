//! Static item definitions — equipment, weapons, consumables.
//!
//! `ItemData` is the immutable definition loaded from the item database.
//! Runtime state (durability, enchants, soulbound status) lives on components.
//!
//! Ref: AzerothCore `ItemTemplate`, WoW `Item.dbc`.

use serde::{Deserialize, Serialize};

use crate::components::{CombatRatings, EquipmentVisualSlot, UnitStats};

/// Equipment slot for an item (matches AzerothCore `InventoryType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipSlot {
    Head,
    Neck,
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
    Finger,
    Trinket,
    MainHand,
    OffHand,
    TwoHand,
    Ranged,
}

impl EquipSlot {
    /// Convert to the visual slot used for appearance display.
    pub fn to_visual_slot(self) -> Option<EquipmentVisualSlot> {
        match self {
            Self::Head => Some(EquipmentVisualSlot::Head),
            Self::Shoulder => Some(EquipmentVisualSlot::Shoulder),
            Self::Back => Some(EquipmentVisualSlot::Back),
            Self::Chest => Some(EquipmentVisualSlot::Chest),
            Self::Shirt => Some(EquipmentVisualSlot::Shirt),
            Self::Tabard => Some(EquipmentVisualSlot::Tabard),
            Self::Wrist => Some(EquipmentVisualSlot::Wrist),
            Self::Hands => Some(EquipmentVisualSlot::Hands),
            Self::Waist => Some(EquipmentVisualSlot::Waist),
            Self::Legs => Some(EquipmentVisualSlot::Legs),
            Self::Feet => Some(EquipmentVisualSlot::Feet),
            Self::MainHand | Self::TwoHand => Some(EquipmentVisualSlot::MainHand),
            Self::OffHand => Some(EquipmentVisualSlot::OffHand),
            _ => None, // Neck, Finger, Trinket, Ranged have no visual
        }
    }
}

/// Item quality tier (affects stat budget, vendor price, name color).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ItemQuality {
    Poor = 0,
    Common = 1,
    Uncommon = 2,
    Rare = 3,
    Epic = 4,
    Legendary = 5,
}

impl ItemQuality {
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::Poor),
            1 => Some(Self::Common),
            2 => Some(Self::Uncommon),
            3 => Some(Self::Rare),
            4 => Some(Self::Epic),
            5 => Some(Self::Legendary),
            _ => None,
        }
    }
}

/// Item class (broad category).
/// Ref: AzerothCore `ItemClass` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemClass {
    Consumable,
    Container,
    Weapon,
    Armor,
    Reagent,
    Projectile,
    TradeGoods,
    Quest,
    Miscellaneous,
}

/// Weapon subclass for weapon items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeaponSubclass {
    Axe1H,
    Axe2H,
    Bow,
    Gun,
    Mace1H,
    Mace2H,
    Polearm,
    Sword1H,
    Sword2H,
    Staff,
    Fist,
    Dagger,
    Crossbow,
    Wand,
}

/// Armor subclass for armor items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArmorSubclass {
    Cloth,
    Leather,
    Mail,
    Plate,
    Shield,
}

/// Item subclass — weapon or armor type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemSubclass {
    Weapon(WeaponSubclass),
    Armor(ArmorSubclass),
    None,
}

/// Static definition of an item.
///
/// Immutable data loaded at startup. Runtime state (durability, enchants,
/// stack count) lives on separate components.
///
/// Ref: AzerothCore `ItemTemplate`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemData {
    /// Unique item ID.
    pub id: u32,
    /// Display name.
    pub name: String,
    /// Equipment slot (None for non-equippable items).
    pub slot: Option<EquipSlot>,
    /// Item level — determines stat budget.
    pub ilvl: u16,
    /// Quality tier.
    pub quality: ItemQuality,
    /// Primary and secondary stats granted when equipped.
    pub stats: ItemStats,
    /// Minimum level required to equip/use.
    pub required_level: u8,
    /// Item class (Weapon, Armor, Consumable, etc.).
    pub class: ItemClass,
    /// Subclass (weapon type, armor type).
    pub subclass: ItemSubclass,
    /// For weapons: min/max damage and speed.
    pub weapon_damage: Option<WeaponDamageData>,
    /// Armor value (for armor items).
    pub armor: u32,
    /// Buy price in copper.
    pub buy_price: u32,
    /// Sell price in copper.
    pub sell_price: u32,
    /// Max stack size (1 for equipment, higher for consumables).
    pub max_stack: u16,
}

/// Stat contributions from an item.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ItemStats {
    pub primary: UnitStats,
    pub secondary: CombatRatings,
}

/// Weapon damage range and speed for weapon items.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WeaponDamageData {
    pub min_damage: f32,
    pub max_damage: f32,
    /// Attack speed in seconds.
    pub speed: f32,
}

impl WeaponDamageData {
    pub fn dps(&self) -> f32 {
        if self.speed > 0.0 {
            (self.min_damage + self.max_damage) / (2.0 * self.speed)
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_data_plate_helm() {
        let item = ItemData {
            id: 51231,
            name: "Sanctified Lightsworn Faceguard".into(),
            slot: Some(EquipSlot::Head),
            ilvl: 264,
            quality: ItemQuality::Epic,
            stats: ItemStats {
                primary: UnitStats {
                    stamina: 117.0,
                    strength: 92.0,
                    ..Default::default()
                },
                secondary: CombatRatings {
                    crit: 60.0,
                    dodge: 52.0,
                    ..Default::default()
                },
            },
            required_level: 80,
            class: ItemClass::Armor,
            subclass: ItemSubclass::Armor(ArmorSubclass::Plate),
            weapon_damage: None,
            armor: 2042,
            buy_price: 0,
            sell_price: 173728,
            max_stack: 1,
        };
        assert_eq!(item.id, 51231);
        assert_eq!(item.quality, ItemQuality::Epic);
        assert_eq!(item.slot, Some(EquipSlot::Head));
        assert_eq!(item.stats.primary.stamina, 117.0);
    }

    #[test]
    fn item_data_2h_sword() {
        let item = ItemData {
            id: 49623,
            name: "Shadowmourne".into(),
            slot: Some(EquipSlot::TwoHand),
            ilvl: 284,
            quality: ItemQuality::Legendary,
            stats: ItemStats {
                primary: UnitStats {
                    strength: 223.0,
                    stamina: 198.0,
                    ..Default::default()
                },
                secondary: CombatRatings {
                    crit: 114.0,
                    haste: 73.0,
                    ..Default::default()
                },
            },
            required_level: 80,
            class: ItemClass::Weapon,
            subclass: ItemSubclass::Weapon(WeaponSubclass::Axe2H),
            weapon_damage: Some(WeaponDamageData {
                min_damage: 954.0,
                max_damage: 1592.0,
                speed: 3.7,
            }),
            armor: 0,
            buy_price: 0,
            sell_price: 524534,
            max_stack: 1,
        };
        assert_eq!(item.quality, ItemQuality::Legendary);
        assert!(item.weapon_damage.is_some());
        let wpn = item.weapon_damage.unwrap();
        assert!((wpn.dps() - 344.05).abs() < 0.1);
    }

    #[test]
    fn item_quality_ordering() {
        assert!(ItemQuality::Poor < ItemQuality::Common);
        assert!(ItemQuality::Rare < ItemQuality::Epic);
        assert!(ItemQuality::Epic < ItemQuality::Legendary);
    }

    #[test]
    fn item_quality_from_id() {
        assert_eq!(ItemQuality::from_id(0), Some(ItemQuality::Poor));
        assert_eq!(ItemQuality::from_id(4), Some(ItemQuality::Epic));
        assert_eq!(ItemQuality::from_id(99), None);
    }

    #[test]
    fn equip_slot_to_visual() {
        assert_eq!(
            EquipSlot::Head.to_visual_slot(),
            Some(EquipmentVisualSlot::Head)
        );
        assert_eq!(
            EquipSlot::TwoHand.to_visual_slot(),
            Some(EquipmentVisualSlot::MainHand)
        );
        assert_eq!(EquipSlot::Finger.to_visual_slot(), None);
        assert_eq!(EquipSlot::Trinket.to_visual_slot(), None);
    }

    #[test]
    fn weapon_dps_calculation() {
        let wpn = WeaponDamageData {
            min_damage: 100.0,
            max_damage: 200.0,
            speed: 3.0,
        };
        assert!((wpn.dps() - 50.0).abs() < 0.01);
    }

    #[test]
    fn weapon_dps_zero_speed() {
        let wpn = WeaponDamageData {
            min_damage: 100.0,
            max_damage: 200.0,
            speed: 0.0,
        };
        assert_eq!(wpn.dps(), 0.0);
    }

    #[test]
    fn item_data_serialization_round_trip() {
        let item = ItemData {
            id: 100,
            name: "Test Item".into(),
            slot: Some(EquipSlot::Chest),
            ilvl: 200,
            quality: ItemQuality::Rare,
            stats: ItemStats::default(),
            required_level: 70,
            class: ItemClass::Armor,
            subclass: ItemSubclass::Armor(ArmorSubclass::Leather),
            weapon_damage: None,
            armor: 500,
            buy_price: 10000,
            sell_price: 2500,
            max_stack: 1,
        };
        let json = serde_json::to_string(&item).unwrap();
        let decoded: ItemData = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, item);
    }
}
