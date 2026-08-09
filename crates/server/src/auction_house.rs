use std::{collections::HashMap, time::Duration};

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

mod handlers;
mod operations;
mod queries;

use self::handlers::{
    expire_auctions, initialize_auction_state, process_bid_list_requests, process_bid_requests,
    process_buyout_requests, process_cancel_requests, process_claim_mail_requests,
    process_create_requests, process_inventory_requests, process_mailbox_requests,
    process_open_requests, process_owned_requests, process_query_requests,
};
#[cfg(test)]
use self::operations::deposit_cost;
use self::operations::{
    buyout_auction, cancel_auction, claim_mail, complete_sale, create_auction, deliver_mail,
    place_bid,
};
use self::queries::{
    duration_secs, inventory_snapshot, min_next_bid, search_auctions, to_listing_summary,
    to_mail_entry, unix_now,
};

const MAX_PAGE_SIZE: u32 = 50;
const HOUSE_CUT_BPS: u32 = 500;
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
        app.add_systems(FixedUpdate, expire_auctions.run_if(auction_expiration_due));
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
