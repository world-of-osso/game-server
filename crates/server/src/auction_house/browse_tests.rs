use super::*;
use crate::item_templates::{ItemTemplate, ItemTemplates};

fn offer(id: u64, item: u32, quantity: u32, price: Option<u64>) -> AuctionData {
    let mut auction = AuctionData {
        auction_id: id,
        seller_character_id: 1,
        seller_name: "Seller".into(),
        item: InventoryItemData {
            item_guid: id,
            item_id: item,
            name: format!("Item {item:05}"),
            quality: 1,
            required_level: 1,
            stack_count: quantity,
            vendor_sell_price: 1,
            unique_auction: false,
        },
        stack_count: quantity,
        min_bid: 100,
        current_bid: None,
        current_bidder_character_id: None,
        buyout_price: price,
        expires_at_unix_secs: unix_now() + 3600,
        faction: 0,
        deposit: 0,
    };
    auction.stack_count = quantity;
    // Deliberately different: actual auction stack, not original inventory stack.
    auction.item.stack_count = 999;
    auction.buyout_price = price;
    auction
}

fn text_query(text: &str) -> AuctionSearchQuery {
    AuctionSearchQuery {
        text: text.into(),
        ..Default::default()
    }
}

fn browse(
    auctions: &HashMap<u64, AuctionData>,
    query: &AuctionSearchQuery,
) -> AuctionBrowseResults {
    browse_for_viewer(auctions, query, None, None)
}

#[test]
fn global_quantity_and_cheapest_offer_include_offers_beyond_old_page() {
    let mut auctions = HashMap::new();
    for id in 1..=75 {
        auctions.insert(id, offer(id, 1, 3, Some(300)));
    }
    auctions.insert(76, offer(76, 1, 7, Some(8)));
    let query = text_query("");
    assert!(
        !super::super::queries::search_auctions(&auctions, &query)
            .results
            .iter()
            .any(|a| a.auction_id == 76)
    );
    let result = browse(&auctions, &query);
    assert_eq!(result.total_results, 1);
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].total_quantity, 232);
    assert_eq!(result.items[0].lowest_unit_price, 2);
}

#[test]
fn distinct_items_page_after_grouping_with_stable_ties_in_both_directions() {
    let mut auctions = HashMap::new();
    for item in 1..=75 {
        for copy in 0..3 {
            let id = u64::from(item) * 3 + copy;
            let mut auction = offer(id, item, 4, Some(400));
            auction.item.name = "Same name".into();
            auctions.insert(id, auction);
        }
    }
    for direction in [AuctionSortDir::Asc, AuctionSortDir::Desc] {
        let mut query = text_query("");
        query.sort_dir = direction;
        let first = browse(&auctions, &query);
        assert_eq!(first.total_results, 75);
        assert_eq!(
            first.items.iter().map(|i| i.item_id).collect::<Vec<_>>(),
            (1..=50).collect::<Vec<_>>()
        );
        assert!(first.items.iter().all(|i| i.total_quantity == 12));
        query.page = 1;
        let second = browse(&auctions, &query);
        assert_eq!(second.total_results, 75);
        assert_eq!(
            second.items.iter().map(|i| i.item_id).collect::<Vec<_>>(),
            (51..=75).collect::<Vec<_>>()
        );
        query.page = 2;
        assert!(browse(&auctions, &query).items.is_empty());
    }
}

#[test]
fn mixed_and_bid_only_offers_use_next_bid_and_round_up_without_overflow() {
    let mut bid = offer(1, 1, 3, None);
    bid.current_bid = Some(100);
    let mut huge = offer(4, 3, u32::MAX, Some(u64::MAX));
    huge.min_bid = u64::MAX;
    let auctions = HashMap::from([
        (1, bid),
        (2, offer(2, 1, 2, Some(71))),
        (3, offer(3, 2, 3, None)),
        (4, huge),
    ]);
    let result = browse(&auctions, &text_query(""));
    assert_eq!(result.items[0].lowest_unit_price, 35);
    assert_eq!(result.items[0].total_quantity, 5);
    assert_eq!(result.items[1].lowest_unit_price, 34);
    assert_eq!(result.items[2].lowest_unit_price, 4_294_967_297);
}

#[test]
fn every_sort_field_uses_global_values_and_honors_direction() {
    let mut a = offer(1, 1, 2, Some(800));
    a.min_bid = 800;
    a.item.quality = 3;
    a.item.required_level = 30;
    a.expires_at_unix_secs = unix_now() + 20_000;
    let mut b = offer(2, 2, 2, Some(200));
    b.min_bid = 200;
    b.item.quality = 1;
    b.item.required_level = 10;
    b.expires_at_unix_secs = unix_now() + 40;
    let auctions = HashMap::from([(1, a), (2, b)]);
    for field in [
        AuctionSortField::Name,
        AuctionSortField::MinBid,
        AuctionSortField::Buyout,
        AuctionSortField::Quality,
        AuctionSortField::RequiredLevel,
        AuctionSortField::TimeLeft,
    ] {
        let mut query = text_query("");
        query.sort_field = field;
        let ascending = if field == AuctionSortField::Name {
            vec![1, 2]
        } else {
            vec![2, 1]
        };
        let result = browse(&auctions, &query);
        assert_eq!(
            result.items.iter().map(|i| i.item_id).collect::<Vec<_>>(),
            ascending,
            "{field:?}"
        );
        query.sort_dir = AuctionSortDir::Desc;
        let result = browse(&auctions, &query);
        assert_eq!(
            result.items.iter().map(|i| i.item_id).collect::<Vec<_>>(),
            ascending.into_iter().rev().collect::<Vec<_>>(),
            "{field:?}"
        );
    }
}

#[test]
fn expired_and_zero_stack_offers_do_not_contribute_and_filters_precede_grouping() {
    let mut auctions = HashMap::new();
    for id in 1..=8 {
        let mut auction = offer(id, id as u32, 3, Some(300));
        auction.item.name = "Iron sword".into();
        auction.item.quality = 2;
        auction.item.required_level = 10;
        auction.faction = 1;
        match id {
            2 => auction.expires_at_unix_secs = unix_now(),
            3 => auction.stack_count = 0,
            4 => auction.item.quality = 3,
            5 => auction.item.required_level = 30,
            6 => auction.faction = 2,
            7 => auction.item.name = "Copper ore".into(),
            8 => auction.faction = 0,
            _ => {}
        }
        auctions.insert(id, auction);
    }
    let mut query = text_query("IRON");
    query.quality = Some(2);
    query.min_level = Some(5);
    query.max_level = Some(20);
    query.faction = 1;
    assert_eq!(
        browse(&auctions, &query)
            .items
            .iter()
            .map(|i| i.item_id)
            .collect::<Vec<_>>(),
        vec![1, 8]
    );
    query.item_id = Some(8);
    assert_eq!(browse(&auctions, &query).total_results, 1);
    query.item_id = Some(999);
    assert_eq!(browse(&auctions, &query).total_results, 0);
}

#[test]
fn class_and_usable_requirements_share_flat_viewer_logic_before_paging() {
    let mut auctions = HashMap::new();
    let mut templates = ItemTemplates::default();
    for id in 1..=120 {
        auctions.insert(id, offer(id, id as u32, 3, Some(300)));
        let mut template = ItemTemplate {
            allowable_class: -1,
            allowable_race: u64::MAX,
            required_level: if id % 2 == 0 { 10 } else { 50 },
            ..Default::default()
        };
        template.scaling.class_id = if id % 2 == 0 { 7 } else { 4 };
        templates.0.insert(id as u32, template);
    }
    templates.0.get_mut(&2).unwrap().allowable_class = 1 << 7;
    templates.0.get_mut(&4).unwrap().allowable_race = 1 << 1;
    let mut query = text_query("");
    query.class_id = Some(7);
    assert_eq!(
        browse_for_viewer(&auctions, &query, Some(&templates), None).total_results,
        60
    );
    query.usable_only = true;
    let first = browse_for_viewer(&auctions, &query, Some(&templates), Some((1, 1, 20)));
    assert_eq!(first.total_results, 58);
    assert_eq!(first.items.len(), 50);
    assert!(
        first
            .items
            .iter()
            .all(|i| i.item_id % 2 == 0 && ![2, 4].contains(&i.item_id))
    );
    query.page = 1;
    assert_eq!(
        browse_for_viewer(&auctions, &query, Some(&templates), Some((1, 1, 20)))
            .items
            .len(),
        8
    );
    assert_eq!(
        browse_for_viewer(&auctions, &query, Some(&templates), None).total_results,
        0
    );
    assert_eq!(
        browse_for_viewer(&auctions, &query, None, Some((1, 1, 20))).total_results,
        0
    );
}

#[test]
#[ignore = "CPU measurement; run with profile.test.package.server.opt-level=2 and --ignored --nocapture"]
fn browse_300k_cpu_under_200ms() {
    use std::time::{Duration, Instant};
    let mut auctions = HashMap::with_capacity(300_000);
    let mut templates = ItemTemplates::default();
    for id in 1..=300_000u64 {
        let item = (id % 75_000 + 1) as u32;
        auctions.insert(id, offer(id, item, 20, Some(401)));
        let mut template = ItemTemplate {
            allowable_class: -1,
            allowable_race: u64::MAX,
            required_level: 1,
            ..Default::default()
        };
        template.scaling.class_id = 7;
        templates.0.entry(item).or_insert(template);
    }
    for (text, class, usable, page, expected) in [
        ("", None, false, 0, 75_000),
        ("Item", None, false, 1, 75_000),
        ("Item", Some(7), true, 1, 75_000),
    ] {
        let mut query = text_query(text);
        query.class_id = class;
        query.usable_only = usable;
        query.page = page;
        query.sort_field = AuctionSortField::Buyout;
        let start = Instant::now();
        let result = browse_for_viewer(&auctions, &query, Some(&templates), Some((1, 1, 20)));
        let elapsed = start.elapsed();
        println!(
            "300k auctions, 75k items, text={text:?}, class={class:?}, usable={usable}, page={page}, CPU={elapsed:?}"
        );
        assert_eq!(result.total_results, expected);
        assert_eq!(result.items.len(), 50);
        assert_eq!(result.items[0].item_id, page * 50 + 1);
        assert!(
            result
                .items
                .iter()
                .all(|i| i.total_quantity == 80 && i.lowest_unit_price == 21)
        );
        assert!(
            elapsed < Duration::from_millis(200),
            "query CPU {elapsed:?}"
        );
    }
}

#[test]
fn fractional_prices_sort_exactly_even_when_displayed_copper_ties() {
    let auctions = HashMap::from([
        (1, offer(1, 1, 2, Some(3))),
        (2, offer(2, 2, 3, Some(4))),
        (3, offer(3, 3, u32::MAX, Some(u64::MAX))),
        (4, offer(4, 3, u32::MAX - 1, Some(u64::MAX - 1))),
    ]);
    let mut query = text_query("");
    query.sort_field = AuctionSortField::Buyout;
    let result = browse(&auctions, &query);
    assert_eq!(
        result.items.iter().map(|i| i.item_id).collect::<Vec<_>>(),
        vec![2, 1, 3]
    );
    assert_eq!(result.items[0].lowest_unit_price, 2);
    assert_eq!(result.items[1].lowest_unit_price, 2);
    assert_eq!(result.items[2].lowest_unit_price, 4_294_967_297);
    assert_eq!(result.items[2].total_quantity, u64::from(u32::MAX) * 2 - 1);
    query.sort_dir = AuctionSortDir::Desc;
    assert_eq!(
        browse(&auctions, &query)
            .items
            .iter()
            .map(|i| i.item_id)
            .collect::<Vec<_>>(),
        vec![3, 1, 2]
    );
}

mod session {
    use super::*;
    use crate::faction::FactionTemplates;
    use crate::faction_reaction::FactionTemplateEntry;
    use crate::networking::PlayerLink;
    use crate::npc_interaction::{ActiveInteraction, NpcInteractionPlugin};
    use lightyear::connection::host::HostClient;
    use shared::components::{Health, Npc, Position, UnitFactionTemplate};
    use shared::protocol::{
        InteractNpc, InteractionChannel, NpcFlags, ProtocolPlugin, QueryAuctionBrowse,
    };

    #[derive(Resource, Default)]
    struct Inbox {
        results: Vec<AuctionBrowseResults>,
        errors: Vec<AuctionOperationResponse>,
    }

    fn collect(
        mut results: Query<&mut MessageReceiver<AuctionBrowseResults>>,
        mut errors: Query<&mut MessageReceiver<AuctionOperationResponse>>,
        mut inbox: ResMut<Inbox>,
    ) {
        for mut receiver in &mut results {
            inbox.results.extend(receiver.receive());
        }
        for mut receiver in &mut errors {
            inbox.errors.extend(receiver.receive());
        }
    }

    fn advance(app: &mut App) {
        for _ in 0..3 {
            app.update();
        }
    }

    fn request(app: &mut App, connection: Entity) {
        app.world_mut()
            .get_mut::<MessageSender<QueryAuctionBrowse>>(connection)
            .unwrap()
            .send::<AuctionChannel>(QueryAuctionBrowse {
                query: text_query(""),
            });
        advance(app);
    }

    #[test]
    fn loopback_browse_rejects_without_session_overrides_house_and_closes_out_of_range() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.add_plugins(lightyear::prelude::server::ServerPlugins {
            tick_duration: Duration::from_millis(50),
        });
        app.add_plugins(ProtocolPlugin);
        app.init_resource::<Inbox>();
        app.insert_resource(FactionTemplates(HashMap::from([(
            1,
            FactionTemplateEntry {
                id: 1,
                faction_group: 2,
                ..Default::default()
            },
        )])));
        app.add_plugins(NpcInteractionPlugin);
        app.init_resource::<crate::gold::GoldUpdateQueue>();
        app.add_plugins(AuctionHousePlugin);
        app.add_systems(First, collect);
        let mut auctions = HashMap::new();
        for (id, faction) in [(1, 1), (2, 2), (3, 0)] {
            let mut auction = offer(id, id as u32, 3, Some(300));
            auction.faction = faction;
            auctions.insert(id, auction);
        }
        app.insert_resource(AuctionHouseState {
            initialized: true,
            auctions,
        });
        app.finish();
        app.cleanup();
        let position = Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let health = Health {
            current: 100.0,
            max: 100.0,
        };
        let player = app
            .world_mut()
            .spawn((position, health, UnitFactionTemplate(1)))
            .id();
        let npc = app
            .world_mut()
            .spawn((
                Npc {
                    template_id: 8719,
                    name: "Auctioneer".into(),
                },
                NpcFlags(NpcFlags::AUCTIONEER),
                position,
                health,
                UnitFactionTemplate(1),
            ))
            .id();
        let connection = app
            .world_mut()
            .spawn((
                RemoteId(PeerId::Local(1)),
                Connected,
                HostClient { buffer: Vec::new() },
                InWorld { character_id: 1 },
                PlayerLink(player),
                MessageSender::<QueryAuctionBrowse>::default(),
                MessageReceiver::<QueryAuctionBrowse>::default(),
                MessageSender::<AuctionBrowseResults>::default(),
                MessageReceiver::<AuctionBrowseResults>::default(),
                MessageSender::<AuctionOperationResponse>::default(),
                MessageReceiver::<AuctionOperationResponse>::default(),
                MessageSender::<InteractNpc>::default(),
                MessageReceiver::<InteractNpc>::default(),
            ))
            .id();
        advance(&mut app);
        request(&mut app, connection);
        {
            let mut inbox = app.world_mut().resource_mut::<Inbox>();
            assert!(inbox.results.is_empty());
            assert_eq!(
                std::mem::take(&mut inbox.errors),
                vec![err(super::super::super::handlers::NO_AUCTIONEER)]
            );
        }
        app.world_mut()
            .get_mut::<MessageSender<InteractNpc>>(connection)
            .unwrap()
            .send::<InteractionChannel>(InteractNpc { npc: npc.to_bits() });
        advance(&mut app);
        assert_eq!(
            app.world()
                .get::<ActiveInteraction>(connection)
                .unwrap()
                .auction_house,
            Some(1)
        );
        request(&mut app, connection);
        {
            let mut inbox = app.world_mut().resource_mut::<Inbox>();
            assert!(inbox.errors.is_empty());
            let results = std::mem::take(&mut inbox.results);
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].query.faction, 1);
            assert_eq!(results[0].total_results, 2);
            assert_eq!(
                results[0]
                    .items
                    .iter()
                    .map(|i| i.item_id)
                    .collect::<Vec<_>>(),
                vec![1, 3]
            );
        }
        app.world_mut().get_mut::<Position>(player).unwrap().x = 20.0;
        advance(&mut app);
        assert!(app.world().get::<ActiveInteraction>(connection).is_none());
        request(&mut app, connection);
        let inbox = app.world().resource::<Inbox>();
        assert!(inbox.results.is_empty());
        assert_eq!(
            inbox.errors,
            vec![err(super::super::super::handlers::NO_AUCTIONEER)]
        );
    }
}
