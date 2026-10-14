use super::*;
use shared::protocol::DecorDye;

fn dye_source() -> crate::housing_catalog::DyeCatalog {
    let db = crate::housing_catalog::tests::catalog_fixture();
    for (table, id, fields) in [
        (
            "DecorDyeSlot",
            28,
            serde_json::json!({"HouseDecorID":"374","DyeColorCategoryID":"2","Channel":"0","OrderIndex":"0"}),
        ),
        (
            "DecorDyeSlot",
            29,
            serde_json::json!({"HouseDecorID":"374","DyeColorCategoryID":"3","Channel":"1","OrderIndex":"1"}),
        ),
        (
            "DyeColor",
            23,
            serde_json::json!({"DyeColorCategoryID":"1","ItemID":"274473"}),
        ),
        (
            "DyeColor",
            24,
            serde_json::json!({"DyeColorCategoryID":"1","ItemID":"274470"}),
        ),
    ] {
        db.execute("INSERT INTO housing_catalog_record(build_key,table_name,record_id,fields_json) VALUES(?,?,?,?)", rusqlite::params![BUILD_KEY,table,id,fields.to_string()]).unwrap();
    }
    crate::housing_catalog::DyeCatalog::read(&db).unwrap()
}

fn dye_item(guid: u64, id: u32, count: u32) -> InventoryItemData {
    InventoryItemData {
        item_guid: guid,
        item_id: id,
        name: "Housing dye".into(),
        quality: 1,
        required_level: 0,
        stack_count: count,
        vendor_sell_price: 0,
        unique_auction: false,
    }
}

fn placed_shelf(db: &Database) -> (Actor, u64) {
    let mut owner = actor(1, 30);
    let property = purchase(db, owner, &plot()).unwrap();
    let mut inventory = crate::persistence::load_inventory(db, 30);
    inventory.items.push(dye_item(100, 245384, 1));
    crate::persistence::save_inventory(db, 30, &inventory);
    redeem(
        db,
        owner,
        100,
        &RedemptionRule {
            item_id: 245384,
            decor_id: 374,
        },
    )
    .unwrap();
    owner.inside_house = Some(property.house_id);
    edit(
        db,
        owner,
        &HousingCommand::PlaceDecor {
            variant_id: None,
            house_id: property.house_id,
            decor_id: 374,
            transform: DecorTransform {
                position: [1.25, 2.0, -3.0],
                yaw: 0.0,
                pitch: 0.0,
                roll: 0.0,
                scale: 1.0,
            },
        },
    )
    .unwrap();
    (owner, property.house_id)
}

#[test]
fn housing_dye_preview_is_owner_local_and_commit_consumes_per_changed_slot_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dyes.redb");
    let db = fixture(&path);
    let (owner, house_id) = placed_shelf(&db);
    let catalog = dye_source();
    let changes = vec![
        DecorDye {
            slot_id: 28,
            color_id: Some(23),
        },
        DecorDye {
            slot_id: 29,
            color_id: Some(23),
        },
    ];
    let mut inventory = crate::persistence::load_inventory(&db, 30);
    inventory.items = vec![dye_item(701, 274473, 1), dye_item(702, 274473, 1)];
    crate::persistence::save_inventory(&db, 30, &inventory);
    let txn = db.begin_write().unwrap();
    inventory_store::write_item_definition_source(&txn, 702, ItemDefinitionSource::Forever70205);
    crate::persistence_commit::commit(txn).unwrap();
    let preview = preview_dyes(&db, owner, house_id, 1, &changes, &catalog).unwrap();
    assert_eq!(preview.changes, changes);
    assert_eq!(crate::persistence::load_inventory(&db, 30), inventory);
    assert!(
        snapshot(&db, owner, Some(house_id), None)
            .unwrap()
            .layout
            .unwrap()
            .decor[0]
            .dyes
            .is_empty()
    );
    assert!(commit_dyes(&db, owner, &preview, &catalog).is_err());
    assert_eq!(crate::persistence::load_inventory(&db, 30), inventory);
    assert!(
        snapshot(&db, owner, Some(house_id), None)
            .unwrap()
            .layout
            .unwrap()
            .decor[0]
            .dyes
            .is_empty()
    );
    let mut visitor = actor(2, 31);
    visitor.inside_house = Some(house_id);
    assert!(preview_dyes(&db, visitor, house_id, 1, &changes, &catalog).is_err());
    assert!(commit_dyes(&db, visitor, &preview, &catalog).is_err());
    assert!(preview_dyes(&db, actor(1, 30), house_id, 1, &changes, &catalog).is_err());
    assert!(
        preview_dyes(
            &db,
            owner,
            house_id,
            1,
            &[DecorDye {
                slot_id: 28,
                color_id: Some(99999)
            }],
            &catalog
        )
        .is_err()
    );
    assert!(
        preview_dyes(
            &db,
            owner,
            house_id,
            1,
            &[DecorDye {
                slot_id: 99999,
                color_id: Some(23)
            }],
            &catalog
        )
        .is_err()
    );
    assert!(
        preview_dyes(
            &db,
            owner,
            house_id,
            1,
            &[changes[0].clone(), changes[0].clone()],
            &catalog
        )
        .is_err()
    );
    inventory.items[0].stack_count = 2;
    crate::persistence::save_inventory(&db, 30, &inventory);
    assert!(commit_dyes(&db, owner, &preview, &catalog).unwrap());
    let paid = crate::persistence::load_inventory(&db, 30);
    assert_eq!(paid.items, vec![dye_item(702, 274473, 1)]);
    assert!(snapshot(&db, owner, None, None).unwrap().stored.is_empty());
    let saved = snapshot(&db, owner, Some(house_id), None)
        .unwrap()
        .layout
        .unwrap();
    assert_eq!(saved.decor[0].dyes, changes);
    assert!(!commit_dyes(&db, owner, &preview, &catalog).unwrap());
    assert_eq!(crate::persistence::load_inventory(&db, 30), paid);
    drop(db);
    let db = Database::open(&path).unwrap();
    assert_eq!(
        snapshot(&db, owner, Some(house_id), None).unwrap().layout,
        Some(saved)
    );
    let clear = preview_dyes(
        &db,
        owner,
        house_id,
        1,
        &[DecorDye {
            slot_id: 28,
            color_id: None,
        }],
        &catalog,
    )
    .unwrap();
    assert!(commit_dyes(&db, owner, &clear, &catalog).unwrap());
    assert_eq!(crate::persistence::load_inventory(&db, 30), paid);
    let dyes = snapshot(&db, owner, Some(house_id), None)
        .unwrap()
        .layout
        .unwrap()
        .decor[0]
        .dyes
        .clone();
    assert_eq!(dyes, vec![changes[1].clone()]);
    edit(
        &db,
        owner,
        &HousingCommand::RemoveDecor {
            house_id,
            instance_id: 1,
        },
    )
    .unwrap();
    let stored = snapshot(&db, owner, None, None).unwrap();
    assert_eq!(
        stored.stored,
        vec![StoredDecor {
            decor_id: 374,
            quantity: 1
        }]
    );
    assert_eq!(stored.stored_variants.len(), 1);
    let variant = stored.stored_variants[0].clone();
    assert_eq!(variant.dyes, dyes);
    drop(db);
    let db = Database::open(&path).unwrap();
    assert_eq!(
        snapshot(&db, owner, None, None).unwrap().stored_variants,
        vec![variant.clone()]
    );
    let placement = HousingCommand::PlaceDecor {
        house_id,
        decor_id: 374,
        variant_id: Some(variant.variant_id),
        transform: DecorTransform {
            position: [1.25, 2.0, -3.0],
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
            scale: 1.0,
        },
    };
    let undyed = HousingCommand::PlaceDecor {
        house_id,
        decor_id: 374,
        variant_id: None,
        transform: DecorTransform {
            position: [1.25, 2.0, -3.0],
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
            scale: 1.0,
        },
    };
    assert!(edit(&db, owner, &undyed).is_err());
    assert!(edit(&db, visitor, &placement).is_err());
    edit(&db, owner, &placement).unwrap();
    let replaced = snapshot(&db, owner, Some(house_id), None).unwrap();
    assert!(replaced.stored.is_empty());
    assert!(replaced.stored_variants.is_empty());
    assert_eq!(replaced.layout.unwrap().decor[0].dyes, dyes);
    assert_eq!(crate::persistence::load_inventory(&db, 30), paid);
}

#[test]
fn housing_dye_failure_after_first_item_rolls_back_real_bag_placements() {
    let dir = tempfile::tempdir().unwrap();
    let db = fixture(&dir.path().join("atomic-bags.redb"));
    let (owner, house_id) = placed_shelf(&db);
    let catalog = dye_source();
    let mut inventory = crate::persistence::load_inventory(&db, 30);
    inventory.items = vec![dye_item(701, 274470, 1), dye_item(702, 274473, 1)];
    crate::persistence::save_inventory(&db, 30, &inventory);
    let placements = vec![
        inventory_store::ItemPlacementData {
            item_guid: 701,
            bag: 0,
            slot: 0,
        },
        inventory_store::ItemPlacementData {
            item_guid: 702,
            bag: 0,
            slot: 1,
        },
    ];
    let txn = db.begin_write().unwrap();
    inventory_store::write_item_definition_source(&txn, 702, ItemDefinitionSource::Forever70205);
    txn.open_table(inventory_store::INVENTORY_LAYOUTS)
        .unwrap()
        .insert(30, bitcode::encode(&placements).as_slice())
        .unwrap();
    crate::persistence_commit::commit(txn).unwrap();
    // Green274470 is consumed first, including its bag placement; Red274473 then refuses its foreign definition.
    let changes = [
        shared::protocol::DecorDye {
            slot_id: 28,
            color_id: Some(24),
        },
        shared::protocol::DecorDye {
            slot_id: 29,
            color_id: Some(23),
        },
    ];
    let preview = preview_dyes(&db, owner, house_id, 1, &changes, &catalog).unwrap();
    assert!(commit_dyes(&db, owner, &preview, &catalog).is_err());
    assert_eq!(crate::persistence::load_inventory(&db, 30), inventory);
    let txn = db.begin_read().unwrap();
    let table = txn.open_table(inventory_store::INVENTORY_LAYOUTS).unwrap();
    let stored: Vec<inventory_store::ItemPlacementData> =
        bitcode::decode(table.get(30).unwrap().unwrap().value()).unwrap();
    assert_eq!(stored, placements);
    assert!(
        snapshot(&db, owner, Some(house_id), None)
            .unwrap()
            .layout
            .unwrap()
            .decor[0]
            .dyes
            .is_empty()
    );
}

#[test]
fn housing_dye_schema2_migration_preserves_existing_undyed_counts_and_permissions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("schema2.redb");
    let db = fixture(&path);
    let (owner, house_id) = placed_shelf(&db);
    let mut inventory = crate::persistence::load_inventory(&db, 30);
    inventory.items.push(dye_item(101, 245384, 2));
    crate::persistence::save_inventory(&db, 30, &inventory);
    for _ in 0..2 {
        redeem(
            &db,
            owner,
            101,
            &RedemptionRule {
                item_id: 245384,
                decor_id: 374,
            },
        )
        .unwrap();
    }
    set_access(
        &db,
        owner,
        house_id,
        shared::protocol::HouseAccess {
            friends: true,
            ..Default::default()
        },
    )
    .unwrap();
    let before = snapshot(&db, owner, Some(house_id), None).unwrap();
    assert_eq!(
        before.stored,
        vec![StoredDecor {
            decor_id: 374,
            quantity: 2
        }]
    );
    let txn = db.begin_write().unwrap();
    let mut value = {
        let table = txn.open_table(STATE).unwrap();
        serde_json::from_slice::<serde_json::Value>(table.get(STATE_ROW).unwrap().unwrap().value())
            .unwrap()
    };
    value["schema_version"] = serde_json::json!(2);
    value.as_object_mut().unwrap().remove("storage_variants");
    for house in value["houses"].as_object_mut().unwrap().values_mut() {
        for decor in house["decor"].as_array_mut().unwrap() {
            decor.as_object_mut().unwrap().remove("dyes");
        }
    }
    txn.open_table(STATE)
        .unwrap()
        .insert(STATE_ROW, serde_json::to_vec(&value).unwrap().as_slice())
        .unwrap();
    migration::migrate(&txn, &rusqlite::Connection::open_in_memory().unwrap()).unwrap();
    crate::persistence_commit::commit(txn).unwrap();
    drop(db);
    let db = Database::open(&path).unwrap();
    assert_eq!(snapshot(&db, owner, Some(house_id), None).unwrap(), before);
    edit(
        &db,
        owner,
        &HousingCommand::RemoveDecor {
            house_id,
            instance_id: 1,
        },
    )
    .unwrap();
    let stored = snapshot(&db, owner, None, None).unwrap();
    assert_eq!(
        stored.stored,
        vec![StoredDecor {
            decor_id: 374,
            quantity: 3
        }]
    );
    assert!(stored.stored_variants.is_empty());
    edit(
        &db,
        owner,
        &HousingCommand::PlaceDecor {
            house_id,
            decor_id: 374,
            variant_id: None,
            transform: DecorTransform {
                position: [1.25, 2.0, -3.0],
                yaw: 0.0,
                pitch: 0.0,
                roll: 0.0,
                scale: 1.0,
            },
        },
    )
    .unwrap();
    assert!(
        snapshot(&db, owner, Some(house_id), None)
            .unwrap()
            .layout
            .unwrap()
            .decor[0]
            .dyes
            .is_empty()
    );
}
