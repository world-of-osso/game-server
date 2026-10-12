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
        owner_type: shared::protocol::HousingPlotOwnerType::None,
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
fn housing_access_settings_survive_reopen_and_check_owner_friends_not_visitor_claims() {
    use shared::protocol::HouseAccess;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("access.redb");
    let db = fixture(&path);
    let owner = actor(1, 30);
    let visitor = actor(2, 31);
    let property = purchase(&db, owner, &plot()).unwrap();
    let (friend_id, _) =
        crate::persistence::create_character(&db, 2, "Accessfriend", 1, 2).unwrap();
    let friend = actor(2, friend_id);
    let access = HouseAccess {
        friends: true,
        ..HouseAccess::default()
    };
    assert!(set_access(&db, friend, property.house_id, access).is_err());
    set_access(&db, owner, property.house_id, access).unwrap();
    assert!(house(&db, friend, property.house_id, None).is_err());
    crate::persistence::save_friends(
        &db,
        30,
        &crate::friends::PlayerFriendsState {
            entries: vec![crate::friends::FriendRecord {
                name: "Accessfriend".into(),
                note: String::new(),
            }],
        },
    );
    let view = house(&db, friend, property.house_id, None).unwrap();
    assert!(!view.can_edit);
    save_house_location(&db, friend, property.house_id, None).unwrap();
    assert!(house(&db, visitor, property.house_id, None).is_err());
    drop(db);
    let db = Database::open(&path).unwrap();
    assert_eq!(
        snapshot(&db, owner, None, None).unwrap().owned_houses[0].house_access,
        Some(access)
    );
    assert_eq!(
        load_house_location(&db, friend_id, None).unwrap().house_id,
        property.house_id
    );
    crate::persistence::save_friends(&db, 30, &crate::friends::PlayerFriendsState::default());
    assert!(load_house_location(&db, friend_id, None).is_err());
    set_access(&db, owner, property.house_id, HouseAccess::default()).unwrap();
    assert!(house(&db, friend, property.house_id, None).is_err());
    assert!(house(&db, owner, property.house_id, None).unwrap().can_edit);
    set_access(
        &db,
        owner,
        property.house_id,
        HouseAccess {
            anyone: true,
            neighbors: true,
            guild: true,
            friends: true,
            party: true,
        },
    )
    .unwrap();
    assert!(house(&db, visitor, property.house_id, None).is_ok());
}

#[test]
fn housing_limited_access_rechecks_neighbors_guild_and_current_party() {
    use shared::protocol::HouseAccess;
    let dir = tempfile::tempdir().unwrap();
    let db = fixture(&dir.path().join("social.redb"));
    let (owner_id, _) = crate::persistence::create_character(&db, 1, "Socialowner", 1, 2).unwrap();
    let (visitor_id, _) =
        crate::persistence::create_character(&db, 2, "Socialguest", 1, 2).unwrap();
    for id in [owner_id, visitor_id] {
        crate::persistence::save_inventory(
            &db,
            id,
            &InventoryData {
                gold: 20_000_000,
                items: vec![],
            },
        );
    }
    let owner = actor(1, owner_id);
    let visitor = actor(2, visitor_id);
    let property = purchase(&db, owner, &plot()).unwrap();
    set_access(
        &db,
        owner,
        property.house_id,
        HouseAccess {
            neighbors: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(house(&db, visitor, property.house_id, None).is_err());
    let other = HousingPlot {
        plot_id: 418,
        plot_index: 1,
        ..plot()
    };
    purchase(&db, visitor, &other).unwrap();
    assert!(
        !house(&db, visitor, property.house_id, None)
            .unwrap()
            .can_edit
    );
    set_access(
        &db,
        owner,
        property.house_id,
        HouseAccess {
            guild: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(house(&db, visitor, property.house_id, None).is_err());
    crate::persistence::set_character_guild_by_id(&db, owner_id, 71, "Housingguild").unwrap();
    crate::persistence::set_character_guild_by_id(&db, visitor_id, 72, "Otherguild").unwrap();
    assert!(house(&db, visitor, property.house_id, None).is_err());
    crate::persistence::set_character_guild_by_id(&db, visitor_id, 71, "Housingguild").unwrap();
    assert!(house(&db, visitor, property.house_id, None).is_ok());
    crate::persistence::set_character_guild_by_id(&db, visitor_id, 72, "Otherguild").unwrap();
    assert!(house(&db, visitor, property.house_id, None).is_err());
    set_access(
        &db,
        owner,
        property.house_id,
        HouseAccess {
            party: true,
            ..Default::default()
        },
    )
    .unwrap();
    let mut groups = crate::group::GroupRegistry::default();
    let online: crate::group::OnlineView = [(owner_id, "Socialowner"), (visitor_id, "Socialguest")]
        .into_iter()
        .map(|(id, name)| {
            (
                id,
                crate::group::OnlineMember {
                    entity: id + 100,
                    name: name.into(),
                    class: 2,
                    level: 10,
                    portrait: Default::default(),
                    dungeon_difficulty: crate::maps::store::DIFFICULTY_NORMAL,
                    raid_difficulty: Default::default(),
                },
            )
        })
        .collect();
    assert!(house(&db, visitor, property.house_id, Some(&groups)).is_err());
    groups.invite(owner_id, "Socialguest", &online);
    assert!(house(&db, visitor, property.house_id, Some(&groups)).is_err());
    groups.respond_invite(visitor_id, true);
    assert!(house(&db, visitor, property.house_id, Some(&groups)).is_ok());
    save_house_location(&db, visitor, property.house_id, Some(&groups)).unwrap();
    assert!(load_house_location(&db, visitor_id, Some(&groups)).is_ok());
    groups.leave(visitor_id);
    assert!(house(&db, visitor, property.house_id, Some(&groups)).is_err());
    assert!(load_house_location(&db, visitor_id, Some(&groups)).is_err());
}

#[test]
fn housing_access_migration_preserves_unknown_closed_open_and_decor() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("migration.redb");
    let db = fixture(&path);
    let legacy = serde_json::json!({
        "schema_version":1, "build_key":BUILD_KEY, "next_house_id":4, "next_instance_id":2,
        "storage":{"1":{"2330":1}},
        "houses": {
            "1":{"house_id":1,"account_id":1,"owner_character_id":30,"neighborhood_id":1,"plot_id":417,"access_anyone":null,"decor":[]},
            "2":{"house_id":2,"account_id":2,"owner_character_id":31,"neighborhood_id":1,"plot_id":418,"access_anyone":false,"decor":[]},
            "3":{"house_id":3,"account_id":3,"owner_character_id":32,"neighborhood_id":1,"plot_id":419,"access_anyone":true,"decor":[{"instance_id":1,"decor_id":1693,"transform":{"position":[1.25,2.0,-3.0],"yaw":0.5,"pitch":0.25,"roll":-0.5,"scale":1.25}}]}
        }
    });
    let txn = db.begin_write().unwrap();
    txn.open_table(STATE)
        .unwrap()
        .insert(STATE_ROW, serde_json::to_vec(&legacy).unwrap().as_slice())
        .unwrap();
    migration::migrate(&txn, &rusqlite::Connection::open_in_memory().unwrap()).unwrap();
    txn.commit().unwrap();
    drop(db);
    let db = Database::open(&path).unwrap();
    assert_eq!(
        snapshot(&db, actor(1, 30), None, None)
            .unwrap()
            .owned_houses[0]
            .house_access,
        None
    );
    assert_eq!(
        snapshot(&db, actor(2, 31), None, None)
            .unwrap()
            .owned_houses[0]
            .house_access,
        Some(Default::default())
    );
    assert!(house(&db, actor(1, 30), 2, None).is_err());
    let open = house(&db, actor(1, 30), 3, None).unwrap();
    assert_eq!(
        open.house_access,
        Some(shared::protocol::HouseAccess {
            anyone: true,
            neighbors: true,
            guild: true,
            friends: true,
            party: true
        })
    );
    let layout = snapshot(&db, actor(1, 30), Some(3), None)
        .unwrap()
        .layout
        .unwrap();
    assert_eq!(layout.decor[0].transform.scale, 1.25);
    assert_eq!(layout.decor[0].transform.position, [1.25, 2.0, -3.0]);
    let txn = db.begin_write().unwrap();
    migration::migrate(&txn, &rusqlite::Connection::open_in_memory().unwrap()).unwrap();
    txn.commit().unwrap();
    assert_eq!(
        snapshot(&db, actor(1, 30), Some(3), None).unwrap().layout,
        Some(layout)
    );
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
    assert!(snapshot(&db, actor(2, 31), Some(house.house_id), None).is_err());
    assert!(snapshot(&db, owner, None, None).unwrap().stored.is_empty());
    drop(db);
    let db = Database::open(&path).unwrap();
    let state = snapshot(&db, actor(1, 32), None, None).unwrap();
    assert_eq!(state.owned_houses.len(), 1);
    assert_eq!(state.owned_houses[0].house_id, house.house_id);
    assert!(state.owned_houses[0].can_edit);
    assert_eq!(state.owned_houses[0].house_access, None);
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
    assert_eq!(
        snapshot(&db, owner, None, None).unwrap().stored[0].quantity,
        1
    );
    edit(
        &db,
        owner,
        &HousingCommand::PlaceDecor {
            house_id: house.house_id,
            decor_id: 1693,
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
    let state = snapshot(&db, owner, Some(house.house_id), None).unwrap();
    assert!(state.stored.is_empty());
    let placed = vec![StoredDecor {
        decor_id: 1693,
        quantity: 1,
    }];
    assert_eq!(state.placed, placed);
    // Retail totalNumPlaced is account-wide, also outside any house layout.
    assert_eq!(snapshot(&db, owner, None, None).unwrap().placed, placed);
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
                    yaw: 0.0,
                    pitch: 0.0,
                    roll: 0.0,
                    scale: 1.0,
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
    set_access(
        &db,
        owner,
        house.house_id,
        shared::protocol::HouseAccess {
            anyone: true,
            ..Default::default()
        },
    )
    .unwrap();
    let mut visitor = actor(2, 31);
    visitor.inside_house = Some(house.house_id);
    let visited = snapshot(&db, visitor, Some(house.house_id), None).unwrap();
    assert!(!visited.layout.as_ref().unwrap().can_edit);
    assert!(visited.stored.is_empty());
    assert!(visited.placed.is_empty());
    assert!(
        set_access(
            &db,
            visitor,
            house.house_id,
            shared::protocol::HouseAccess {
                anyone: true,
                ..Default::default()
            }
        )
        .is_err()
    );
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
    let layout = snapshot(&db, visitor, Some(house.house_id), None)
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
    let removed = snapshot(&db, owner, Some(house.house_id), None).unwrap();
    assert_eq!(removed.stored[0].quantity, 1);
    assert!(removed.placed.is_empty());
    assert!(
        snapshot(&db, visitor, Some(house.house_id), None)
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

#[test]
fn housing_precision_migration_is_atomic_sourced_and_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("housing.redb");
    let db = fixture(&path);
    let source = crate::housing_catalog::tests::catalog_fixture();
    source.execute("INSERT INTO housing_catalog_record VALUES (?, 'HouseDecor', 1693, ?)", rusqlite::params![BUILD_KEY, r#"{"InitialRotation_0":"0.0","InitialRotation_1":"0.0","InitialRotation_2":"0.0","InitialScale":"1.0"}"#]).unwrap();
    // Actual pre-precision schema and paid plot/account identities. No newly
    // initialized typed State can stand in for this on-disk migration boundary.
    let legacy = serde_json::json!({
        "build_key": BUILD_KEY, "next_house_id":2, "next_instance_id":8,
        "houses":{"1":{"house_id":1,"account_id":1,"owner_character_id":30,
            "neighborhood_id":1,"plot_id":417,"access_anyone":true,
            "decor":[{"instance_id":7,"decor_id":1693,"transform":{"position":[1.25,-2.0,3.0],"yaw":0.5}}]}},
        "storage":{"1":{"2330":1}}
    });
    let bytes = serde_json::to_vec(&legacy).unwrap();
    let txn = db.begin_write().unwrap();
    txn.open_table(STATE)
        .unwrap()
        .insert(STATE_ROW, bytes.as_slice())
        .unwrap();
    txn.commit().unwrap();
    source
        .execute(
            "DELETE FROM housing_catalog_record WHERE table_name='HouseDecor'",
            [],
        )
        .unwrap();
    let txn = db.begin_write().unwrap();
    assert!(migration::migrate(&txn, &source).is_err());
    drop(txn);
    assert_eq!(
        db.begin_read()
            .unwrap()
            .open_table(STATE)
            .unwrap()
            .get(STATE_ROW)
            .unwrap()
            .unwrap()
            .value(),
        bytes
    );
    source.execute("INSERT INTO housing_catalog_record VALUES (?, 'HouseDecor', 1693, ?)", rusqlite::params![BUILD_KEY, r#"{"InitialRotation_0":"0.0","InitialRotation_1":"0.0","InitialRotation_2":"0.0","InitialScale":"1.0"}"#]).unwrap();
    let txn = db.begin_write().unwrap();
    migration::migrate(&txn, &source).unwrap();
    txn.commit().unwrap();
    drop(db);
    let db = Database::open(&path).unwrap();
    let owner = Actor {
        inside_house: Some(1),
        ..actor(1, 30)
    };
    let state = snapshot(&db, owner, Some(1), None).unwrap();
    assert_eq!(
        state.stored,
        vec![StoredDecor {
            decor_id: 2330,
            quantity: 1
        }]
    );
    assert_eq!(
        state.placed,
        vec![StoredDecor {
            decor_id: 1693,
            quantity: 1
        }]
    );
    let decor = &state.layout.unwrap().decor[0];
    assert_eq!(decor.instance_id, 7);
    assert_eq!(
        decor.transform,
        DecorTransform {
            position: [1.25, -2.0, 3.0],
            yaw: 0.5,
            pitch: 0.0,
            roll: -0.0,
            scale: 1.0
        }
    );
    let transform = DecorTransform {
        position: [4.0, -1.5, 7.0],
        yaw: 0.75,
        pitch: 0.25,
        roll: -0.4,
        scale: 1.25,
    };
    edit(
        &db,
        owner,
        &HousingCommand::TransformDecor {
            house_id: 1,
            instance_id: 7,
            transform,
        },
    )
    .unwrap();
    let visitor = Actor {
        inside_house: Some(1),
        ..actor(2, 31)
    };
    assert!(
        edit(
            &db,
            visitor,
            &HousingCommand::TransformDecor {
                house_id: 1,
                instance_id: 7,
                transform
            }
        )
        .is_err()
    );
    let invalid = DecorTransform {
        scale: 0.0,
        ..transform
    };
    assert!(
        edit(
            &db,
            owner,
            &HousingCommand::TransformDecor {
                house_id: 1,
                instance_id: 7,
                transform: invalid
            }
        )
        .is_err()
    );
    drop(db);
    let db = Database::open(&path).unwrap();
    assert_eq!(
        snapshot(&db, owner, Some(1), None)
            .unwrap()
            .layout
            .unwrap()
            .decor[0]
            .transform,
        transform
    );
    let txn = db.begin_write().unwrap();
    migration::migrate(&txn, &source).unwrap();
    txn.commit().unwrap();
    assert_eq!(
        snapshot(&db, owner, Some(1), None)
            .unwrap()
            .layout
            .unwrap()
            .decor[0]
            .transform,
        transform
    );
}
