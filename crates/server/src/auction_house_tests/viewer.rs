use super::*;
use crate::item_templates::{ItemTemplate, ItemTemplates};

fn catalog() -> (HashMap<u64, AuctionData>, ItemTemplates) {
    let mut auctions = HashMap::new();
    let mut templates = ItemTemplates::default();
    for id in 1..=120 {
        let mut auction = test_auction(id, "Market item", 1, 1);
        auction.item.item_id = id as u32;
        auctions.insert(id, auction);
        let mut template = ItemTemplate {
            allowable_class: -1,
            allowable_race: u64::MAX,
            required_level: if id % 2 == 0 { 10 } else { 50 },
            ..Default::default()
        };
        template.scaling.class_id = if id % 2 == 0 { 7 } else { 4 };
        templates.retail.insert(id as u32, template);
    }
    (auctions, templates)
}

#[test]
fn viewer_class_filters_before_count_and_pagination() {
    let (auctions, templates) = catalog();
    let mut query = text_query("");
    query.class_id = Some(7);
    let result = super::super::handlers::search_for_viewer(
        &auctions,
        &query,
        Some(&templates),
        None,
        &|_| shared::item_data::ItemDefinitionSource::Retail,
    );
    assert_eq!(result.total_results, 60);
    assert_eq!(result.results.len(), 50);
    assert!(result.results.iter().all(|a| a.item.item_id % 2 == 0));
    query.page = 1;
    let next = super::super::handlers::search_for_viewer(
        &auctions,
        &query,
        Some(&templates),
        None,
        &|_| shared::item_data::ItemDefinitionSource::Retail,
    );
    assert_eq!(next.total_results, 60);
    assert_eq!(next.results.len(), 10);
    assert!(next.results.iter().all(|a| {
        !result
            .results
            .iter()
            .any(|first| first.auction_id == a.auction_id)
    }));
}

#[test]
fn usable_only_applies_player_requirements_before_pagination() {
    let (auctions, mut templates) = catalog();
    templates.retail.get_mut(&2).unwrap().allowable_class = 1 << 7;
    templates.retail.get_mut(&4).unwrap().allowable_race = 1 << 1;
    let mut query = text_query("");
    query.usable_only = true;
    let result = super::super::handlers::search_for_viewer(
        &auctions,
        &query,
        Some(&templates),
        Some((1, 1, 20)),
        &|_| shared::item_data::ItemDefinitionSource::Retail,
    );
    assert_eq!(result.total_results, 58);
    assert_eq!(result.results.len(), 50);
    assert!(
        result
            .results
            .iter()
            .all(|a| a.item.item_id % 2 == 0 && ![2, 4].contains(&a.item.item_id))
    );
    query.page = 1;
    let next = super::super::handlers::search_for_viewer(
        &auctions,
        &query,
        Some(&templates),
        Some((1, 1, 20)),
        &|_| shared::item_data::ItemDefinitionSource::Retail,
    );
    assert_eq!(next.results.len(), 8);
}
