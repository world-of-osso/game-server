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
    browse_for_viewer(auctions, query, None, None, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    })
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
        !super::super::queries::search_auctions(&auctions, &query, &|_| {
            shared::item_data::ItemDefinitionSource::Retail
        })
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
        templates.retail.insert(id as u32, template);
    }
    templates.retail.get_mut(&2).unwrap().allowable_class = 1 << 7;
    templates.retail.get_mut(&4).unwrap().allowable_race = 1 << 1;
    let mut query = text_query("");
    query.class_id = Some(7);
    assert_eq!(
        browse_for_viewer(&auctions, &query, Some(&templates), None, &|_| {
            shared::item_data::ItemDefinitionSource::Retail
        })
        .total_results,
        60
    );
    query.usable_only = true;
    let first = browse_for_viewer(
        &auctions,
        &query,
        Some(&templates),
        Some((1, 1, 20)),
        &|_| shared::item_data::ItemDefinitionSource::Retail,
    );
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
        browse_for_viewer(
            &auctions,
            &query,
            Some(&templates),
            Some((1, 1, 20)),
            &|_| shared::item_data::ItemDefinitionSource::Retail
        )
        .items
        .len(),
        8
    );
    assert_eq!(
        browse_for_viewer(&auctions, &query, Some(&templates), None, &|_| {
            shared::item_data::ItemDefinitionSource::Retail
        })
        .total_results,
        0
    );
    assert_eq!(
        browse_for_viewer(&auctions, &query, None, Some((1, 1, 20)), &|_| {
            shared::item_data::ItemDefinitionSource::Retail
        })
        .total_results,
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
        templates.retail.entry(item).or_insert(template);
    }
    let mut slowest = Duration::ZERO;
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
        let cpu_before = thread_cpu_ticks();
        let start = Instant::now();
        let result = browse_for_viewer(
            &auctions,
            &query,
            Some(&templates),
            Some((1, 1, 20)),
            &|_| shared::item_data::ItemDefinitionSource::Retail,
        );
        let elapsed = start.elapsed();
        let cpu_ticks = thread_cpu_ticks() - cpu_before;
        slowest = slowest.max(elapsed);
        println!(
            "300k auctions, 75k items, text={text:?}, class={class:?}, usable={usable}, page={page}, wall={elapsed:?}, thread_cpu_ticks={cpu_ticks}"
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
    }
    assert!(
        slowest < Duration::from_millis(200),
        "query wall {slowest:?}"
    );
}

/// Linux clock ticks expose descheduling separately from the wall-time gate.
/// Convert with the host's `getconf CLK_TCK`, not an assumed tick frequency.
fn thread_cpu_ticks() -> u64 {
    let stat = std::fs::read_to_string("/proc/thread-self/stat").expect("thread CPU stat");
    let (_, fields) = stat.rsplit_once(')').expect("thread comm terminator");
    fields
        .split_whitespace()
        .skip(11)
        .take(2)
        .map(|ticks| ticks.parse::<u64>().expect("thread CPU ticks"))
        .sum()
}

#[test]
fn cached_item_matching_still_filters_each_inconsistent_snapshot() {
    let mut auctions = HashMap::new();
    for id in 1..=6 {
        let mut auction = offer(id, 1, 3, Some(id * 100));
        auction.item.name = "Iron sword".into();
        auction.item.quality = 2;
        auction.item.required_level = 10;
        auction.faction = 1;
        match id {
            2 => auction.item.name = "Copper ore".into(),
            3 => auction.item.quality = 3,
            4 => auction.item.required_level = 30,
            5 => auction.faction = 2,
            _ => {}
        }
        auctions.insert(id, auction);
    }
    let mut query = text_query("IRON");
    query.quality = Some(2);
    query.max_level = Some(20);
    query.faction = 1;
    let result = browse(&auctions, &query);
    assert_eq!(result.total_results, 1);
    assert_eq!(result.items[0].total_quantity, 6);
    assert_eq!(result.items[0].lowest_unit_price, 34);
    query.text = "copper".into();
    let result = browse(&auctions, &query);
    assert_eq!(result.total_results, 1);
    assert_eq!(result.items[0].name, "Copper ore");
    assert_eq!(result.items[0].total_quantity, 3);
    assert_eq!(result.items[0].lowest_unit_price, 67);
}

#[test]
fn ascii_case_insensitive_search_preserves_literal_non_ascii_and_empty_queries() {
    let mut auctions = HashMap::new();
    for (id, name) in [(1, "Étain Sword"), (2, "étain Ore"), (3, "IRON Sword")] {
        let mut auction = offer(id, id as u32, 3, Some(300));
        auction.item.name = name.into();
        auctions.insert(id, auction);
    }
    for (text, expected) in [
        ("", vec![3, 1, 2]),
        ("ÉTAIN", vec![1]),
        ("étAIN", vec![2]),
        ("iRoN", vec![3]),
        ("n sW", vec![3, 1]),
        ("much longer absent item name", vec![]),
    ] {
        let query = text_query(text);
        let result = browse(&auctions, &query);
        assert_eq!(
            result.items.iter().map(|i| i.item_id).collect::<Vec<_>>(),
            expected,
            "{text:?}"
        );
        let flat = super::super::queries::search_auctions(&auctions, &query, &|_| {
            shared::item_data::ItemDefinitionSource::Retail
        });
        assert_eq!(
            flat.results
                .iter()
                .map(|i| i.item.item_id)
                .collect::<Vec<_>>(),
            expected,
            "flat {text:?}"
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
        app.insert_resource(GameDatabase::new(persistence::temp_database()));
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
            ..Default::default()
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
            .spawn((
                position,
                health,
                UnitFactionTemplate(1),
                crate::maps::InMap::continent(0),
            ))
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
                crate::maps::InMap::continent(0),
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

// Concrete mixed catalog, including mismatching subclasses/slots and three bid ties.
fn ahsort_fixture() -> (HashMap<u64, AuctionData>, ItemTemplates) {
    let mut auctions = HashMap::new();
    let mut templates = ItemTemplates::default();
    for id in 1..=90u64 {
        let mut auction = offer(id, id as u32, (id / 3 + 1) as u32, Some(90_000));
        auction.min_bid = 1;
        auction.current_bid = Some(9_000 - (id / 3) * 30);
        if id == 75 {
            auction.current_bid = None;
            auction.min_bid = 100;
        }
        let mut template = ItemTemplate {
            inventory_type: if id <= 75 { 5 } else { 1 },
            ..Default::default()
        };
        template.scaling.class_id = 4;
        template.scaling.subclass_id = if id <= 80 { 1 } else { 2 };
        templates.retail.insert(id as u32, template);
        auctions.insert(id, auction);
    }
    (auctions, templates)
}

fn ahsort_query(field: AuctionSortField, dir: AuctionSortDir) -> AuctionSearchQuery {
    AuctionSearchQuery {
        class_id: Some(4),
        subcategory_filters: vec![shared::protocol::AuctionItemFilter {
            class_id: 4,
            subclass_id: Some(1),
            inventory_type: Some(5),
        }],
        sort_field: field,
        sort_dir: dir,
        ..Default::default()
    }
}

#[test]
fn ahsort_subcategory_filters_before_flat_and_grouped_paging() {
    let (mut auctions, mut templates) = ahsort_fixture();
    templates.retail.get_mut(&75).unwrap().inventory_type = 20;
    templates.retail.get_mut(&74).unwrap().scaling.class_id = 2;
    // Also reject a known class/slot with no authoritative template.
    templates.retail.remove(&73);
    auctions.get_mut(&72).unwrap().item.name = "Excluded text".into();
    let mut query = ahsort_query(AuctionSortField::Name, AuctionSortDir::Asc);
    query.text = "Item".into();
    query
        .subcategory_filters
        .push(shared::protocol::AuctionItemFilter {
            class_id: 4,
            subclass_id: Some(1),
            inventory_type: Some(20),
        });
    for page in 0..2 {
        query.page = page;
        let grouped = browse_for_viewer(&auctions, &query, Some(&templates), None, &|_| {
            shared::item_data::ItemDefinitionSource::Retail
        });
        let flat = super::super::handlers::search_for_viewer(
            &auctions,
            &query,
            Some(&templates),
            None,
            &|_| shared::item_data::ItemDefinitionSource::Retail,
        );
        let expected = if page == 0 {
            (1..=50).collect::<Vec<_>>()
        } else {
            (51..=71).chain([75]).collect()
        };
        assert_eq!(grouped.total_results, 72);
        assert_eq!(flat.total_results, 72);
        assert_eq!(
            grouped.items.iter().map(|i| i.item_id).collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            flat.results
                .iter()
                .map(|i| i.item.item_id)
                .collect::<Vec<_>>(),
            expected
        );
    }
    query.class_id = None; // Child filters must work without a redundant parent filter.
    query.page = 0;
    assert_eq!(
        browse_for_viewer(&auctions, &query, Some(&templates), None, &|_| {
            shared::item_data::ItemDefinitionSource::Retail
        })
        .total_results,
        72
    );
    query.subcategory_filters[0].inventory_type = None;
    assert_eq!(
        browse_for_viewer(&auctions, &query, Some(&templates), None, &|_| {
            shared::item_data::ItemDefinitionSource::Retail
        })
        .total_results,
        77
    );
}

#[test]
fn ahsort_current_bid_and_available_flat_order_across_page_boundary() {
    let (auctions, templates) = ahsort_fixture();
    for field in [AuctionSortField::Bid, AuctionSortField::Quantity] {
        for dir in [AuctionSortDir::Asc, AuctionSortDir::Desc] {
            let mut query = ahsort_query(field, dir);
            let mut expected = (1..=75u64).collect::<Vec<_>>();
            expected.sort_by_key(|id| {
                let a = &auctions[id];
                let value = if field == AuctionSortField::Bid {
                    a.current_bid.unwrap_or(a.min_bid)
                } else {
                    u64::from(a.stack_count)
                };
                (
                    if dir == AuctionSortDir::Desc {
                        u64::MAX - value
                    } else {
                        value
                    },
                    *id,
                )
            });
            let mut actual = Vec::new();
            for page in 0..2 {
                query.page = page;
                let result = super::super::handlers::search_for_viewer(
                    &auctions,
                    &query,
                    Some(&templates),
                    None,
                    &|_| shared::item_data::ItemDefinitionSource::Retail,
                );
                assert_eq!(result.total_results, 75);
                assert_eq!(result.results.len(), if page == 0 { 50 } else { 25 });
                actual.extend(result.results.into_iter().map(|i| i.auction_id));
            }
            assert_eq!(actual, expected, "{field:?} {dir:?}");
        }
    }
}

#[test]
fn ahsort_grouped_quantity_and_current_bid_order_across_page_boundary() {
    let (mut auctions, templates) = ahsort_fixture();
    // Quantity and lowest current bid must include offers outside the first flat page.
    for id in 1..=75u64 {
        let mut copy = auctions[&id].clone();
        copy.auction_id = 1000 + id;
        copy.item.item_guid = copy.auction_id;
        copy.stack_count = 100 - id as u32;
        copy.current_bid = Some((id / 3 + 1) * u64::from(copy.stack_count));
        auctions.insert(copy.auction_id, copy);
    }
    for field in [AuctionSortField::Quantity, AuctionSortField::Bid] {
        for dir in [AuctionSortDir::Asc, AuctionSortDir::Desc] {
            let mut query = ahsort_query(field, dir);
            let mut expected = (1..=75u32).collect::<Vec<_>>();
            let value = |id: u32| -> (u64, u64) {
                if field == AuctionSortField::Quantity {
                    (
                        u64::from(auctions[&u64::from(id)].stack_count) + 100 - u64::from(id),
                        1,
                    )
                } else if id == 75 {
                    (100, u64::from(auctions[&75].stack_count))
                } else {
                    (u64::from(id) / 3 + 1, 1)
                }
            };
            expected.sort_by(|a, b| {
                let (an, ad) = value(*a);
                let (bn, bd) = value(*b);
                let order = (an * bd).cmp(&(bn * ad));
                (if dir == AuctionSortDir::Desc {
                    order.reverse()
                } else {
                    order
                })
                .then(a.cmp(b))
            });
            let mut actual = Vec::new();
            for page in 0..2 {
                query.page = page;
                let result = browse_for_viewer(&auctions, &query, Some(&templates), None, &|_| {
                    shared::item_data::ItemDefinitionSource::Retail
                });
                assert_eq!(result.total_results, 75);
                assert_eq!(result.items.len(), if page == 0 { 50 } else { 25 });
                actual.extend(result.items.into_iter().map(|i| i.item_id));
            }
            assert_eq!(actual, expected, "{field:?} {dir:?}");
        }
    }
}

#[test]
fn ahsort_source_local_filters_and_stable_group_ties_span_two_pages() {
    use shared::item_data::ItemDefinitionSource::{Forever70205, Retail};
    let mut auctions = HashMap::new();
    let mut templates = ItemTemplates::default();
    for item in 1..=40u32 {
        let mut template = ItemTemplate {
            inventory_type: 5,
            ..Default::default()
        };
        template.scaling.class_id = 4;
        template.scaling.subclass_id = 1;
        templates.retail.insert(item, template);
        if item == 40 {
            template.scaling.subclass_id = 2;
        }
        templates.forever.insert(item, template);
        for base in [0, 1000] {
            let id = u64::from(item) + base;
            auctions.insert(id, offer(id, item, 3, Some(300)));
        }
    }
    let source = |guid| if guid < 1000 { Retail } else { Forever70205 };
    let expected = (1..=40)
        .flat_map(|item| {
            if item == 40 {
                vec![(item, Retail)]
            } else {
                vec![(item, Retail), (item, Forever70205)]
            }
        })
        .collect::<Vec<_>>();
    for field in [AuctionSortField::Bid, AuctionSortField::Quantity] {
        for dir in [AuctionSortDir::Asc, AuctionSortDir::Desc] {
            let mut query = ahsort_query(field, dir);
            let mut actual = Vec::new();
            for page in 0..2 {
                query.page = page;
                let result = browse_for_viewer(&auctions, &query, Some(&templates), None, &source);
                assert_eq!(result.total_results, 79);
                assert_eq!(result.items.len(), if page == 0 { 50 } else { 29 });
                actual.extend(
                    result
                        .items
                        .into_iter()
                        .map(|i| (i.item_id, i.definition_source)),
                );
            }
            assert_eq!(actual, expected, "{field:?} {dir:?}");
        }
    }
}
