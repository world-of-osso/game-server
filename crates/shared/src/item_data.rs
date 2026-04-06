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

    /// Stat budget multiplier relative to item level.
    ///
    /// Higher quality items get more stats per ilvl. These approximate
    /// WoW's per-quality scaling from AzerothCore `RandPropPoints.dbc`.
    pub fn stat_budget_multiplier(self) -> f32 {
        match self {
            Self::Poor => 0.5,
            Self::Common => 1.0,
            Self::Uncommon => 1.0,
            Self::Rare => 1.2,
            Self::Epic => 1.5,
            Self::Legendary => 1.8,
        }
    }

    /// Vendor sell price multiplier relative to item level.
    ///
    /// Quality increases vendor value. Base price is `ilvl * slot_factor`;
    /// this multiplier scales it by quality tier.
    pub fn vendor_price_multiplier(self) -> f32 {
        match self {
            Self::Poor => 0.1,
            Self::Common => 0.5,
            Self::Uncommon => 1.0,
            Self::Rare => 2.5,
            Self::Epic => 5.0,
            Self::Legendary => 10.0,
        }
    }

    /// Display name color (WoW convention).
    pub fn color_name(self) -> &'static str {
        match self {
            Self::Poor => "gray",
            Self::Common => "white",
            Self::Uncommon => "green",
            Self::Rare => "blue",
            Self::Epic => "purple",
            Self::Legendary => "orange",
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

/// When an item becomes soulbound to a character.
///
/// Ref: AzerothCore `ItemTemplate::Bonding`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BindingType {
    /// Never binds — freely tradeable.
    None,
    /// Bind on Pickup — soulbound the moment it enters inventory.
    BindOnPickup,
    /// Bind on Equip — soulbound when first equipped.
    BindOnEquip,
    /// Bind on Use — soulbound when first used (consumables, quest items).
    BindOnUse,
}

/// Runtime binding state of an item instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundState {
    /// Item is not yet bound (can be traded/sold on AH).
    Unbound,
    /// Item is soulbound to a specific character.
    Bound { character_id: u64 },
}

impl BoundState {
    /// Check if the item is bound to a specific character.
    pub fn is_bound_to(&self, character_id: u64) -> bool {
        matches!(self, Self::Bound { character_id: id } if *id == character_id)
    }

    /// Check if the item is tradeable (not bound).
    pub fn is_tradeable(&self) -> bool {
        matches!(self, Self::Unbound)
    }

    /// Bind the item to a character. Returns the new state.
    pub fn bind(character_id: u64) -> Self {
        Self::Bound { character_id }
    }
}

/// Determine the binding state after a pickup/equip/use event.
///
/// Returns the new `BoundState` given the item's binding type and current state.
pub fn resolve_binding(
    binding: BindingType,
    current: BoundState,
    event: BindingEvent,
    character_id: u64,
) -> BoundState {
    if !matches!(current, BoundState::Unbound) {
        return current; // already bound, no change
    }
    let should_bind = matches!(
        (binding, event),
        (BindingType::BindOnPickup, BindingEvent::Pickup)
            | (BindingType::BindOnEquip, BindingEvent::Equip)
            | (BindingType::BindOnUse, BindingEvent::Use)
    );
    if should_bind {
        BoundState::Bound { character_id }
    } else {
        current
    }
}

/// Events that can trigger binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingEvent {
    Pickup,
    Equip,
    Use,
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
    /// Binding type (BoP, BoE, BoU, or None).
    pub binding: BindingType,
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
    /// Average damage per hit: `(min + max) / 2`.
    pub fn average_damage(&self) -> f32 {
        (self.min_damage + self.max_damage) / 2.0
    }

    /// Damage per second: `average / speed`.
    pub fn dps(&self) -> f32 {
        if self.speed > 0.0 {
            self.average_damage() / self.speed
        } else {
            0.0
        }
    }

    /// Convert to the runtime `WeaponDamage` component used by the combat system.
    pub fn to_weapon_component(&self, subclass: WeaponSubclass) -> crate::components::WeaponDamage {
        crate::components::WeaponDamage {
            min_damage: self.min_damage,
            max_damage: self.max_damage,
            speed: self.speed,
            weapon_type: subclass.to_weapon_type(),
        }
    }
}

impl WeaponSubclass {
    /// Map weapon subclass to the combat system's `WeaponType` for normalized speed.
    pub fn to_weapon_type(self) -> crate::components::WeaponType {
        use crate::components::WeaponType;
        match self {
            Self::Dagger => WeaponType::Dagger,
            Self::Axe2H | Self::Mace2H | Self::Sword2H | Self::Polearm | Self::Staff => {
                WeaponType::TwoHand
            }
            _ => WeaponType::OneHand,
        }
    }

    /// Whether this is a two-handed weapon.
    pub fn is_two_handed(self) -> bool {
        matches!(
            self,
            Self::Axe2H | Self::Mace2H | Self::Sword2H | Self::Polearm | Self::Staff
        )
    }
}

/// Estimate total stat budget points for an item.
///
/// `ilvl * quality_multiplier` gives the total stat points available
/// to distribute across primary and secondary stats.
pub fn stat_budget(ilvl: u16, quality: ItemQuality) -> f32 {
    ilvl as f32 * quality.stat_budget_multiplier()
}

/// Estimate vendor sell price in copper for an item.
///
/// `ilvl * quality_multiplier * 100` (base rate 100 copper per ilvl at Uncommon).
pub fn estimated_vendor_price(ilvl: u16, quality: ItemQuality) -> u32 {
    (ilvl as f32 * quality.vendor_price_multiplier() * 100.0) as u32
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

    /// Try to add `count` of `item_id` by stacking onto existing slots first,
    /// then using empty slots. Returns the amount that could NOT be added.
    pub fn add_stacking(&mut self, item_id: u32, mut count: u16, max_stack: u16) -> u16 {
        // Stack onto existing partial stacks
        for slot in &mut self.slots {
            if count == 0 {
                break;
            }
            if slot.item_id == item_id && slot.count < max_stack {
                let space = max_stack - slot.count;
                let add = count.min(space);
                slot.count += add;
                count -= add;
            }
        }
        // Use empty slots for remaining
        for slot in &mut self.slots {
            if count == 0 {
                break;
            }
            if slot.is_empty() {
                let add = count.min(max_stack);
                *slot = InvSlot { item_id, count: add };
                count -= add;
            }
        }
        count // leftover that didn't fit
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
    /// Try to add an item (non-stacking, max_stack=1).
    /// Returns `true` if the item was added.
    pub fn add_item(&mut self, item_id: u32, count: u16) -> bool {
        self.add_item_stacking(item_id, count, 1) == 0
    }

    /// Add items with stacking support. Fills existing partial stacks first,
    /// then uses empty slots. Returns the count that could NOT be added.
    pub fn add_item_stacking(&mut self, item_id: u32, count: u16, max_stack: u16) -> u16 {
        let remaining = self.backpack.add_stacking(item_id, count, max_stack);
        if remaining == 0 {
            return 0;
        }
        let mut left = remaining;
        for bag in &mut self.bags {
            left = bag.add_stacking(item_id, left, max_stack);
            if left == 0 {
                break;
            }
        }
        left
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
            binding: BindingType::BindOnPickup,
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
            binding: BindingType::BindOnPickup,
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
    fn weapon_average_damage() {
        let wpn = WeaponDamageData {
            min_damage: 100.0,
            max_damage: 200.0,
            speed: 2.0,
        };
        assert_eq!(wpn.average_damage(), 150.0);
    }

    #[test]
    fn weapon_to_component_dagger() {
        let wpn = WeaponDamageData {
            min_damage: 80.0,
            max_damage: 120.0,
            speed: 1.8,
        };
        let comp = wpn.to_weapon_component(WeaponSubclass::Dagger);
        assert_eq!(comp.min_damage, 80.0);
        assert_eq!(comp.speed, 1.8);
        assert_eq!(comp.weapon_type, crate::components::WeaponType::Dagger);
    }

    #[test]
    fn weapon_to_component_2h() {
        let wpn = WeaponDamageData {
            min_damage: 300.0,
            max_damage: 500.0,
            speed: 3.6,
        };
        let comp = wpn.to_weapon_component(WeaponSubclass::Sword2H);
        assert_eq!(comp.weapon_type, crate::components::WeaponType::TwoHand);
    }

    #[test]
    fn weapon_subclass_two_handed() {
        assert!(WeaponSubclass::Axe2H.is_two_handed());
        assert!(WeaponSubclass::Polearm.is_two_handed());
        assert!(WeaponSubclass::Staff.is_two_handed());
        assert!(!WeaponSubclass::Sword1H.is_two_handed());
        assert!(!WeaponSubclass::Dagger.is_two_handed());
    }

    #[test]
    fn weapon_subclass_to_weapon_type() {
        use crate::components::WeaponType;
        assert_eq!(WeaponSubclass::Dagger.to_weapon_type(), WeaponType::Dagger);
        assert_eq!(WeaponSubclass::Sword1H.to_weapon_type(), WeaponType::OneHand);
        assert_eq!(WeaponSubclass::Mace2H.to_weapon_type(), WeaponType::TwoHand);
        assert_eq!(WeaponSubclass::Bow.to_weapon_type(), WeaponType::OneHand);
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
            binding: BindingType::BindOnPickup,
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

    // --- Quality tier tests ---

    #[test]
    fn quality_stat_budget_scales_with_tier() {
        let uncommon = stat_budget(200, ItemQuality::Uncommon);
        let rare = stat_budget(200, ItemQuality::Rare);
        let epic = stat_budget(200, ItemQuality::Epic);
        assert!(rare > uncommon);
        assert!(epic > rare);
        assert!((uncommon - 200.0).abs() < 0.01); // 1.0x
        assert!((rare - 240.0).abs() < 0.01);     // 1.2x
        assert!((epic - 300.0).abs() < 0.01);     // 1.5x
    }

    #[test]
    fn quality_vendor_price_scales() {
        let common = estimated_vendor_price(100, ItemQuality::Common);
        let rare = estimated_vendor_price(100, ItemQuality::Rare);
        let epic = estimated_vendor_price(100, ItemQuality::Epic);
        assert!(rare > common);
        assert!(epic > rare);
        assert_eq!(common, 5000);   // 100 * 0.5 * 100
        assert_eq!(rare, 25000);    // 100 * 2.5 * 100
        assert_eq!(epic, 50000);    // 100 * 5.0 * 100
    }

    #[test]
    fn quality_poor_has_lowest_budget() {
        let poor = stat_budget(200, ItemQuality::Poor);
        assert!((poor - 100.0).abs() < 0.01); // 0.5x
    }

    #[test]
    fn quality_legendary_highest_budget() {
        let legendary = stat_budget(200, ItemQuality::Legendary);
        assert!((legendary - 360.0).abs() < 0.01); // 1.8x
    }

    #[test]
    fn quality_color_names() {
        assert_eq!(ItemQuality::Poor.color_name(), "gray");
        assert_eq!(ItemQuality::Rare.color_name(), "blue");
        assert_eq!(ItemQuality::Epic.color_name(), "purple");
        assert_eq!(ItemQuality::Legendary.color_name(), "orange");
    }

    // --- Stacking tests ---

    #[test]
    fn stacking_fills_existing_stack() {
        let mut bag = Bag::new(4);
        bag.slots[0] = InvSlot { item_id: 100, count: 15 };
        let leftover = bag.add_stacking(100, 5, 20);
        assert_eq!(leftover, 0);
        assert_eq!(bag.slots[0].count, 20);
        assert!(bag.slots[1].is_empty()); // didn't use a new slot
    }

    #[test]
    fn stacking_overflow_to_new_slot() {
        let mut bag = Bag::new(4);
        bag.slots[0] = InvSlot { item_id: 100, count: 18 };
        let leftover = bag.add_stacking(100, 5, 20);
        assert_eq!(leftover, 0);
        assert_eq!(bag.slots[0].count, 20); // filled to max
        assert_eq!(bag.slots[1].count, 3);  // overflow
    }

    #[test]
    fn stacking_no_room_returns_leftover() {
        let mut bag = Bag::new(1);
        bag.slots[0] = InvSlot { item_id: 100, count: 20 };
        let leftover = bag.add_stacking(100, 5, 20);
        assert_eq!(leftover, 5);
    }

    #[test]
    fn stacking_multiple_partial_stacks() {
        let mut bag = Bag::new(4);
        bag.slots[0] = InvSlot { item_id: 100, count: 15 };
        bag.slots[1] = InvSlot { item_id: 100, count: 10 };
        let leftover = bag.add_stacking(100, 12, 20);
        assert_eq!(leftover, 0);
        assert_eq!(bag.slots[0].count, 20); // +5
        assert_eq!(bag.slots[1].count, 17); // +7
    }

    #[test]
    fn inventory_add_stacking_across_bags() {
        let mut inv = Inventory::default();
        // Fill backpack with 16 slots of partial stacks
        for slot in &mut inv.backpack.slots {
            *slot = InvSlot { item_id: 100, count: 18 };
        }
        inv.bags[0] = Bag::new(4);

        // 40 potions: 16 slots * 2 space each = 32 in backpack, 8 overflow to bag
        let leftover = inv.add_item_stacking(100, 40, 20);
        assert_eq!(leftover, 0);
        // Backpack slots should all be 20
        assert!(inv.backpack.slots.iter().all(|s| s.count == 20));
        // Bag should have the overflow
        assert_eq!(inv.bags[0].slots[0].count, 8);
    }

    #[test]
    fn inventory_add_stacking_returns_leftover() {
        let mut inv = Inventory::default();
        // Fill all backpack slots to max
        for slot in &mut inv.backpack.slots {
            *slot = InvSlot { item_id: 100, count: 20 };
        }
        // No bags
        let leftover = inv.add_item_stacking(100, 5, 20);
        assert_eq!(leftover, 5);
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

    // --- Binding tests ---

    #[test]
    fn bop_binds_on_pickup() {
        let state = resolve_binding(
            BindingType::BindOnPickup,
            BoundState::Unbound,
            BindingEvent::Pickup,
            42,
        );
        assert_eq!(state, BoundState::Bound { character_id: 42 });
    }

    #[test]
    fn boe_does_not_bind_on_pickup() {
        let state = resolve_binding(
            BindingType::BindOnEquip,
            BoundState::Unbound,
            BindingEvent::Pickup,
            42,
        );
        assert_eq!(state, BoundState::Unbound);
    }

    #[test]
    fn boe_binds_on_equip() {
        let state = resolve_binding(
            BindingType::BindOnEquip,
            BoundState::Unbound,
            BindingEvent::Equip,
            42,
        );
        assert_eq!(state, BoundState::Bound { character_id: 42 });
    }

    #[test]
    fn bou_binds_on_use() {
        let state = resolve_binding(
            BindingType::BindOnUse,
            BoundState::Unbound,
            BindingEvent::Use,
            42,
        );
        assert_eq!(state, BoundState::Bound { character_id: 42 });
    }

    #[test]
    fn no_binding_never_binds() {
        let state = resolve_binding(
            BindingType::None,
            BoundState::Unbound,
            BindingEvent::Equip,
            42,
        );
        assert_eq!(state, BoundState::Unbound);
    }

    #[test]
    fn already_bound_stays_bound() {
        let already = BoundState::Bound { character_id: 10 };
        let state = resolve_binding(
            BindingType::BindOnEquip,
            already,
            BindingEvent::Equip,
            42,
        );
        assert_eq!(state, BoundState::Bound { character_id: 10 });
    }

    #[test]
    fn bound_state_helpers() {
        let unbound = BoundState::Unbound;
        assert!(unbound.is_tradeable());
        assert!(!unbound.is_bound_to(1));

        let bound = BoundState::bind(42);
        assert!(!bound.is_tradeable());
        assert!(bound.is_bound_to(42));
        assert!(!bound.is_bound_to(99));
    }
}
