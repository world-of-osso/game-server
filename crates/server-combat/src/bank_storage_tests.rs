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

fn retail_sources(tabs: &[BankTabData], incoming: u64) -> HashMap<u64, ItemDefinitionSource> {
    tabs.iter()
        .flat_map(|tab| tab.slots.iter().flatten())
        .map(|item| item.item_guid)
        .chain([incoming])
        .map(|guid| (guid, ItemDefinitionSource::Retail))
        .collect()
}

fn no_mint() -> u64 {
    panic!("no new stack expected")
}

#[test]
fn deposit_tops_up_existing_stack_before_empty_slots() {
    let mut tabs = vec![BankTabData::new("Tab 1".into())];
    tabs[0].slots[5] = Some(item(1, LINEN, 190));
    let mut sources = retail_sources(&tabs, 2);
    deposit(
        &mut tabs,
        &[0],
        item(2, LINEN, 20),
        200,
        &mut sources,
        &mut no_mint,
    )
    .unwrap();
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
    let mut sources = retail_sources(&tabs, 3);
    deposit(
        &mut tabs,
        &tab_order(2, 1),
        item(3, LINEN, 400),
        200,
        &mut sources,
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
    let mut sources = retail_sources(&tabs, 4);
    assert_eq!(
        deposit(
            &mut tabs,
            &[0],
            item(4, LINEN, 1),
            200,
            &mut sources,
            &mut no_mint
        ),
        Err(TabsFull)
    );
    assert_eq!(tabs, before);
}

#[test]
fn owned_sources_bank_deposit_keeps_collision_stacks_and_minted_identity() {
    use shared::item_data::ItemDefinitionSource::{Forever70205, Retail};
    let mut tabs = vec![BankTabData::new("Mixed origins".into())];
    tabs[0].slots[5] = Some(item(1, 2512, 190));
    tabs[0].slots[6] = Some(item(2, 2512, 990));
    let mut sources = HashMap::from([(1, Retail), (2, Forever70205), (3, Forever70205)]);
    deposit(
        &mut tabs,
        &[0],
        item(3, 2512, 1050),
        1000,
        &mut sources,
        &mut || 1000,
    )
    .unwrap();
    assert_eq!(
        counts(&tabs[0], 2512),
        vec![(0, 1000), (1, 40), (5, 190), (6, 1000)]
    );
    assert_eq!(tabs[0].slots[0].as_ref().unwrap().item_guid, 3);
    assert_eq!(tabs[0].slots[1].as_ref().unwrap().item_guid, 1000);
    assert_eq!(sources[&1], Retail);
    assert_eq!(sources[&1000], Forever70205);
    let view = tab_slots(&tabs[0], |_| false, |guid| sources[&guid], |_| None);
    assert_eq!(view[5].as_ref().unwrap().definition_source, Retail);
    assert_eq!(view[0].as_ref().unwrap().definition_source, Forever70205);
}

#[test]
fn take_empties_the_slot() {
    let mut tabs = vec![BankTabData::new("Tab 1".into())];
    tabs[0].slots[7] = Some(item(5, LINEN, 20));
    assert_eq!(take(&mut tabs, 0, 7), Some(item(5, LINEN, 20)));
    assert_eq!(take(&mut tabs, 0, 7), None);
    assert_eq!(take(&mut tabs, 3, 0), None);
}
