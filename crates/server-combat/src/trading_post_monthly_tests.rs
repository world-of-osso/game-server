use super::*;

const OCTOBER_START: i64 = 1_790_812_800;
const NOVEMBER_START: i64 = 1_793_491_200;
const OCTOBER_OFFER: u32 = 1192;
const OLD_OFFER: u32 = 4;

fn monthly_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/trading-post-monthly-catalog.json")
}

fn prepare_monthly(h: &mut Harness, now: i64) {
    prepare(h, now);
    let conn = rusqlite::Connection::open_with_flags(
        std::env::var_os("TRADING_POST_WORLD_COPY").unwrap(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(catalog::Catalog(catalog::load(&conn, &monthly_path())));
}

fn october_ids() -> Vec<u32> {
    let document: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(monthly_path()).unwrap()).unwrap();
    document["months"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["perks_month"] == 46)
        .unwrap()["vendor_item_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_u64().unwrap() as u32)
        .collect()
}

#[test]
fn trading_post_monthly_query_exact_october_and_empty_uncovered_utc_rollover() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Monthquery").unwrap();
    prepare_monthly(&mut h, NOVEMBER_START - 1);
    let october = request(&mut h, TradingPostRequest::Query);
    let mut actual: Vec<_> = october.items.iter().map(|row| row.id).collect();
    actual.sort_unstable();
    assert_eq!(actual, october_ids(), "only the42 authored October offers");
    assert_eq!(october.now_unix, NOVEMBER_START - 1);
    assert_eq!(october.month_end_unix, NOVEMBER_START);
    h.app
        .world_mut()
        .resource_mut::<crate::maps::ServerClock>()
        .fixed_unix = Some(NOVEMBER_START);
    let november = request(&mut h, TradingPostRequest::Query);
    assert!(november.catalog_complete);
    assert!(
        november.items.is_empty(),
        "no all-catalog fallback for month47"
    );
    assert!(
        november
            .source_gaps
            .iter()
            .any(|gap| gap.contains("TP_MONTHLY_CATALOG_MONTH_UNCOVERED") && gap.contains("47"))
    );
    assert!(november.month_end_unix > november.now_unix);
}

#[test]
fn trading_post_monthly_rejects_offmonth_buy_freeze_and_mixed_checkout_atomically() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Monthreject").unwrap();
    prepare_monthly(&mut h, OCTOBER_START + 100);
    store::grant_tender(h.db(), ACCOUNT, 10_000).unwrap();
    open_tawny(&mut h);
    for command in [
        TradingPostRequest::Purchase {
            vendor_item_id: OLD_OFFER,
        },
        TradingPostRequest::Freeze {
            vendor_item_id: Some(OLD_OFFER),
        },
        TradingPostRequest::Checkout {
            vendor_item_ids: vec![OCTOBER_OFFER, OLD_OFFER],
        },
    ] {
        let reply = request(&mut h, command);
        assert!(
            reply
                .error
                .as_deref()
                .is_some_and(|error| error.contains("not available")),
            "{:?}",
            reply.error
        );
        let state = store::load(h.db(), ACCOUNT);
        assert_eq!(state.tender, 10_000);
        assert!(state.purchases.is_empty());
        assert_eq!(state.frozen, None);
    }
    let paid = request(
        &mut h,
        TradingPostRequest::Purchase {
            vendor_item_id: OCTOBER_OFFER,
        },
    );
    assert!(paid.error.is_none(), "{:?}", paid.error);
    assert!(
        store::load(h.db(), ACCOUNT)
            .purchases
            .contains_key(&OCTOBER_OFFER)
    );
}

#[test]
fn trading_post_monthly_frozen_oldmonth_reopen_purchase_and_receipt_refund() {
    let file = tempfile::NamedTempFile::new().unwrap();
    {
        let mut h = crate::bank_test_harness::harness_at(file.path(), "Monthfrozen").unwrap();
        prepare_monthly(&mut h, OCTOBER_START + 100);
        store::grant_tender(h.db(), ACCOUNT, 10_000).unwrap();
        // Durable previous-month freeze; production membership must read this fresh
        // account state inside the FIFO, not an empty tick-thread placeholder.
        store::freeze(h.db(), ACCOUNT, Some(OLD_OFFER));
    }
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Monthreopen").unwrap();
    prepare_monthly(&mut h, NOVEMBER_START + 10);
    h.app.insert_resource(RefundWindow(600));
    open_tawny(&mut h);
    let frozen = request(&mut h, TradingPostRequest::Query);
    assert_eq!(frozen.frozen_item_id, Some(OLD_OFFER));
    assert_eq!(
        frozen.items.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![OLD_OFFER]
    );
    let paid = request(
        &mut h,
        TradingPostRequest::Purchase {
            vendor_item_id: OLD_OFFER,
        },
    );
    assert!(paid.error.is_none(), "{:?}", paid.error);
    let tender = paid.tender;
    let price = paid
        .items
        .iter()
        .find(|row| row.id == OLD_OFFER)
        .unwrap()
        .price;
    let unfrozen = request(
        &mut h,
        TradingPostRequest::Freeze {
            vendor_item_id: None,
        },
    );
    assert_eq!(unfrozen.frozen_item_id, None);
    let empty = request(&mut h, TradingPostRequest::Query);
    assert!(empty.items.is_empty());
    h.app.insert_resource(catalog::Catalog(Err(
        "TP_MONTHLY_CATALOG_LOAD missing file".into(),
    )));
    let refund = request(
        &mut h,
        TradingPostRequest::Refund {
            vendor_item_id: OLD_OFFER,
        },
    );
    assert!(refund.error.is_none(), "{:?}", refund.error);
    assert_eq!(refund.tender, tender + price);
    assert!(
        !store::load(h.db(), ACCOUNT)
            .purchases
            .contains_key(&OLD_OFFER)
    );
}

#[test]
fn trading_post_monthly_loader_rejects_missing_malformed_and_inconsistent_data() {
    let original: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(monthly_path()).unwrap()).unwrap();
    let build = original["db2_build"].as_str().unwrap();
    let vendors: std::collections::BTreeSet<u32> = original["months"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|month| month["vendor_item_ids"].as_array().unwrap())
        .map(|id| id.as_u64().unwrap() as u32)
        .collect();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("trading-post-monthly-catalog.json");
    assert!(super::super::monthly::load(&file, build, &vendors).is_err());
    std::fs::write(&file, "not JSON").unwrap();
    assert!(super::super::monthly::load(&file, build, &vendors).is_err());
    for (pointer, value) in [
        ("/schema_version", serde_json::json!(9)),
        ("/policy", serde_json::json!("unapproved")),
        ("/db2_build", serde_json::json!("0.0.0.1")),
        ("/month_epoch_policy", serde_json::json!("unknown epoch")),
        ("/months/0/vendor_item_ids", serde_json::json!([u32::MAX])),
        ("/months/0/calendar", serde_json::json!("2023-03")),
        ("/months/0/source/sha256", serde_json::json!("bad hash")),
        (
            "/permanent_vendor_item_ids",
            serde_json::json!([OCTOBER_OFFER]),
        ),
    ] {
        let mut invalid = original.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        std::fs::write(&file, serde_json::to_vec(&invalid).unwrap()).unwrap();
        assert!(
            super::super::monthly::load(&file, build, &vendors).is_err(),
            "accepted {pointer}"
        );
    }
    std::fs::write(&file, serde_json::to_vec(&original).unwrap()).unwrap();
    let loaded = super::super::monthly::load(&file, build, &vendors).unwrap();
    assert!(loaded.at(OCTOBER_START).contains(OCTOBER_OFFER, None));
    assert!(!loaded.at(NOVEMBER_START).contains(OCTOBER_OFFER, None));
}

#[test]
fn trading_post_monthly_fifo_reads_freeze_committed_ahead_of_query_and_buy() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Monthfifo").unwrap();
    prepare_monthly(&mut h, NOVEMBER_START + 10);
    store::grant_tender(h.db(), ACCOUNT, 10_000).unwrap();
    open_tawny(&mut h);
    let held = h.db().begin_write().unwrap();
    let receipt = h
        .app
        .world()
        .resource::<GameDatabase>()
        .writer
        .enqueue_result("fixture preceding freeze", move |db| {
            store::freeze(db, ACCOUNT, Some(OLD_OFFER));
            Ok(())
        });
    h.app
        .world_mut()
        .get_mut::<MessageSender<TradingPostRequest>>(h.connection)
        .unwrap()
        .send::<InstanceChannel>(TradingPostRequest::Query);
    h.app
        .world_mut()
        .get_mut::<MessageSender<TradingPostRequest>>(h.connection)
        .unwrap()
        .send::<InstanceChannel>(TradingPostRequest::Purchase {
            vendor_item_id: OLD_OFFER,
        });
    for _ in 0..4 {
        h.app.update();
    }
    assert!(
        receipt.result().is_none(),
        "held transaction must fence publication"
    );
    held.abort().unwrap();
    crate::test_data::drain_persistence(h.app.world().resource::<GameDatabase>());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut replies = Vec::new();
    while replies.len() < 2 && std::time::Instant::now() < deadline {
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
    assert_eq!(replies.len(), 2);
    assert!(
        replies.iter().all(|reply| reply.error.is_none()),
        "{replies:?}"
    );
    let query = replies.iter().find(|reply| reply.catalog_complete).unwrap();
    assert_eq!(
        query.items.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![OLD_OFFER]
    );
    assert!(
        store::load(h.db(), ACCOUNT)
            .purchases
            .contains_key(&OLD_OFFER)
    );
}
