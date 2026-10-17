use super::*;
use crate::bank_test_harness::{ACCOUNT, Harness};
use shared::components::{Health, Level, Npc, Position, UnitFactionTemplate};
use shared::protocol::AdventureGuideClock;

#[test]
fn db2_follow_install_trading_post_reads_next_build_and_rejects_mixed_sources() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE trading_post_source(name TEXT,build TEXT); INSERT INTO trading_post_source VALUES('PerksVendorCategory','12.1.5.70391'); CREATE TABLE perks_vendor_category(ID INTEGER,payload TEXT); CREATE TABLE perks_vendor_item(ID INTEGER,payload TEXT); CREATE TABLE perks_vendor_item_ui_group(ID INTEGER,payload TEXT);").unwrap();
    db.execute(
        "INSERT INTO perks_vendor_category VALUES(1,?)",
        [r#"{"ID":1,"DisplayName_lang":"Armor","DefaultUIModelSceneID":23}"#],
    )
    .unwrap();
    let loaded = catalog::load(&db).unwrap();
    assert_eq!(loaded.categories[0].name, "Armor");
    db.execute(
        "INSERT INTO trading_post_source VALUES('PerksVendorItem','12.1.0.69933')",
        [],
    )
    .unwrap();
    assert!(catalog::load(&db).is_err());
}

fn prepare(h: &mut Harness, now: i64) {
    let path = std::env::var_os("TRADING_POST_WORLD_COPY")
        .expect("explicit private Trading Post world copy");
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let criteria = crate::achievement::load_achievement_catalog(&conn).unwrap();
    let journal_csv = crate::test_data::world_db_path()
        .parent()
        .unwrap()
        .join("db2/active/AdventureJournal.csv");
    let activities = crate::adventure_guide::load_catalog(&conn, &criteria, &journal_csv);
    h.app
        .insert_resource(criteria)
        .insert_resource(activities)
        .insert_resource(catalog::Catalog(catalog::load(&conn)))
        .insert_resource(crate::maps::ServerClock {
            fixed_unix: Some(now),
            offset_secs: 0,
        });
    crate::adventure_guide::AdventureGuidePlugin.build(&mut h.app);
    h.app.add_systems(
        Update,
        receive_requests.after(crate::npc_interaction::NpcInteractionsProcessed),
    );
    h.app.world_mut().entity_mut(h.player).insert(Level(80));
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<TradingPostRequest>::default(),
        MessageReceiver::<TradingPostRequest>::default(),
        MessageSender::<TradingPostSnapshot>::default(),
        MessageReceiver::<TradingPostSnapshot>::default(),
        MessageSender::<AdventureGuideRequest>::default(),
        MessageReceiver::<AdventureGuideRequest>::default(),
        MessageSender::<AdventureGuideSnapshot>::default(),
        MessageReceiver::<AdventureGuideSnapshot>::default(),
        MessageSender::<AdventureGuideClock>::default(),
        MessageReceiver::<AdventureGuideClock>::default(),
    ));
}
fn request(h: &mut Harness, command: TradingPostRequest) -> TradingPostSnapshot {
    h.app
        .world_mut()
        .get_mut::<MessageSender<TradingPostRequest>>(h.connection)
        .unwrap()
        .send::<InstanceChannel>(command);
    let mut replies = Vec::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while replies.is_empty() && std::time::Instant::now() < deadline {
        h.app.update();
        replies.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<TradingPostSnapshot>>(h.connection)
                .unwrap()
                .receive(),
        );
        std::thread::yield_now();
    }
    replies.pop().expect("Trading Post server reply")
}

#[test]
fn trading_post_query_never_waits_for_a_busy_durable_writer() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Tenderqueue").unwrap();
    prepare(&mut h, 1790812900);
    assert_eq!(request(&mut h, TradingPostRequest::Balance).tender, 0);
    let held = h.db().begin_write().unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        held.abort().unwrap();
    });
    h.app
        .world_mut()
        .get_mut::<MessageSender<TradingPostRequest>>(h.connection)
        .unwrap()
        .send::<InstanceChannel>(TradingPostRequest::Balance);
    let started = std::time::Instant::now();
    let mut replies = Vec::new();
    for _ in 0..4 {
        h.app.update();
        replies.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<TradingPostSnapshot>>(h.connection)
                .unwrap()
                .receive(),
        );
    }
    let elapsed = started.elapsed();
    let replies_before_commit = replies.len();
    release.join().unwrap();
    crate::test_data::drain_persistence(h.app.world().resource::<GameDatabase>());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while replies.is_empty() && std::time::Instant::now() < deadline {
        h.app.update();
        replies.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<TradingPostSnapshot>>(h.connection)
                .unwrap()
                .receive(),
        );
        std::thread::yield_now();
    }
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "Trading Post request tick waited: {elapsed:?}"
    );
    assert_eq!(
        replies_before_commit, 0,
        "no reply before settlement and request commit"
    );
    assert_eq!(replies.len(), 1);
    assert!(replies[0].error.is_none());
    assert_eq!(replies[0].tender, 0);
}

#[test]
fn trading_post_same_tick_earned_tender_funds_purchase_in_fifo_order() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Tenderorder").unwrap();
    prepare(&mut h, 1790812900);
    open_tawny(&mut h);
    assert_eq!(request(&mut h, TradingPostRequest::Balance).tender, 0);
    let held = h.db().begin_write().unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        held.abort().unwrap();
    });
    for asset in [117491, 117490, 117489, 213134] {
        let creature = h
            .app
            .world_mut()
            .spawn((
                Npc {
                    template_id: asset,
                    name: String::new(),
                },
                crate::maps::InMap::continent(0),
            ))
            .id();
        crate::achievement::record_creature_death(h.app.world_mut(), creature, Some(h.player));
    }
    // No App update separates the real earned credit from this purchase request.
    let purchased = request(&mut h, TradingPostRequest::Purchase { vendor_item_id: 4 });
    release.join().unwrap();
    assert!(purchased.error.is_none(), "{:?}", purchased.error);
    assert!(purchased.points >= 200);
    assert_eq!(
        purchased.tender, 0,
        "the purchase spends the newly earned100 Tender"
    );
    let row = purchased.items.iter().find(|row| row.id == 4).unwrap();
    assert!(row.purchased && row.collected);
    let committed = store::load(h.db(), ACCOUNT);
    assert_eq!(committed.tender, 0);
    assert!(committed.claimed.contains(&1));
    assert!(committed.purchases.contains_key(&4));
    assert!(
        crate::persistence::appearance_store::load_account_appearances(h.db(), ACCOUNT).has(37866)
    );
    drop(h);
    let db = redb::Database::open(file.path()).unwrap();
    assert_eq!(store::load(&db, ACCOUNT), committed);
    assert!(
        crate::persistence::appearance_store::load_account_appearances(&db, ACCOUNT).has(37866)
    );
}

#[test]
fn trading_post_real_gameplay_credit_awards_once_and_resets_at_midnight_not_daily_hour() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Tenderhunter").unwrap();
    let october = 1790812800;
    prepare(&mut h, october + 100);
    h.app
        .insert_resource(crate::guild_bank::DailyReset { hour_utc: 15 });
    assert_eq!(request(&mut h, TradingPostRequest::Query).tender, 0);
    // Authentic499's three hive Colossus assets plus518's follower dungeon boss.
    // Production achievement death boundary, not counters inserted by the client.
    for asset in [117491, 117490, 117489, 213134] {
        let creature = h
            .app
            .world_mut()
            .spawn((
                Npc {
                    template_id: asset,
                    name: String::new(),
                },
                crate::maps::InMap::continent(0),
            ))
            .id();
        crate::achievement::record_creature_death(h.app.world_mut(), creature, Some(h.player));
    }
    let earned = request(&mut h, TradingPostRequest::Query);
    assert!(
        earned.points >= 200,
        "actual supported activities: {}",
        earned.points
    );
    assert_eq!(earned.tender, 100);
    assert_eq!(request(&mut h, TradingPostRequest::Query).tender, 100);
    assert_eq!(
        earned
            .thresholds
            .iter()
            .filter(|threshold| threshold.claimed)
            .count(),
        1
    );
    h.app
        .world_mut()
        .resource_mut::<crate::maps::ServerClock>()
        .fixed_unix = Some(1793491200);
    let rollover = request(&mut h, TradingPostRequest::Query);
    assert_eq!(rollover.points, 0);
    assert_eq!(rollover.tender, 100);
    assert!(
        rollover
            .thresholds
            .iter()
            .all(|threshold| !threshold.claimed)
    );
}

#[test]
fn trading_post_source_vendor_wire_purchase_refuses_closed_and_out_of_range_interactions() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Tenderbuyer").unwrap();
    let october = 1790812800;
    prepare(&mut h, october + 100);
    let thresholds = vec![store::Threshold {
        id: 1,
        points: 200,
        tender: 100,
    }];
    store::settle(h.db(), ACCOUNT, october, 200, &thresholds);
    let refused = request(&mut h, TradingPostRequest::Purchase { vendor_item_id: 4 });
    assert!(refused.error.as_ref().unwrap().contains("No active"));
    assert_eq!(refused.tender, 100);
    assert!(refused.items.is_empty());
    let catalog = request(&mut h, TradingPostRequest::Query);
    assert_eq!(catalog.items.len(), 1321);
    assert!(
        catalog
            .items
            .iter()
            .find(|row| row.id == 4)
            .unwrap()
            .grant_unavailable
            .is_none()
    );
    let (name, flags, faction) = h
        .conn
        .query_row(
            "SELECT name,npcflag,faction FROM tdb_creature_template WHERE entry=185468",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)? as u64,
                    row.get::<_, u32>(2)?,
                ))
            },
        )
        .unwrap();
    let npc = h
        .app
        .world_mut()
        .spawn((
            Npc {
                template_id: 185468,
                name,
            },
            NpcFlags(flags),
            Position {
                x: 0.,
                y: 0.,
                z: 0.,
            },
            Health {
                current: 100.,
                max: 100.,
            },
            UnitFactionTemplate(faction),
            crate::maps::InMap::continent(0),
        ))
        .id();
    h.send::<_, InteractionChannel>(InteractNpc { npc: npc.to_bits() });
    let opened = h.take::<InteractionOpened>();
    assert!(opened.iter().any(|message|matches!(&message.kind,InteractionKind::Gossip(menu) if menu.options.iter().any(|row|row.text=="I'd like to see what you have to offer this month."))));
    h.send::<_, InteractionChannel>(SelectGossipOption {
        npc: npc.to_bits(),
        option_id: 0,
    });
    let frozen = request(
        &mut h,
        TradingPostRequest::Freeze {
            vendor_item_id: Some(4),
        },
    );
    assert_eq!(frozen.frozen_item_id, Some(4));
    let purchased = request(&mut h, TradingPostRequest::Purchase { vendor_item_id: 4 });
    assert!(purchased.error.is_none(), "{:?}", purchased.error);
    assert_eq!(purchased.tender, 0);
    let row = purchased.items.iter().find(|row| row.id == 4).unwrap();
    assert!(row.purchased && row.collected);
    assert_eq!(row.refund_seconds, 0);
    let refused = request(&mut h, TradingPostRequest::Refund { vendor_item_id: 4 });
    assert!(refused.error.as_ref().unwrap().contains("not refundable"));
    h.move_player(32.);
    let refused = request(
        &mut h,
        TradingPostRequest::Freeze {
            vendor_item_id: Some(5),
        },
    );
    assert!(refused.error.is_some());
    assert_eq!(refused.frozen_item_id, Some(4));
    assert!(
        crate::persistence::appearance_store::load_account_appearances(h.db(), ACCOUNT).has(37866)
    );
}

fn open_tawny(h: &mut Harness) {
    let (name, flags, faction) = h
        .conn
        .query_row(
            "SELECT name,npcflag,faction FROM tdb_creature_template WHERE entry=185468",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)? as u64,
                    row.get::<_, u32>(2)?,
                ))
            },
        )
        .unwrap();
    let npc = h
        .app
        .world_mut()
        .spawn((
            Npc {
                template_id: 185468,
                name,
            },
            NpcFlags(flags),
            Position {
                x: 0.,
                y: 0.,
                z: 0.,
            },
            Health {
                current: 100.,
                max: 100.,
            },
            UnitFactionTemplate(faction),
            crate::maps::InMap::continent(0),
        ))
        .id();
    h.send::<_, InteractionChannel>(InteractNpc { npc: npc.to_bits() });
    h.take::<InteractionOpened>();
    h.send::<_, InteractionChannel>(SelectGossipOption {
        npc: npc.to_bits(),
        option_id: 0,
    });
}

#[test]
fn trading_post_admin_tender_and_refund_window_enable_wire_refund() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Tenderadmin").unwrap();
    prepare(&mut h, 1790812800 + 100);
    let (account, _) =
        crate::persistence_accounts::register_account(h.db(), "fb_tender", "fbtest").unwrap();
    assert_eq!(
        account, ACCOUNT,
        "harness character belongs to the first account"
    );
    assert!(admin_grant_tender(h.db(), "no_such_account", 700).is_err());
    assert_eq!(admin_grant_tender(h.db(), "fb_tender", 700).unwrap(), 700);
    h.app.insert_resource(RefundWindow(600));
    let balance = request(&mut h, TradingPostRequest::Balance);
    assert_eq!((balance.tender, balance.items.len()), (700, 0));
    assert!(!balance.catalog_complete);
    open_tawny(&mut h);
    let catalog = request(&mut h, TradingPostRequest::Query);
    assert!(catalog.catalog_complete);
    assert_eq!(catalog.items.len(), 1321);
    let purchased = request(&mut h, TradingPostRequest::Purchase { vendor_item_id: 4 });
    assert!(purchased.error.is_none(), "{:?}", purchased.error);
    assert_eq!(purchased.tender, 600);
    // Only rows whose collection state the purchase can change, never the catalog.
    assert!(!purchased.catalog_complete);
    assert!(
        purchased.items.len() < 20,
        "partial reply has {} rows",
        purchased.items.len()
    );
    let frozen = request(
        &mut h,
        TradingPostRequest::Freeze {
            vendor_item_id: Some(4),
        },
    );
    assert_eq!((frozen.frozen_item_id, frozen.items.len()), (Some(4), 0));
    let row = purchased.items.iter().find(|row| row.id == 4).unwrap();
    assert!(row.purchased && row.collected);
    assert_eq!(row.refund_seconds, 600);
    assert!(
        purchased
            .source_gaps
            .iter()
            .any(|gap| gap.contains("600") && gap.contains("administrator"))
    );
    let refunded = request(&mut h, TradingPostRequest::Refund { vendor_item_id: 4 });
    assert!(refunded.error.is_none(), "{:?}", refunded.error);
    assert_eq!(refunded.tender, 700);
    let row = refunded.items.iter().find(|row| row.id == 4).unwrap();
    assert!(!row.purchased && !row.collected);
    assert!(
        !crate::persistence::appearance_store::load_account_appearances(h.db(), ACCOUNT).has(37866)
    );
}

#[test]
fn trading_post_purchase_reply_does_not_publish_an_adventure_guide_snapshot() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Tenderreply").unwrap();
    prepare(&mut h, 1790812900);
    store::grant_tender(h.db(), ACCOUNT, 700).unwrap();
    open_tawny(&mut h);
    h.app
        .world_mut()
        .get_mut::<MessageReceiver<AdventureGuideSnapshot>>(h.connection)
        .unwrap()
        .receive()
        .for_each(drop);
    assert!(matches!(
        h.app
            .world()
            .resource::<crate::adventure_guide::AdventureCatalog>(),
        crate::adventure_guide::AdventureCatalog::Ready(_)
    ));
    h.app
        .world_mut()
        .get_mut::<MessageSender<TradingPostRequest>>(h.connection)
        .unwrap()
        .send::<InstanceChannel>(TradingPostRequest::Purchase { vendor_item_id: 4 });
    let mut replies = Vec::new();
    let mut guide_replies = Vec::new();
    for _ in 0..10 {
        h.app.update();
        replies.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<TradingPostSnapshot>>(h.connection)
                .unwrap()
                .receive(),
        );
        guide_replies.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<AdventureGuideSnapshot>>(h.connection)
                .unwrap()
                .receive(),
        );
    }
    let purchased = replies.pop().expect("purchase reply");
    assert!(purchased.error.is_none(), "{:?}", purchased.error);
    assert_eq!(purchased.tender, 600);
    assert!(
        purchased
            .items
            .iter()
            .any(|row| row.id == 4 && row.purchased && row.collected)
    );
    assert!(
        guide_replies.is_empty(),
        "purchase emitted {} unsolicited Guide snapshots",
        guide_replies.len()
    );
}

#[test]
fn trading_post_cart_wire_rejects_unknown_offer_without_partial_charge_then_publishes_all_rows() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Tendercart").unwrap();
    prepare(&mut h, 1790812900);
    #[derive(Resource, Default)]
    struct AppearanceReplies(Vec<AppearanceCollectionUpdate>);
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<AppearanceCollectionUpdate>::default(),
        MessageReceiver::<AppearanceCollectionUpdate>::default(),
    ));
    h.app.init_resource::<AppearanceReplies>().add_systems(
        First,
        |mut receivers: Query<&mut MessageReceiver<AppearanceCollectionUpdate>>,
         mut replies: ResMut<AppearanceReplies>| {
            for mut receiver in &mut receivers {
                replies.0.extend(receiver.receive());
            }
        },
    );
    store::grant_tender(h.db(), ACCOUNT, 1000).unwrap();
    h.app.insert_resource(RefundWindow(600));
    let closed = request(
        &mut h,
        TradingPostRequest::Checkout {
            vendor_item_ids: vec![4, 5],
        },
    );
    assert!(closed.error.is_some());
    assert_eq!(closed.tender, 1000);
    open_tawny(&mut h);
    let catalog = request(&mut h, TradingPostRequest::Query);
    for id in [4, 5] {
        let row = catalog.items.iter().find(|row| row.id == id).unwrap();
        assert!(
            row.grant_unavailable.is_none(),
            "{}: {:?}",
            row.name,
            row.grant_unavailable
        );
    }
    let invalid = request(
        &mut h,
        TradingPostRequest::Checkout {
            vendor_item_ids: vec![4, u32::MAX],
        },
    );
    assert!(invalid.error.is_some());
    assert_eq!(invalid.tender, 1000);
    assert!(store::load(h.db(), ACCOUNT).purchases.is_empty());
    h.app
        .world_mut()
        .resource_mut::<AppearanceReplies>()
        .0
        .clear();
    let paid = request(
        &mut h,
        TradingPostRequest::Checkout {
            vendor_item_ids: vec![4, 5],
        },
    );
    let updates = &h.app.world().resource::<AppearanceReplies>().0;
    assert_eq!(
        updates.len(),
        1,
        "one final collection update for the whole checkout"
    );
    assert!(updates[0].appearances.contains(&37866));
    assert!(paid.error.is_none(), "{:?}", paid.error);
    let total = catalog
        .items
        .iter()
        .filter(|row| [4, 5].contains(&row.id))
        .map(|row| row.price)
        .sum::<u32>();
    assert_eq!(paid.tender, 1000 - total);
    assert!(!paid.catalog_complete);
    assert!(paid.items.len() < 20);
    for id in [4, 5] {
        let row = paid.items.iter().find(|row| row.id == id).unwrap();
        assert!(row.purchased && row.collected);
        assert_eq!(row.refund_seconds, 600);
    }
    let first = request(&mut h, TradingPostRequest::Refund { vendor_item_id: 4 });
    assert!(first.error.is_none(), "{:?}", first.error);
    assert!(store::load(h.db(), ACCOUNT).purchases.contains_key(&5));
    let second = request(&mut h, TradingPostRequest::Refund { vendor_item_id: 5 });
    assert!(second.error.is_none(), "{:?}", second.error);
    assert_eq!(second.tender, 1000);
}

#[test]
fn trading_post_item_usability_tracks_player_level_class_race_and_unknown_metadata() {
    use crate::item_templates::{ItemTemplate, ItemTemplates};
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Tenderuse").unwrap();
    prepare(&mut h, 1790812900);
    let mut templates = ItemTemplates::default();
    templates.retail.insert(
        189869,
        ItemTemplate {
            required_level: 81,
            allowable_class: -1,
            allowable_race: u64::MAX,
            ..Default::default()
        },
    );
    h.app.insert_resource(templates);
    open_tawny(&mut h);
    let usable =
        |reply: &TradingPostSnapshot| reply.items.iter().find(|row| row.id == 4).unwrap().usable;
    assert_eq!(
        usable(&request(&mut h, TradingPostRequest::Query)),
        Some(false)
    );
    h.app.world_mut().entity_mut(h.player).insert(Level(81));
    assert_eq!(
        usable(&request(&mut h, TradingPostRequest::Query)),
        Some(true)
    );
    h.app
        .world_mut()
        .resource_mut::<ItemTemplates>()
        .retail
        .get_mut(&189869)
        .unwrap()
        .allowable_class = 2;
    assert_eq!(
        usable(&request(&mut h, TradingPostRequest::Query)),
        Some(false)
    );
    let mut templates = h.app.world_mut().resource_mut::<ItemTemplates>();
    let item = templates.retail.get_mut(&189869).unwrap();
    item.allowable_class = -1;
    item.allowable_race = 0;
    drop(templates);
    assert_eq!(
        usable(&request(&mut h, TradingPostRequest::Query)),
        Some(false)
    );
    h.app
        .world_mut()
        .resource_mut::<ItemTemplates>()
        .retail
        .remove(&189869);
    assert_eq!(usable(&request(&mut h, TradingPostRequest::Query)), None);
}

#[test]
fn trading_post_catalog_uses_authored_base_appearance_icons_not_empty_item_icons() {
    let path = std::env::var_os("TRADING_POST_WORLD_COPY").expect("private world fixture");
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let catalog = catalog::load(&conn).unwrap();
    let cloak = &catalog.items[&729].0;
    assert_eq!(cloak.item_id, 223185);
    assert_eq!(
        cloak.icon_fdid, 135411,
        "ItemModifiedAppearance modifier0 → ItemAppearance.DefaultIconFileDataID"
    );
    assert_eq!(
        catalog.items[&456].0.icon_fdid, 2957417,
        "mounts without item appearances keep their direct authored icon"
    );
}
