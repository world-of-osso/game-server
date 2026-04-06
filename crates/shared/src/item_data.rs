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

// --- Durability ---

/// Fraction of max durability lost on each death.
const DEATH_DURABILITY_LOSS: f32 = 0.10;
/// Repair cost per point of durability (in copper) — base rate scaled by ilvl.
const REPAIR_COST_PER_POINT_BASE: f32 = 0.5;

/// Runtime durability state for a single equipped item.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Durability {
    pub current: u32,
    pub max: u32,
}

impl Durability {
    pub fn new(max: u32) -> Self {
        Self { current: max, max }
    }

    /// Apply death durability loss (10% of max, rounded up).
    pub fn apply_death_loss(&mut self) {
        let loss = ((self.max as f32 * DEATH_DURABILITY_LOSS).ceil() as u32).min(self.current);
        self.current -= loss;
    }

    /// Whether the item is broken (0 durability).
    pub fn is_broken(&self) -> bool {
        self.max > 0 && self.current == 0
    }

    /// Repair fully. Returns the cost in copper.
    pub fn repair(&mut self, ilvl: u16) -> u32 {
        let missing = self.max - self.current;
        if missing == 0 {
            return 0;
        }
        let cost = (missing as f32 * REPAIR_COST_PER_POINT_BASE * ilvl as f32) as u32;
        self.current = self.max;
        cost
    }

    /// Damage durability by a flat amount (e.g. from combat).
    pub fn damage(&mut self, amount: u32) {
        self.current = self.current.saturating_sub(amount);
    }
}

// --- Inventory ---

/// Default backpack size (16 slots, matching WoW).
const BACKPACK_SIZE: usize = 16;
/// Number of bag slots (4, matching WoW).
const BAG_SLOT_COUNT: usize = 4;
/// Default bank size (28 base slots).
const BANK_SIZE: usize = 28;

/// Number of equipped item slots.
const EQUIP_SLOT_COUNT: usize = 18;

/// An inventory slot that may hold an item (by ID) and stack count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct InvSlot {
    /// Item ID (0 = empty).
    pub item_id: u32,
    /// Stack count (1 for equipment, >1 for consumables).
    pub count: u16,
}

impl InvSlot {
    pub fn is_empty(&self) -> bool {
        self.item_id == 0
    }
}

/// A bag with a fixed number of slots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bag {
    pub slots: Vec<InvSlot>,
}

impl Bag {
    pub fn new(size: usize) -> Self {
        Self {
            slots: vec![InvSlot::default(); size],
        }
    }

    /// Find the first empty slot index, if any.
    pub fn first_empty(&self) -> Option<usize> {
        self.slots.iter().position(|s| s.is_empty())
    }

    /// Number of occupied slots.
    pub fn used_slots(&self) -> usize {
        self.slots.iter().filter(|s| !s.is_empty()).count()
    }
}

/// Player inventory: backpack + bags + bank + equipped items.
///
/// - Backpack: 16 fixed slots (always available)
/// - Bags: 4 bag slots, each with variable size (0 if no bag equipped)
/// - Bank: 28 base slots
/// - Equipped: one slot per EquipSlot
///
/// Ref: AzerothCore `Player.cpp` inventory handling.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Inventory {
    pub backpack: Bag,
    pub bags: [Bag; BAG_SLOT_COUNT],
    pub bank: Bag,
    pub equipped: [InvSlot; EQUIP_SLOT_COUNT],
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            backpack: Bag::new(BACKPACK_SIZE),
            bags: std::array::from_fn(|_| Bag::new(0)),
            bank: Bag::new(BANK_SIZE),
            equipped: [InvSlot::default(); EQUIP_SLOT_COUNT],
        }
    }
}

impl Inventory {
    /// Try to add an item to the first available backpack or bag slot.
    /// Returns `true` if the item was added.
    pub fn add_item(&mut self, item_id: u32, count: u16) -> bool {
        // Try backpack first
        if let Some(idx) = self.backpack.first_empty() {
            self.backpack.slots[idx] = InvSlot { item_id, count };
            return true;
        }
        // Then try each bag
        for bag in &mut self.bags {
            if let Some(idx) = bag.first_empty() {
                bag.slots[idx] = InvSlot { item_id, count };
                return true;
            }
        }
        false
    }

    /// Remove an item from any bag/backpack slot. Returns `true` if found and removed.
    pub fn remove_item(&mut self, item_id: u32) -> bool {
        if let Some(slot) = self.backpack.slots.iter_mut().find(|s| s.item_id == item_id) {
            *slot = InvSlot::default();
            return true;
        }
        for bag in &mut self.bags {
            if let Some(slot) = bag.slots.iter_mut().find(|s| s.item_id == item_id) {
                *slot = InvSlot::default();
                return true;
            }
        }
        false
    }

    /// Equip an item in a slot (by EquipSlot index 0–17).
    /// Returns the previously equipped item_id (0 if empty).
    pub fn equip(&mut self, slot_index: usize, item_id: u32) -> u32 {
        if slot_index >= EQUIP_SLOT_COUNT {
            return 0;
        }
        let prev = self.equipped[slot_index].item_id;
        self.equipped[slot_index] = InvSlot {
            item_id,
            count: 1,
        };
        prev
    }

    /// Unequip an item from a slot. Returns the item_id (0 if empty).
    pub fn unequip(&mut self, slot_index: usize) -> u32 {
        if slot_index >= EQUIP_SLOT_COUNT {
            return 0;
        }
        let prev = self.equipped[slot_index].item_id;
        self.equipped[slot_index] = InvSlot::default();
        prev
    }

    /// Get the equipped item in a slot.
    pub fn equipped_item(&self, slot_index: usize) -> Option<u32> {
        self.equipped
            .get(slot_index)
            .and_then(|s| if s.is_empty() { None } else { Some(s.item_id) })
    }

    /// Total free slots across backpack and bags.
    pub fn free_slots(&self) -> usize {
        let backpack_free = self.backpack.slots.iter().filter(|s| s.is_empty()).count();
        let bag_free: usize = self.bags.iter().map(|b| b.slots.iter().filter(|s| s.is_empty()).count()).sum();
        backpack_free + bag_free
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

    // --- Inventory tests ---

    #[test]
    fn inventory_default_sizes() {
        let inv = Inventory::default();
        assert_eq!(inv.backpack.slots.len(), 16);
        assert_eq!(inv.bags.len(), 4);
        assert_eq!(inv.bank.slots.len(), 28);
        assert_eq!(inv.equipped.len(), 18);
        assert_eq!(inv.free_slots(), 16); // bags have 0 slots by default
    }

    #[test]
    fn inventory_add_item_to_backpack() {
        let mut inv = Inventory::default();
        assert!(inv.add_item(12345, 1));
        assert_eq!(inv.backpack.slots[0].item_id, 12345);
        assert_eq!(inv.free_slots(), 15);
    }

    #[test]
    fn inventory_add_overflows_to_bags() {
        let mut inv = Inventory::default();
        inv.bags[0] = Bag::new(8); // equip an 8-slot bag
        // Fill backpack
        for i in 1..=16 {
            assert!(inv.add_item(i, 1));
        }
        assert_eq!(inv.free_slots(), 8); // only bag space left
        assert!(inv.add_item(17, 1));
        assert_eq!(inv.bags[0].slots[0].item_id, 17);
    }

    #[test]
    fn inventory_add_full_returns_false() {
        let mut inv = Inventory::default();
        for i in 1..=16 {
            inv.add_item(i, 1);
        }
        assert!(!inv.add_item(100, 1)); // backpack full, no bags
    }

    #[test]
    fn inventory_remove_item() {
        let mut inv = Inventory::default();
        inv.add_item(100, 1);
        inv.add_item(200, 1);
        assert!(inv.remove_item(100));
        assert!(inv.backpack.slots[0].is_empty());
        assert_eq!(inv.backpack.slots[1].item_id, 200);
    }

    #[test]
    fn inventory_remove_nonexistent_returns_false() {
        let mut inv = Inventory::default();
        assert!(!inv.remove_item(999));
    }

    #[test]
    fn inventory_equip_and_unequip() {
        let mut inv = Inventory::default();
        let prev = inv.equip(0, 51231); // equip helm in slot 0
        assert_eq!(prev, 0);
        assert_eq!(inv.equipped_item(0), Some(51231));

        let prev = inv.unequip(0);
        assert_eq!(prev, 51231);
        assert_eq!(inv.equipped_item(0), None);
    }

    #[test]
    fn inventory_equip_swap() {
        let mut inv = Inventory::default();
        inv.equip(0, 100);
        let prev = inv.equip(0, 200); // swap
        assert_eq!(prev, 100);
        assert_eq!(inv.equipped_item(0), Some(200));
    }

    #[test]
    fn inventory_equip_out_of_bounds() {
        let mut inv = Inventory::default();
        let prev = inv.equip(99, 100);
        assert_eq!(prev, 0);
    }

    #[test]
    fn bag_used_slots() {
        let mut bag = Bag::new(8);
        assert_eq!(bag.used_slots(), 0);
        bag.slots[0] = InvSlot { item_id: 1, count: 1 };
        bag.slots[3] = InvSlot { item_id: 2, count: 5 };
        assert_eq!(bag.used_slots(), 2);
    }

    // --- Durability tests ---

    #[test]
    fn durability_new_is_full() {
        let dur = Durability::new(100);
        assert_eq!(dur.current, 100);
        assert_eq!(dur.max, 100);
        assert!(!dur.is_broken());
    }

    #[test]
    fn durability_death_loss_10_percent() {
        let mut dur = Durability::new(100);
        dur.apply_death_loss();
        assert_eq!(dur.current, 90); // 10% of 100 = 10
    }

    #[test]
    fn durability_death_loss_rounds_up() {
        let mut dur = Durability::new(75);
        dur.apply_death_loss();
        // 10% of 75 = 7.5, ceil = 8
        assert_eq!(dur.current, 67);
    }

    #[test]
    fn durability_death_loss_clamps_to_zero() {
        let mut dur = Durability::new(5);
        dur.current = 1;
        dur.apply_death_loss(); // loss = ceil(0.5) = 1
        assert_eq!(dur.current, 0);
        assert!(dur.is_broken());
    }

    #[test]
    fn durability_repair_cost() {
        let mut dur = Durability::new(100);
        dur.current = 50; // 50 missing
        let cost = dur.repair(200); // ilvl 200
        // 50 * 0.5 * 200 = 5000 copper
        assert_eq!(cost, 5000);
        assert_eq!(dur.current, 100);
    }

    #[test]
    fn durability_repair_full_no_cost() {
        let mut dur = Durability::new(100);
        let cost = dur.repair(200);
        assert_eq!(cost, 0);
    }

    #[test]
    fn durability_damage_flat() {
        let mut dur = Durability::new(100);
        dur.damage(15);
        assert_eq!(dur.current, 85);
    }

    #[test]
    fn durability_damage_clamps_to_zero() {
        let mut dur = Durability::new(100);
        dur.damage(200);
        assert_eq!(dur.current, 0);
        assert!(dur.is_broken());
    }

    #[test]
    fn durability_zero_max_not_broken() {
        // Items with 0 max durability (consumables) are never "broken"
        let dur = Durability::new(0);
        assert!(!dur.is_broken());
    }
}
