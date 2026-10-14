use super::*;
use crate::persistence::{self, InventoryData, InventoryItemData};
use redb::ReadableDatabase;
use shared::components::{
    CharacterAppearance, EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry,
};

fn manifest() -> Vec<EquipmentGrantItem> {
    [
        (25, "Worn Shortsword", 21),
        (38, "Recruit's Shirt", 4),
        (39, "Recruit's Pants", 7),
        (40, "Recruit's Boots", 8),
        (2362, "Worn Wooden Shield", 14),
    ]
    .into_iter()
    .map(|(item_id, name, inventory_type)| EquipmentGrantItem {
        item_id,
        name: name.into(),
        quality: if item_id == 38 { 2 } else { 1 },
        required_level: 1,
        inventory_type,
        vendor_sell_price: 1,
        allowable_class: 1,
        allowable_race: 1,
    })
    .collect()
}

fn initialize(path: &std::path::Path) -> Database {
    let db = Database::create(path).unwrap();
    persistence::create_tables(&db);
    let character = CharacterData {
        name: "Theron".into(),
        x: -9000.0,
        y: -100.0,
        z: 80.0,
        health: 20.0,
        max_health: 20.0,
        race: 1,
        class: 1,
        level: 1,
        appearance: CharacterAppearance::default(),
        durability: vec![],
        equipment_appearance: EquipmentAppearance {
            entries: vec![EquippedAppearanceEntry {
                slot: EquipmentVisualSlot::Head,
                item_id: None,
                definition_source: None,
                display_info_id: None,
                inventory_type: 0,
                hidden: true,
            }],
        },
    };
    persistence::save_character(&db, 6, &character);
    let txn = db.begin_write().unwrap();
    txn.open_table(persistence::CHARACTER_NAMES)
        .unwrap()
        .insert("Theron", 6)
        .unwrap();
    txn.open_table(persistence::NEXT_IDS)
        .unwrap()
        .insert("item_instance", 50)
        .unwrap();
    crate::persistence_commit::commit(txn).unwrap();
    persistence::save_inventory(
        &db,
        6,
        &InventoryData {
            gold: 12345,
            items: vec![InventoryItemData {
                item_guid: 50,
                item_id: 2589,
                name: "Linen Cloth".into(),
                quality: 1,
                required_level: 0,
                stack_count: 20,
                vendor_sell_price: 3,
                unique_auction: false,
            }],
        },
    );
    db
}

fn counter(db: &Database) -> u64 {
    let txn = db.begin_read().unwrap();
    txn.open_table(persistence::NEXT_IDS)
        .unwrap()
        .get("item_instance")
        .unwrap()
        .unwrap()
        .value()
}

#[test]
fn equipment_batch_persists_owned_items_and_appearance_without_changing_bags() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("game.redb");
    let db = initialize(&path);
    let bags = persistence::load_inventory(&db, 6);
    let (id, character, items) = grant_equipment(&db, "Theron", &manifest()).unwrap();
    assert_eq!(id, 6);
    assert_eq!(items.len(), 5);
    assert_eq!(
        items.iter().map(|item| item.item_guid).collect::<Vec<_>>(),
        vec![51, 52, 53, 54, 55]
    );
    assert!(items.iter().all(|item| item.stack_count == 1));
    assert_eq!(counter(&db), 55);
    assert_eq!(persistence::load_inventory(&db, 6), bags);
    assert_eq!(character.equipment_appearance.entries.len(), 6);
    assert!(character.equipment_appearance.entries[0].hidden);
    for definition in manifest() {
        let appearance = character
            .equipment_appearance
            .entries
            .iter()
            .find(|e| e.item_id == Some(definition.item_id))
            .unwrap();
        assert_eq!(appearance.inventory_type, definition.inventory_type);
        assert_eq!(appearance.display_info_id, None);
        assert!(!appearance.hidden);
    }
    // Simulate the normal logout/save boundary: it updates position on freshly loaded data.
    let mut saved = persistence::load_character(&db, 6).unwrap();
    saved.x -= 1.0;
    persistence::save_character(&db, 6, &saved);
    drop(db);
    let reopened = Database::open(&path).unwrap();
    assert_eq!(list_equipment(&reopened, "Theron").unwrap(), (6, items));
    assert_eq!(persistence::load_character(&reopened, 6).unwrap(), saved);
    assert_eq!(persistence::load_inventory(&reopened, 6), bags);
}

#[test]
fn invalid_equipment_batches_leave_all_persistent_state_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let db = initialize(&dir.path().join("game.redb"));
    let before = persistence::load_character(&db, 6).unwrap();
    let bags = persistence::load_inventory(&db, 6);
    let mut cases = vec![vec![]];
    for mutation in 0..7 {
        let mut items = manifest();
        match mutation {
            0 => items[4].required_level = 2,
            1 => items[4].allowable_class = 2,
            2 => items[4].allowable_race = 2,
            3 => items[4].inventory_type = 99,
            4 => items[4].inventory_type = items[0].inventory_type,
            5 => items[4].item_id = items[0].item_id,
            _ => items[4].name.clear(),
        }
        cases.push(items);
    }
    for items in cases {
        assert!(grant_equipment(&db, "Theron", &items).is_err());
        assert_eq!(counter(&db), 50);
        assert_eq!(persistence::load_character(&db, 6).unwrap(), before);
        assert_eq!(persistence::load_inventory(&db, 6), bags);
        assert!(list_equipment(&db, "Theron").unwrap().1.is_empty());
    }
    assert!(
        grant_equipment(&db, "Nobody", &manifest())
            .unwrap_err()
            .contains("Nobody")
    );
    assert!(
        list_equipment(&db, "Nobody")
            .unwrap_err()
            .contains("Nobody")
    );
}

#[test]
fn equipment_guid_exhaustion_rolls_back_the_complete_batch() {
    let directory = tempfile::tempdir().unwrap();
    let db = initialize(&directory.path().join("game.redb"));
    let before = persistence::load_character(&db, 6).unwrap();
    let txn = db.begin_write().unwrap();
    txn.open_table(persistence::NEXT_IDS)
        .unwrap()
        .insert("item_instance", u64::MAX - 1)
        .unwrap();
    crate::persistence_commit::commit(txn).unwrap();
    let error = grant_equipment(&db, "Theron", &manifest()).unwrap_err();
    assert!(error.contains("exhausted"), "{error}");
    assert_eq!(counter(&db), u64::MAX - 1);
    assert!(list_equipment(&db, "Theron").unwrap().1.is_empty());
    assert_eq!(persistence::load_character(&db, 6).unwrap(), before);
}

#[test]
fn equipment_preserves_hidden_overrides_and_rejects_conflicting_hands() {
    let directory = tempfile::tempdir().unwrap();
    let db = initialize(&directory.path().join("game.redb"));
    let before = persistence::load_character(&db, 6).unwrap();
    let mut helmet = manifest()[0].clone();
    helmet.inventory_type = 1;
    assert!(
        grant_equipment(&db, "Theron", &[helmet])
            .unwrap_err()
            .contains("occupied")
    );
    let mut two_handed = manifest()[0].clone();
    two_handed.inventory_type = 17;
    assert!(
        grant_equipment(&db, "Theron", &[two_handed, manifest()[4].clone()])
            .unwrap_err()
            .contains("off-hand")
    );
    assert_eq!(counter(&db), 50);
    assert_eq!(persistence::load_character(&db, 6).unwrap(), before);
    let mut unrestricted = manifest();
    for item in &mut unrestricted {
        item.allowable_class = -1;
        item.allowable_race = -1;
    }
    assert_eq!(
        grant_equipment(&db, "Theron", &unrestricted)
            .unwrap()
            .2
            .len(),
        5
    );
}

#[test]
fn occupied_slots_reject_batch_without_consuming_guids_or_replacing_items() {
    let dir = tempfile::tempdir().unwrap();
    let db = initialize(&dir.path().join("game.redb"));
    grant_equipment(&db, "Theron", &manifest()[..1]).unwrap();
    let original = list_equipment(&db, "Theron").unwrap();
    let character = persistence::load_character(&db, 6).unwrap();
    let batch = vec![manifest()[1].clone(), manifest()[0].clone()];
    assert!(
        grant_equipment(&db, "Theron", &batch)
            .unwrap_err()
            .contains("occupied")
    );
    assert_eq!(list_equipment(&db, "Theron").unwrap(), original);
    assert_eq!(persistence::load_character(&db, 6).unwrap(), character);
    assert_eq!(counter(&db), 51);
    let (_, _, all) = grant_equipment(&db, "Theron", &manifest()[1..]).unwrap();
    assert_eq!(all.len(), 5);
    assert_eq!(counter(&db), 55);
}
