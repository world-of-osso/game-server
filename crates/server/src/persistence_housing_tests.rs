use super::*;
use crate::persistence::{INVENTORIES, InventoryData, InventoryItemData};
use redb::Database;
use shared::protocol::{DecorTransform, HousingCommand, HousingPlot};

fn plot() -> HousingPlot {
    HousingPlot {
        plot_id: 417,
        neighborhood_id: 1,
        plot_index: 0,
        name: String::new(),
        cornerstone: [2942.744, -214.26563, 113.11285],
        teleport: [2939.3438, -212.2554, 113.321],
        cost: 10_000_000,
        house_id: None,
        owner_character_id: None,
    }
}

fn actor(account: u64, character: u64) -> Actor {
    Actor {
        account_id: account,
        character_id: character,
        neighborhood_id: Some(1),
        inside_house: None,
    }
}

fn fixture(path: &std::path::Path) -> Database {
    let db = Database::create(path).unwrap();
    crate::persistence::create_tables(&db);
    let txn = db.begin_write().unwrap();
    let mut inventories = txn.open_table(INVENTORIES).unwrap();
    for character in [30, 31] {
        let inventory = InventoryData {
            gold: 20_000_000,
            items: vec![],
        };
        inventories
            .insert(character, bitcode::encode(&inventory).as_slice())
            .unwrap();
    }
    drop(inventories);
    txn.commit().unwrap();
    db
}

#[test]
fn housing_purchase_is_account_owned_paid_once_and_survives_database_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("housing.redb");
    let db = fixture(&path);
    let owner = actor(1, 30);
    let house = purchase(&db, owner, &plot()).unwrap();
    assert_eq!(house.plot_id, 417);
    assert_eq!(crate::persistence::load_inventory(&db, 30).gold, 10_000_000);
    assert!(purchase(&db, actor(2, 31), &plot()).is_err());
    assert_eq!(crate::persistence::load_inventory(&db, 31).gold, 20_000_000);
    assert!(snapshot(&db, actor(2, 31), Some(house.house_id)).is_err());
    assert!(snapshot(&db, owner, None).unwrap().stored.is_empty());
    drop(db);
    let db = Database::open(&path).unwrap();
    let state = snapshot(&db, actor(1, 32), None).unwrap();
    assert_eq!(state.owned_houses.len(), 1);
    assert_eq!(state.owned_houses[0].house_id, house.house_id);
    assert!(state.owned_houses[0].can_edit);
    assert_eq!(state.owned_houses[0].house_access_anyone, None);
    assert!(!state.owned_houses[0].exterior_grant_available);
    assert!(!state.owned_houses[0].interior_grant_available);
}

#[test]
fn housing_one_vendor_item_redeems_then_layout_conserves_quantity_and_visit_is_read_only() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("housing.redb");
    let db = fixture(&path);
    let mut owner = actor(1, 30);
    let house = purchase(&db, owner, &plot()).unwrap();
    owner.inside_house = Some(house.house_id);
    let txn = db.begin_write().unwrap();
    let inventory = InventoryData {
        gold: 10_000_000,
        items: vec![InventoryItemData {
            item_guid: 100,
            item_id: 245259,
            name: "Small Val'sharah Bookcase".into(),
            quality: 2,
            required_level: 0,
            stack_count: 1,
            vendor_sell_price: 10000,
            unique_auction: false,
        }],
    };
    txn.open_table(INVENTORIES)
        .unwrap()
        .insert(30, bitcode::encode(&inventory).as_slice())
        .unwrap();
    txn.commit().unwrap();
    redeem(
        &db,
        owner,
        100,
        &RedemptionRule {
            item_id: 245259,
            decor_id: 1693,
        },
    )
    .unwrap();
    assert!(
        redeem(
            &db,
            owner,
            100,
            &RedemptionRule {
                item_id: 245259,
                decor_id: 1693
            }
        )
        .is_err()
    );
    assert!(crate::persistence::load_inventory(&db, 30).items.is_empty());
    assert_eq!(snapshot(&db, owner, None).unwrap().stored[0].quantity, 1);
    edit(
        &db,
        owner,
        &HousingCommand::PlaceDecor {
            house_id: house.house_id,
            decor_id: 1693,
            transform: DecorTransform {
                position: [1.25, 2.0, -3.0],
                yaw: 0.0,
            },
        },
    )
    .unwrap();
    let state = snapshot(&db, owner, Some(house.house_id)).unwrap();
    assert!(state.stored.is_empty());
    let placed = vec![StoredDecor {
        decor_id: 1693,
        quantity: 1,
    }];
    assert_eq!(state.placed, placed);
    // Retail totalNumPlaced is account-wide, also outside any house layout.
    assert_eq!(snapshot(&db, owner, None).unwrap().placed, placed);
    let instance = state.layout.unwrap().decor[0].instance_id;
    assert!(
        edit(
            &db,
            owner,
            &HousingCommand::PlaceDecor {
                house_id: house.house_id,
                decor_id: 1693,
                transform: DecorTransform {
                    position: [0.0; 3],
                    yaw: 0.0
                }
            }
        )
        .is_err()
    );
    edit(
        &db,
        owner,
        &HousingCommand::MoveDecor {
            house_id: house.house_id,
            instance_id: instance,
            position: [4.0, 2.0, -5.0],
        },
    )
    .unwrap();
    edit(
        &db,
        owner,
        &HousingCommand::RotateDecor {
            house_id: house.house_id,
            instance_id: instance,
            yaw: std::f32::consts::FRAC_PI_2,
        },
    )
    .unwrap();
    assert!(
        edit(
            &db,
            owner,
            &HousingCommand::MoveDecor {
                house_id: house.house_id,
                instance_id: instance,
                position: [f32::NAN, 0.0, 0.0]
            }
        )
        .is_err()
    );
    set_access(&db, owner, house.house_id, true).unwrap();
    let mut visitor = actor(2, 31);
    visitor.inside_house = Some(house.house_id);
    let visited = snapshot(&db, visitor, Some(house.house_id)).unwrap();
    assert!(!visited.layout.as_ref().unwrap().can_edit);
    assert!(visited.stored.is_empty());
    assert!(visited.placed.is_empty());
    assert!(set_access(&db, visitor, house.house_id, true).is_err());
    assert!(
        edit(
            &db,
            visitor,
            &HousingCommand::RemoveDecor {
                house_id: house.house_id,
                instance_id: instance
            }
        )
        .is_err()
    );
    drop(db);
    let db = Database::open(&path).unwrap();
    let layout = snapshot(&db, visitor, Some(house.house_id))
        .unwrap()
        .layout
        .unwrap();
    assert_eq!(layout.decor[0].transform.position, [4.0, 2.0, -5.0]);
    assert_eq!(layout.decor[0].transform.yaw, std::f32::consts::FRAC_PI_2);
    edit(
        &db,
        owner,
        &HousingCommand::RemoveDecor {
            house_id: house.house_id,
            instance_id: instance,
        },
    )
    .unwrap();
    let removed = snapshot(&db, owner, Some(house.house_id)).unwrap();
    assert_eq!(removed.stored[0].quantity, 1);
    assert!(removed.placed.is_empty());
    assert!(
        snapshot(&db, visitor, Some(house.house_id))
            .unwrap()
            .layout
            .unwrap()
            .decor
            .is_empty()
    );
}

#[test]
fn housing_competing_accounts_cannot_both_purchase_one_plot() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("housing.redb");
    let db = std::sync::Arc::new(fixture(&path));
    let jobs: Vec<_> = [actor(1, 30), actor(2, 31)]
        .into_iter()
        .map(|actor| {
            let db = db.clone();
            std::thread::spawn(move || purchase(&db, actor, &plot()))
        })
        .collect();
    let successes = jobs
        .into_iter()
        .map(|job| job.join().unwrap().is_ok())
        .filter(|success| *success)
        .count();
    assert_eq!(successes, 1);
    let paid = [30, 31]
        .into_iter()
        .map(|character| crate::persistence::load_inventory(&db, character).gold)
        .sum::<u64>();
    assert_eq!(paid, 30_000_000);
}
