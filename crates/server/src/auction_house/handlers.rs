use bevy::ecs::query::QueryFilter;

use super::*;
use crate::gold::{GoldUpdateQueue, record_persisted_gold};

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

type OpenAuctionReceiver<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static mut MessageReceiver<OpenAuctionHouse>),
    (With<Authenticated>, With<InWorld>),
>;

pub(super) fn process_open_requests(
    mut receivers: OpenAuctionReceiver,
    mut senders: Query<&mut MessageSender<AuctionHouseOpened>>,
) {
    process_entity_requests(&mut receivers, &mut senders, |_| AuctionHouseOpened {
        success: true,
        error: None,
    });
}

pub(super) fn process_query_requests(
    mut receivers: Query<(Entity, &mut MessageReceiver<QueryAuctions>), With<InWorld>>,
    state: Res<AuctionHouseState>,
    mut senders: Query<&mut MessageSender<AuctionSearchResults>>,
) {
    process_entity_requests(&mut receivers, &mut senders, |req: QueryAuctions| {
        search_auctions(&state.auctions, &req.query)
    });
}

pub(super) fn process_inventory_requests(
    mut receivers: Query<(
        Entity,
        &InWorld,
        &mut MessageReceiver<QueryAuctionInventory>,
    )>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<AuctionInventorySnapshot>>,
) {
    let Some(db) = db else { return };
    process_in_world_requests(&mut receivers, &mut senders, |character_id, _| {
        let inventory = persistence::load_inventory(&db.db, character_id);
        inventory_snapshot(&inventory)
    });
}

pub(super) fn process_owned_requests(
    mut receivers: Query<(Entity, &InWorld, &mut MessageReceiver<QueryOwnedAuctions>)>,
    state: Res<AuctionHouseState>,
    mut senders: Query<&mut MessageSender<OwnedAuctionListResponse>>,
) {
    process_in_world_requests(&mut receivers, &mut senders, |character_id, _| {
        let listings = state
            .auctions
            .values()
            .filter(|auction| auction.seller_character_id == character_id)
            .map(to_listing_summary)
            .collect();
        OwnedAuctionListResponse { listings }
    });
}

pub(super) fn process_bid_list_requests(
    mut receivers: Query<(Entity, &InWorld, &mut MessageReceiver<QueryBidAuctions>)>,
    state: Res<AuctionHouseState>,
    mut senders: Query<&mut MessageSender<BidAuctionListResponse>>,
) {
    process_in_world_requests(&mut receivers, &mut senders, |character_id, _| {
        let listings = state
            .auctions
            .values()
            .filter(|auction| auction.current_bidder_character_id == Some(character_id))
            .map(to_listing_summary)
            .collect();
        BidAuctionListResponse { listings }
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
    mut receivers: Query<(Entity, &InWorld, &mut MessageReceiver<CreateAuction>)>,
    db: Option<Res<GameDatabase>>,
    mut state: ResMut<AuctionHouseState>,
    mut gold_updates: ResMut<GoldUpdateQueue>,
    mut senders: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    process_operation_requests(&mut receivers, db, &mut senders, |db, character_id, req| {
        create_auction_with_gold_update(&db.db, &mut state, character_id, &req, &mut gold_updates)
    });
}

pub(super) fn process_bid_requests(
    mut receivers: Query<(Entity, &InWorld, &mut MessageReceiver<PlaceBid>)>,
    db: Option<Res<GameDatabase>>,
    mut state: ResMut<AuctionHouseState>,
    mut gold_updates: ResMut<GoldUpdateQueue>,
    mut senders: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    process_stateful_operation_requests(
        &mut receivers,
        db,
        &mut state,
        &mut senders,
        |db, state, character_id, req| {
            place_bid_with_gold_updates(
                &db.db,
                state,
                character_id,
                req.auction_id,
                req.amount,
                &mut gold_updates,
            )
        },
    );
}

pub(super) fn process_buyout_requests(
    mut receivers: Query<(Entity, &InWorld, &mut MessageReceiver<BuyoutAuction>)>,
    db: Option<Res<GameDatabase>>,
    mut state: ResMut<AuctionHouseState>,
    mut gold_updates: ResMut<GoldUpdateQueue>,
    mut senders: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    process_stateful_operation_requests(
        &mut receivers,
        db,
        &mut state,
        &mut senders,
        |db, state, character_id, req| {
            buyout_auction_with_gold_updates(
                &db.db,
                state,
                character_id,
                req.auction_id,
                &mut gold_updates,
            )
        },
    );
}

pub(super) fn process_cancel_requests(
    mut receivers: Query<(Entity, &InWorld, &mut MessageReceiver<CancelAuction>)>,
    db: Option<Res<GameDatabase>>,
    mut state: ResMut<AuctionHouseState>,
    mut senders: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    process_stateful_operation_requests(
        &mut receivers,
        db,
        &mut state,
        &mut senders,
        |db, state, character_id, req| cancel_auction(&db.db, state, character_id, req.auction_id),
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
    request: &CreateAuction,
    gold_updates: &mut GoldUpdateQueue,
) -> AuctionOperationResponse {
    let response = create_auction(db, state, character_id, request);
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

fn process_entity_requests<M, R, F, QF>(
    receivers: &mut Query<(Entity, &mut MessageReceiver<M>), QF>,
    senders: &mut Query<&mut MessageSender<R>>,
    mut build_response: F,
) where
    M: lightyear::prelude::Message,
    R: lightyear::prelude::Message,
    F: FnMut(M) -> R,
    QF: QueryFilter,
{
    for (entity, mut receiver) in &mut *receivers {
        for req in receiver.receive() {
            send_auction_message(senders, entity, build_response(req));
        }
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

fn process_stateful_operation_requests<M, F>(
    receivers: &mut Query<(Entity, &InWorld, &mut MessageReceiver<M>)>,
    db: Option<Res<GameDatabase>>,
    state: &mut AuctionHouseState,
    senders: &mut Query<&mut MessageSender<AuctionOperationResponse>>,
    mut build_response: F,
) where
    M: lightyear::prelude::Message,
    F: FnMut(&GameDatabase, &mut AuctionHouseState, u64, M) -> AuctionOperationResponse,
{
    let Some(db) = db else { return };
    process_in_world_requests(receivers, senders, |character_id, req| {
        build_response(&db, state, character_id, req)
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
