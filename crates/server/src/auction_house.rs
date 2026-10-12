use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{
    AuctionChannel, AuctionDuration, AuctionHouseOpened, AuctionInventoryItem,
    AuctionInventorySnapshot, AuctionListingSummary, AuctionOperationResponse, AuctionSearchQuery,
    AuctionSearchResults, AuctionSortDir, AuctionSortField, AuctionTimeLeft,
    BidAuctionListResponse, BuyoutAuction, CancelAuction, CreateAuction, OpenAuctionHouse,
    OwnedAuctionListResponse, PlaceBid, QueryAuctionInventory, QueryAuctions, QueryBidAuctions,
    QueryOwnedAuctions,
};

use crate::auth::InWorld;
use crate::item_templates::{ItemTemplate, ItemTemplates};
use crate::persistence::{
    self, AuctionData, GameDatabase, InventoryData, InventoryItemData,
    mail_store::{MailData, MailItemData, MailSender},
};

mod browse;
#[cfg(test)]
use crate::persistence::mail_store;

mod handlers;
mod operations;
mod queries;

use self::handlers::{
    expire_auctions, initialize_auction_state, process_bid_list_requests, process_bid_requests,
    process_browse_requests, process_buyout_requests, process_cancel_requests,
    process_create_requests, process_inventory_requests, process_open_requests,
    process_owned_requests, process_query_requests,
};
pub(crate) use self::operations::deposit_cost;
pub(crate) use self::operations::{auction_sale_events, seller_sale_mail};
use self::operations::{buyout_auction, cancel_auction, create_auction, place_bid};
pub(crate) use self::queries::{duration_secs, unix_now};
use self::queries::{inventory_snapshot, min_next_bid, search_auctions, to_listing_summary};

const MAX_PAGE_SIZE: u32 = 50;
const HOUSE_CUT_BPS: u64 = 500;
const MAX_AUCTIONS_PER_CHARACTER: usize = 50;
const AUCTION_EXPIRATION_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Default)]
struct AuctionExpirationCadence {
    has_run: bool,
    elapsed: Duration,
}

impl AuctionExpirationCadence {
    fn should_run(&mut self, delta: Duration) -> bool {
        if !self.has_run {
            self.has_run = true;
            return true;
        }
        self.elapsed += delta;
        if self.elapsed < AUCTION_EXPIRATION_INTERVAL {
            return false;
        }
        self.elapsed = Duration::ZERO;
        true
    }
}

fn auction_expiration_due(time: Res<Time>, mut cadence: Local<AuctionExpirationCadence>) -> bool {
    cadence.should_run(time.delta())
}

#[derive(Resource, Default)]
pub(crate) struct AuctionHouseState {
    pub(crate) initialized: bool,
    pub(crate) auctions: HashMap<u64, AuctionData>,
    /// Transient gameplay facts published only after their market transaction commits.
    pub(crate) pending_criteria: Vec<(u64, crate::achievement::CriteriaEvent)>,
}

pub struct AuctionHousePlugin;

impl Plugin for AuctionHousePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AuctionHouseState>();
        app.init_resource::<crate::gold::GoldUpdateQueue>();
        app.init_resource::<crate::maps::ServerClock>();
        app.add_systems(Update, initialize_auction_state);
        app.add_systems(Update, process_open_requests);
        app.add_systems(Update, process_query_requests);
        app.add_systems(Update, process_browse_requests);
        app.add_systems(Update, process_inventory_requests);
        app.add_systems(Update, process_owned_requests);
        app.add_systems(Update, process_bid_list_requests);
        app.add_systems(Update, process_create_requests);
        app.add_systems(Update, process_bid_requests);
        app.add_systems(Update, process_buyout_requests);
        app.add_systems(Update, process_cancel_requests);
        app.add_systems(FixedUpdate, expire_auctions.run_if(auction_expiration_due));
        app.add_systems(PostUpdate, publish_committed_auction_criteria);
    }
}

fn publish_committed_auction_criteria(
    mut auctions: ResMut<AuctionHouseState>,
    mut gold: ResMut<crate::gold::GoldUpdateQueue>,
    actors: Query<&InWorld, (With<crate::auth::Authenticated>, With<Connected>)>,
) {
    let online: HashSet<u64> = actors.iter().map(|actor| actor.character_id).collect();
    for (character, event) in std::mem::take(&mut auctions.pending_criteria) {
        // TC FindConnectedPlayer: offline sellers/winners never receive login backfill.
        if online.contains(&character) {
            gold.record_criteria(character, event);
        }
    }
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
