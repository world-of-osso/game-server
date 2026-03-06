use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{
    AuctionChannel, AuctionDuration, AuctionHouseOpened, AuctionInventoryItem,
    AuctionInventorySnapshot, AuctionListingSummary, AuctionMailboxSnapshot,
    AuctionOperationResponse, AuctionSearchQuery, AuctionSearchResults, AuctionSortDir,
    AuctionSortField, AuctionTimeLeft, BidAuctionListResponse, BuyoutAuction, CancelAuction,
    ClaimAuctionMail, CreateAuction, OpenAuctionHouse, OwnedAuctionListResponse, PlaceBid,
    QueryAuctionInventory, QueryAuctionMailbox, QueryAuctions, QueryBidAuctions,
    QueryOwnedAuctions,
};

use crate::auth::{Authenticated, InWorld};
use crate::persistence::{
    self, AuctionData, GameDatabase, InventoryData, InventoryItemData, MailEntryData,
};

const MAX_PAGE_SIZE: u32 = 50;
const HOUSE_CUT_BPS: u32 = 500;

#[derive(Resource, Default)]
struct AuctionHouseState {
    initialized: bool,
    auctions: HashMap<u64, AuctionData>,
}

pub struct AuctionHousePlugin;

impl Plugin for AuctionHousePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AuctionHouseState>();
        app.add_systems(Update, initialize_auction_state);
        app.add_systems(Update, process_open_requests);
        app.add_systems(Update, process_query_requests);
        app.add_systems(Update, process_inventory_requests);
        app.add_systems(Update, process_owned_requests);
        app.add_systems(Update, process_bid_list_requests);
        app.add_systems(Update, process_mailbox_requests);
        app.add_systems(Update, process_claim_mail_requests);
        app.add_systems(Update, process_create_requests);
        app.add_systems(Update, process_bid_requests);
        app.add_systems(Update, process_buyout_requests);
        app.add_systems(Update, process_cancel_requests);
        app.add_systems(FixedUpdate, expire_auctions);
    }
}

fn initialize_auction_state(db: Option<Res<GameDatabase>>, mut state: ResMut<AuctionHouseState>) {
    let Some(db) = db else { return };
    if state.initialized {
        return;
    }
    state.auctions = persistence::load_auctions(&db.db)
        .into_iter()
        .map(|auction| (auction.auction_id, auction))
        .collect();
    state.initialized = true;
}

fn process_open_requests(
    mut receivers: Query<
        (Entity, &InWorld, &mut MessageReceiver<OpenAuctionHouse>),
        With<Authenticated>,
    >,
    mut senders: Query<&mut MessageSender<AuctionHouseOpened>>,
) {
    for (entity, _in_world, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            if let Ok(mut sender) = senders.get_mut(entity) {
                sender.send::<AuctionChannel>(AuctionHouseOpened {
                    success: true,
                    error: None,
                });
            }
        }
    }
}

fn process_query_requests(
    mut receivers: Query<(Entity, &mut MessageReceiver<QueryAuctions>), With<InWorld>>,
    state: Res<AuctionHouseState>,
    mut senders: Query<&mut MessageSender<AuctionSearchResults>>,
) {
    for (entity, mut receiver) in &mut receivers {
        for req in receiver.receive() {
            let results = search_auctions(&state.auctions, &req.query);
            if let Ok(mut sender) = senders.get_mut(entity) {
                sender.send::<AuctionChannel>(results);
            }
        }
    }
}

fn process_inventory_requests(
    mut receivers: Query<(
        Entity,
        &InWorld,
        &mut MessageReceiver<QueryAuctionInventory>,
    )>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<AuctionInventorySnapshot>>,
) {
    let Some(db) = db else { return };
    for (entity, in_world, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let inventory = persistence::load_inventory(&db.db, in_world.character_id);
            if let Ok(mut sender) = senders.get_mut(entity) {
                sender.send::<AuctionChannel>(inventory_snapshot(&inventory));
            }
        }
    }
}

fn process_owned_requests(
    mut receivers: Query<(Entity, &InWorld, &mut MessageReceiver<QueryOwnedAuctions>)>,
    state: Res<AuctionHouseState>,
    mut senders: Query<&mut MessageSender<OwnedAuctionListResponse>>,
) {
    for (entity, in_world, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let listings = state
                .auctions
                .values()
                .filter(|auction| auction.seller_character_id == in_world.character_id)
                .map(to_listing_summary)
                .collect();
            if let Ok(mut sender) = senders.get_mut(entity) {
                sender.send::<AuctionChannel>(OwnedAuctionListResponse { listings });
            }
        }
    }
}

fn process_bid_list_requests(
    mut receivers: Query<(Entity, &InWorld, &mut MessageReceiver<QueryBidAuctions>)>,
    state: Res<AuctionHouseState>,
    mut senders: Query<&mut MessageSender<BidAuctionListResponse>>,
) {
    for (entity, in_world, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let listings = state
                .auctions
                .values()
                .filter(|auction| {
                    auction.current_bidder_character_id == Some(in_world.character_id)
                })
                .map(to_listing_summary)
                .collect();
            if let Ok(mut sender) = senders.get_mut(entity) {
                sender.send::<AuctionChannel>(BidAuctionListResponse { listings });
            }
        }
    }
}

fn process_mailbox_requests(
    mut receivers: Query<(Entity, &InWorld, &mut MessageReceiver<QueryAuctionMailbox>)>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<AuctionMailboxSnapshot>>,
) {
    let Some(db) = db else { return };
    for (entity, in_world, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let entries = persistence::load_mailbox(&db.db, in_world.character_id)
                .into_iter()
                .map(to_mail_entry)
                .collect();
            if let Ok(mut sender) = senders.get_mut(entity) {
                sender.send::<AuctionChannel>(AuctionMailboxSnapshot { entries });
            }
        }
    }
}

fn process_claim_mail_requests(
    mut receivers: Query<(Entity, &InWorld, &mut MessageReceiver<ClaimAuctionMail>)>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    let Some(db) = db else { return };
    for (entity, in_world, mut receiver) in &mut receivers {
        for req in receiver.receive() {
            let response = claim_mail(&db.db, in_world.character_id, req.mail_id);
            if let Ok(mut sender) = senders.get_mut(entity) {
                sender.send::<AuctionChannel>(response);
            }
        }
    }
}

fn process_create_requests(
    mut receivers: Query<(Entity, &InWorld, &mut MessageReceiver<CreateAuction>)>,
    db: Option<Res<GameDatabase>>,
    mut state: ResMut<AuctionHouseState>,
    mut senders: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    let Some(db) = db else { return };
    for (entity, in_world, mut receiver) in &mut receivers {
        for req in receiver.receive() {
            let response = create_auction(&db.db, &mut state, in_world.character_id, &req);
            if let Ok(mut sender) = senders.get_mut(entity) {
                sender.send::<AuctionChannel>(response);
            }
        }
    }
}

fn process_bid_requests(
    mut receivers: Query<(Entity, &InWorld, &mut MessageReceiver<PlaceBid>)>,
    db: Option<Res<GameDatabase>>,
    mut state: ResMut<AuctionHouseState>,
    mut senders: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    let Some(db) = db else { return };
    for (entity, in_world, mut receiver) in &mut receivers {
        for req in receiver.receive() {
            let response = place_bid(
                &db.db,
                &mut state,
                in_world.character_id,
                req.auction_id,
                req.amount,
            );
            if let Ok(mut sender) = senders.get_mut(entity) {
                sender.send::<AuctionChannel>(response);
            }
        }
    }
}

fn process_buyout_requests(
    mut receivers: Query<(Entity, &InWorld, &mut MessageReceiver<BuyoutAuction>)>,
    db: Option<Res<GameDatabase>>,
    mut state: ResMut<AuctionHouseState>,
    mut senders: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    let Some(db) = db else { return };
    for (entity, in_world, mut receiver) in &mut receivers {
        for req in receiver.receive() {
            let response =
                buyout_auction(&db.db, &mut state, in_world.character_id, req.auction_id);
            if let Ok(mut sender) = senders.get_mut(entity) {
                sender.send::<AuctionChannel>(response);
            }
        }
    }
}

fn process_cancel_requests(
    mut receivers: Query<(Entity, &InWorld, &mut MessageReceiver<CancelAuction>)>,
    db: Option<Res<GameDatabase>>,
    mut state: ResMut<AuctionHouseState>,
    mut senders: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    let Some(db) = db else { return };
    for (entity, in_world, mut receiver) in &mut receivers {
        for req in receiver.receive() {
            let response =
                cancel_auction(&db.db, &mut state, in_world.character_id, req.auction_id);
            if let Ok(mut sender) = senders.get_mut(entity) {
                sender.send::<AuctionChannel>(response);
            }
        }
    }
}

fn expire_auctions(db: Option<Res<GameDatabase>>, mut state: ResMut<AuctionHouseState>) {
    let Some(db) = db else { return };
    let now = unix_now();
    let expired: Vec<u64> = state
        .auctions
        .values()
        .filter(|auction| auction.expires_at_unix_secs <= now)
        .map(|auction| auction.auction_id)
        .collect();
    for auction_id in expired {
        let Some(auction) = state.auctions.remove(&auction_id) else {
            continue;
        };
        persistence::delete_auction(&db.db, auction_id);
        if let Some(winner_id) = auction.current_bidder_character_id {
            complete_sale(
                &db.db,
                &auction,
                winner_id,
                auction.current_bid.unwrap_or(auction.min_bid),
            );
        } else {
            deliver_mail(
                &db.db,
                auction.seller_character_id,
                "Auction expired",
                &format!(
                    "Your auction for {} expired without a buyer.",
                    auction.item.name
                ),
                0,
                Some(auction.item.clone()),
            );
        }
    }
}

fn claim_mail(db: &redb::Database, character_id: u64, mail_id: u64) -> AuctionOperationResponse {
    let mut mailbox = persistence::load_mailbox(db, character_id);
    let Some(index) = mailbox.iter().position(|entry| entry.mail_id == mail_id) else {
        return err("mail not found");
    };
    let mail = mailbox.remove(index);
    persistence::save_mailbox(db, character_id, &mailbox);
    let mut inventory = persistence::load_inventory(db, character_id);
    inventory.gold = inventory.gold.saturating_add(mail.attached_money);
    if let Some(item) = mail.attached_item {
        inventory.items.push(item);
    }
    persistence::save_inventory(db, character_id, &inventory);
    ok("mail claimed")
}

fn create_auction(
    db: &redb::Database,
    state: &mut AuctionHouseState,
    character_id: u64,
    req: &CreateAuction,
) -> AuctionOperationResponse {
    if req.stack_count == 0 {
        return err("stack count must be greater than zero");
    }
    if req.min_bid == 0 {
        return err("minimum bid must be greater than zero");
    }
    if let Some(buyout) = req.buyout_price
        && buyout < req.min_bid
    {
        return err("buyout must be greater than or equal to minimum bid");
    }

    let Some(character) = persistence::load_character(db, character_id) else {
        return err("character not found");
    };
    let mut inventory = persistence::load_inventory(db, character_id);
    let Some(index) = inventory
        .items
        .iter()
        .position(|item| item.item_guid == req.item_guid)
    else {
        return err("item not found in inventory");
    };
    let source = inventory.items[index].clone();
    if req.stack_count > source.stack_count {
        return err("requested stack exceeds available stack");
    }

    if inventory.items[index].stack_count == req.stack_count {
        inventory.items.remove(index);
    } else {
        inventory.items[index].stack_count -= req.stack_count;
    }
    persistence::save_inventory(db, character_id, &inventory);

    let auction = AuctionData {
        auction_id: persistence::next_id(db, "auction"),
        seller_character_id: character_id,
        seller_name: character.name,
        item: InventoryItemData {
            item_guid: persistence::next_id(db, "item_instance"),
            stack_count: req.stack_count,
            ..source
        },
        stack_count: req.stack_count,
        min_bid: req.min_bid,
        current_bid: None,
        current_bidder_character_id: None,
        buyout_price: req.buyout_price,
        expires_at_unix_secs: unix_now() + duration_secs(req.duration),
    };
    persistence::save_auction(db, &auction);
    state.auctions.insert(auction.auction_id, auction);
    ok("auction created")
}

fn place_bid(
    db: &redb::Database,
    state: &mut AuctionHouseState,
    character_id: u64,
    auction_id: u64,
    amount: u32,
) -> AuctionOperationResponse {
    let Some(auction) = state.auctions.get_mut(&auction_id) else {
        return err("auction not found");
    };
    if auction.seller_character_id == character_id {
        return err("cannot bid on your own auction");
    }
    let min_next = min_next_bid(auction);
    if amount < min_next {
        return err("bid is below the minimum next bid");
    }
    let mut bidder_inventory = persistence::load_inventory(db, character_id);
    let previous_bid = if auction.current_bidder_character_id == Some(character_id) {
        auction.current_bid.unwrap_or(0)
    } else {
        0
    };
    let additional = amount.saturating_sub(previous_bid);
    if bidder_inventory.gold < additional {
        return err("not enough gold");
    }
    bidder_inventory.gold -= additional;
    persistence::save_inventory(db, character_id, &bidder_inventory);

    if let Some(prev_bidder) = auction.current_bidder_character_id
        && prev_bidder != character_id
    {
        let mut prev_inventory = persistence::load_inventory(db, prev_bidder);
        prev_inventory.gold = prev_inventory
            .gold
            .saturating_add(auction.current_bid.unwrap_or(0));
        persistence::save_inventory(db, prev_bidder, &prev_inventory);
    }

    auction.current_bid = Some(amount);
    auction.current_bidder_character_id = Some(character_id);
    persistence::save_auction(db, auction);
    ok("bid placed")
}

fn buyout_auction(
    db: &redb::Database,
    state: &mut AuctionHouseState,
    character_id: u64,
    auction_id: u64,
) -> AuctionOperationResponse {
    let Some(auction) = state.auctions.remove(&auction_id) else {
        return err("auction not found");
    };
    if auction.seller_character_id == character_id {
        state.auctions.insert(auction_id, auction);
        return err("cannot buy out your own auction");
    }
    let Some(price) = auction.buyout_price else {
        state.auctions.insert(auction_id, auction);
        return err("auction has no buyout price");
    };

    let mut buyer_inventory = persistence::load_inventory(db, character_id);
    let previous_bid = if auction.current_bidder_character_id == Some(character_id) {
        auction.current_bid.unwrap_or(0)
    } else {
        0
    };
    let additional = price.saturating_sub(previous_bid);
    if buyer_inventory.gold < additional {
        state.auctions.insert(auction_id, auction);
        return err("not enough gold");
    }
    buyer_inventory.gold -= additional;
    persistence::save_inventory(db, character_id, &buyer_inventory);

    if let Some(prev_bidder) = auction.current_bidder_character_id
        && prev_bidder != character_id
    {
        let mut prev_inventory = persistence::load_inventory(db, prev_bidder);
        prev_inventory.gold = prev_inventory
            .gold
            .saturating_add(auction.current_bid.unwrap_or(0));
        persistence::save_inventory(db, prev_bidder, &prev_inventory);
    }

    persistence::delete_auction(db, auction_id);
    complete_sale(db, &auction, character_id, price);
    ok("auction bought out")
}

fn cancel_auction(
    db: &redb::Database,
    state: &mut AuctionHouseState,
    character_id: u64,
    auction_id: u64,
) -> AuctionOperationResponse {
    let Some(auction) = state.auctions.get(&auction_id) else {
        return err("auction not found");
    };
    if auction.seller_character_id != character_id {
        return err("cannot cancel another player's auction");
    }
    if auction.current_bidder_character_id.is_some() {
        return err("cannot cancel an auction that already has bids");
    }
    let auction = state.auctions.remove(&auction_id).expect("auction present");
    persistence::delete_auction(db, auction_id);
    deliver_mail(
        db,
        character_id,
        "Auction cancelled",
        &format!("Your auction for {} was cancelled.", auction.item.name),
        0,
        Some(auction.item),
    );
    ok("auction cancelled")
}

fn complete_sale(
    db: &redb::Database,
    auction: &AuctionData,
    buyer_character_id: u64,
    sale_price: u32,
) {
    let seller_cut = sale_price.saturating_mul(HOUSE_CUT_BPS) / 10_000;
    let seller_proceeds = sale_price.saturating_sub(seller_cut);
    deliver_mail(
        db,
        auction.seller_character_id,
        "Auction sold",
        &format!("Your auction for {} sold.", auction.item.name),
        seller_proceeds,
        None,
    );
    deliver_mail(
        db,
        buyer_character_id,
        "Auction won",
        &format!("You won {}.", auction.item.name),
        0,
        Some(auction.item.clone()),
    );
}

fn deliver_mail(
    db: &redb::Database,
    character_id: u64,
    subject: &str,
    body: &str,
    attached_money: u32,
    attached_item: Option<InventoryItemData>,
) {
    persistence::append_mail(
        db,
        character_id,
        MailEntryData {
            mail_id: persistence::next_id(db, "mail"),
            subject: subject.to_string(),
            body: body.to_string(),
            attached_money,
            attached_item,
        },
    );
}

fn search_auctions(
    auctions: &HashMap<u64, AuctionData>,
    query: &AuctionSearchQuery,
) -> AuctionSearchResults {
    let needle = query.text.to_ascii_lowercase();
    let mut listings: Vec<_> = auctions
        .values()
        .filter(|auction| {
            needle.is_empty() || auction.item.name.to_ascii_lowercase().contains(&needle)
        })
        .filter(|auction| query.quality.is_none_or(|q| auction.item.quality == q))
        .filter(|auction| {
            query
                .min_level
                .is_none_or(|min| auction.item.required_level >= min)
        })
        .filter(|auction| {
            query
                .max_level
                .is_none_or(|max| auction.item.required_level <= max)
        })
        .map(to_listing_summary)
        .collect();

    listings.sort_by(|a, b| compare_listing(a, b, query.sort_field));
    if matches!(query.sort_dir, AuctionSortDir::Desc) {
        listings.reverse();
    }

    let total_results = listings.len() as u32;
    let page_size = query.page_size.clamp(1, MAX_PAGE_SIZE) as usize;
    let start = query.page as usize * page_size;
    let results = listings.into_iter().skip(start).take(page_size).collect();
    AuctionSearchResults {
        query: query.clone(),
        total_results,
        results,
    }
}

fn compare_listing(
    a: &AuctionListingSummary,
    b: &AuctionListingSummary,
    field: AuctionSortField,
) -> std::cmp::Ordering {
    match field {
        AuctionSortField::Name => a.item.name.cmp(&b.item.name),
        AuctionSortField::MinBid => a.min_bid.cmp(&b.min_bid),
        AuctionSortField::Buyout => a
            .buyout_price
            .unwrap_or(u32::MAX)
            .cmp(&b.buyout_price.unwrap_or(u32::MAX)),
        AuctionSortField::TimeLeft => {
            auction_time_rank(a.time_left).cmp(&auction_time_rank(b.time_left))
        }
        AuctionSortField::Quality => a.item.quality.cmp(&b.item.quality),
        AuctionSortField::RequiredLevel => a.item.required_level.cmp(&b.item.required_level),
    }
}

fn auction_time_rank(time_left: AuctionTimeLeft) -> u8 {
    match time_left {
        AuctionTimeLeft::Short => 0,
        AuctionTimeLeft::Medium => 1,
        AuctionTimeLeft::Long => 2,
        AuctionTimeLeft::VeryLong => 3,
    }
}

fn inventory_snapshot(inventory: &InventoryData) -> AuctionInventorySnapshot {
    AuctionInventorySnapshot {
        gold: inventory.gold,
        items: inventory
            .items
            .iter()
            .cloned()
            .map(to_inventory_item)
            .collect(),
    }
}

fn to_inventory_item(item: InventoryItemData) -> AuctionInventoryItem {
    AuctionInventoryItem {
        item_guid: item.item_guid,
        item_id: item.item_id,
        name: item.name,
        quality: item.quality,
        required_level: item.required_level,
        stack_count: item.stack_count,
        vendor_sell_price: item.vendor_sell_price,
    }
}

fn to_listing_summary(auction: &AuctionData) -> AuctionListingSummary {
    AuctionListingSummary {
        auction_id: auction.auction_id,
        item: to_inventory_item(auction.item.clone()),
        owner_name: auction.seller_name.clone(),
        stack_count: auction.stack_count,
        min_bid: auction.min_bid,
        current_bid: auction.current_bid,
        min_next_bid: min_next_bid(auction),
        buyout_price: auction.buyout_price,
        time_left: time_left(auction.expires_at_unix_secs),
    }
}

fn to_mail_entry(mail: MailEntryData) -> shared::protocol::AuctionMailEntry {
    shared::protocol::AuctionMailEntry {
        mail_id: mail.mail_id,
        subject: mail.subject,
        body: mail.body,
        attached_money: mail.attached_money,
        attached_item: mail.attached_item.map(to_inventory_item),
    }
}

fn min_next_bid(auction: &AuctionData) -> u32 {
    match auction.current_bid {
        Some(current) => current.saturating_add((current / 20).max(1)),
        None => auction.min_bid,
    }
}

fn time_left(expires_at_unix_secs: u64) -> AuctionTimeLeft {
    let remaining = expires_at_unix_secs.saturating_sub(unix_now());
    if remaining <= 60 {
        AuctionTimeLeft::Short
    } else if remaining <= 60 * 30 {
        AuctionTimeLeft::Medium
    } else if remaining <= 60 * 60 * 6 {
        AuctionTimeLeft::Long
    } else {
        AuctionTimeLeft::VeryLong
    }
}

fn duration_secs(duration: AuctionDuration) -> u64 {
    match duration {
        AuctionDuration::Short => 60 * 30,
        AuctionDuration::Medium => 60 * 60 * 2,
        AuctionDuration::Long => 60 * 60 * 8,
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn ok(message: &str) -> AuctionOperationResponse {
    AuctionOperationResponse {
        success: true,
        message: message.to_string(),
    }
}

fn err(message: &str) -> AuctionOperationResponse {
    AuctionOperationResponse {
        success: false,
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_next_bid_rounds_up() {
        let auction = AuctionData {
            auction_id: 1,
            seller_character_id: 1,
            seller_name: "seller".into(),
            item: InventoryItemData {
                item_guid: 1,
                item_id: 1,
                name: "Linen Cloth".into(),
                quality: 1,
                required_level: 1,
                stack_count: 5,
                vendor_sell_price: 1,
            },
            stack_count: 5,
            min_bid: 100,
            current_bid: Some(100),
            current_bidder_character_id: Some(2),
            buyout_price: Some(150),
            expires_at_unix_secs: unix_now() + 60,
        };
        assert_eq!(min_next_bid(&auction), 105);
    }
}
