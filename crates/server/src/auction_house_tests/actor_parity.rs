//! Real gatherer/player creation, settlement and restart boundaries; no seeded auctions.
use super::*;
use crate::economy::{
    Config, ItemMetadata, model::EconomyState, model::MarketItem, run_cycle, store,
};
use std::{collections::BTreeMap, path::Path};

const CAPITAL: u64 = 10_000;
const PRICE: u64 = 1_000;
const COUNT: u32 = 4;

#[test]
fn gatherer_and_player_buyouts_have_identical_terms_after_reopen() {
    for (duration, seconds, deposit) in durations() {
        assert_actor_lifecycle(duration, seconds, deposit, true);
    }
}

#[test]
fn gatherer_and_player_unsold_expiry_refunds_once_after_reopen() {
    for (duration, seconds, deposit) in durations() {
        assert_actor_lifecycle(duration, seconds, deposit, false);
    }
}

fn durations() -> [(AuctionDuration, u64, u64); 3] {
    [
        (AuctionDuration::Short, 86_400, 60),
        (AuctionDuration::Medium, 604_800, 120),
        (AuctionDuration::Long, 1_209_600, 240),
    ]
}

fn leather_config() -> Config {
    Config {
        cycle_secs: 60,
        gatherer_listing_limit: 1,
        gatherer_stack_max: COUNT,
        gatherer_capital: CAPITAL,
        gatherables: vec![2318],
        vendor_supplies: BTreeMap::new(),
        customer_budget: 0,
        customer_quantity: 0,
        // Read-only content_item source: 2318, Light Leather, quality 1,
        // required level 0, SellPrice 15. No world/catalog files are written.
        items: BTreeMap::from([(
            2318,
            ItemMetadata {
                name: "Light Leather".into(),
                quality: 1,
                required_level: 0,
                vendor_sell_price: 15,
            },
        )]),
        markets: BTreeMap::from([(
            2318,
            MarketItem {
                item_id: 2318,
                anchor_price: 250,
                min_price: 250,
                max_price: 250,
                popularity_bps: 10_000,
            },
        )]),
        merchants: Vec::new(),
    }
}

fn reload_auctions(db: &redb::Database) -> AuctionHouseState {
    let mut state = AuctionHouseState::default();
    for auction in persistence::load_auctions(db) {
        state.auctions.insert(auction.auction_id, auction);
    }
    state
}

fn reopen(db: redb::Database, path: &Path) -> redb::Database {
    drop(db);
    redb::Database::open(path).expect("reopen named auction parity database")
}

fn run_expiry(db: redb::Database) -> redb::Database {
    let mut app = App::new();
    app.insert_resource(reload_auctions(&db));
    app.insert_resource(GameDatabase::new(db));
    app.init_resource::<crate::maps::ServerClock>();
    app.add_systems(Update, super::super::handlers::expire_auctions);
    app.update();
    app.update();
    app.world_mut()
        .remove_resource::<GameDatabase>()
        .unwrap()
        .into_database()
}

fn create_actor_pair(
    db: &redb::Database,
    duration: AuctionDuration,
    seconds: u64,
    deposit: u64,
) -> (Config, [AuctionData; 2], u64) {
    let (player, _) = persistence::create_character_with_test_bags(db, 1, "Seller", 1, 1)
        .expect("player seller with real inventory bags");
    let (buyer, _) = persistence::create_character_with_test_bags(db, 2, "Buyer", 1, 1)
        .expect("buyer with real inventory bags");
    let source = InventoryItemData {
        item_guid: persistence::next_id(db, "item_instance"),
        item_id: 2318,
        name: "Light Leather".into(),
        quality: 1,
        required_level: 0,
        stack_count: COUNT,
        vendor_sell_price: 15,
        unique_auction: false,
    };
    persistence::save_inventory(
        db,
        player,
        &InventoryData {
            gold: CAPITAL,
            items: vec![source.clone()],
        },
    );
    persistence::save_inventory(
        db,
        buyer,
        &InventoryData {
            gold: CAPITAL,
            items: Vec::new(),
        },
    );
    let mut auctions = AuctionHouseState::default();
    let before = unix_now();
    let response = create_auction(
        db,
        &ItemTemplates::default(),
        &mut auctions,
        player,
        0,
        &CreateAuction {
            item_guid: source.item_guid,
            stack_count: COUNT,
            min_bid: PRICE,
            buyout_price: Some(PRICE),
            duration,
        },
    );
    assert!(response.success, "{response:?}");
    let player_auction = auctions.auctions.values().next().unwrap().clone();
    let origin = player_auction.expires_at_unix_secs - seconds;
    assert!((before..=unix_now()).contains(&origin));

    let config = leather_config();
    let mut economy = EconomyState::default();
    let duration_draw = match duration {
        AuctionDuration::Short => 0,
        AuctionDuration::Medium => 1,
        AuctionDuration::Long => 2,
    };
    let mut draws = [3, duration_draw, 0].into_iter();
    run_cycle(
        db,
        &mut auctions,
        &mut economy,
        &config,
        origin,
        &mut || draws.next().expect("one stack, duration and price draw"),
    )
    .expect("real gatherer actor and auction creation");
    assert!(draws.next().is_none());
    let bot = economy.gatherers[&2318];
    assert_ne!(bot, player);
    assert_ne!(bot, buyer);
    assert!(persistence::load_character(db, bot).is_none());
    assert!(persistence::load_character(db, player).is_some());
    assert_eq!(store::load_state(db).unwrap().unwrap(), economy);
    assert_eq!(auctions.auctions.len(), 2);
    let bot_auction = auctions
        .auctions
        .values()
        .find(|auction| auction.seller_character_id == bot)
        .unwrap()
        .clone();
    assert_eq!(bot_auction.seller_name, "Gatherer 2318");
    assert_eq!(bot_auction.expires_at_unix_secs, origin + seconds);
    assert_eq!(
        bot_auction.expires_at_unix_secs,
        player_auction.expires_at_unix_secs
    );
    assert_ne!(bot_auction.item.item_guid, player_auction.item.item_guid);
    for auction in [&bot_auction, &player_auction] {
        assert_eq!(auction.deposit, deposit);
        assert_eq!(auction.stack_count, COUNT);
        assert_eq!(auction.min_bid, PRICE);
        assert_eq!(auction.buyout_price, Some(PRICE));
        assert_eq!(
            auction.item,
            InventoryItemData {
                item_guid: auction.item.item_guid,
                ..source.clone()
            }
        );
        let inventory = persistence::load_inventory(db, auction.seller_character_id);
        assert_eq!(inventory.gold, CAPITAL - deposit);
        assert!(inventory.items.is_empty(), "stack held in auction escrow");
    }
    (config, [bot_auction, player_auction], buyer)
}

fn assert_actor_lifecycle(duration: AuctionDuration, seconds: u64, deposit: u64, sold: bool) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gatherer-player-auction-parity.redb");
    let mut db = redb::Database::create(&path).unwrap();
    persistence::create_tables(&db);
    let (mut config, offers, buyer) = create_actor_pair(&db, duration, seconds, deposit);
    let [bot, player] = offers.each_ref().map(|offer| offer.seller_character_id);
    db = reopen(db, &path);
    let mut state = reload_auctions(&db);
    assert_eq!(state.auctions.len(), 2);
    for offer in &offers {
        assert_eq!(&state.auctions[&offer.auction_id], offer);
    }
    // The real expiry system must leave both not-yet-due auctions untouched.
    db = run_expiry(db);
    assert_eq!(persistence::load_auctions(&db).len(), 2);
    assert!(mail_store::load_mailbox(&db, bot).is_empty());
    assert!(mail_store::load_mailbox(&db, player).is_empty());
    if sold {
        for offer in &offers {
            assert!(buyout_auction(&db, &mut state, buyer, offer.auction_id).success);
        }
    } else {
        // Expiry uses the real wall clock, so age only these test-owned persisted
        // deadlines; no sleep or replacement settlement helper is needed.
        for offer in &offers {
            let mut due = offer.clone();
            due.expires_at_unix_secs -= seconds + 1;
            persistence::save_auction(&db, &due);
        }
        db = reopen(db, &path);
        db = run_expiry(db);
    }
    assert!(persistence::load_auctions(&db).is_empty());
    assert_eq!(
        persistence::load_inventory(&db, buyer).gold,
        if sold { 8_000 } else { CAPITAL }
    );
    let seller_money = if sold { 950 + deposit } else { deposit };
    for offer in &offers {
        let mails = mail_store::load_mailbox(&db, offer.seller_character_id);
        assert_eq!(mails.len(), 1);
        assert_eq!(mails[0].money, seller_money);
        if sold {
            assert!(mails[0].items.is_empty());
        } else {
            assert_eq!(mails[0].items.len(), 1);
            assert_eq!(mails[0].items[0].item, offer.item);
        }
        assert_eq!(
            persistence::load_inventory(&db, offer.seller_character_id).gold,
            CAPITAL - deposit
        );
    }
    let buyer_mails = mail_store::load_mailbox(&db, buyer);
    assert_eq!(buyer_mails.len(), if sold { 2 } else { 0 });
    for offer in &offers {
        if sold {
            let mail = buyer_mails
                .iter()
                .find(|mail| mail.items[0].item.item_guid == offer.item.item_guid)
                .unwrap();
            assert_eq!(mail.money, 0);
            assert_eq!(mail.items.len(), 1);
            assert_eq!(mail.items[0].item, offer.item);
        }
    }
    let mail_before = [bot, player, buyer].map(|id| mail_store::load_mailbox(&db, id));
    db = reopen(db, &path);
    db = run_expiry(db);
    let mut empty_state = reload_auctions(&db);
    for offer in &offers {
        assert!(!buyout_auction(&db, &mut empty_state, buyer, offer.auction_id).success);
    }
    assert_eq!(
        [bot, player, buyer].map(|id| mail_store::load_mailbox(&db, id)),
        mail_before
    );

    let mut gold = GoldUpdateQueue::default();
    take_mail(&db, player, mail_before[1][0].mail_id, &mut gold);
    for mail in &buyer_mails {
        take_mail(&db, buyer, mail.mail_id, &mut gold);
    }
    // Gatherer has no login character: its real cycle claims mail, caps capital,
    // and records surplus as withdrawn. Stop new supply through market demand.
    config.gatherer_listing_limit = 0;
    let mut economy = store::load_state(&db).unwrap().unwrap();
    let now = unix_now().max(economy.last_cycle + 60);
    run_cycle(
        &db,
        &mut empty_state,
        &mut economy,
        &config,
        now,
        &mut || panic!("no new supply"),
    )
    .expect("gatherer claims settlement mail");
    assert_eq!(persistence::load_inventory(&db, bot).gold, CAPITAL);
    assert_eq!(
        persistence::load_inventory(&db, bot).items,
        if sold {
            Vec::new()
        } else {
            vec![offers[0].item.clone()]
        },
        "collector must retain exact expired stack and GUID"
    );
    assert_eq!(economy.withdrawn, if sold { 950 } else { 0 });
    assert_eq!(
        persistence::load_inventory(&db, player).gold,
        if sold { 10_950 } else { CAPITAL }
    );
    assert!(mail_store::load_mailbox(&db, bot).is_empty());
    assert_eq!(leather_count(&db, player), if sold { 0 } else { COUNT });
    assert_eq!(leather_count(&db, buyer), if sold { 2 * COUNT } else { 0 });

    let inventories = [bot, player, buyer].map(|id| persistence::load_inventory(&db, id));
    let mail_after = [bot, player, buyer].map(|id| mail_store::load_mailbox(&db, id));
    db = reopen(db, &path);
    assert_eq!(store::load_state(&db).unwrap().unwrap(), economy);
    db = run_expiry(db);
    let mut restarted = store::load_state(&db).unwrap().unwrap();
    run_cycle(
        &db,
        &mut reload_auctions(&db),
        &mut restarted,
        &config,
        now + 60,
        &mut || panic!("no new supply"),
    )
    .expect("restarted gatherer cannot claim the same money twice");
    take_mail(&db, player, mail_before[1][0].mail_id, &mut gold);
    for mail in &buyer_mails {
        take_mail(&db, buyer, mail.mail_id, &mut gold);
    }
    assert_eq!(restarted.withdrawn, economy.withdrawn);
    assert_eq!(
        [bot, player, buyer].map(|id| persistence::load_inventory(&db, id)),
        inventories
    );
    assert_eq!(
        [bot, player, buyer].map(|id| mail_store::load_mailbox(&db, id)),
        mail_after
    );
    assert!(persistence::load_auctions(&db).is_empty());
}

fn leather_count(db: &redb::Database, actor: u64) -> u32 {
    persistence::load_inventory(db, actor)
        .items
        .iter()
        .filter(|item| item.item_id == 2318)
        .map(|item| item.stack_count)
        .sum()
}

#[test]
fn gatherer_relists_expired_goods_before_generating_supply() {
    for requested in [2, COUNT, 8] {
        assert_returned_stack_relisting(requested);
    }
}

fn assert_returned_stack_relisting(requested: u32) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("returned-stack.redb");
    let mut db = redb::Database::create(&path).unwrap();
    persistence::create_tables(&db);
    let (mut config, offers, _) = create_actor_pair(&db, AuctionDuration::Short, 86_400, 60);
    let offer = &offers[0];
    let bot = offer.seller_character_id;
    for original in &offers {
        let mut due = original.clone();
        due.expires_at_unix_secs = unix_now() - 1;
        persistence::save_auction(&db, &due);
    }
    db = run_expiry(db);
    assert_eq!(
        mail_store::load_mailbox(&db, bot)[0].items[0].item,
        offer.item
    );
    config.gatherer_listing_limit = 0;
    let now = unix_now();
    let mut economy = store::load_state(&db).unwrap().unwrap();
    run_cycle(
        &db,
        &mut reload_auctions(&db),
        &mut economy,
        &config,
        now + 60,
        &mut || panic!("zero listing target"),
    )
    .unwrap();
    assert_eq!(
        persistence::load_inventory(&db, bot).items,
        vec![offer.item.clone()]
    );
    db = reopen(db, &path);
    economy = store::load_state(&db).unwrap().unwrap();
    run_cycle(
        &db,
        &mut reload_auctions(&db),
        &mut economy,
        &config,
        now + 120,
        &mut || panic!("claim must not repeat"),
    )
    .unwrap();
    assert_eq!(
        persistence::load_inventory(&db, bot).items,
        vec![offer.item.clone()]
    );

    config.gatherer_listing_limit = 1;
    config.gatherer_stack_max = 8;
    // Relisting uses the retained item's value, not changed catalog metadata.
    config.items.get_mut(&2318).unwrap().vendor_sell_price = 999;
    let mut auctions = reload_auctions(&db);
    let mut draws = [u64::from(requested - 1), 1, 0].into_iter();
    run_cycle(
        &db,
        &mut auctions,
        &mut economy,
        &config,
        now + 180,
        &mut || draws.next().expect("quantity, duration, price"),
    )
    .unwrap();
    assert!(draws.next().is_none());
    assert_eq!(auctions.auctions.len(), 1);
    let relisted = auctions.auctions.values().next().unwrap();
    let listed_count = requested.min(COUNT);
    assert_eq!(relisted.stack_count, listed_count);
    assert_eq!(
        relisted.deposit,
        deposit_cost(15, listed_count, AuctionDuration::Medium)
    );
    assert_eq!(relisted.expires_at_unix_secs, now + 180 + 604_800);
    assert_eq!(relisted.buyout_price, Some(250 * u64::from(listed_count)));
    if requested < COUNT {
        assert_ne!(relisted.item.item_guid, offer.item.item_guid);
    } else {
        assert_eq!(relisted.item.item_guid, offer.item.item_guid);
    }
    assert_eq!(
        relisted.item,
        InventoryItemData {
            item_guid: relisted.item.item_guid,
            stack_count: listed_count,
            ..offer.item.clone()
        }
    );
    let inventory = persistence::load_inventory(&db, bot);
    let remaining = COUNT - listed_count;
    assert_eq!(
        inventory.items,
        if remaining == 0 {
            vec![]
        } else {
            vec![InventoryItemData {
                stack_count: remaining,
                ..offer.item.clone()
            }]
        }
    );
    assert_eq!(inventory.gold, CAPITAL - relisted.deposit);
    assert_eq!(leather_count(&db, bot) + relisted.stack_count, COUNT);
    let persisted_offer = relisted.clone();
    db = reopen(db, &path);
    economy = store::load_state(&db).unwrap().unwrap();
    config.gatherer_listing_limit = 0;
    run_cycle(
        &db,
        &mut reload_auctions(&db),
        &mut economy,
        &config,
        now + 240,
        &mut || panic!("no new listing"),
    )
    .unwrap();
    assert_eq!(persistence::load_inventory(&db, bot), inventory);
    assert_eq!(persistence::load_auctions(&db), vec![persisted_offer]);
    assert!(mail_store::load_mailbox(&db, bot).is_empty());
}
