use super::*;
use crate::gold::{GoldUpdateQueue, record_persisted_gold};
use crate::npc_interaction::ActiveInteraction;

pub(super) fn initialize_auction_state(
    db: Option<Res<GameDatabase>>,
    mut state: ResMut<AuctionHouseState>,
) {
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

/// Rejection for auction requests without an open auctioneer interaction.
pub(super) const NO_AUCTIONEER: &str = "not interacting with an auctioneer";

/// Auction requests from in-world connections plus their open NPC interaction.
type SessionReceiver<'w, 's, M> = Query<
    'w,
    's,
    (
        Entity,
        &'static InWorld,
        Option<&'static ActiveInteraction>,
        &'static mut MessageReceiver<M>,
    ),
>;

/// Hands each request its character and auction house, or `None` when the
/// connection has no auctioneer frame open. TrinityCore validates the auctioneer
/// (`GetNPCIfCanInteractWith(.., UNIT_NPC_FLAG_AUCTIONEER)`) on every auction packet.
fn for_each_session_request<M, F>(receivers: &mut SessionReceiver<M>, mut handle: F)
where
    M: lightyear::prelude::Message,
    F: FnMut(Entity, u64, Option<u8>, M),
{
    for (entity, in_world, session, mut receiver) in &mut *receivers {
        let house = session.and_then(|interaction| interaction.auction_house);
        for req in receiver.receive() {
            handle(entity, in_world.character_id, house, req);
        }
    }
}

pub(super) fn process_open_requests(
    mut receivers: SessionReceiver<OpenAuctionHouse>,
    mut senders: Query<&mut MessageSender<AuctionHouseOpened>>,
) {
    for_each_session_request(&mut receivers, |entity, _, house, _| {
        let response = AuctionHouseOpened {
            success: house.is_some(),
            error: house.is_none().then(|| NO_AUCTIONEER.to_string()),
        };
        send_auction_message(&mut senders, entity, response);
    });
}

pub(super) fn process_query_requests(
    mut receivers: SessionReceiver<QueryAuctions>,
    state: Res<AuctionHouseState>,
    mut senders: Query<&mut MessageSender<AuctionSearchResults>>,
    mut rejections: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    for_each_session_request(&mut receivers, |entity, _, house, mut req| {
        let Some(house) = house else {
            send_auction_message(&mut rejections, entity, err(NO_AUCTIONEER));
            return;
        };
        req.query.faction = house;
        send_auction_message(
            &mut senders,
            entity,
            search_auctions(&state.auctions, &req.query),
        );
    });
}

pub(super) fn process_inventory_requests(
    mut receivers: SessionReceiver<QueryAuctionInventory>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<AuctionInventorySnapshot>>,
    mut rejections: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    let Some(db) = db else { return };
    for_each_session_request(&mut receivers, |entity, character_id, house, _| {
        if house.is_none() {
            send_auction_message(&mut rejections, entity, err(NO_AUCTIONEER));
            return;
        }
        let inventory = persistence::load_inventory(&db.db, character_id);
        send_auction_message(&mut senders, entity, inventory_snapshot(&inventory));
    });
}

pub(super) fn process_owned_requests(
    mut receivers: SessionReceiver<QueryOwnedAuctions>,
    state: Res<AuctionHouseState>,
    mut senders: Query<&mut MessageSender<OwnedAuctionListResponse>>,
    mut rejections: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    for_each_session_request(&mut receivers, |entity, character_id, house, _| {
        if house.is_none() {
            send_auction_message(&mut rejections, entity, err(NO_AUCTIONEER));
            return;
        }
        let listings = state
            .auctions
            .values()
            .filter(|auction| auction.seller_character_id == character_id)
            .map(to_listing_summary)
            .collect();
        send_auction_message(&mut senders, entity, OwnedAuctionListResponse { listings });
    });
}

pub(super) fn process_bid_list_requests(
    mut receivers: SessionReceiver<QueryBidAuctions>,
    state: Res<AuctionHouseState>,
    mut senders: Query<&mut MessageSender<BidAuctionListResponse>>,
    mut rejections: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    for_each_session_request(&mut receivers, |entity, character_id, house, _| {
        if house.is_none() {
            send_auction_message(&mut rejections, entity, err(NO_AUCTIONEER));
            return;
        }
        let listings = state
            .auctions
            .values()
            .filter(|auction| auction.current_bidder_character_id == Some(character_id))
            .map(to_listing_summary)
            .collect();
        send_auction_message(&mut senders, entity, BidAuctionListResponse { listings });
    });
}

pub(super) fn process_mailbox_requests(
    mut receivers: Query<(Entity, &InWorld, &mut MessageReceiver<QueryAuctionMailbox>)>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<AuctionMailboxSnapshot>>,
) {
    let Some(db) = db else { return };
    process_in_world_requests(&mut receivers, &mut senders, |character_id, _| {
        let entries = persistence::load_mailbox(&db.db, character_id)
            .into_iter()
            .map(to_mail_entry)
            .collect();
        AuctionMailboxSnapshot { entries }
    });
}

pub(super) fn process_claim_mail_requests(
    mut receivers: Query<(Entity, &InWorld, &mut MessageReceiver<ClaimAuctionMail>)>,
    db: Option<Res<GameDatabase>>,
    mut gold_updates: ResMut<GoldUpdateQueue>,
    mut senders: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    process_operation_requests(&mut receivers, db, &mut senders, |db, character_id, req| {
        claim_mail_with_gold_update(&db.db, character_id, req.mail_id, &mut gold_updates)
    });
}

pub(super) fn process_create_requests(
    mut receivers: SessionReceiver<CreateAuction>,
    db: Option<Res<GameDatabase>>,
    mut state: ResMut<AuctionHouseState>,
    mut gold_updates: ResMut<GoldUpdateQueue>,
    mut senders: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    process_session_operations(
        &mut receivers,
        db,
        &mut senders,
        |db, character_id, house, req| {
            create_auction_with_gold_update(
                &db.db,
                &mut state,
                character_id,
                house,
                &req,
                &mut gold_updates,
            )
        },
    );
}

pub(super) fn process_bid_requests(
    mut receivers: SessionReceiver<PlaceBid>,
    db: Option<Res<GameDatabase>>,
    mut state: ResMut<AuctionHouseState>,
    mut gold_updates: ResMut<GoldUpdateQueue>,
    mut senders: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    process_session_operations(
        &mut receivers,
        db,
        &mut senders,
        |db, character_id, _, req| {
            place_bid_with_gold_updates(
                &db.db,
                &mut state,
                character_id,
                req.auction_id,
                req.amount,
                &mut gold_updates,
            )
        },
    );
}

pub(super) fn process_buyout_requests(
    mut receivers: SessionReceiver<BuyoutAuction>,
    db: Option<Res<GameDatabase>>,
    mut state: ResMut<AuctionHouseState>,
    mut gold_updates: ResMut<GoldUpdateQueue>,
    mut senders: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    process_session_operations(
        &mut receivers,
        db,
        &mut senders,
        |db, character_id, _, req| {
            buyout_auction_with_gold_updates(
                &db.db,
                &mut state,
                character_id,
                req.auction_id,
                &mut gold_updates,
            )
        },
    );
}

pub(super) fn process_cancel_requests(
    mut receivers: SessionReceiver<CancelAuction>,
    db: Option<Res<GameDatabase>>,
    mut state: ResMut<AuctionHouseState>,
    mut senders: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    process_session_operations(
        &mut receivers,
        db,
        &mut senders,
        |db, character_id, _, req| cancel_auction(&db.db, &mut state, character_id, req.auction_id),
    );
}

pub(super) fn expire_auctions(db: Option<Res<GameDatabase>>, mut state: ResMut<AuctionHouseState>) {
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
        return;
    }
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

pub(super) fn claim_mail_with_gold_update(
    db: &redb::Database,
    character_id: u64,
    mail_id: u64,
    gold_updates: &mut GoldUpdateQueue,
) -> AuctionOperationResponse {
    let response = claim_mail(db, character_id, mail_id);
    record_successful_gold_updates(db, &response, [character_id], gold_updates);
    response
}

pub(super) fn create_auction_with_gold_update(
    db: &redb::Database,
    state: &mut AuctionHouseState,
    character_id: u64,
    house: u8,
    request: &CreateAuction,
    gold_updates: &mut GoldUpdateQueue,
) -> AuctionOperationResponse {
    let response = create_auction(db, state, character_id, house, request);
    record_successful_gold_updates(db, &response, [character_id], gold_updates);
    response
}

pub(super) fn place_bid_with_gold_updates(
    db: &redb::Database,
    state: &mut AuctionHouseState,
    character_id: u64,
    auction_id: u64,
    amount: u32,
    gold_updates: &mut GoldUpdateQueue,
) -> AuctionOperationResponse {
    let previous_bidder = state
        .auctions
        .get(&auction_id)
        .and_then(|auction| auction.current_bidder_character_id);
    let response = place_bid(db, state, character_id, auction_id, amount);
    record_successful_gold_updates(
        db,
        &response,
        [Some(character_id), previous_bidder].into_iter().flatten(),
        gold_updates,
    );
    response
}

pub(super) fn buyout_auction_with_gold_updates(
    db: &redb::Database,
    state: &mut AuctionHouseState,
    character_id: u64,
    auction_id: u64,
    gold_updates: &mut GoldUpdateQueue,
) -> AuctionOperationResponse {
    let previous_bidder = state
        .auctions
        .get(&auction_id)
        .and_then(|auction| auction.current_bidder_character_id);
    let response = buyout_auction(db, state, character_id, auction_id);
    record_successful_gold_updates(
        db,
        &response,
        [Some(character_id), previous_bidder].into_iter().flatten(),
        gold_updates,
    );
    response
}

fn record_successful_gold_updates(
    db: &redb::Database,
    response: &AuctionOperationResponse,
    character_ids: impl IntoIterator<Item = u64>,
    gold_updates: &mut GoldUpdateQueue,
) {
    if !response.success {
        return;
    }
    for character_id in character_ids {
        record_persisted_gold(db, character_id, gold_updates);
    }
}

fn process_in_world_requests<M, R, F>(
    receivers: &mut Query<(Entity, &InWorld, &mut MessageReceiver<M>)>,
    senders: &mut Query<&mut MessageSender<R>>,
    mut build_response: F,
) where
    M: lightyear::prelude::Message,
    R: lightyear::prelude::Message,
    F: FnMut(u64, M) -> R,
{
    for (entity, in_world, mut receiver) in &mut *receivers {
        for req in receiver.receive() {
            let response = build_response(in_world.character_id, req);
            send_auction_message(senders, entity, response);
        }
    }
}

fn process_operation_requests<M, F>(
    receivers: &mut Query<(Entity, &InWorld, &mut MessageReceiver<M>)>,
    db: Option<Res<GameDatabase>>,
    senders: &mut Query<&mut MessageSender<AuctionOperationResponse>>,
    mut build_response: F,
) where
    M: lightyear::prelude::Message,
    F: FnMut(&GameDatabase, u64, M) -> AuctionOperationResponse,
{
    let Some(db) = db else { return };
    process_in_world_requests(receivers, senders, |character_id, req| {
        build_response(&db, character_id, req)
    });
}

fn process_session_operations<M, F>(
    receivers: &mut SessionReceiver<M>,
    db: Option<Res<GameDatabase>>,
    senders: &mut Query<&mut MessageSender<AuctionOperationResponse>>,
    mut build_response: F,
) where
    M: lightyear::prelude::Message,
    F: FnMut(&GameDatabase, u64, u8, M) -> AuctionOperationResponse,
{
    let Some(db) = db else { return };
    for_each_session_request(receivers, |entity, character_id, house, req| {
        let response = match house {
            Some(house) => build_response(&db, character_id, house, req),
            None => err(NO_AUCTIONEER),
        };
        send_auction_message(senders, entity, response);
    });
}

fn send_auction_message<M>(senders: &mut Query<&mut MessageSender<M>>, entity: Entity, message: M)
where
    M: lightyear::prelude::Message,
{
    if let Ok(mut sender) = senders.get_mut(entity) {
        sender.send::<AuctionChannel>(message);
    }
}
