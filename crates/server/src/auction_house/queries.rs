use std::time::{SystemTime, UNIX_EPOCH};

use super::*;

pub(super) fn search_auctions(
    auctions: &HashMap<u64, AuctionData>,
    query: &AuctionSearchQuery,
) -> AuctionSearchResults {
    search_auctions_matching(auctions, query, |_| true)
}

pub(super) fn search_auctions_matching(
    auctions: &HashMap<u64, AuctionData>,
    query: &AuctionSearchQuery,
    additional_match: impl Fn(&AuctionData) -> bool,
) -> AuctionSearchResults {
    let needle = query.text.to_ascii_lowercase();
    let mut listings: Vec<_> = auctions
        .values()
        .filter(|auction| matches_search_query(auction, query, &needle))
        .filter(|auction| additional_match(auction))
        .collect();
    let total_results = listings.len() as u32;
    let now = unix_now();
    let results = paged_listings(&mut listings, query, now);
    AuctionSearchResults {
        query: query.clone(),
        total_results,
        results,
    }
}

fn matches_search_query(auction: &AuctionData, query: &AuctionSearchQuery, needle: &str) -> bool {
    query.item_id.is_none_or(|id| auction.item.item_id == id)
        && quality_matches(auction, query)
        && level_matches(auction, query)
        && faction_matches(auction, query)
        && name_matches(auction, needle)
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
    listings: &mut [&AuctionData],
    query: &AuctionSearchQuery,
    now: u64,
) -> Vec<AuctionListingSummary> {
    let page_size = query.page_size.clamp(1, MAX_PAGE_SIZE) as usize;
    let start = (query.page as usize).saturating_mul(page_size);
    if start >= listings.len() {
        return Vec::new();
    }
    let compare = |a: &&AuctionData, b: &&AuctionData| compare_listing(a, b, query, now);
    if start > 0 {
        listings.select_nth_unstable_by(start, compare);
    }
    let remaining = &mut listings[start..];
    if remaining.len() > page_size {
        remaining.select_nth_unstable_by(page_size, compare);
    }
    let page_len = remaining.len().min(page_size);
    let page = &mut remaining[..page_len];
    page.sort_unstable_by(compare);
    page.iter()
        .map(|auction| to_listing_summary_at(auction, now))
        .collect()
}

fn compare_listing(
    a: &AuctionData,
    b: &AuctionData,
    query: &AuctionSearchQuery,
    now: u64,
) -> std::cmp::Ordering {
    let order = match query.sort_field {
        AuctionSortField::Name => a.item.name.cmp(&b.item.name),
        AuctionSortField::MinBid => a.min_bid.cmp(&b.min_bid),
        AuctionSortField::Buyout => a
            .buyout_price
            .unwrap_or(u64::MAX)
            .cmp(&b.buyout_price.unwrap_or(u64::MAX)),
        AuctionSortField::TimeLeft => auction_time_rank(time_left(a.expires_at_unix_secs, now))
            .cmp(&auction_time_rank(time_left(b.expires_at_unix_secs, now))),
        AuctionSortField::Quality => a.item.quality.cmp(&b.item.quality),
        AuctionSortField::RequiredLevel => a.item.required_level.cmp(&b.item.required_level),
    };
    let directed = match query.sort_dir {
        AuctionSortDir::Asc => order,
        AuctionSortDir::Desc => order.reverse(),
    };
    directed.then_with(|| a.auction_id.cmp(&b.auction_id))
}

fn auction_time_rank(time_left: AuctionTimeLeft) -> u8 {
    match time_left {
        AuctionTimeLeft::Short => 0,
        AuctionTimeLeft::Medium => 1,
        AuctionTimeLeft::Long => 2,
        AuctionTimeLeft::VeryLong => 3,
    }
}

pub(super) fn inventory_snapshot(inventory: &InventoryData) -> AuctionInventorySnapshot {
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

pub(super) fn to_listing_summary(auction: &AuctionData) -> AuctionListingSummary {
    to_listing_summary_at(auction, unix_now())
}

fn to_listing_summary_at(auction: &AuctionData, now: u64) -> AuctionListingSummary {
    AuctionListingSummary {
        auction_id: auction.auction_id,
        item: to_inventory_item(auction.item.clone()),
        owner_name: auction.seller_name.clone(),
        stack_count: auction.stack_count,
        min_bid: auction.min_bid,
        current_bid: auction.current_bid,
        min_next_bid: min_next_bid(auction),
        buyout_price: auction.buyout_price,
        time_left: time_left(auction.expires_at_unix_secs, now),
    }
}

pub(super) fn min_next_bid(auction: &AuctionData) -> u64 {
    match auction.current_bid {
        Some(current) => current.saturating_add((current / 20).max(1)),
        None => auction.min_bid,
    }
}

const TIME_LEFT_SHORT_SECS: u64 = 60;
const TIME_LEFT_MEDIUM_SECS: u64 = 60 * 30;
const TIME_LEFT_LONG_SECS: u64 = 60 * 60 * 6;

fn time_left(expires_at_unix_secs: u64, now: u64) -> AuctionTimeLeft {
    let remaining = expires_at_unix_secs.saturating_sub(now);
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

pub(crate) fn duration_secs(duration: AuctionDuration) -> u64 {
    match duration {
        AuctionDuration::Short => 24 * 60 * 60,
        AuctionDuration::Medium => 7 * 24 * 60 * 60,
        AuctionDuration::Long => 14 * 24 * 60 * 60,
    }
}

pub(crate) fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
