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
    assert_eq!(
        WeaponSubclass::Sword1H.to_weapon_type(),
        WeaponType::OneHand
    );
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
    bag.slots[0] = InvSlot {
        item_id: 1,
        count: 1,
    };
    bag.slots[3] = InvSlot {
        item_id: 2,
        count: 5,
    };
    assert_eq!(bag.used_slots(), 2);
}

// --- Set bonus tests ---

fn sample_set() -> ItemSetDef {
    ItemSetDef {
        id: 1,
        name: "Sanctified Lightsworn".into(),
        item_ids: vec![51231, 51232, 51233, 51234, 51235],
        bonuses: vec![
            SetBonus {
                required_pieces: 2,
                effects: vec![AuraEffect::ModCritChance { percent: 0.05 }],
            },
            SetBonus {
                required_pieces: 4,
                effects: vec![AuraEffect::ModDamageDone { percent: 0.10 }],
            },
        ],
    }
}

#[test]
fn set_bonus_none_with_zero_pieces() {
    let sets = vec![sample_set()];
    let effects = active_set_bonuses(&sets, &[99999]);
    assert!(effects.is_empty());
}

#[test]
fn set_bonus_none_with_one_piece() {
    let sets = vec![sample_set()];
    let effects = active_set_bonuses(&sets, &[51231]);
    assert!(effects.is_empty());
}

#[test]
fn set_bonus_2pc_active() {
    let sets = vec![sample_set()];
    let effects = active_set_bonuses(&sets, &[51231, 51232]);
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0], AuraEffect::ModCritChance { percent: 0.05 });
}

#[test]
fn set_bonus_4pc_includes_2pc() {
    let sets = vec![sample_set()];
    let effects = active_set_bonuses(&sets, &[51231, 51232, 51233, 51234]);
    assert_eq!(effects.len(), 2); // both 2pc and 4pc
    assert!(effects.contains(&AuraEffect::ModCritChance { percent: 0.05 }));
    assert!(effects.contains(&AuraEffect::ModDamageDone { percent: 0.10 }));
}

#[test]
fn set_bonus_5pc_still_gets_both() {
    let sets = vec![sample_set()];
    let effects = active_set_bonuses(&sets, &[51231, 51232, 51233, 51234, 51235]);
    assert_eq!(effects.len(), 2);
}

#[test]
fn set_bonus_no_sets_defined() {
    let effects = active_set_bonuses(&[], &[51231, 51232]);
    assert!(effects.is_empty());
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
    assert!((rare - 240.0).abs() < 0.01); // 1.2x
    assert!((epic - 300.0).abs() < 0.01); // 1.5x
}

#[test]
fn quality_vendor_price_scales() {
    let common = estimated_vendor_price(100, ItemQuality::Common);
    let rare = estimated_vendor_price(100, ItemQuality::Rare);
    let epic = estimated_vendor_price(100, ItemQuality::Epic);
    assert!(rare > common);
    assert!(epic > rare);
    assert_eq!(common, 5000); // 100 * 0.5 * 100
    assert_eq!(rare, 25000); // 100 * 2.5 * 100
    assert_eq!(epic, 50000); // 100 * 5.0 * 100
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
    bag.slots[0] = InvSlot {
        item_id: 100,
        count: 15,
    };
    let leftover = bag.add_stacking(100, 5, 20);
    assert_eq!(leftover, 0);
    assert_eq!(bag.slots[0].count, 20);
    assert!(bag.slots[1].is_empty()); // didn't use a new slot
}

#[test]
fn stacking_overflow_to_new_slot() {
    let mut bag = Bag::new(4);
    bag.slots[0] = InvSlot {
        item_id: 100,
        count: 18,
    };
    let leftover = bag.add_stacking(100, 5, 20);
    assert_eq!(leftover, 0);
    assert_eq!(bag.slots[0].count, 20); // filled to max
    assert_eq!(bag.slots[1].count, 3); // overflow
}

#[test]
fn stacking_no_room_returns_leftover() {
    let mut bag = Bag::new(1);
    bag.slots[0] = InvSlot {
        item_id: 100,
        count: 20,
    };
    let leftover = bag.add_stacking(100, 5, 20);
    assert_eq!(leftover, 5);
}

#[test]
fn stacking_multiple_partial_stacks() {
    let mut bag = Bag::new(4);
    bag.slots[0] = InvSlot {
        item_id: 100,
        count: 15,
    };
    bag.slots[1] = InvSlot {
        item_id: 100,
        count: 10,
    };
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
        *slot = InvSlot {
            item_id: 100,
            count: 18,
        };
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
        *slot = InvSlot {
            item_id: 100,
            count: 20,
        };
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
    let state = resolve_binding(BindingType::BindOnEquip, already, BindingEvent::Equip, 42);
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
