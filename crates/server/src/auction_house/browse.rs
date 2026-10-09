use std::cmp::Ordering;

use bevy::platform::collections::HashMap as BrowseMap;

use super::*;
use shared::protocol::{AuctionBrowseItem, AuctionBrowseResults};

/// Stack prices stay rational until serialization, so fractional copper does not
/// collapse ordering. u64 copper times u32 quantity always fits u128.
#[derive(Clone, Copy)]
struct UnitPrice {
    copper: u64,
    quantity: u32,
}

impl UnitPrice {
    fn compare(self, other: Self) -> Ordering {
        let left = u128::from(self.copper)
            .checked_mul(u128::from(other.quantity))
            .expect("u64 price times u32 quantity fits u128");
        let right = u128::from(other.copper)
            .checked_mul(u128::from(self.quantity))
            .expect("u64 price times u32 quantity fits u128");
        left.cmp(&right)
    }

    fn rounded_copper(self) -> u64 {
        self.copper.div_ceil(u64::from(self.quantity))
    }
}

struct BrowseGroup<'a> {
    item: &'a AuctionData,
    source: shared::item_data::ItemDefinitionSource,
    lowest_price: UnitPrice,
    lowest_bid: UnitPrice,
    lowest_current_bid: UnitPrice,
    total_quantity: u64,
    earliest_expiry: u64,
}

impl<'a> BrowseGroup<'a> {
    fn from_auction(
        auction: &'a AuctionData,
        source: shared::item_data::ItemDefinitionSource,
    ) -> Self {
        let bid = UnitPrice {
            copper: min_next_bid(auction),
            quantity: auction.stack_count,
        };
        Self {
            item: auction,
            source,
            lowest_price: UnitPrice {
                copper: auction.buyout_price.unwrap_or(bid.copper),
                quantity: auction.stack_count,
            },
            lowest_bid: bid,
            lowest_current_bid: UnitPrice {
                copper: auction.current_bid.unwrap_or(auction.min_bid),
                quantity: auction.stack_count,
            },
            total_quantity: u64::from(auction.stack_count),
            earliest_expiry: auction.expires_at_unix_secs,
        }
    }

    fn include(&mut self, auction: &'a AuctionData) {
        let offer = Self::from_auction(auction, self.source);
        if offer.lowest_price.compare(self.lowest_price).is_lt() {
            self.lowest_price = offer.lowest_price;
        }
        if offer.lowest_bid.compare(self.lowest_bid).is_lt() {
            self.lowest_bid = offer.lowest_bid;
        }
        if offer
            .lowest_current_bid
            .compare(self.lowest_current_bid)
            .is_lt()
        {
            self.lowest_current_bid = offer.lowest_current_bid;
        }
        self.total_quantity = self
            .total_quantity
            .checked_add(offer.total_quantity)
            .expect("auction quantity total fits u64");
        self.earliest_expiry = self.earliest_expiry.min(offer.earliest_expiry);
        // Stable metadata even if inconsistent persisted item snapshots exist.
        if auction.auction_id < self.item.auction_id {
            self.item = auction;
        }
    }

    fn into_item(self) -> AuctionBrowseItem {
        AuctionBrowseItem {
            item_id: self.item.item.item_id,
            definition_source: self.source,
            name: self.item.item.name.clone(),
            quality: self.item.item.quality,
            required_level: self.item.item.required_level,
            lowest_unit_price: self.lowest_price.rounded_copper(),
            total_quantity: self.total_quantity,
        }
    }
}

/// Viewer requirements depend only on item ID and immutable template/viewer
/// data, so cache them once. Snapshot fields still filter every auction.
struct BrowseCandidate<'a> {
    viewer_matches: bool,
    group: Option<BrowseGroup<'a>>,
}

impl<'a> BrowseCandidate<'a> {
    fn include_if_matching(
        &mut self,
        auction: &'a AuctionData,
        needle: &str,
        source: shared::item_data::ItemDefinitionSource,
    ) {
        if !self.viewer_matches || !super::queries::name_matches(auction, needle) {
            return;
        }
        match &mut self.group {
            Some(group) => group.include(auction),
            None => self.group = Some(BrowseGroup::from_auction(auction, source)),
        }
    }
}

pub(super) fn browse_for_viewer(
    auctions: &HashMap<u64, AuctionData>,
    query: &AuctionSearchQuery,
    templates: Option<&crate::item_templates::ItemTemplates>,
    viewer: Option<(u8, u8, u16)>,
    sources: super::queries::DefinitionSources<'_>,
) -> AuctionBrowseResults {
    let now = unix_now();
    let groups = group_matching_auctions(auctions, query, templates, viewer, now, sources);
    let total_results = u32::try_from(groups.len()).expect("distinct u32 item IDs fit u32 count");
    AuctionBrowseResults {
        query: query.clone(),
        total_results,
        items: page_groups(groups, query, now),
    }
}

fn group_matching_auctions<'a>(
    auctions: &'a HashMap<u64, AuctionData>,
    query: &AuctionSearchQuery,
    templates: Option<&crate::item_templates::ItemTemplates>,
    viewer: Option<(u8, u8, u16)>,
    now: u64,
    sources: super::queries::DefinitionSources<'_>,
) -> Vec<BrowseGroup<'a>> {
    let needle = query.text.to_ascii_lowercase();
    let mut candidates: BrowseMap<
        (shared::item_data::ItemDefinitionSource, u32),
        BrowseCandidate<'_>,
    > = BrowseMap::new();
    for auction in auctions.values() {
        if auction.expires_at_unix_secs <= now || auction.stack_count == 0 {
            continue;
        }
        // Defer name matching until after the cached viewer requirement.
        if !super::queries::matches_search_query(auction, query, "") {
            continue;
        }
        let source = sources(auction.item.item_guid);
        let candidate = candidates
            .entry((source, auction.item.item_id))
            .or_insert_with(|| BrowseCandidate {
                viewer_matches: super::handlers::auction_matches_viewer(
                    auction, query, templates, viewer, source,
                ),
                group: None,
            });
        candidate.include_if_matching(auction, &needle, source);
    }
    candidates
        .into_values()
        .filter_map(|candidate| candidate.group)
        .collect()
}

fn page_groups(
    mut groups: Vec<BrowseGroup<'_>>,
    query: &AuctionSearchQuery,
    now: u64,
) -> Vec<AuctionBrowseItem> {
    let page_size = query.page_size.clamp(1, MAX_PAGE_SIZE) as usize;
    let start = (query.page as usize).saturating_mul(page_size);
    if start >= groups.len() {
        return Vec::new();
    }
    let compare = |a: &BrowseGroup<'_>, b: &BrowseGroup<'_>| compare_groups(a, b, query, now);
    if start > 0 {
        groups.select_nth_unstable_by(start, compare);
    }
    let remaining = &mut groups[start..];
    if remaining.len() > page_size {
        remaining.select_nth_unstable_by(page_size, compare);
    }
    let page_len = remaining.len().min(page_size);
    remaining[..page_len].sort_unstable_by(compare);
    groups
        .into_iter()
        .skip(start)
        .take(page_size)
        .map(BrowseGroup::into_item)
        .collect()
}

fn compare_groups(
    a: &BrowseGroup<'_>,
    b: &BrowseGroup<'_>,
    query: &AuctionSearchQuery,
    now: u64,
) -> Ordering {
    let order = match query.sort_field {
        AuctionSortField::Bid => a.lowest_current_bid.compare(b.lowest_current_bid),
        AuctionSortField::Quantity => a.total_quantity.cmp(&b.total_quantity),
        AuctionSortField::Name => a.item.item.name.cmp(&b.item.item.name),
        AuctionSortField::MinBid => a.lowest_bid.compare(b.lowest_bid),
        AuctionSortField::Buyout => a.lowest_price.compare(b.lowest_price),
        AuctionSortField::TimeLeft => {
            super::queries::auction_time_rank(super::queries::time_left(a.earliest_expiry, now))
                .cmp(&super::queries::auction_time_rank(
                    super::queries::time_left(b.earliest_expiry, now),
                ))
        }
        AuctionSortField::Quality => a.item.item.quality.cmp(&b.item.item.quality),
        AuctionSortField::RequiredLevel => {
            a.item.item.required_level.cmp(&b.item.item.required_level)
        }
    };
    let directed = match query.sort_dir {
        AuctionSortDir::Asc => order,
        AuctionSortDir::Desc => order.reverse(),
    };
    directed
        .then_with(|| a.item.item.item_id.cmp(&b.item.item.item_id))
        .then_with(|| (a.source as u8).cmp(&(b.source as u8)))
}

#[cfg(test)]
#[path = "browse_tests.rs"]
mod tests;
