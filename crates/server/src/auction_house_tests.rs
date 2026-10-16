#[path = "owned_commerce_auction_tests.rs"]
mod owned_commerce_sources;

#[path = "auction_house_tests/criteria.rs"]
pub(crate) mod criteria;

use super::*;
use crate::gold::GoldUpdateQueue;
use crate::persistence;

#[path = "auction_house_tests/load_fixture.rs"]
mod load_fixture;

/// Take a mail's money and items at a mailbox (`TakeInboxMoney`, `TakeInboxItem`).
fn take_mail(db: &redb::Database, character_id: u64, mail_id: u64, gold: &mut GoldUpdateQueue) {
    use shared::protocol::MailAction;
    let templates =
        crate::item_templates::load_runtime_item_templates(&crate::test_data::world_db_path())
            .expect("load runtime item catalogs for mailbox claims");
    let mail = mail_store::load_mailbox(db, character_id)
        .into_iter()
        .find(|mail| mail.mail_id == mail_id)
        .expect("mail in the mailbox");
    let actions = std::iter::once(MailAction::TakeMoney).chain(mail.items.iter().map(|attached| {
        MailAction::TakeAttachment {
            slot: attached.slot,
        }
    }));
    for action in actions {
        let changes =
            crate::mail::act_on_mail(db, &templates, character_id, mail_id, action, unix_now())
                .expect("take from mail");
        for (id, copper) in changes.gold {
            gold.record(id, copper);
        }
    }
}

fn test_auction(id: u64, name: &str, quality: u8, required_level: u16) -> AuctionData {
    AuctionData {
        auction_id: id,
        seller_character_id: 1,
        seller_name: "Seller".into(),
        item: InventoryItemData {
            item_guid: id,
            item_id: id as u32,
            name: name.into(),
            quality,
            required_level,
            stack_count: 1,
            vendor_sell_price: 1,
            unique_auction: false,
        },
        stack_count: 1,
        min_bid: 100,
        current_bid: None,
        current_bidder_character_id: None,
        buyout_price: Some(200),
        expires_at_unix_secs: unix_now() + 3600,
        faction: 0,
        deposit: 0,
    }
}

fn text_query(text: &str) -> AuctionSearchQuery {
    AuctionSearchQuery {
        text: text.into(),
        page: 0,
        page_size: 50,
        min_level: None,
        max_level: None,
        quality: None,
        usable_only: false,
        sort_field: AuctionSortField::Name,
        sort_dir: AuctionSortDir::Asc,
        faction: 0,
        ..Default::default()
    }
}

#[path = "auction_house_tests/actor_parity.rs"]
mod actor_parity;
#[path = "auction_house_tests/clock.rs"]
mod clock;
#[path = "auction_house_tests/core.rs"]
mod core;
#[path = "auction_house_tests/lifecycle.rs"]
mod lifecycle;
#[path = "auction_house_tests/search.rs"]
mod search;
#[path = "auction_house_tests/settlement.rs"]
mod settlement;
#[path = "auction_house_tests/viewer.rs"]
mod viewer;

mod bounded_slice {
    use super::*;

    #[test]
    fn durations_use_days_and_weeks() {
        assert_eq!(duration_secs(AuctionDuration::Short), 86_400);
        assert_eq!(duration_secs(AuctionDuration::Medium), 604_800);
        assert_eq!(duration_secs(AuctionDuration::Long), 1_209_600);
    }

    #[test]
    fn expired_unsold_returns_deposit_and_item_once_cancellation_forfeits() {
        let db = persistence::temp_database();
        let mut app = App::new();
        let mut state = AuctionHouseState::default();
        for (id, name) in [(1, "Player Seller"), (2, "Bot Seller")] {
            let mut auction = test_auction(id, "Iron Ore", 1, 1);
            auction.seller_character_id = id;
            auction.seller_name = name.into();
            auction.expires_at_unix_secs = unix_now() - 1;
            auction.deposit = 123;
            auction.item.stack_count = 17;
            auction.stack_count = 17;
            persistence::save_auction(&db, &auction);
            state.auctions.insert(id, auction);
        }
        let mut cancelled = test_auction(3, "Copper Ore", 1, 1);
        cancelled.seller_character_id = 3;
        cancelled.deposit = 456;
        persistence::save_auction(&db, &cancelled);
        state.auctions.insert(3, cancelled);
        assert!(cancel_auction(&db, &mut state, 3, 3).success);
        assert_eq!(mail_store::load_mailbox(&db, 3)[0].money, 0);
        app.insert_resource(GameDatabase { db });
        app.insert_resource(state);
        app.init_resource::<crate::maps::ServerClock>();
        app.add_systems(Update, super::super::handlers::expire_auctions);
        app.update();
        app.update();
        let db = &app.world().resource::<GameDatabase>().db;
        assert!(persistence::load_auctions(db).is_empty());
        for id in [1, 2] {
            let mail = mail_store::load_mailbox(db, id);
            assert_eq!(mail.len(), 1);
            assert_eq!(mail[0].money, 123);
            assert_eq!(mail[0].items.len(), 1);
            assert_eq!(mail[0].items[0].item.stack_count, 17);
            assert_eq!(mail[0].items[0].item.item_guid, id);
        }
    }

    fn expected_order(
        a: &AuctionData,
        b: &AuctionData,
        query: &AuctionSearchQuery,
    ) -> std::cmp::Ordering {
        let order = match query.sort_field {
            AuctionSortField::Bid => a
                .current_bid
                .unwrap_or(a.min_bid)
                .cmp(&b.current_bid.unwrap_or(b.min_bid)),
            AuctionSortField::Quantity => a.stack_count.cmp(&b.stack_count),
            AuctionSortField::Name => a.item.name.cmp(&b.item.name),
            AuctionSortField::MinBid => a.min_bid.cmp(&b.min_bid),
            AuctionSortField::Buyout => a
                .buyout_price
                .unwrap_or(u64::MAX)
                .cmp(&b.buyout_price.unwrap_or(u64::MAX)),
            AuctionSortField::TimeLeft => time_bucket(a).cmp(&time_bucket(b)),
            AuctionSortField::Quality => a.item.quality.cmp(&b.item.quality),
            AuctionSortField::RequiredLevel => a.item.required_level.cmp(&b.item.required_level),
        };
        let directed = match query.sort_dir {
            AuctionSortDir::Asc => order,
            AuctionSortDir::Desc => order.reverse(),
        };
        directed.then_with(|| a.auction_id.cmp(&b.auction_id))
    }

    fn time_bucket(auction: &AuctionData) -> u8 {
        match auction.expires_at_unix_secs.saturating_sub(unix_now()) {
            0..=60 => 0,
            61..=1800 => 1,
            1801..=21600 => 2,
            _ => 3,
        }
    }

    #[test]
    fn all_sorts_match_filtered_multi_page_reference_with_id_ties() {
        let auctions: HashMap<_, _> = (1..=360)
            .map(|id| {
                let mut a = test_auction(
                    id,
                    if id % 2 == 0 {
                        "Iron Ore"
                    } else {
                        "Copper Ore"
                    },
                    (id % 4) as u8,
                    (id % 20) as u16,
                );
                a.min_bid = (id % 7) * 100;
                a.buyout_price = (id % 5 != 0).then_some((id % 9) * 200);
                a.faction = (id % 3) as u8;
                a.expires_at_unix_secs = unix_now() + [30, 900, 7200, 86400][id as usize % 4];
                (id, a)
            })
            .collect();
        for field in [
            AuctionSortField::Name,
            AuctionSortField::MinBid,
            AuctionSortField::Buyout,
            AuctionSortField::TimeLeft,
            AuctionSortField::Quality,
            AuctionSortField::RequiredLevel,
        ] {
            for dir in [AuctionSortDir::Asc, AuctionSortDir::Desc] {
                for filtered in [false, true] {
                    let mut query = text_query(if filtered { "iRoN" } else { "" });
                    query.sort_field = field;
                    query.sort_dir = dir;
                    query.page_size = 7;
                    if filtered {
                        query.quality = Some(2);
                        query.min_level = Some(2);
                        query.max_level = Some(14);
                        query.faction = 1;
                    }
                    let mut expected: Vec<_> = auctions
                        .values()
                        .filter(|a| {
                            !filtered
                                || (a.item.name == "Iron Ore"
                                    && a.item.quality == 2
                                    && (2..=14).contains(&a.item.required_level)
                                    && a.faction != 2)
                        })
                        .collect();
                    expected.sort_by(|a, b| expected_order(a, b, &query));
                    assert!(expected.len() > 14);
                    for page in 0..=(expected.len() / 7 + 1) {
                        query.page = page as u32;
                        let result = search_auctions(&auctions, &query, &|_| {
                            shared::item_data::ItemDefinitionSource::Retail
                        });
                        assert_eq!(result.total_results as usize, expected.len());
                        let ids: Vec<_> = result.results.iter().map(|a| a.auction_id).collect();
                        let expected_ids: Vec<_> = expected
                            .iter()
                            .skip(page * 7)
                            .take(7)
                            .map(|a| a.auction_id)
                            .collect();
                        assert_eq!(
                            ids, expected_ids,
                            "field={field:?} dir={dir:?} page={page} filtered={filtered}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn exact_item_and_additional_filters_precede_count_and_paging() {
        let auctions: HashMap<_, _> = (1..=160)
            .map(|id| {
                let mut auction = test_auction(id, "Iron Ore", 1, 1);
                auction.item.item_id = if id % 2 == 0 { 42 } else { 43 };
                (id, auction)
            })
            .collect();
        let mut query = text_query("");
        query.item_id = Some(42);
        query.page_size = 10;
        query.page = 1;
        let result = super::super::queries::search_auctions_matching(
            &auctions,
            &query,
            &|_| shared::item_data::ItemDefinitionSource::Retail,
            |auction| auction.auction_id % 3 == 0,
        );
        assert_eq!(result.total_results, 26);
        let ids: Vec<_> = result.results.iter().map(|a| a.auction_id).collect();
        assert_eq!(ids, (11..=20).map(|index| index * 6).collect::<Vec<_>>());
    }

    #[test]
    fn query_300k_listings_scans_once_and_materializes_only_the_requested_page() {
        let auctions: HashMap<_, _> = (1..=300_000)
            .map(|id| {
                let mut a = test_auction(
                    id,
                    &format!("Iron Ore {:04}", id % 1024),
                    (id % 5) as u8,
                    (id % 80) as u16,
                );
                a.min_bid = (id * 7919) % 1_000_000;
                a.buyout_price = Some(a.min_bid * 2);
                a.stack_count = (id % 200 + 1) as u32;
                a.item.stack_count = a.stack_count;
                a.faction = (id % 3) as u8;
                (id, a)
            })
            .collect();
        for (text, field, page) in [
            ("", AuctionSortField::Name, 0),
            ("", AuctionSortField::MinBid, 2000),
            ("iRoN", AuctionSortField::Buyout, 1),
        ] {
            let mut query = text_query(text);
            query.sort_field = field;
            query.page = page;
            let visited = std::cell::Cell::new(0_usize);
            let resolved = std::cell::RefCell::new(Vec::new());
            let result = super::super::queries::search_auctions_matching(
                &auctions,
                &query,
                &|guid| {
                    resolved.borrow_mut().push(guid);
                    shared::item_data::ItemDefinitionSource::Retail
                },
                |_| {
                    visited.set(visited.get() + 1);
                    true
                },
            );
            assert_eq!(result.total_results, 300_000);
            assert_eq!(result.results.len(), 50);
            assert_eq!(visited.get(), 300_000, "one matching pass for exact counts");
            assert_eq!(
                resolved.borrow().len(),
                50,
                "only page rows resolve item definitions"
            );
            let mut reference: Vec<_> = auctions.values().collect();
            reference.sort_unstable_by(|a, b| expected_order(a, b, &query));
            let start = page as usize * 50;
            assert!(
                start + 50 < reference.len(),
                "exercise a complete deep page and its successor"
            );
            let expected: Vec<_> = reference[start..start + 50]
                .iter()
                .map(|a| a.auction_id)
                .collect();
            let actual: Vec<_> = result.results.iter().map(|a| a.auction_id).collect();
            assert_eq!(
                actual, expected,
                "text={text:?}, field={field:?}, page={page}"
            );
            let expected_guids: Vec<_> = reference[start..start + 50]
                .iter()
                .map(|a| a.item.item_guid)
                .collect();
            assert_eq!(*resolved.borrow(), expected_guids);
            eprintln!(
                "300k query text={text:?} field={field:?} page={page}: visited={}, resolved={}",
                visited.get(),
                resolved.borrow().len()
            );
        }
    }
}
