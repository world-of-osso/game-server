use super::*;
use shared::item_data::ItemDefinitionSource::{Forever70205, Retail};

fn record(name: &str, guid: u64) -> InventoryRecord {
    InventoryRecord {
        character: CharacterData {
            name: name.into(),
            x: 0.0,
            y: 0.0,
            z: 0.0,
            health: 100.0,
            max_health: 100.0,
            race: 1,
            class: 1,
            level: 1,
            appearance: Default::default(),
            equipment_appearance: Default::default(),
            durability: vec![],
        },
        inventory: InventoryData {
            gold: 0,
            items: vec![InventoryItemData {
                item_guid: guid,
                item_id: 2947,
                name: "Test item".into(),
                stack_count: 1,
                quality: 1,
                required_level: 1,
                vendor_sell_price: 0,
                unique_auction: false,
            }],
        },
        placements: vec![],
        equipped: vec![],
        bag_slots: vec![],
        nonvisual_equipped: Vec::new(),
        instances: HashMap::from([(guid, ItemInstanceData { soulbound: false })]),
        crafting_origins: HashMap::new(),
        item_durability: HashMap::new(),
        definition_sources: HashMap::new(),
        appearance_account: None,
        appearance_grants: Vec::new(),
        transmogs: Default::default(),
    }
}

#[test]
fn item_sources_persist_across_reopen_and_two_owner_transaction_transfer() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = Database::create(file.path()).unwrap();
    super::super::create_tables(&db);
    let mut retail = record("Retail", 101);
    retail.definition_sources.insert(101, Retail);
    let mut forever = record("Forever", 202);
    forever.definition_sources.insert(202, Forever70205);
    let txn = db.begin_write().unwrap();
    write_inventory_record(&txn, 1, &retail);
    write_inventory_record(&txn, 2, &forever);
    txn.commit().unwrap();
    drop(db);

    let db = Database::open(file.path()).unwrap();
    let mut retail = load_inventory_record(&db, 1).unwrap();
    let mut forever = load_inventory_record(&db, 2).unwrap();
    assert_eq!(retail.definition_sources[&101], Retail);
    assert_eq!(forever.definition_sources[&202], Forever70205);
    // Same ItemID, different GUIDs: swap owners in the existing record-write
    // transaction boundary used by trade, without moving source metadata.
    std::mem::swap(&mut retail.inventory.items, &mut forever.inventory.items);
    let txn = db.begin_write().unwrap();
    write_inventory_record(&txn, 1, &retail);
    write_inventory_record(&txn, 2, &forever);
    txn.commit().unwrap();
    drop(db);

    let db = Database::open(file.path()).unwrap();
    let retail_owner = load_inventory_record(&db, 1).unwrap();
    let forever_owner = load_inventory_record(&db, 2).unwrap();
    assert_eq!(retail_owner.inventory.items[0].item_guid, 202);
    assert_eq!(
        retail_owner.definition_sources,
        HashMap::from([(202, Forever70205)])
    );
    assert_eq!(forever_owner.inventory.items[0].item_guid, 101);
    assert_eq!(
        forever_owner.definition_sources,
        HashMap::from([(101, Retail)])
    );
}

#[test]
fn item_sources_unmarked_legacy_guid_is_retail() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = Database::create(file.path()).unwrap();
    super::super::create_tables(&db);
    save_inventory_record(&db, 1, &record("Legacy", 303));
    drop(db);
    let db = Database::open(file.path()).unwrap();
    assert_eq!(load_item_definition_source(&db, 303), Retail);
    assert_eq!(
        load_inventory_record(&db, 1).unwrap().definition_sources[&303],
        Retail
    );
}

#[test]
fn item_sources_model_duplicate_split_and_partial_take_keep_provenance() {
    use crate::inventory::inventory_model::InventoryModel;
    use crate::item_templates::ItemTemplates;
    use shared::protocol::ItemLocation;

    let db = super::super::temp_database();
    let mut original = record("Forever", 601);
    original.inventory.items[0].stack_count = 4;
    original
        .inventory
        .items
        .push(original.inventory.items[0].clone());
    original.definition_sources.insert(601, Forever70205);
    save_inventory_record(&db, 1, &original);
    let loaded = load_inventory_record(&db, 1).unwrap();
    let mut model = InventoryModel::from_record(loaded, &ItemTemplates::default(), &mut || 602);
    model
        .split(
            ItemLocation::Bag { bag: 0, slot: 0 },
            ItemLocation::Bag { bag: 0, slot: 2 },
            1,
            603,
        )
        .unwrap();
    let taken = model.take_bag_item(602, 2, &mut || 604).unwrap();
    assert_eq!((taken.item_guid, taken.stack_count), (604, 2));
    let saved = model.into_record();
    assert_eq!(
        saved
            .inventory
            .items
            .iter()
            .map(|item| (item.item_guid, item.stack_count))
            .collect::<Vec<_>>(),
        vec![(601, 3), (602, 2), (603, 1)]
    );
    save_inventory_record(&db, 1, &saved);
    for guid in [601, 602, 603, 604] {
        assert_eq!(load_item_definition_source(&db, guid), Forever70205);
    }
}

#[test]
#[should_panic(expected = "invalid item definition source 255 for GUID 404")]
fn item_sources_malformed_marker_fails_explicitly() {
    let db = super::super::temp_database();
    let txn = db.begin_write().unwrap();
    txn.open_table(ITEM_DEFINITION_SOURCES)
        .unwrap()
        .insert(404, 255)
        .unwrap();
    txn.commit().unwrap();
    load_item_definition_source(&db, 404);
}

#[test]
#[should_panic(expected = "cannot change item definition source for GUID 505")]
fn item_sources_existing_marker_is_immutable() {
    let db = super::super::temp_database();
    let txn = db.begin_write().unwrap();
    write_item_definition_source(&txn, 505, Forever70205);
    write_item_definition_source(&txn, 505, Forever70205);
    txn.commit().unwrap();
    let txn = db.begin_write().unwrap();
    write_item_definition_source(&txn, 505, Retail);
}
