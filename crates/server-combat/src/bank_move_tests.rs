//! Character bank moves through `SwapItem` / `SplitItem`: TrinityCore a352b1fa
//! `HandleSwapItem` / `HandleSplitItemOpcode` (ItemHandler.cpp:33-128) and
//! `Player::SwapItem` / `SplitItem` (Player.cpp:12473-12960) with bank positions.
//! Real-DB tests against data/world.db.
use bevy::prelude::Entity;
use shared::protocol::*;

use crate::bank_test_harness::{COPPER_ORE, Harness, LINEN, PEACEBLOOM, harness};
use crate::item_templates::ItemTemplates;
use crate::persistence::{self, InventoryItemData};

#[path = "bank_move_rule_tests.rs"]
mod rule_tests;

fn bank(tab: u8, slot: u8) -> ItemLocation {
    ItemLocation::Bank { tab, slot }
}

/// Where the bag stack `guid` sits, as the server places it.
fn location_of(h: &Harness, guid: u64) -> ItemLocation {
    let templates = h.app.world().resource::<ItemTemplates>();
    crate::inventory::load_model(h.db(), templates, h.character_id)
        .unwrap()
        .view()
        .into_iter()
        .find(|(_, stack)| stack.item_guid == guid)
        .map(|(location, _)| location)
        .expect("item in bags")
}

/// `(tab, slot, item, count)` of every filled slot in the last character bank sent.
fn bank_items(h: &mut Harness) -> Vec<(usize, usize, u32, u32)> {
    let contents = h
        .take::<BankContents>()
        .into_iter()
        .rev()
        .find(|contents| contents.bank == BankType::Character)
        .expect("character bank contents sent");
    contents
        .tabs
        .iter()
        .enumerate()
        .flat_map(|(tab, view)| {
            view.slots
                .iter()
                .enumerate()
                .filter_map(move |(slot, item)| {
                    item.as_ref()
                        .map(|item| (tab, slot, item.item_id, item.count))
                })
        })
        .collect()
}

fn errors(h: &mut Harness) -> Vec<InventoryErrorReason> {
    h.take::<InventoryError>()
        .into_iter()
        .map(|error| error.reason)
        .collect()
}

/// Open the banker and buy `tabs` character bank tabs (1g, then 100g).
fn banker_with_tabs(h: &mut Harness, tabs: usize) -> Entity {
    h.set_gold(2_000_000);
    let npc = h.open_banker();
    for _ in 0..tabs {
        h.send::<_, BankChannel>(BankPurchaseTab {
            npc: npc.to_bits(),
            bank: BankType::Character,
        });
    }
    h.take::<BankContents>();
    npc
}

fn swap(h: &mut Harness, from: ItemLocation, to: ItemLocation) {
    h.send::<_, InventoryChannel>(SwapItem { from, to });
}

fn split(h: &mut Harness, from: ItemLocation, to: ItemLocation, count: u32) {
    h.send::<_, InventoryChannel>(SplitItem { from, to, count });
}

#[test]
fn a_bag_stack_moves_into_a_bank_slot_and_back() {
    let Some(mut h) = harness() else { return };
    banker_with_tabs(&mut h, 1);
    let linen = h.guid_of(LINEN);
    let home = location_of(&h, linen);
    swap(&mut h, home, bank(0, 5));
    assert_eq!(bank_items(&mut h), vec![(0, 5, LINEN, 20)]);
    assert_eq!(h.count_of(LINEN), 0);

    swap(&mut h, bank(0, 5), home);
    assert_eq!(bank_items(&mut h), vec![]);
    assert_eq!(h.count_of(LINEN), 20);
    assert_eq!(location_of(&h, linen), home);
    assert_eq!(errors(&mut h), vec![]);
}

#[test]
fn bank_stacks_swap_within_a_tab_across_tabs_and_with_the_bags() {
    let Some(mut h) = harness() else { return };
    banker_with_tabs(&mut h, 2);
    let linen = location_of(&h, h.guid_of(LINEN));
    let peacebloom = location_of(&h, h.guid_of(PEACEBLOOM));
    swap(&mut h, linen, bank(0, 0));
    swap(&mut h, peacebloom, bank(0, 1));
    h.take::<BankContents>();

    swap(&mut h, bank(0, 0), bank(0, 1));
    assert_eq!(
        bank_items(&mut h),
        vec![(0, 0, PEACEBLOOM, 10), (0, 1, LINEN, 20)]
    );
    swap(&mut h, bank(0, 1), bank(1, 3));
    assert_eq!(
        bank_items(&mut h),
        vec![(0, 0, PEACEBLOOM, 10), (1, 3, LINEN, 20)]
    );
    // Copper Ore in the bags trades places with the Linen in tab 2.
    let ore = location_of(&h, h.guid_of(COPPER_ORE));
    swap(&mut h, ore, bank(1, 3));
    assert_eq!(
        bank_items(&mut h),
        vec![(0, 0, PEACEBLOOM, 10), (1, 3, COPPER_ORE, 10)]
    );
    assert_eq!(h.count_of(COPPER_ORE), 0);
    assert_eq!(h.count_of(LINEN), 20);
    assert_eq!(location_of(&h, h.guid_of(LINEN)), ore);
    assert_eq!(errors(&mut h), vec![]);
}

#[test]
fn splits_move_part_of_a_stack_between_the_bags_and_bank_tabs() {
    let Some(mut h) = harness() else { return };
    banker_with_tabs(&mut h, 2);
    let linen = location_of(&h, h.guid_of(LINEN));
    split(&mut h, linen, bank(0, 0), 5);
    assert_eq!(bank_items(&mut h), vec![(0, 0, LINEN, 5)]);
    assert_eq!(h.count_of(LINEN), 15);

    split(&mut h, bank(0, 0), bank(1, 0), 2);
    assert_eq!(bank_items(&mut h), vec![(0, 0, LINEN, 3), (1, 0, LINEN, 2)]);
    let empty_cell = ItemLocation::Bag { bag: 0, slot: 10 };
    split(&mut h, bank(1, 0), empty_cell, 1);
    assert_eq!(bank_items(&mut h), vec![(0, 0, LINEN, 3), (1, 0, LINEN, 1)]);
    assert_eq!(h.count_of(LINEN), 16);
    // Onto a bank stack of the same item with room for all of them.
    split(&mut h, linen, bank(0, 0), 4);
    assert_eq!(bank_items(&mut h), vec![(0, 0, LINEN, 7), (1, 0, LINEN, 1)]);
    assert_eq!(h.count_of(LINEN), 12);
    assert_eq!(errors(&mut h), vec![]);
}

#[test]
fn swapping_onto_the_same_item_fills_the_stack_and_keeps_the_rest() {
    let Some(mut h) = harness() else { return };
    banker_with_tabs(&mut h, 1);
    // Linen Cloth stacks to 1000: one stack of 990 and a second of 20.
    let mut inventory = h.inventory();
    let big = inventory
        .items
        .iter_mut()
        .find(|item| item.item_id == LINEN)
        .unwrap();
    big.stack_count = 990;
    let big_guid = big.item_guid;
    let small = InventoryItemData {
        item_guid: persistence::next_id(h.db(), "item_instance"),
        stack_count: 20,
        ..big.clone()
    };
    let small_guid = small.item_guid;
    inventory.items.push(small);
    persistence::save_inventory(h.db(), h.character_id, &inventory);

    let big_home = location_of(&h, big_guid);
    swap(&mut h, big_home, bank(0, 0));
    let small_home = location_of(&h, small_guid);
    swap(&mut h, small_home, bank(0, 0));
    // Player::SwapItem: the target fills to the stack size, the source keeps 10.
    assert_eq!(bank_items(&mut h), vec![(0, 0, LINEN, 1000)]);
    assert_eq!(h.count_of(LINEN), 10);

    swap(&mut h, bank(0, 0), small_home);
    assert_eq!(bank_items(&mut h), vec![(0, 0, LINEN, 10)]);
    assert_eq!(h.count_of(LINEN), 1000);
    assert_eq!(errors(&mut h), vec![]);
}

#[test]
fn a_move_request_after_closing_the_bank_rejects_without_changing_inventory() {
    let mut server = harness().expect("real world.db and faction templates");
    let npc = banker_with_tabs(&mut server, 1);
    let linen = location_of(&server, server.guid_of(LINEN));
    // Positive control: the bought bank slot accepts this exact request while open.
    swap(&mut server, linen, bank(0, 5));
    assert_eq!(bank_items(&mut server), vec![(0, 5, LINEN, 20)]);
    swap(&mut server, bank(0, 5), linen);
    assert_eq!(bank_items(&mut server), vec![]);
    assert_eq!(errors(&mut server), vec![]);

    server.send::<_, InteractionChannel>(CloseInteraction { npc: npc.to_bits() });
    assert!(
        server
            .app
            .world()
            .get::<crate::npc_interaction::ActiveInteraction>(server.connection)
            .is_none()
    );
    let inventory_before = server.inventory();
    let bank_before = persistence::banks::load_character_bank(server.db(), server.character_id);
    // Bypass the UI cursor cancellation: send the real InventoryChannel request.
    swap(&mut server, linen, bank(0, 5));
    assert_eq!(
        errors(&mut server),
        vec![InventoryErrorReason::CantDoRightNow]
    );
    assert_eq!(server.inventory(), inventory_before);
    assert_eq!(
        persistence::banks::load_character_bank(server.db(), server.character_id),
        bank_before
    );
    assert!(server.take::<BankContents>().is_empty());
}

#[test]
fn bank_moves_refuse_oversized_splits_unbought_tabs_and_a_closed_bank() {
    let Some(mut h) = harness() else { return };
    let linen = location_of(&h, h.guid_of(LINEN));
    // No banker open (TrinityCore `CanUseBank`).
    swap(&mut h, linen, bank(0, 0));
    assert_eq!(errors(&mut h), vec![InventoryErrorReason::CantDoRightNow]);

    banker_with_tabs(&mut h, 1);
    split(&mut h, linen, bank(0, 0), 25);
    assert_eq!(errors(&mut h), vec![InventoryErrorReason::TooFewToSplit]);
    // Tab 2 is not bought: not a valid position (`IsValidPos`, EQUIP_ERR_WRONG_SLOT).
    swap(&mut h, linen, bank(1, 0));
    split(&mut h, linen, bank(1, 0), 5);
    assert_eq!(
        errors(&mut h),
        vec![
            InventoryErrorReason::WrongSlot,
            InventoryErrorReason::WrongSlot
        ]
    );
    assert_eq!(h.count_of(LINEN), 20);
    assert_eq!(location_of(&h, h.guid_of(LINEN)), linen);
}
