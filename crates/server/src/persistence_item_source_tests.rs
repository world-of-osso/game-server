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
        item_modifiers: HashMap::new(),
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

fn native_equipped_record(guid: u64) -> InventoryRecord {
    let mut record = record("ParityOwned", guid);
    record.character.class = 2;
    record.character.race = 6;
    record.character.level = 90;
    let mut item = record.inventory.items.pop().unwrap();
    item.item_id = 237832;
    item.name = "Spellbreaker's Cover".into();
    item.quality = 4;
    item.required_level = 90;
    record.equipped.push(EquippedItem {
        slot: shared::components::EquipmentVisualSlot::Head,
        item,
    });
    record.definition_sources.insert(guid, Retail);
    record
}

fn native_modifiers() -> server::item_instance::ItemInstanceModifiers {
    server::item_instance::ItemInstanceModifiers {
        bonus_list: vec![8960, 12384, 13750, 13751, 13836, 9627],
        item_level_override: Some(334),
        enchant: 7961,
        gems: vec![240983],
        crafted_stats: vec![32, 36],
    }
}

#[test]
fn item_instance_preexisting_character_without_modifier_table_reopens_as_unmodified() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = Database::create(file.path()).unwrap();
    super::super::create_tables(&db);
    let record = native_equipped_record(41);
    save_inventory_record(&db, 1, &record);
    let txn = db.begin_write().unwrap();
    assert!(
        txn.delete_table(server::item_instance::store::ITEM_MODIFIERS)
            .unwrap()
    );
    txn.commit().unwrap();
    drop(db);

    let db = Database::open(file.path()).unwrap();
    let loaded = load_inventory_record(&db, 1).expect("pre-existing character remains loadable");
    assert_eq!(loaded.equipped, record.equipped);
    assert!(loaded.item_modifiers.is_empty());
    let standalone = server::item_instance::store::load_modifiers(&db, 41).unwrap();
    assert_eq!(standalone, Default::default());
    assert!(
        matches!(
            db.begin_read()
                .unwrap()
                .open_table(server::item_instance::store::ITEM_MODIFIERS),
            Err(redb::TableError::TableDoesNotExist(_))
        ),
        "reading legacy data must not create a table"
    );
}

#[test]
fn item_instance_inventory_record_hydrates_guid_modifiers_after_database_reopen() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = Database::create(file.path()).unwrap();
    super::super::create_tables(&db);
    let initial = native_equipped_record(41);
    let modifiers = native_modifiers();
    let txn = db.begin_write().unwrap();
    write_inventory_record(&txn, 1, &initial);
    server::item_instance::store::write_modifiers(&txn, 41, &modifiers).unwrap();
    txn.commit().unwrap();
    drop(db);
    let db = Database::open(file.path()).unwrap();
    let mut loaded = load_inventory_record(&db, 1).unwrap();
    assert_eq!(loaded.item_modifiers.get(&41), Some(&modifiers));
    let mut changed = modifiers.clone();
    changed.item_level_override = Some(344);
    loaded.item_modifiers.insert(41, changed.clone());
    save_inventory_record(&db, 1, &loaded);
    drop(db);
    let db = Database::open(file.path()).unwrap();
    let mut sender = load_inventory_record(&db, 1).unwrap();
    assert_eq!(sender.item_modifiers.get(&41), Some(&changed));
    let mut receiver = record("ParityReceiver", 42);
    receiver.inventory.items.clear();
    receiver.character.class = 2;
    receiver.character.race = 6;
    receiver.character.level = 90;
    receiver.equipped = std::mem::take(&mut sender.equipped);
    let txn = db.begin_write().unwrap();
    write_inventory_record(&txn, 1, &sender);
    write_inventory_record(&txn, 2, &receiver);
    txn.commit().unwrap();
    drop(db);
    let db = Database::open(file.path()).unwrap();
    assert_eq!(
        load_inventory_record(&db, 2)
            .unwrap()
            .item_modifiers
            .get(&41),
        Some(&changed)
    );
    assert!(
        load_inventory_record(&db, 1)
            .unwrap()
            .item_modifiers
            .is_empty()
    );
}

#[test]
fn item_instance_inventory_model_projects_owned_native_modifiers_to_stats_input() {
    let mut record = native_equipped_record(41);
    let modifiers = native_modifiers();
    record.item_modifiers.insert(41, modifiers.clone());
    let path = std::env::var_os("DPS_PARITY_WORLD_DB")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(crate::test_data::world_db_path);
    let templates = crate::item_templates::load_item_templates(&path).unwrap();
    let model = crate::inventory::inventory_model::InventoryModel::from_record(
        record,
        &templates,
        &mut || 42,
    );
    // This is the concrete input consumed by the separately proved StatsPlugin path.
    assert_eq!(
        model.equipped_modifiers().items,
        vec![(shared::protocol::EquipmentSlot::Head, modifiers)]
    );
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
