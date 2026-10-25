use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    time::{SystemTime, UNIX_EPOCH},
};

use super::*;
use shared::item_data::ItemDefinitionSource;
pub(super) type DefinitionSources<'a> = &'a dyn Fn(u64) -> ItemDefinitionSource;

#[cfg(test)]
#[path = "search_first_page_tests.rs"]
mod search_first_page_tests;

pub(super) fn search_auctions(
    auctions: &HashMap<u64, AuctionData>,
    query: &AuctionSearchQuery,
    sources: DefinitionSources<'_>,
) -> AuctionSearchResults {
    search_auctions_matching(auctions, query, sources, |_| true)
}

pub(super) fn search_auctions_matching(
    auctions: &HashMap<u64, AuctionData>,
    query: &AuctionSearchQuery,
    sources: DefinitionSources<'_>,
    additional_match: impl Fn(&AuctionData) -> bool,
) -> AuctionSearchResults {
    let needle = query.text.to_ascii_lowercase();
    let listings = auctions
        .values()
        .filter(|auction| matches_search_query(auction, query, &needle))
        .filter(|auction| additional_match(auction));
    let now = unix_now();
    let (total_results, results) = if let Some((start, end)) = bounded_prefix(query, auctions.len())
    {
        prefix_page_listings(listings, query, now, start, end, sources)
    } else {
        let mut listings: Vec<_> = listings.collect();
        let total_results = listings.len() as u32;
        (
            total_results,
            paged_listings(&mut listings, query, now, sources),
        )
    };
    AuctionSearchResults {
        query: query.clone(),
        total_results,
        results,
    }
}

pub(super) fn matches_search_query(
    auction: &AuctionData,
    query: &AuctionSearchQuery,
    needle: &str,
) -> bool {
    query.item_id.is_none_or(|id| auction.item.item_id == id)
        && quality_matches(auction, query)
        && level_matches(auction, query)
        && faction_matches(auction, query)
        && name_matches(auction, needle)
}

pub(super) fn name_matches(auction: &AuctionData, needle: &str) -> bool {
    needle.is_empty()
        || auction
            .item
            .name
            .as_bytes()
            .windows(needle.len())
            .any(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
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

struct RankedListing<'a> {
    auction: &'a AuctionData,
    query: &'a AuctionSearchQuery,
    now: u64,
}

impl PartialEq for RankedListing<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for RankedListing<'_> {}

impl PartialOrd for RankedListing<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RankedListing<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_listing(self.auction, other.auction, self.query, self.now)
    }
}

fn bounded_prefix(query: &AuctionSearchQuery, input_len: usize) -> Option<(usize, usize)> {
    let page_size = query.page_size.clamp(1, MAX_PAGE_SIZE) as usize;
    let start = (query.page as usize).checked_mul(page_size)?;
    let end = start.checked_add(page_size)?;
    // Both paths scan all input for exact filtering/counts and order the final page.
    // Limit prefix ordering work k*ceil(log2(k)) to the vector partition span n
    // plus final-page ordering p*ceil(log2(p)). This is a work/space bound, not
    // a worst-case time bound: heap replacements can still cost O(n log k).
    // Use the input upper bound without a second filtering pass; selective queries
    // may retain fewer than k entries. Deep prefixes instead use O(n) nth selection.
    let prefix_work = ordering_work(end)?;
    let selection_work = input_len.checked_add(ordering_work(page_size)?)?;
    (prefix_work <= selection_work).then_some((start, end))
}

fn ordering_work(len: usize) -> Option<usize> {
    let height = usize::BITS - (len - 1).leading_zeros();
    len.checked_mul(height as usize)
}

fn prefix_page_listings<'a>(
    listings: impl Iterator<Item = &'a AuctionData>,
    query: &'a AuctionSearchQuery,
    now: u64,
    start: usize,
    end: usize,
    sources: DefinitionSources<'_>,
) -> (u32, Vec<AuctionListingSummary>) {
    let mut best = BinaryHeap::with_capacity(end);
    let mut total_results = 0;
    for auction in listings {
        total_results += 1;
        let candidate = RankedListing {
            auction,
            query,
            now,
        };
        if best.len() < end {
            best.push(candidate);
        } else if best.peek().is_some_and(|worst| candidate < *worst) {
            *best.peek_mut().expect("full prefix heap") = candidate;
        }
    }
    let results = best
        .into_sorted_vec()
        .into_iter()
        .skip(start)
        .map(|listing| to_listing_summary_at(listing.auction, now, sources))
        .collect();
    (total_results, results)
}

fn paged_listings(
    listings: &mut [&AuctionData],
    query: &AuctionSearchQuery,
    now: u64,
    sources: DefinitionSources<'_>,
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
        .map(|auction| to_listing_summary_at(auction, now, sources))
        .collect()
}

fn compare_listing(
    a: &AuctionData,
    b: &AuctionData,
    query: &AuctionSearchQuery,
    now: u64,
) -> std::cmp::Ordering {
    let order = match query.sort_field {
        AuctionSortField::Bid => a
            .current_bid
            .unwrap_or(a.min_bid)
            .cmp(&b.current_bid.unwrap_or(b.min_bid)),
        AuctionSortField::Quantity => a.stack_count.cmp(&b.stack_count),
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

pub(super) fn auction_time_rank(time_left: AuctionTimeLeft) -> u8 {
    match time_left {
        AuctionTimeLeft::Short => 0,
        AuctionTimeLeft::Medium => 1,
        AuctionTimeLeft::Long => 2,
        AuctionTimeLeft::VeryLong => 3,
    }
}

pub(super) fn inventory_snapshot(
    inventory: &InventoryData,
    sources: DefinitionSources<'_>,
) -> AuctionInventorySnapshot {
    AuctionInventorySnapshot {
        gold: inventory.gold,
        items: inventory
            .items
            .iter()
            .cloned()
            .map(|item| {
                let source = sources(item.item_guid);
                to_inventory_item(item, source)
            })
            .collect(),
    }
}

fn to_inventory_item(
    item: InventoryItemData,
    source: ItemDefinitionSource,
) -> AuctionInventoryItem {
    AuctionInventoryItem {
        item_guid: item.item_guid,
        item_id: item.item_id,
        definition_source: source,
        name: item.name,
        quality: item.quality,
        required_level: item.required_level,
        stack_count: item.stack_count,
        vendor_sell_price: item.vendor_sell_price,
    }
}

pub(super) fn to_listing_summary(
    auction: &AuctionData,
    sources: DefinitionSources<'_>,
) -> AuctionListingSummary {
    to_listing_summary_at(auction, unix_now(), sources)
}

fn to_listing_summary_at(
    auction: &AuctionData,
    now: u64,
    sources: DefinitionSources<'_>,
) -> AuctionListingSummary {
    AuctionListingSummary {
        auction_id: auction.auction_id,
        item: to_inventory_item(auction.item.clone(), sources(auction.item.item_guid)),
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

pub(super) fn time_left(expires_at_unix_secs: u64, now: u64) -> AuctionTimeLeft {
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
