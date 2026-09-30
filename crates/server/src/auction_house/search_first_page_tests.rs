use super::*;

fn test_auction(id: u64, name: &str, quality: u8, required_level: u16) -> AuctionData {
    AuctionData {
        auction_id: id,
        seller_character_id: 1,
        seller_name: "Seller".into(),
        item: InventoryItemData {
            item_guid: id,
            item_id: 42,
            name: name.into(),
            quality,
            required_level,
            stack_count: 1,
            vendor_sell_price: 1,
            unique_auction: false,
        },
        stack_count: 1,
        min_bid: 100,
        current_bid: None,
        current_bidder_character_id: None,
        buyout_price: Some(200),
        expires_at_unix_secs: unix_now() + 3600,
        faction: 0,
        deposit: 0,
    }
}

fn text_query(text: &str) -> AuctionSearchQuery {
    AuctionSearchQuery {
        text: text.into(),
        page_size: 50,
        ..Default::default()
    }
}

#[test]
fn first_page_all_sorts_keep_ascending_id_ties_and_count_every_match() {
    let now = unix_now();
    let auctions: HashMap<_, _> = (1..=120)
        .map(|id| {
            let rank = id % 3;
            let mut auction = test_auction(
                id,
                ["Iron A", "Iron B", "Iron C"][rank as usize],
                rank as u8,
                rank as u16,
            );
            auction.min_bid = rank * 100;
            auction.buyout_price = match rank {
                0 => Some(100),
                1 => Some(200),
                _ => None,
            };
            auction.expires_at_unix_secs = now + [30, 900, 7200][rank as usize];
            (id, auction)
        })
        .collect();
    for field in [
        AuctionSortField::Name,
        AuctionSortField::MinBid,
        AuctionSortField::Buyout,
        AuctionSortField::TimeLeft,
        AuctionSortField::Quality,
        AuctionSortField::RequiredLevel,
    ] {
        for dir in [AuctionSortDir::Asc, AuctionSortDir::Desc] {
            let ranks = match dir {
                AuctionSortDir::Asc => [0, 1, 2],
                AuctionSortDir::Desc => [2, 1, 0],
            };
            let expected: Vec<u64> = ranks
                .into_iter()
                .flat_map(|rank| (1..=120).filter(move |id| id % 3 == rank && id % 5 != 0))
                .collect();
            for requested_size in [0, 7, 50, 99] {
                let mut query = text_query("iRoN");
                query.sort_field = field;
                query.sort_dir = dir;
                query.page_size = requested_size;
                let result = search_auctions_matching(&auctions, &query, |auction| {
                    auction.auction_id % 5 != 0
                });
                let ids: Vec<_> = result.results.iter().map(|a| a.auction_id).collect();
                let page_len = requested_size.clamp(1, 50) as usize;
                assert_eq!(result.total_results, 96);
                assert_eq!(result.query, query);
                assert_eq!(
                    ids,
                    expected[..page_len],
                    "{field:?} {dir:?} size={requested_size}"
                );
            }
        }
    }
}

#[test]
fn first_page_returns_short_and_empty_filtered_results() {
    let auctions = HashMap::from([
        (1, test_auction(1, "Iron Ore", 1, 1)),
        (2, test_auction(2, "Iron Ore", 1, 1)),
        (3, test_auction(3, "Copper Ore", 1, 1)),
    ]);
    let query = text_query("IRON");
    let result = search_auctions(&auctions, &query);
    assert_eq!(result.total_results, 2);
    assert_eq!(
        result
            .results
            .iter()
            .map(|a| a.auction_id)
            .collect::<Vec<_>>(),
        [1, 2]
    );
    let result = search_auctions_matching(&auctions, &query, |_| false);
    assert_eq!(result.total_results, 0);
    assert!(result.results.is_empty());
}
