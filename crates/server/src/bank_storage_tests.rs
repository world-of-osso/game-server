use super::*;

const LINEN: u32 = 2589;
const COPPER_ORE: u32 = 2770;

fn item(guid: u64, item_id: u32, count: u32) -> InventoryItemData {
    InventoryItemData {
        item_guid: guid,
        item_id,
        name: String::new(),
        quality: 1,
        required_level: 0,
        stack_count: count,
        vendor_sell_price: 13,
        unique_auction: false,
    }
}

fn counts(tab: &BankTabData, item_id: u32) -> Vec<(usize, u32)> {
    tab.slots
        .iter()
        .enumerate()
        .filter_map(|(slot, item)| {
            item.as_ref()
                .filter(|item| item.item_id == item_id)
                .map(|item| (slot, item.stack_count))
        })
        .collect()
}

fn no_mint() -> u64 {
    panic!("no new stack expected")
}

#[test]
fn deposit_tops_up_existing_stack_before_empty_slots() {
    let mut tabs = vec![BankTabData::new("Tab 1".into())];
    tabs[0].slots[5] = Some(item(1, LINEN, 190));
    deposit(&mut tabs, &[0], item(2, LINEN, 20), 200, &mut no_mint).unwrap();
    // 10 top the stack in slot 5 up to 200, the other 10 open slot 0 under guid 2.
    assert_eq!(counts(&tabs[0], LINEN), vec![(0, 10), (5, 200)]);
    assert_eq!(tabs[0].slots[0].as_ref().unwrap().item_guid, 2);
}

#[test]
fn deposit_spills_into_next_tab_in_order() {
    let mut tabs = vec![
        BankTabData::new("Tab 1".into()),
        BankTabData::new("Tab 2".into()),
    ];
    for slot in &mut tabs[1].slots {
        *slot = Some(item(9, COPPER_ORE, 20));
    }
    tabs[1].slots[97] = None;
    // Preferred tab 2 has one free slot; the second stack goes to tab 1.
    let mut next = 100;
    let mut mint = || {
        next += 1;
        next
    };
    deposit(
        &mut tabs,
        &tab_order(2, 1),
        item(3, LINEN, 400),
        200,
        &mut mint,
    )
    .unwrap();
    assert_eq!(counts(&tabs[1], LINEN), vec![(97, 200)]);
    assert_eq!(counts(&tabs[0], LINEN), vec![(0, 200)]);
    assert_eq!(tabs[1].slots[97].as_ref().unwrap().item_guid, 3);
    assert_eq!(tabs[0].slots[0].as_ref().unwrap().item_guid, 101);
}

#[test]
fn deposit_that_does_not_fit_changes_nothing() {
    let mut tabs = vec![BankTabData::new("Tab 1".into())];
    for slot in &mut tabs[0].slots {
        *slot = Some(item(9, COPPER_ORE, 20));
    }
    let before = tabs.clone();
    assert_eq!(
        deposit(&mut tabs, &[0], item(4, LINEN, 1), 200, &mut no_mint),
        Err(TabsFull)
    );
    assert_eq!(tabs, before);
}

#[test]
fn take_empties_the_slot() {
    let mut tabs = vec![BankTabData::new("Tab 1".into())];
    tabs[0].slots[7] = Some(item(5, LINEN, 20));
    assert_eq!(take(&mut tabs, 0, 7), Some(item(5, LINEN, 20)));
    assert_eq!(take(&mut tabs, 0, 7), None);
    assert_eq!(take(&mut tabs, 3, 0), None);
}
