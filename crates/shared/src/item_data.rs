//! Static item definitions — equipment, weapons, consumables.
//!
//! `ItemData` is the immutable definition loaded from the item database.
//! Runtime state (durability, enchants, soulbound status) lives on components.
//!
//! Ref: AzerothCore `ItemTemplate`, WoW `Item.dbc`.

use serde::{Deserialize, Serialize};

use crate::components::{CombatRatings, EquipmentVisualSlot, UnitStats, weapon_damage_per_second};

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
        weapon_damage_per_second(self.average_damage(), self.speed)
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

// --- Set bonuses ---

use crate::components::AuraEffect;

/// A set bonus that activates when N pieces of a set are equipped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetBonus {
    /// Number of pieces required (e.g. 2 or 4).
    pub required_pieces: u8,
    /// Passive aura effects granted by this bonus.
    pub effects: Vec<AuraEffect>,
}

/// Definition of an item set (e.g. Tier 10 Warrior).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemSetDef {
    /// Unique set ID.
    pub id: u32,
    /// Display name.
    pub name: String,
    /// Item IDs that belong to this set.
    pub item_ids: Vec<u32>,
    /// Bonuses at various piece thresholds.
    pub bonuses: Vec<SetBonus>,
}

/// Determine which set bonuses are active given the equipped item IDs.
///
/// Returns the aura effects from all set bonuses whose piece threshold is met.
pub fn active_set_bonuses(sets: &[ItemSetDef], equipped_item_ids: &[u32]) -> Vec<AuraEffect> {
    sets.iter()
        .flat_map(|set| {
            let count = set
                .item_ids
                .iter()
                .filter(|id| equipped_item_ids.contains(id))
                .count() as u8;
            set.bonuses
                .iter()
                .filter(move |b| count >= b.required_pieces)
                .flat_map(|b| b.effects.iter().copied())
        })
        .collect()
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
                *slot = InvSlot {
                    item_id,
                    count: add,
                };
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
        if let Some(slot) = self
            .backpack
            .slots
            .iter_mut()
            .find(|s| s.item_id == item_id)
        {
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
        self.equipped[slot_index] = InvSlot { item_id, count: 1 };
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
        let bag_free: usize = self
            .bags
            .iter()
            .map(|b| b.slots.iter().filter(|s| s.is_empty()).count())
            .sum();
        backpack_free + bag_free
    }
}

#[cfg(test)]
#[path = "item_data_tests.rs"]
mod tests;
