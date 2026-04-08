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
const MAX_AUCTIONS_PER_CHARACTER: usize = 50;

/// Deposit = vendor_sell_price * stack_count * duration_multiplier.
fn deposit_cost(vendor_sell_price: u32, stack_count: u32, duration: AuctionDuration) -> u32 {
    let multiplier = match duration {
        AuctionDuration::Short => 1,
        AuctionDuration::Medium => 2,
        AuctionDuration::Long => 4,
    };
    vendor_sell_price
        .saturating_mul(stack_count)
        .saturating_mul(multiplier)
}

/// Map race ID to auction faction: 1=Alliance, 2=Horde.
fn faction_for_race(race: u8) -> u8 {
    match race {
        1 | 3 | 4 | 7 | 11 | 22 | 25 | 29 | 30 | 32 | 34 | 37 => 1, // Alliance
        2 | 5 | 6 | 8 | 9 | 10 | 26 | 27 | 28 | 31 | 35 | 36 => 2,  // Horde
        _ => 0,                                                     // Neutral
    }
}

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
        expire_single_auction(&db.db, auction_id, &auction);
    }
}

fn expire_single_auction(db: &redb::Database, auction_id: u64, auction: &AuctionData) {
    persistence::delete_auction(db, auction_id);
    if let Some(winner_id) = auction.current_bidder_character_id {
        complete_sale(
            db,
            auction,
            winner_id,
            auction.current_bid.unwrap_or(auction.min_bid),
        );
    } else {
        deliver_mail(
            db,
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

/// Validated inputs for creating an auction.
struct ValidatedAuction {
    character_name: String,
    race: u8,
    source: InventoryItemData,
    item_index: usize,
    deposit: u32,
}

fn validate_auction_request(req: &CreateAuction) -> Result<(), AuctionOperationResponse> {
    if req.stack_count == 0 {
        return Err(err("stack count must be greater than zero"));
    }
    if req.min_bid == 0 {
        return Err(err("minimum bid must be greater than zero"));
    }
    if let Some(buyout) = req.buyout_price
        && buyout < req.min_bid
    {
        return Err(err("buyout must be greater than or equal to minimum bid"));
    }
    Ok(())
}

fn validate_create_auction(
    db: &redb::Database,
    state: &AuctionHouseState,
    character_id: u64,
    req: &CreateAuction,
) -> Result<ValidatedAuction, AuctionOperationResponse> {
    validate_auction_request(req)?;
    validate_auction_slot_limit(state, character_id)?;
    let Some(character) = persistence::load_character(db, character_id) else {
        return Err(err("character not found"));
    };
    let inventory = persistence::load_inventory(db, character_id);
    let (index, source) = find_auction_source_item(&inventory, req)?;
    if req.stack_count > source.stack_count {
        return Err(err("requested stack exceeds available stack"));
    }
    validate_unique_auction_listing(state, &source)?;
    let deposit = deposit_cost(source.vendor_sell_price, req.stack_count, req.duration);
    validate_auction_deposit(&inventory, deposit)?;
    Ok(ValidatedAuction {
        character_name: character.name,
        race: character.race,
        source,
        item_index: index,
        deposit,
    })
}

fn validate_auction_slot_limit(
    state: &AuctionHouseState,
    character_id: u64,
) -> Result<(), AuctionOperationResponse> {
    let active_count = state
        .auctions
        .values()
        .filter(|auction| auction.seller_character_id == character_id)
        .count();
    if active_count >= MAX_AUCTIONS_PER_CHARACTER {
        return Err(err("maximum auction slots reached"));
    }
    Ok(())
}

fn find_auction_source_item(
    inventory: &InventoryData,
    req: &CreateAuction,
) -> Result<(usize, InventoryItemData), AuctionOperationResponse> {
    let Some(index) = inventory
        .items
        .iter()
        .position(|item| item.item_guid == req.item_guid)
    else {
        return Err(err("item not found in inventory"));
    };
    Ok((index, inventory.items[index].clone()))
}

fn validate_unique_auction_listing(
    state: &AuctionHouseState,
    source: &InventoryItemData,
) -> Result<(), AuctionOperationResponse> {
    if source.unique_auction
        && state
            .auctions
            .values()
            .any(|auction| auction.item.item_id == source.item_id)
    {
        return Err(err("unique item already listed on the auction house"));
    }
    Ok(())
}

fn validate_auction_deposit(
    inventory: &InventoryData,
    deposit: u32,
) -> Result<(), AuctionOperationResponse> {
    if deposit > 0 && inventory.gold < deposit {
        return Err(err("not enough gold for auction deposit"));
    }
    Ok(())
}

fn create_auction(
    db: &redb::Database,
    state: &mut AuctionHouseState,
    character_id: u64,
    req: &CreateAuction,
) -> AuctionOperationResponse {
    let validated = match validate_create_auction(db, state, character_id, req) {
        Ok(v) => v,
        Err(e) => return e,
    };

    let mut inventory = persistence::load_inventory(db, character_id);
    inventory.gold -= validated.deposit;
    if inventory.items[validated.item_index].stack_count == req.stack_count {
        inventory.items.remove(validated.item_index);
    } else {
        inventory.items[validated.item_index].stack_count -= req.stack_count;
    }
    persistence::save_inventory(db, character_id, &inventory);

    let auction = AuctionData {
        auction_id: persistence::next_id(db, "auction"),
        seller_character_id: character_id,
        seller_name: validated.character_name,
        item: InventoryItemData {
            item_guid: persistence::next_id(db, "item_instance"),
            stack_count: req.stack_count,
            ..validated.source
        },
        stack_count: req.stack_count,
        min_bid: req.min_bid,
        current_bid: None,
        current_bidder_character_id: None,
        buyout_price: req.buyout_price,
        expires_at_unix_secs: unix_now() + duration_secs(req.duration),
        faction: faction_for_race(validated.race),
        deposit: validated.deposit,
    };
    persistence::save_auction(db, &auction);
    state.auctions.insert(auction.auction_id, auction);
    ok("auction created")
}

/// Gold already held by the auction house from a previous bid by this character.
fn previous_bid_credit(auction: &AuctionData, character_id: u64) -> u32 {
    if auction.current_bidder_character_id == Some(character_id) {
        auction.current_bid.unwrap_or(0)
    } else {
        0
    }
}

/// Refund the previous bidder's gold and send outbid notification mail.
fn refund_previous_bidder(db: &redb::Database, auction: &AuctionData, new_bidder: u64) {
    let Some(prev_bidder) = auction.current_bidder_character_id else {
        return;
    };
    if prev_bidder == new_bidder {
        return;
    }
    let refund = auction.current_bid.unwrap_or(0);
    let mut prev_inventory = persistence::load_inventory(db, prev_bidder);
    prev_inventory.gold = prev_inventory.gold.saturating_add(refund);
    persistence::save_inventory(db, prev_bidder, &prev_inventory);
    deliver_mail(
        db,
        prev_bidder,
        "Outbid",
        &format!(
            "You have been outbid on {}. Your {} gold has been refunded.",
            auction.item.name, refund
        ),
        0,
        None,
    );
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
    let credit = previous_bid_credit(auction, character_id);
    let additional = amount.saturating_sub(credit);
    if bidder_inventory.gold < additional {
        return err("not enough gold");
    }
    bidder_inventory.gold -= additional;
    persistence::save_inventory(db, character_id, &bidder_inventory);

    refund_previous_bidder(db, auction, character_id);

    auction.current_bid = Some(amount);
    auction.current_bidder_character_id = Some(character_id);

    // Bid meeting buyout price triggers immediate sale
    if let Some(buyout) = auction.buyout_price
        && amount >= buyout
    {
        let auction = state.auctions.remove(&auction_id).expect("auction present");
        persistence::delete_auction(db, auction_id);
        complete_sale(db, &auction, character_id, amount);
        return ok("bid met buyout — auction sold");
    }

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
    let credit = previous_bid_credit(&auction, character_id);
    let additional = price.saturating_sub(credit);
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
    let house_fee = sale_price.saturating_mul(HOUSE_CUT_BPS) / 10_000;
    let seller_proceeds = sale_price
        .saturating_sub(house_fee)
        .saturating_add(auction.deposit);
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
        .filter(|auction| matches_search_query(auction, query, &needle))
        .map(to_listing_summary)
        .collect();

    listings.sort_by(|a, b| compare_listing(a, b, query.sort_field));
    if matches!(query.sort_dir, AuctionSortDir::Desc) {
        listings.reverse();
    }

    let total_results = listings.len() as u32;
    let results = paged_listings(listings, query);
    AuctionSearchResults {
        query: query.clone(),
        total_results,
        results,
    }
}

fn matches_search_query(auction: &AuctionData, query: &AuctionSearchQuery, needle: &str) -> bool {
    name_matches(auction, needle)
        && quality_matches(auction, query)
        && level_matches(auction, query)
        && faction_matches(auction, query)
}

fn name_matches(auction: &AuctionData, needle: &str) -> bool {
    needle.is_empty() || auction.item.name.to_ascii_lowercase().contains(needle)
}

fn quality_matches(auction: &AuctionData, query: &AuctionSearchQuery) -> bool {
    query
        .quality
        .is_none_or(|quality| auction.item.quality == quality)
}

fn level_matches(auction: &AuctionData, query: &AuctionSearchQuery) -> bool {
    query
        .min_level
        .is_none_or(|min_level| auction.item.required_level >= min_level)
        && query
            .max_level
            .is_none_or(|max_level| auction.item.required_level <= max_level)
}

fn faction_matches(auction: &AuctionData, query: &AuctionSearchQuery) -> bool {
    query.faction == 0 || auction.faction == 0 || auction.faction == query.faction
}

fn paged_listings(
    listings: Vec<AuctionListingSummary>,
    query: &AuctionSearchQuery,
) -> Vec<AuctionListingSummary> {
    let page_size = query.page_size.clamp(1, MAX_PAGE_SIZE) as usize;
    let start = query.page as usize * page_size;
    listings.into_iter().skip(start).take(page_size).collect()
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

const TIME_LEFT_SHORT_SECS: u64 = 60;
const TIME_LEFT_MEDIUM_SECS: u64 = 60 * 30;
const TIME_LEFT_LONG_SECS: u64 = 60 * 60 * 6;

fn time_left(expires_at_unix_secs: u64) -> AuctionTimeLeft {
    let remaining = expires_at_unix_secs.saturating_sub(unix_now());
    if remaining <= TIME_LEFT_SHORT_SECS {
        AuctionTimeLeft::Short
    } else if remaining <= TIME_LEFT_MEDIUM_SECS {
        AuctionTimeLeft::Medium
    } else if remaining <= TIME_LEFT_LONG_SECS {
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
#[path = "auction_house_tests.rs"]
mod tests;
