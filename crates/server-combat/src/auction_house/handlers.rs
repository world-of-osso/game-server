use super::*;
use crate::gold::{GoldUpdateQueue, record_persisted_gold};
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

pub(super) fn process_create_requests(
    mut receivers: SessionReceiver<CreateAuction>,
    db: Option<Res<GameDatabase>>,
    templates: Option<Res<ItemTemplates>>,
    mut state: ResMut<AuctionHouseState>,
    mut gold_updates: ResMut<GoldUpdateQueue>,
    mut senders: Query<&mut MessageSender<AuctionOperationResponse>>,
) {
    process_session_operations(
        &mut receivers,
        db,
        &mut senders,
        |db, character_id, house, req| {
            let Some(templates) = templates.as_deref() else {
                return err("item templates unavailable");
            };
            create_auction_with_gold_update(
                &db.db,
                templates,
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

pub(super) fn expire_auctions(
    db: Option<Res<GameDatabase>>,
    mut state: ResMut<AuctionHouseState>,
    clock: Res<crate::maps::ServerClock>,
) {
    let Some(db) = db else { return };
    let now = clock.unix().max(0) as u64;
    let expired: Vec<u64> = state
        .auctions
        .values()
        .filter(|auction| auction.expires_at_unix_secs <= now)
        .map(|auction| auction.auction_id)
        .collect();
    for auction_id in expired {
        let Some(auction) = state.auctions.get(&auction_id).cloned() else {
            continue;
        };
        match expire_single_auction(&db.db, &auction) {
            Ok(()) => {
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
                state.auctions.remove(&auction_id);
            }
            Err(error) => error!(auction_id, %error, "auction expiry settlement failed"),
        }
    }
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
                persistence::next_id(db, "mail"),
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
