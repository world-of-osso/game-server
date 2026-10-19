use super::*;

const OCTOBER_START: i64 = 1_790_812_800;
const NOVEMBER_START: i64 = 1_793_491_200;
const OCTOBER_OFFER: u32 = 1192;
const OLD_OFFER: u32 = 4;

fn prepare_monthly(h: &mut Harness, now: i64) {
    prepare(h, now);
}

fn october_ids() -> Vec<u32> {
    let document: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/trading-post-monthly-catalog.json"),
        )
        .unwrap(),
    )
    .unwrap();
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
#[ignore = "TP_MONTHLY_CATALOG_WOWHEAD_NEWS_BEST_GUESS RED: runtime membership pending"]
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
#[ignore = "TP_MONTHLY_CATALOG_WOWHEAD_NEWS_BEST_GUESS RED: runtime membership pending"]
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
#[ignore = "TP_MONTHLY_CATALOG_WOWHEAD_NEWS_BEST_GUESS RED: runtime membership pending"]
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
