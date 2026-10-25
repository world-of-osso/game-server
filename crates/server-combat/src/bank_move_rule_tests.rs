use super::*;

#[test]
fn bank_moves_keep_specialty_bag_rules_and_refuse_banking_a_nonempty_bag() {
    let Some(mut h) = harness() else {
        panic!("bank fixture unavailable")
    };
    banker_with_tabs(&mut h, 1);
    // Retail Herb Pouch (22250): a 12-slot herb-only container.
    const HERB_POUCH: u32 = 22250;
    let mut inventory = h.inventory();
    let pouch = InventoryItemData {
        item_guid: persistence::next_id(h.db(), "item_instance"),
        item_id: HERB_POUCH,
        name: "Herb Pouch".into(),
        stack_count: 1,
        ..inventory.items[0].clone()
    };
    let pouch_guid = pouch.item_guid;
    inventory.items.push(pouch);
    persistence::save_inventory(h.db(), h.character_id, &inventory);
    let pouch_location = location_of(&h, pouch_guid);
    let equipped = ItemLocation::Equipment(EquipmentSlot::Bag1);
    swap(&mut h, pouch_location, equipped);
    assert!(errors(&mut h).is_empty());
    let herb = location_of(&h, h.guid_of(PEACEBLOOM));
    let specialty = ItemLocation::Bag { bag: 1, slot: 0 };
    swap(&mut h, herb, specialty);
    let linen = location_of(&h, h.guid_of(LINEN));
    swap(&mut h, linen, bank(0, 0));
    let before = persistence::banks::load_character_bank(h.db(), h.character_id);
    // Both directions are validated before swapping: cloth cannot replace the herb.
    swap(&mut h, bank(0, 0), specialty);
    assert_eq!(errors(&mut h), vec![InventoryErrorReason::WrongBagType]);
    assert_eq!(
        persistence::banks::load_character_bank(h.db(), h.character_id),
        before
    );
    assert_eq!(h.count_of(PEACEBLOOM), 10);
    // TC CanStoreItem_InSpecificSlot: nonempty equipped containers cannot go into bank cells.
    swap(&mut h, equipped, bank(0, 1));
    assert_eq!(
        errors(&mut h),
        vec![InventoryErrorReason::DestroyNonemptyBag]
    );
    assert_eq!(
        persistence::banks::load_character_bank(h.db(), h.character_id),
        before
    );
    assert_eq!(location_of(&h, pouch_guid), equipped);
}

#[test]
fn bank_to_bank_merge_and_same_tab_split_preserve_total_count() {
    let Some(mut h) = harness() else {
        panic!("bank fixture unavailable")
    };
    banker_with_tabs(&mut h, 2);
    let linen = location_of(&h, h.guid_of(LINEN));
    swap(&mut h, linen, bank(0, 0));
    split(&mut h, bank(0, 0), bank(0, 1), 6);
    assert_eq!(
        bank_items(&mut h),
        vec![(0, 0, LINEN, 14), (0, 1, LINEN, 6)]
    );
    swap(&mut h, bank(0, 1), bank(1, 5));
    swap(&mut h, bank(1, 5), bank(0, 0));
    assert_eq!(bank_items(&mut h), vec![(0, 0, LINEN, 20)]);
    assert_eq!(h.count_of(LINEN), 0);
    assert!(errors(&mut h).is_empty());
}

#[test]
fn bank_splits_preserve_binding_guids_and_counts_after_database_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("game.redb");
    let (character, original, split_guid) = {
        let Some(mut h) = crate::bank_test_harness::harness_at(&path, "Banker") else {
            panic!("bank fixture unavailable")
        };
        banker_with_tabs(&mut h, 2);
        let original = h.guid_of(LINEN);
        h.make_soulbound(original);
        let home = location_of(&h, original);
        swap(&mut h, home, bank(0, 0));
        split(&mut h, bank(0, 0), bank(1, 1), 7);
        let bank = persistence::banks::load_character_bank(h.db(), h.character_id);
        let split_guid = bank.tabs[1].slots[1].as_ref().unwrap().item_guid;
        assert_ne!(split_guid, original);
        assert!(errors(&mut h).is_empty());
        (h.character_id, original, split_guid)
    };
    let Some(mut h) = crate::bank_test_harness::harness_at(&path, "Restarted") else {
        panic!("bank fixture unavailable")
    };
    h.play_as(character);
    h.open_banker();
    let contents = h
        .take::<BankContents>()
        .into_iter()
        .rev()
        .find(|c| c.bank == BankType::Character)
        .unwrap();
    let kept = contents.tabs[0].slots[0].as_ref().unwrap();
    let split = contents.tabs[1].slots[1].as_ref().unwrap();
    assert_eq!(
        (kept.item_guid, kept.count, kept.soulbound),
        (original, 13, true)
    );
    assert_eq!(
        (split.item_guid, split.count, split.soulbound),
        (split_guid, 7, true)
    );
    let cell = ItemLocation::Bag { bag: 0, slot: 10 };
    swap(&mut h, bank(1, 1), cell);
    let templates = h.app.world().resource::<ItemTemplates>();
    let model = crate::inventory::load_model(h.db(), templates, character).unwrap();
    let bag = &model.view()[&cell];
    assert_eq!(
        (bag.item_guid, bag.count, bag.soulbound),
        (split_guid, 7, true)
    );
    assert_eq!(h.count_of(LINEN), 7);
    assert_eq!(bank_items(&mut h), vec![(0, 0, LINEN, 13)]);
}
