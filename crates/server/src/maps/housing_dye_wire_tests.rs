use super::*;
use crate::persistence::{GameDatabase, InventoryData, InventoryItemData, housing_store as store};
use shared::protocol::{DecorDye, DecorTransform};

fn dye_world_db() -> rusqlite::Connection {
    let source = crate::housing_catalog::tests::catalog_fixture();
    for (table, id, fields) in [
        (
            "HouseDecor",
            374,
            serde_json::json!({"ModelType":"1","InitialScale":"1"}),
        ),
        (
            "DecorDyeSlot",
            28,
            serde_json::json!({"HouseDecorID":"374","DyeColorCategoryID":"2","Channel":"0","OrderIndex":"0"}),
        ),
        (
            "DyeColor",
            23,
            serde_json::json!({"DyeColorCategoryID":"1","ItemID":"274473"}),
        ),
    ] {
        source.execute("INSERT INTO housing_catalog_record(build_key,table_name,record_id,fields_json) VALUES(?,?,?,?)",rusqlite::params![store::BUILD_KEY,table,id,fields.to_string()]).unwrap();
    }
    source
}

fn dye_editor(db: redb::Database) -> (Harness, Traveler, u64) {
    crate::persistence::create_tables(&db);
    crate::persistence::save_inventory(
        &db,
        30,
        &InventoryData {
            gold: 20_000_000,
            items: vec![
                InventoryItemData {
                    item_guid: 100,
                    item_id: 245384,
                    name: "Sturdy Wooden Shelf".into(),
                    quality: 1,
                    required_level: 0,
                    stack_count: 1,
                    vendor_sell_price: 0,
                    unique_auction: false,
                },
                InventoryItemData {
                    item_guid: 701,
                    item_id: 274473,
                    name: "Red Housing Dye".into(),
                    quality: 1,
                    required_level: 0,
                    stack_count: 1,
                    vendor_sell_price: 0,
                    unique_auction: false,
                },
            ],
        },
    );
    let source = dye_world_db();
    let source = crate::housing_catalog::HousingCatalog::read(&source).unwrap();
    let actor = store::Actor {
        account_id: 30,
        character_id: 30,
        neighborhood_id: Some(1),
        inside_house: None,
    };
    let house = store::purchase(&db, actor, source.plot(417).unwrap()).unwrap();
    store::redeem(
        &db,
        actor,
        100,
        &store::RedemptionRule {
            item_id: 245384,
            decor_id: 374,
        },
    )
    .unwrap();
    let mut h = housing_harness(db);
    h.app
        .insert_resource(crate::housing::HousingAvailability(Ok(source)));
    let owner = h.spawn(30, InMap::continent(0), STOCKADE_ENTRANCE_TRIGGER);
    h.app
        .world_mut()
        .entity_mut(owner.connection)
        .insert((pair::<HousingRequest>(), pair::<HousingReply>()));
    assert_eq!(
        h.housing_request(
            &owner,
            HousingCommand::EnterHouse {
                house_id: house.house_id
            }
        )
        .error,
        None
    );
    h.ack(&owner);
    assert_eq!(
        h.housing_request(
            &owner,
            HousingCommand::PlaceDecor {
                house_id: house.house_id,
                decor_id: 374,
                variant_id: None,
                transform: DecorTransform {
                    position: [-996.0, -2.0, 1000.0],
                    yaw: 0.0,
                    pitch: 0.0,
                    roll: 0.0,
                    scale: 1.0
                }
            }
        )
        .error,
        None
    );
    (h, owner, house.house_id)
}

#[derive(Clone, Copy)]
enum PriorDyeState {
    Available,
    Removed,
    Applied,
}

#[test]
fn housing_dye_commit_waits_for_fifo_and_preserves_earlier_wallet() {
    assert_dye_commit_fifo(PriorDyeState::Available);
}

#[test]
fn housing_dye_failed_fifo_commit_keeps_preview_and_consumes_nothing() {
    assert_dye_commit_fifo(PriorDyeState::Removed);
}

#[test]
fn housing_dye_fifo_commit_treats_an_earlier_color_change_as_noop() {
    assert_dye_commit_fifo(PriorDyeState::Applied);
}

fn assert_dye_commit_fifo(prior_state: PriorDyeState) {
    use std::time::{Duration, Instant};
    let dir = tempfile::tempdir().unwrap();
    let db = redb::Database::create(dir.path().join("fifo-dyes.redb")).unwrap();
    let (mut h, owner, house_id) = dye_editor(db);
    let staged = h.housing_request(
        &owner,
        HousingCommand::PreviewDecorDyes {
            house_id,
            instance_id: 1,
            changes: vec![DecorDye {
                slot_id: 28,
                color_id: Some(23),
            }],
        },
    );
    assert_eq!(staged.error, None);
    let prior_preview = staged.state.dye_preview.clone().unwrap();
    let prior_dyes = h
        .app
        .world()
        .resource::<crate::housing::HousingAvailability>()
        .0
        .as_ref()
        .unwrap()
        .dyes
        .clone();
    let database = h.app.world().resource::<GameDatabase>();
    let held = database.db.begin_write().unwrap();
    let prior = database.writer.enqueue("earlier dye wallet", move |db| {
        let mut inventory = crate::persistence::load_inventory(db, 30);
        inventory.gold = 777;
        if matches!(prior_state, PriorDyeState::Removed) {
            inventory.items.clear();
        }
        crate::persistence::save_inventory(db, 30, &inventory);
        let changed = if matches!(prior_state, PriorDyeState::Applied) {
            store::commit_dyes(
                db,
                store::Actor {
                    account_id: 30,
                    character_id: 30,
                    neighborhood_id: None,
                    inside_house: Some(house_id),
                },
                &prior_preview,
                &prior_dyes,
            )?
        } else {
            false
        };
        Ok(changed)
    });
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        held.abort().unwrap();
    });
    h.app
        .world_mut()
        .get_mut::<MessageSender<HousingRequest>>(owner.connection)
        .unwrap()
        .send::<HousingChannel>(HousingRequest {
            request_id: 91,
            command: HousingCommand::CommitDecorDyes {
                house_id,
                instance_id: 1,
            },
        });
    let started = Instant::now();
    h.advance(3);
    let elapsed = started.elapsed();
    let premature = h
        .app
        .world_mut()
        .get_mut::<MessageReceiver<HousingReply>>(owner.connection)
        .unwrap()
        .receive()
        .next();
    release.join().unwrap();
    assert!(
        elapsed < Duration::from_millis(50),
        "housing dye tick waited: {elapsed:?}"
    );
    assert!(premature.is_none(), "dye reply preceded durable commit");
    let deadline = Instant::now() + Duration::from_secs(3);
    let saved = loop {
        h.advance(1);
        if let Some(reply) = h
            .app
            .world_mut()
            .get_mut::<MessageReceiver<HousingReply>>(owner.connection)
            .unwrap()
            .receive()
            .next()
        {
            break reply;
        }
        assert!(Instant::now() < deadline, "no durable dye result");
        std::thread::sleep(Duration::from_millis(1));
    };
    assert_eq!(
        prior.result(),
        Some(Ok(matches!(prior_state, PriorDyeState::Applied)))
    );
    assert_eq!(saved.request_id, 91);
    if !matches!(prior_state, PriorDyeState::Removed) {
        assert_eq!(saved.error, None);
        assert!(saved.state.dye_preview.is_none());
        assert_eq!(
            saved.state.layout.unwrap().decor[0].dyes,
            vec![DecorDye {
                slot_id: 28,
                color_id: Some(23)
            }]
        );
    } else {
        assert!(saved.error.unwrap().contains("Not enough owned Retail dye"));
        assert_eq!(
            saved.state.dye_preview.unwrap().changes,
            vec![DecorDye {
                slot_id: 28,
                color_id: Some(23)
            }]
        );
        assert!(saved.state.layout.unwrap().decor[0].dyes.is_empty());
    }
    let inventory =
        crate::persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, 30);
    assert_eq!(inventory.gold, 777);
    assert!(inventory.items.is_empty());
}

#[test]
fn housing_dye_wire_preview_cancel_and_player_replacement_never_spend_or_restore_drafts() {
    let dir = tempfile::tempdir().unwrap();
    let db = redb::Database::create(dir.path().join("wire-dyes.redb")).unwrap();
    let (mut h, owner, house_id) = dye_editor(db);
    let preview = HousingCommand::PreviewDecorDyes {
        house_id,
        instance_id: 1,
        changes: vec![DecorDye {
            slot_id: 28,
            color_id: Some(23),
        }],
    };
    let before =
        crate::persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, 30);
    let staged = h.housing_request(&owner, preview.clone());
    assert_eq!(staged.error, None);
    assert_eq!(
        staged.state.dye_preview.unwrap().changes,
        vec![DecorDye {
            slot_id: 28,
            color_id: Some(23)
        }]
    );
    assert!(staged.state.layout.unwrap().decor[0].dyes.is_empty());
    assert_eq!(
        crate::persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, 30),
        before
    );
    let cancelled = h.housing_request(&owner, HousingCommand::CancelDecorDyes);
    assert_eq!(cancelled.error, None);
    assert!(cancelled.state.dye_preview.is_none());
    let commit = HousingCommand::CommitDecorDyes {
        house_id,
        instance_id: 1,
    };
    assert!(
        h.housing_request(&owner, commit.clone())
            .error
            .unwrap()
            .contains("No active")
    );
    assert_eq!(h.housing_request(&owner, preview.clone()).error, None);
    let map = *h.app.world().get::<InMap>(owner.player).unwrap();
    let replacement = h.app.world_mut().spawn((map, ClientId(30))).id();
    h.app
        .world_mut()
        .entity_mut(owner.connection)
        .insert(crate::networking::PlayerLink(replacement));
    let stale = h.housing_request(&owner, commit.clone());
    assert!(stale.error.unwrap().contains("No active"));
    assert!(stale.state.dye_preview.is_none());
    assert_eq!(
        crate::persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, 30),
        before
    );
    assert_eq!(h.housing_request(&owner, preview).error, None);
    let saved = h.housing_request(&owner, commit.clone());
    assert_eq!(saved.error, None);
    assert!(saved.state.dye_preview.is_none());
    assert_eq!(
        saved.state.layout.unwrap().decor[0].dyes,
        vec![DecorDye {
            slot_id: 28,
            color_id: Some(23)
        }]
    );
    assert!(
        crate::persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, 30)
            .items
            .is_empty()
    );
    assert!(
        h.housing_request(&owner, commit)
            .error
            .unwrap()
            .contains("No active")
    );
}
