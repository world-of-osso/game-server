use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::components::{EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry};
use crate::item_data::{ItemData, ItemSubclass};

/// Account-wide collection of learned item appearances.
///
/// Each appearance is identified by its `display_info_id` (u32). Once an item
/// is equipped or bound, its appearance is added to the collection and can be
/// used for transmogrification on any character on the account.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AppearanceCollection {
    appearances: BTreeSet<u32>,
}

impl AppearanceCollection {
    /// Learn a new appearance. Returns `true` if it was newly added.
    pub fn learn(&mut self, display_info_id: u32) -> bool {
        self.appearances.insert(display_info_id)
    }

    /// Whether this appearance has been learned.
    pub fn has(&self, display_info_id: u32) -> bool {
        self.appearances.contains(&display_info_id)
    }

    /// Number of learned appearances.
    pub fn count(&self) -> usize {
        self.appearances.len()
    }

    /// Iterator over all learned appearance IDs (sorted).
    pub fn all(&self) -> impl Iterator<Item = u32> + '_ {
        self.appearances.iter().copied()
    }

    /// Learn multiple appearances at once. Returns the count of newly added.
    pub fn learn_many(&mut self, ids: impl IntoIterator<Item = u32>) -> usize {
        ids.into_iter().filter(|&id| self.learn(id)).count()
    }

    /// Remove an appearance (for GM commands). Returns `true` if it was present.
    pub fn remove(&mut self, display_info_id: u32) -> bool {
        self.appearances.remove(&display_info_id)
    }
}

// -- Transmog validation --

/// Reason a transmogrification was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransmogError {
    /// Equipped item has no visual slot (ring, trinket, neck).
    NoVisualSlot,
    /// Appearance source has no visual slot.
    SourceNoVisualSlot,
    /// Item classes don't match (e.g. armor → weapon).
    ClassMismatch,
    /// Armor subclasses don't match (e.g. plate → cloth).
    ArmorTypeMismatch,
    /// Weapon subclasses don't match (e.g. sword → axe).
    WeaponTypeMismatch,
    /// Appearance not learned in the collection.
    AppearanceNotLearned,
}

/// Validate whether a transmog appearance can be applied to an equipped item.
///
/// Rules (matching WoW):
/// - Both items must have a visual slot
/// - Item class must match (armor↔armor, weapon↔weapon)
/// - Armor subclass must match (plate↔plate, leather↔leather, etc.)
/// - Weapon subclass must match (sword1h↔sword1h, axe2h↔axe2h, etc.)
/// - Appearance must be learned in the account's collection
pub fn validate_transmog(
    equipped: &ItemData,
    source: &ItemData,
    collection: &AppearanceCollection,
    source_display_id: u32,
) -> Result<(), TransmogError> {
    if equipped.slot.and_then(|s| s.to_visual_slot()).is_none() {
        return Err(TransmogError::NoVisualSlot);
    }
    if source.slot.and_then(|s| s.to_visual_slot()).is_none() {
        return Err(TransmogError::SourceNoVisualSlot);
    }
    validate_class_match(equipped, source)?;
    if !collection.has(source_display_id) {
        return Err(TransmogError::AppearanceNotLearned);
    }
    Ok(())
}

fn validate_class_match(equipped: &ItemData, source: &ItemData) -> Result<(), TransmogError> {
    match (&equipped.subclass, &source.subclass) {
        (ItemSubclass::Armor(a), ItemSubclass::Armor(b)) => {
            if a != b {
                return Err(TransmogError::ArmorTypeMismatch);
            }
        }
        (ItemSubclass::Weapon(a), ItemSubclass::Weapon(b)) => {
            if a != b {
                return Err(TransmogError::WeaponTypeMismatch);
            }
        }
        _ => {
            if equipped.class != source.class {
                return Err(TransmogError::ClassMismatch);
            }
            if equipped.subclass != source.subclass {
                return Err(TransmogError::ClassMismatch);
            }
        }
    }
    Ok(())
}

// -- Transmog cost --

/// Minimum transmog cost in copper (1 silver).
const MIN_TRANSMOG_COST: u32 = 100;

/// Transmog cost in copper, based on the source item's vendor sell price.
///
/// WoW charges the vendor sell price of the appearance item, with a floor
/// of 1 silver to prevent free transmogs on worthless items.
pub fn transmog_cost(source: &ItemData) -> u32 {
    source.sell_price.max(MIN_TRANSMOG_COST)
}

// -- Apply / remove transmog --

/// Apply a transmog appearance to an equipment slot.
///
/// Sets the `display_info_id` on the matching slot entry, creating the entry
/// if it doesn't exist. Does not validate rules or deduct gold — call
/// `validate_transmog` and `transmog_cost` first.
pub fn apply_transmog(
    appearance: &mut EquipmentAppearance,
    slot: EquipmentVisualSlot,
    display_info_id: u32,
) {
    let entry = find_or_create_entry(appearance, slot);
    entry.display_info_id = Some(display_info_id);
}

/// Remove a transmog from an equipment slot, reverting to the item's base look.
///
/// Clears the `display_info_id` on the matching slot entry. No-op if the slot
/// has no entry or no transmog applied.
pub fn remove_transmog(appearance: &mut EquipmentAppearance, slot: EquipmentVisualSlot) {
    if let Some(entry) = appearance.entries.iter_mut().find(|e| e.slot == slot) {
        entry.display_info_id = None;
    }
}

fn find_or_create_entry(
    appearance: &mut EquipmentAppearance,
    slot: EquipmentVisualSlot,
) -> &mut EquippedAppearanceEntry {
    let exists = appearance.entries.iter().any(|e| e.slot == slot);
    if !exists {
        appearance.entries.push(EquippedAppearanceEntry {
            slot,
            item_id: None,
            display_info_id: None,
            inventory_type: 0,
            hidden: false,
        });
    }
    appearance
        .entries
        .iter_mut()
        .find(|e| e.slot == slot)
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item_data::*;

    #[test]
    fn empty_collection() {
        let collection = AppearanceCollection::default();
        assert_eq!(collection.count(), 0);
        assert!(!collection.has(100));
        assert_eq!(collection.all().count(), 0);
    }

    #[test]
    fn learn_new_appearance() {
        let mut collection = AppearanceCollection::default();
        assert!(collection.learn(42));
        assert!(collection.has(42));
        assert_eq!(collection.count(), 1);
    }

    #[test]
    fn learn_duplicate_returns_false() {
        let mut collection = AppearanceCollection::default();
        assert!(collection.learn(42));
        assert!(!collection.learn(42));
        assert_eq!(collection.count(), 1);
    }

    #[test]
    fn learn_many_returns_new_count() {
        let mut collection = AppearanceCollection::default();
        collection.learn(10);
        let added = collection.learn_many([10, 20, 30]);
        assert_eq!(added, 2);
        assert_eq!(collection.count(), 3);
    }

    #[test]
    fn remove_existing() {
        let mut collection = AppearanceCollection::default();
        collection.learn(42);
        assert!(collection.remove(42));
        assert!(!collection.has(42));
        assert_eq!(collection.count(), 0);
    }

    #[test]
    fn remove_nonexistent() {
        let mut collection = AppearanceCollection::default();
        assert!(!collection.remove(99));
    }

    #[test]
    fn all_returns_sorted() {
        let mut collection = AppearanceCollection::default();
        collection.learn(30);
        collection.learn(10);
        collection.learn(20);
        let ids: Vec<u32> = collection.all().collect();
        assert_eq!(ids, vec![10, 20, 30]);
    }

    #[test]
    fn serialization_round_trip() {
        let mut collection = AppearanceCollection::default();
        collection.learn(100);
        collection.learn(200);
        collection.learn(300);

        let json = serde_json::to_string(&collection).unwrap();
        let restored: AppearanceCollection = serde_json::from_str(&json).unwrap();
        assert_eq!(collection, restored);
    }

    // -- Transmog rule tests --

    fn make_item(slot: Option<EquipSlot>, class: ItemClass, subclass: ItemSubclass) -> ItemData {
        ItemData {
            id: 1,
            name: "Test".into(),
            slot,
            ilvl: 100,
            quality: ItemQuality::Rare,
            stats: ItemStats::default(),
            required_level: 1,
            class,
            subclass,
            weapon_damage: None,
            armor: 0,
            buy_price: 0,
            sell_price: 100,
            max_stack: 1,
            binding: BindingType::BindOnPickup,
        }
    }

    fn plate_chest() -> ItemData {
        make_item(
            Some(EquipSlot::Chest),
            ItemClass::Armor,
            ItemSubclass::Armor(ArmorSubclass::Plate),
        )
    }

    fn leather_chest() -> ItemData {
        make_item(
            Some(EquipSlot::Chest),
            ItemClass::Armor,
            ItemSubclass::Armor(ArmorSubclass::Leather),
        )
    }

    fn sword_1h() -> ItemData {
        make_item(
            Some(EquipSlot::MainHand),
            ItemClass::Weapon,
            ItemSubclass::Weapon(WeaponSubclass::Sword1H),
        )
    }

    fn axe_1h() -> ItemData {
        make_item(
            Some(EquipSlot::MainHand),
            ItemClass::Weapon,
            ItemSubclass::Weapon(WeaponSubclass::Axe1H),
        )
    }

    fn learned(id: u32) -> AppearanceCollection {
        let mut c = AppearanceCollection::default();
        c.learn(id);
        c
    }

    #[test]
    fn transmog_same_armor_type_ok() {
        let equipped = plate_chest();
        let source = plate_chest();
        let collection = learned(500);
        assert_eq!(
            validate_transmog(&equipped, &source, &collection, 500),
            Ok(())
        );
    }

    #[test]
    fn transmog_armor_type_mismatch() {
        let equipped = plate_chest();
        let source = leather_chest();
        let collection = learned(500);
        assert_eq!(
            validate_transmog(&equipped, &source, &collection, 500),
            Err(TransmogError::ArmorTypeMismatch)
        );
    }

    #[test]
    fn transmog_same_weapon_type_ok() {
        let equipped = sword_1h();
        let source = sword_1h();
        let collection = learned(600);
        assert_eq!(
            validate_transmog(&equipped, &source, &collection, 600),
            Ok(())
        );
    }

    #[test]
    fn transmog_weapon_type_mismatch() {
        let equipped = sword_1h();
        let source = axe_1h();
        let collection = learned(600);
        assert_eq!(
            validate_transmog(&equipped, &source, &collection, 600),
            Err(TransmogError::WeaponTypeMismatch)
        );
    }

    #[test]
    fn transmog_class_mismatch_armor_to_weapon() {
        let equipped = plate_chest();
        let source = sword_1h();
        let collection = learned(600);
        assert_eq!(
            validate_transmog(&equipped, &source, &collection, 600),
            Err(TransmogError::ClassMismatch)
        );
    }

    #[test]
    fn transmog_no_visual_slot_ring() {
        let ring = make_item(
            Some(EquipSlot::Finger),
            ItemClass::Armor,
            ItemSubclass::None,
        );
        let source = plate_chest();
        let collection = learned(500);
        assert_eq!(
            validate_transmog(&ring, &source, &collection, 500),
            Err(TransmogError::NoVisualSlot)
        );
    }

    #[test]
    fn transmog_source_no_visual_slot() {
        let equipped = plate_chest();
        let trinket = make_item(
            Some(EquipSlot::Trinket),
            ItemClass::Armor,
            ItemSubclass::None,
        );
        let collection = learned(500);
        assert_eq!(
            validate_transmog(&equipped, &trinket, &collection, 500),
            Err(TransmogError::SourceNoVisualSlot)
        );
    }

    #[test]
    fn transmog_appearance_not_learned() {
        let equipped = plate_chest();
        let source = plate_chest();
        let collection = AppearanceCollection::default();
        assert_eq!(
            validate_transmog(&equipped, &source, &collection, 500),
            Err(TransmogError::AppearanceNotLearned)
        );
    }

    #[test]
    fn transmog_no_equip_slot() {
        let no_slot = make_item(None, ItemClass::Consumable, ItemSubclass::None);
        let source = plate_chest();
        let collection = learned(500);
        assert_eq!(
            validate_transmog(&no_slot, &source, &collection, 500),
            Err(TransmogError::NoVisualSlot)
        );
    }

    // -- Cost tests --

    #[test]
    fn cost_uses_sell_price() {
        let mut item = plate_chest();
        item.sell_price = 5000;
        assert_eq!(transmog_cost(&item), 5000);
    }

    #[test]
    fn cost_floors_at_one_silver() {
        let mut item = plate_chest();
        item.sell_price = 0;
        assert_eq!(transmog_cost(&item), 100);
    }

    #[test]
    fn cost_floor_at_low_price() {
        let mut item = plate_chest();
        item.sell_price = 50;
        assert_eq!(transmog_cost(&item), 100);
    }

    #[test]
    fn cost_exactly_one_silver() {
        let mut item = plate_chest();
        item.sell_price = 100;
        assert_eq!(transmog_cost(&item), 100);
    }

    // -- Apply / remove tests --

    use crate::components::{EquipmentAppearance, EquipmentVisualSlot};

    #[test]
    fn apply_transmog_sets_display_id() {
        let mut appearance = EquipmentAppearance::default();
        apply_transmog(&mut appearance, EquipmentVisualSlot::Chest, 999);

        let entry = appearance
            .entries
            .iter()
            .find(|e| e.slot == EquipmentVisualSlot::Chest)
            .unwrap();
        assert_eq!(entry.display_info_id, Some(999));
    }

    #[test]
    fn apply_transmog_overwrites_existing() {
        let mut appearance = EquipmentAppearance::default();
        apply_transmog(&mut appearance, EquipmentVisualSlot::Chest, 100);
        apply_transmog(&mut appearance, EquipmentVisualSlot::Chest, 200);

        let entry = appearance
            .entries
            .iter()
            .find(|e| e.slot == EquipmentVisualSlot::Chest)
            .unwrap();
        assert_eq!(entry.display_info_id, Some(200));
        // Should not duplicate the entry
        let chest_count = appearance
            .entries
            .iter()
            .filter(|e| e.slot == EquipmentVisualSlot::Chest)
            .count();
        assert_eq!(chest_count, 1);
    }

    #[test]
    fn apply_transmog_different_slots() {
        let mut appearance = EquipmentAppearance::default();
        apply_transmog(&mut appearance, EquipmentVisualSlot::Chest, 100);
        apply_transmog(&mut appearance, EquipmentVisualSlot::Legs, 200);
        assert_eq!(appearance.entries.len(), 2);
    }

    #[test]
    fn remove_transmog_clears_display_id() {
        let mut appearance = EquipmentAppearance::default();
        apply_transmog(&mut appearance, EquipmentVisualSlot::Chest, 999);
        remove_transmog(&mut appearance, EquipmentVisualSlot::Chest);

        let entry = appearance
            .entries
            .iter()
            .find(|e| e.slot == EquipmentVisualSlot::Chest)
            .unwrap();
        assert_eq!(entry.display_info_id, None);
    }

    #[test]
    fn remove_transmog_no_entry_is_noop() {
        let mut appearance = EquipmentAppearance::default();
        remove_transmog(&mut appearance, EquipmentVisualSlot::Head);
        assert!(appearance.entries.is_empty());
    }

    #[test]
    fn remove_transmog_only_affects_target_slot() {
        let mut appearance = EquipmentAppearance::default();
        apply_transmog(&mut appearance, EquipmentVisualSlot::Chest, 100);
        apply_transmog(&mut appearance, EquipmentVisualSlot::Legs, 200);
        remove_transmog(&mut appearance, EquipmentVisualSlot::Chest);

        let chest = appearance
            .entries
            .iter()
            .find(|e| e.slot == EquipmentVisualSlot::Chest)
            .unwrap();
        assert_eq!(chest.display_info_id, None);

        let legs = appearance
            .entries
            .iter()
            .find(|e| e.slot == EquipmentVisualSlot::Legs)
            .unwrap();
        assert_eq!(legs.display_info_id, Some(200));
    }

    // -- Integration tests --

    #[test]
    fn full_transmog_workflow() {
        // 1. Learn an appearance
        let mut collection = AppearanceCollection::default();
        assert!(collection.learn(500));

        // 2. Validate transmog (plate chest → plate chest appearance)
        let equipped = plate_chest();
        let source = plate_chest();
        assert_eq!(
            validate_transmog(&equipped, &source, &collection, 500),
            Ok(())
        );

        // 3. Calculate cost
        let cost = transmog_cost(&source);
        assert!(cost >= 100);

        // 4. Apply transmog
        let mut appearance = EquipmentAppearance::default();
        apply_transmog(&mut appearance, EquipmentVisualSlot::Chest, 500);
        let entry = appearance
            .entries
            .iter()
            .find(|e| e.slot == EquipmentVisualSlot::Chest)
            .unwrap();
        assert_eq!(entry.display_info_id, Some(500));

        // 5. Remove transmog
        remove_transmog(&mut appearance, EquipmentVisualSlot::Chest);
        let entry = appearance
            .entries
            .iter()
            .find(|e| e.slot == EquipmentVisualSlot::Chest)
            .unwrap();
        assert_eq!(entry.display_info_id, None);
    }

    #[test]
    fn validate_rejects_then_learn_allows() {
        let equipped = plate_chest();
        let source = plate_chest();
        let mut collection = AppearanceCollection::default();

        // Not learned yet — rejected
        assert_eq!(
            validate_transmog(&equipped, &source, &collection, 500),
            Err(TransmogError::AppearanceNotLearned)
        );

        // Learn it — now accepted
        collection.learn(500);
        assert_eq!(
            validate_transmog(&equipped, &source, &collection, 500),
            Ok(())
        );
    }

    #[test]
    fn all_armor_subclasses_match_themselves() {
        let collection = learned(500);
        let subclasses = [
            ArmorSubclass::Cloth,
            ArmorSubclass::Leather,
            ArmorSubclass::Mail,
            ArmorSubclass::Plate,
            ArmorSubclass::Shield,
        ];
        for sub in subclasses {
            let item = make_item(
                Some(EquipSlot::Chest),
                ItemClass::Armor,
                ItemSubclass::Armor(sub),
            );
            assert_eq!(
                validate_transmog(&item, &item, &collection, 500),
                Ok(()),
                "{sub:?} should match itself"
            );
        }
    }

    #[test]
    fn all_armor_subclasses_reject_cross_type() {
        let collection = learned(500);
        let plate = make_item(
            Some(EquipSlot::Chest),
            ItemClass::Armor,
            ItemSubclass::Armor(ArmorSubclass::Plate),
        );
        let others = [
            ArmorSubclass::Cloth,
            ArmorSubclass::Leather,
            ArmorSubclass::Mail,
            ArmorSubclass::Shield,
        ];
        for sub in others {
            let other = make_item(
                Some(EquipSlot::Chest),
                ItemClass::Armor,
                ItemSubclass::Armor(sub),
            );
            assert_eq!(
                validate_transmog(&plate, &other, &collection, 500),
                Err(TransmogError::ArmorTypeMismatch),
                "plate should not mog {sub:?}"
            );
        }
    }

    #[test]
    fn weapon_2h_types_match() {
        let collection = learned(700);
        let sword2h = make_item(
            Some(EquipSlot::TwoHand),
            ItemClass::Weapon,
            ItemSubclass::Weapon(WeaponSubclass::Sword2H),
        );
        assert_eq!(
            validate_transmog(&sword2h, &sword2h, &collection, 700),
            Ok(())
        );
    }

    #[test]
    fn weapon_2h_cross_type_rejected() {
        let collection = learned(700);
        let sword2h = make_item(
            Some(EquipSlot::TwoHand),
            ItemClass::Weapon,
            ItemSubclass::Weapon(WeaponSubclass::Sword2H),
        );
        let axe2h = make_item(
            Some(EquipSlot::TwoHand),
            ItemClass::Weapon,
            ItemSubclass::Weapon(WeaponSubclass::Axe2H),
        );
        assert_eq!(
            validate_transmog(&sword2h, &axe2h, &collection, 700),
            Err(TransmogError::WeaponTypeMismatch)
        );
    }

    #[test]
    fn multi_slot_transmog_independent() {
        let mut appearance = EquipmentAppearance::default();

        // Apply to multiple slots
        apply_transmog(&mut appearance, EquipmentVisualSlot::Head, 100);
        apply_transmog(&mut appearance, EquipmentVisualSlot::Chest, 200);
        apply_transmog(&mut appearance, EquipmentVisualSlot::Legs, 300);
        assert_eq!(appearance.entries.len(), 3);

        // Remove one — others untouched
        remove_transmog(&mut appearance, EquipmentVisualSlot::Chest);
        assert_eq!(
            appearance
                .entries
                .iter()
                .find(|e| e.slot == EquipmentVisualSlot::Head)
                .unwrap()
                .display_info_id,
            Some(100)
        );
        assert_eq!(
            appearance
                .entries
                .iter()
                .find(|e| e.slot == EquipmentVisualSlot::Chest)
                .unwrap()
                .display_info_id,
            None
        );
        assert_eq!(
            appearance
                .entries
                .iter()
                .find(|e| e.slot == EquipmentVisualSlot::Legs)
                .unwrap()
                .display_info_id,
            Some(300)
        );
    }
}
