use super::*;
use crate::gold::GoldUpdateQueue;
#[cfg(test)]
use crate::gold::record_persisted_gold;
use crate::npc_interaction::ActiveInteraction;
use redb::ReadableDatabase;

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

pub(super) fn search_for_viewer(
    auctions: &HashMap<u64, AuctionData>,
    query: &AuctionSearchQuery,
    templates: Option<&crate::item_templates::ItemTemplates>,
    viewer: Option<(u8, u8, u16)>,
    sources: super::queries::DefinitionSources<'_>,
) -> AuctionSearchResults {
    if query.class_id.is_none() && query.subcategory_filters.is_empty() && !query.usable_only {
        return search_auctions(auctions, query, sources);
    }
    super::queries::search_auctions_matching(auctions, query, sources, |auction| {
        auction_matches_viewer(
            auction,
            query,
            templates,
            viewer,
            sources(auction.item.item_guid),
        )
    })
}

pub(super) fn auction_matches_viewer(
    auction: &AuctionData,
    query: &AuctionSearchQuery,
    templates: Option<&crate::item_templates::ItemTemplates>,
    viewer: Option<(u8, u8, u16)>,
    source: shared::item_data::ItemDefinitionSource,
) -> bool {
    if query.class_id.is_none() && query.subcategory_filters.is_empty() && !query.usable_only {
        return true;
    }
    let Some(template) =
        templates.and_then(|templates| templates.get(source, auction.item.item_id))
    else {
        return false;
    };
    if query
        .class_id
        .is_some_and(|class| class != template.scaling.class_id)
    {
        return false;
    }
    // Retail category nodes OR their class/subclass/inventory alternatives;
    // TrinityCore a352b1fa AuctionHouseMgr.cpp:1182-1198 filters before paging.
    if !query.subcategory_filters.is_empty()
        && !query.subcategory_filters.iter().any(|filter| {
            filter.class_id == template.scaling.class_id
                && filter
                    .subclass_id
                    .is_none_or(|id| id == template.scaling.subclass_id)
                && filter
                    .inventory_type
                    .is_none_or(|kind| kind == template.inventory_type)
        })
    {
        return false;
    }
    !query.usable_only
        || viewer.is_some_and(|(race, class, level)| {
            template.required_level <= level && template.allows(class, race)
        })
}

pub(super) fn process_browse_requests(
    db: Option<Res<GameDatabase>>,
    mut receivers: SessionReceiver<shared::protocol::QueryAuctionBrowse>,
    state: Res<AuctionHouseState>,
    templates: Option<Res<crate::item_templates::ItemTemplates>>,
    links: Query<&crate::networking::PlayerLink>,
    players: Query<(
        &shared::components::Player,
        Option<&shared::components::Level>,
    )>,
    mut senders: Query<&mut MessageSender<shared::protocol::AuctionBrowseResults>>,
    mut rejections: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    let Some(db) = db else {
        return;
    };
    let txn = db.db.begin_read().expect("begin auction source read");
    let source_table = txn
        .open_table(persistence::inventory_store::ITEM_DEFINITION_SOURCES)
        .expect("open auction sources");
    let load_source =
        |guid| persistence::inventory_store::read_item_definition_source(&source_table, guid);
    for_each_session_request(&mut receivers, |entity, _, house, mut req| {
        let Some(house) = house else {
            send_auction_message(&mut rejections, entity, err(NO_AUCTIONEER));
            return;
        };
        req.query.faction = house;
        let viewer = links
            .get(entity)
            .ok()
            .and_then(|link| players.get(link.0).ok())
            .map(|(player, level)| {
                (
                    player.race,
                    player.class,
                    level.map_or(1, |level| u16::from(level.0)),
                )
            });
        send_auction_message(
            &mut senders,
            entity,
            super::browse::browse_for_viewer(
                &state.auctions,
                &req.query,
                templates.as_deref(),
                viewer,
                &load_source,
            ),
        );
    });
}

pub(super) fn process_query_requests(
    db: Option<Res<GameDatabase>>,
    mut receivers: SessionReceiver<QueryAuctions>,
    state: Res<AuctionHouseState>,
    templates: Option<Res<crate::item_templates::ItemTemplates>>,
    links: Query<&crate::networking::PlayerLink>,
    players: Query<(
        &shared::components::Player,
        Option<&shared::components::Level>,
    )>,
    mut senders: Query<&mut MessageSender<AuctionSearchResults>>,
    mut rejections: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    let Some(db) = db else {
        return;
    };
    let txn = db.db.begin_read().expect("begin auction source read");
    let source_table = txn
        .open_table(persistence::inventory_store::ITEM_DEFINITION_SOURCES)
        .expect("open auction sources");
    let load_source =
        |guid| persistence::inventory_store::read_item_definition_source(&source_table, guid);
    for_each_session_request(&mut receivers, |entity, _, house, mut req| {
        let Some(house) = house else {
            send_auction_message(&mut rejections, entity, err(NO_AUCTIONEER));
            return;
        };
        req.query.faction = house;
        let viewer = links
            .get(entity)
            .ok()
            .and_then(|link| players.get(link.0).ok())
            .map(|(player, level)| {
                (
                    player.race,
                    player.class,
                    level.map_or(1, |level| u16::from(level.0)),
                )
            });
        send_auction_message(
            &mut senders,
            entity,
            search_for_viewer(
                &state.auctions,
                &req.query,
                templates.as_deref(),
                viewer,
                &load_source,
            ),
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
        send_auction_message(
            &mut senders,
            entity,
            inventory_snapshot(&inventory, &|guid| {
                persistence::inventory_store::load_item_definition_source(&db.db, guid)
            }),
        );
    });
}

pub(super) fn process_owned_requests(
    db: Option<Res<GameDatabase>>,
    mut receivers: SessionReceiver<QueryOwnedAuctions>,
    state: Res<AuctionHouseState>,
    mut senders: Query<&mut MessageSender<OwnedAuctionListResponse>>,
    mut rejections: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    let Some(db) = db else {
        return;
    };
    let txn = db.db.begin_read().expect("begin auction source read");
    let source_table = txn
        .open_table(persistence::inventory_store::ITEM_DEFINITION_SOURCES)
        .expect("open auction sources");
    let load_source =
        |guid| persistence::inventory_store::read_item_definition_source(&source_table, guid);
    for_each_session_request(&mut receivers, |entity, character_id, house, _| {
        if house.is_none() {
            send_auction_message(&mut rejections, entity, err(NO_AUCTIONEER));
            return;
        }
        let listings = state
            .auctions
            .values()
            .filter(|auction| auction.seller_character_id == character_id)
            .map(|auction| to_listing_summary(auction, &load_source))
            .collect();
        send_auction_message(&mut senders, entity, OwnedAuctionListResponse { listings });
    });
}

pub(super) fn process_bid_list_requests(
    db: Option<Res<GameDatabase>>,
    mut receivers: SessionReceiver<QueryBidAuctions>,
    state: Res<AuctionHouseState>,
    mut senders: Query<&mut MessageSender<BidAuctionListResponse>>,
    mut rejections: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    let Some(db) = db else {
        return;
    };
    let txn = db.db.begin_read().expect("begin auction source read");
    let source_table = txn
        .open_table(persistence::inventory_store::ITEM_DEFINITION_SOURCES)
        .expect("open auction sources");
    let load_source =
        |guid| persistence::inventory_store::read_item_definition_source(&source_table, guid);
    for_each_session_request(&mut receivers, |entity, character_id, house, _| {
        if house.is_none() {
            send_auction_message(&mut rejections, entity, err(NO_AUCTIONEER));
            return;
        }
        let listings = state
            .auctions
            .values()
            .filter(|auction| auction.current_bidder_character_id == Some(character_id))
            .map(|auction| to_listing_summary(auction, &load_source))
            .collect();
        send_auction_message(&mut senders, entity, BidAuctionListResponse { listings });
    });
}

#[derive(Resource, Default)]
pub(crate) struct PendingMarketCommits(std::collections::VecDeque<PendingMarketCommit>);

impl PendingMarketCommits {
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

struct PendingMarketCommit {
    connection: Option<Entity>,
    receipt: crate::persistence_writer::CommitReceipt<MarketPublication>,
}

#[derive(Clone)]
pub(crate) struct MarketPublication {
    pub(crate) auctions: HashMap<u64, AuctionData>,
    pub(crate) criteria: Vec<(u64, crate::achievement::CriteriaEvent)>,
    pub(crate) gold: Vec<(u64, u64)>,
    pub(crate) economy: Option<crate::economy::model::EconomyState>,
    pub(crate) response: Option<AuctionOperationResponse>,
}

impl MarketPublication {
    pub(crate) fn from_auctions(mut state: AuctionHouseState) -> Self {
        Self {
            auctions: state.auctions,
            criteria: std::mem::take(&mut state.pending_criteria),
            gold: Vec::new(),
            economy: None,
            response: None,
        }
    }
}

pub(crate) fn load_auction_state(db: &redb::Database) -> AuctionHouseState {
    AuctionHouseState {
        initialized: true,
        auctions: persistence::load_auctions(db)
            .into_iter()
            .map(|auction| (auction.auction_id, auction))
            .collect(),
        pending_criteria: Vec::new(),
    }
}

pub(crate) fn enqueue_market_commit(
    db: &GameDatabase,
    pending: &mut PendingMarketCommits,
    connection: Option<Entity>,
    name: &'static str,
    operation: impl FnOnce(&redb::Database) -> Result<MarketPublication, String> + Send + 'static,
) -> crate::persistence_writer::CommitReceipt<MarketPublication> {
    let receipt = db.writer.enqueue(name, operation);
    pending.0.push_back(PendingMarketCommit {
        connection,
        receipt: receipt.clone(),
    });
    receipt
}

pub(super) fn publish_market_commits(
    mut pending: ResMut<PendingMarketCommits>,
    mut auctions: ResMut<AuctionHouseState>,
    mut gold: ResMut<GoldUpdateQueue>,
    mut economy: Option<ResMut<crate::economy::Economy>>,
    mut senders: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    while let Some(result) = pending.0.front().and_then(|commit| commit.receipt.result()) {
        let commit = pending.0.pop_front().expect("completed market commit");
        let response = match result {
            Ok(publication) => {
                auctions.auctions = publication.auctions;
                auctions.pending_criteria.extend(publication.criteria);
                for (character, value) in publication.gold {
                    gold.record(character, value);
                }
                if let (Some(state), Some(economy)) = (publication.economy, economy.as_deref_mut())
                {
                    *economy = crate::economy::Economy::from_state(state);
                }
                publication.response
            }
            Err(error) => Some(err(&error)),
        };
        if let (Some(connection), Some(response)) = (commit.connection, response) {
            send_auction_message(&mut senders, connection, response);
        }
    }
}

pub(super) fn process_create_requests(
    mut receivers: SessionReceiver<CreateAuction>,
    db: Option<Res<GameDatabase>>,
    templates: Option<Res<ItemTemplates>>,
    mut pending: ResMut<PendingMarketCommits>,
    mut senders: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    let Some(db) = db else {
        return;
    };
    for_each_session_request(&mut receivers, |connection, character, house, request| {
        let (Some(house), Some(templates)) = (house, templates.as_deref()) else {
            send_auction_message(&mut senders, connection, err(NO_AUCTIONEER));
            return;
        };
        let templates = templates.clone();
        enqueue_market_commit(
            &db,
            &mut pending,
            Some(connection),
            "create auction",
            move |db| {
                let mut state = load_auction_state(db);
                let response =
                    create_auction(db, &templates, &mut state, character, house, &request);
                let mut publication = MarketPublication::from_auctions(state);
                if response.success {
                    publication
                        .gold
                        .push((character, persistence::load_inventory(db, character).gold));
                }
                publication.response = Some(response);
                Ok(publication)
            },
        );
    });
}

pub(super) fn process_bid_requests(
    mut receivers: SessionReceiver<PlaceBid>,
    db: Option<Res<GameDatabase>>,
    mut pending: ResMut<PendingMarketCommits>,
    mut senders: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    enqueue_auction_operations(
        &mut receivers,
        db.as_deref(),
        &mut pending,
        &mut senders,
        |db, state, character, request| {
            place_bid(db, state, character, request.auction_id, request.amount)
        },
        |request| request.auction_id,
    );
}

pub(super) fn process_buyout_requests(
    mut receivers: SessionReceiver<BuyoutAuction>,
    db: Option<Res<GameDatabase>>,
    mut pending: ResMut<PendingMarketCommits>,
    mut senders: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    enqueue_auction_operations(
        &mut receivers,
        db.as_deref(),
        &mut pending,
        &mut senders,
        |db, state, character, request| buyout_auction(db, state, character, request.auction_id),
        |request| request.auction_id,
    );
}

pub(super) fn process_cancel_requests(
    mut receivers: SessionReceiver<CancelAuction>,
    db: Option<Res<GameDatabase>>,
    mut pending: ResMut<PendingMarketCommits>,
    mut senders: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    enqueue_auction_operations(
        &mut receivers,
        db.as_deref(),
        &mut pending,
        &mut senders,
        |db, state, character, request| cancel_auction(db, state, character, request.auction_id),
        |request| request.auction_id,
    );
}

fn enqueue_auction_operations<M: lightyear::prelude::Message, F, I>(
    receivers: &mut SessionReceiver<M>,
    db: Option<&GameDatabase>,
    pending: &mut PendingMarketCommits,
    senders: &mut Query<&mut MessageSender<AuctionOperationResponse>>,
    operation: F,
    auction_id: I,
) where
    F: Fn(&redb::Database, &mut AuctionHouseState, u64, &M) -> AuctionOperationResponse
        + Copy
        + Send
        + 'static,
    I: Fn(&M) -> u64,
{
    let Some(db) = db else {
        return;
    };
    for_each_session_request(receivers, |connection, character, house, request| {
        if house.is_none() {
            send_auction_message(senders, connection, err(NO_AUCTIONEER));
            return;
        }
        let id = auction_id(&request);
        enqueue_market_commit(
            db,
            pending,
            Some(connection),
            "auction operation",
            move |db| {
                let mut state = load_auction_state(db);
                let previous = state
                    .auctions
                    .get(&id)
                    .and_then(|auction| auction.current_bidder_character_id);
                let response = operation(db, &mut state, character, &request);
                let mut publication = MarketPublication::from_auctions(state);
                if response.success {
                    publication.gold = [Some(character), previous]
                        .into_iter()
                        .flatten()
                        .map(|id| (id, persistence::load_inventory(db, id).gold))
                        .collect();
                }
                publication.response = Some(response);
                Ok(publication)
            },
        );
    });
}

pub(super) fn expire_auctions(
    db: Option<Res<GameDatabase>>,
    state: Res<AuctionHouseState>,
    clock: Res<crate::maps::ServerClock>,
    mut pending: ResMut<PendingMarketCommits>,
    mut expiry: Local<Option<crate::persistence_writer::CommitReceipt<MarketPublication>>>,
) {
    let Some(db) = db else { return };
    if let Some(receipt) = expiry.as_ref() {
        if receipt.result().is_some() {
            *expiry = None;
        }
        return;
    }
    let now = clock.unix().max(0) as u64;
    if !state
        .auctions
        .values()
        .any(|auction| auction.expires_at_unix_secs <= now)
    {
        return;
    }
    *expiry = Some(enqueue_market_commit(
        &db,
        &mut pending,
        None,
        "expire auctions",
        move |db| {
            let mut state = load_auction_state(db);
            let expired: Vec<_> = state
                .auctions
                .values()
                .filter(|auction| auction.expires_at_unix_secs <= now)
                .cloned()
                .collect();
            for auction in expired {
                expire_single_auction(db, &auction)?;
                if let (Some(winner), Some(price)) =
                    (auction.current_bidder_character_id, auction.current_bid)
                {
                    state
                        .pending_criteria
                        .extend(super::operations::auction_sale_events(
                            &auction,
                            Some(winner),
                            price,
                        ));
                }
                state.auctions.remove(&auction.auction_id);
            }
            Ok(MarketPublication::from_auctions(state))
        },
    ));
}

fn expire_single_auction(db: &redb::Database, auction: &AuctionData) -> Result<(), String> {
    let mails = if let Some(winner_id) = auction.current_bidder_character_id {
        super::operations::sale_mails(
            db,
            auction,
            winner_id,
            auction.current_bid.unwrap_or(auction.min_bid),
        )
    } else {
        vec![(
            auction.seller_character_id,
            super::operations::build_auction_mail(
                db,
                &super::operations::auction_mail_subject("AUCTION_EXPIRED_MAIL_SUBJECT", auction),
                "",
                auction.deposit,
                Some(auction.item.clone()),
            ),
        )]
    };
    crate::economy::store::commit_market(
        db,
        None,
        &crate::economy::store::MarketChanges {
            expected_auctions: vec![(auction.auction_id, auction.clone())],
            deleted_auctions: vec![auction.auction_id],
            mails,
            ..Default::default()
        },
    )
}

#[cfg(test)]
pub(super) fn create_auction_with_gold_update(
    db: &redb::Database,
    templates: &ItemTemplates,
    state: &mut AuctionHouseState,
    character_id: u64,
    house: u8,
    request: &CreateAuction,
    gold_updates: &mut GoldUpdateQueue,
) -> AuctionOperationResponse {
    let response = create_auction(db, templates, state, character_id, house, request);
    record_successful_gold_updates(db, &response, [character_id], gold_updates);
    response
}

#[cfg(test)]
pub(super) fn place_bid_with_gold_updates(
    db: &redb::Database,
    state: &mut AuctionHouseState,
    character_id: u64,
    auction_id: u64,
    amount: u64,
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

#[cfg(test)]
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

#[cfg(test)]
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

fn send_auction_message<M>(senders: &mut Query<&mut MessageSender<M>>, entity: Entity, message: M)
where
    M: lightyear::prelude::Message,
{
    if let Ok(mut sender) = senders.get_mut(entity) {
        sender.send::<AuctionChannel>(message);
    }
}
