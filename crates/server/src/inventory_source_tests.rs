//! Concrete collision fixtures exercise production trade, persistence, validation and stat boundaries.
use std::collections::HashMap;

use bevy::prelude::*;
use redb::Database;
use shared::components::{CombatRatings, DerivedStats, Health, Level, Player, UnitStats};
use shared::item_data::ItemDefinitionSource::{self, Forever70205, Retail};
use shared::protocol::{InventoryErrorReason, ItemLocation};
use shared::trade::{CompletedTrade, TradeItem, TradeOffer};

use crate::gold::GoldUpdateQueue;
use crate::inventory::load_model;
use crate::item_stats::{ForeverItemScalingTables, ItemScaling, ItemScalingTables};
use crate::item_templates::{ItemTemplate, ItemTemplates};
use crate::persistence::inventory_store::{
    InventoryRecord, load_item_definition_source, save_inventory_record,
};
use crate::persistence::{self, CharacterData, InventoryData, InventoryItemData};
use crate::stats::{CharacterStats, StatsPlugin};

fn catalog() -> ItemTemplates {
    let plain = ItemTemplate {
        max_stack: 200,
        allowable_class: -1,
        allowable_race: u64::MAX,
        ..Default::default()
    };
    let thrown = ItemTemplate {
        inventory_type: 25,
        max_stack: 1,
        scaling: ItemScaling {
            class_id: 2,
            subclass_id: 16,
            quality: 1,
            item_level: 1,
            delay_ms: 2000,
            ..Default::default()
        },
        ..plain
    };
    ItemTemplates {
        retail: HashMap::from([
            (
                2947,
                ItemTemplate {
                    max_stack: 1,
                    ..plain
                },
            ),
            (2512, plain),
        ]),
        forever: HashMap::from([
            (2947, thrown),
            (
                2512,
                ItemTemplate {
                    max_stack: 1000,
                    ..plain
                },
            ),
        ]),
        loot_need: Default::default(),
        retail_learning_spells: Default::default(),
    }
}

fn record(
    race: u8,
    guid: u64,
    id: u32,
    count: u32,
    source: ItemDefinitionSource,
) -> InventoryRecord {
    InventoryRecord {
        character: CharacterData {
            name: format!("Owner{race}"),
            x: 0.0,
            y: 0.0,
            z: 0.0,
            health: 100.0,
            max_health: 100.0,
            race,
            class: 1,
            level: 1,
            appearance: Default::default(),
            equipment_appearance: Default::default(),
            durability: vec![],
        },
        inventory: InventoryData {
            gold: 1000,
            items: vec![InventoryItemData {
                item_guid: guid,
                item_id: id,
                name: format!("{source:?} {id}"),
                quality: 1,
                required_level: 0,
                stack_count: count,
                vendor_sell_price: 1,
                unique_auction: false,
            }],
        },
        placements: vec![],
        equipped: vec![],
        bag_slots: vec![],
        nonvisual_equipped: Vec::new(),
        instances: HashMap::new(),
        crafting_origins: HashMap::new(),
        item_modifiers: HashMap::new(),
        item_durability: HashMap::new(),
        definition_sources: HashMap::from([(guid, source)]),
        appearance_account: None,
        appearance_grants: Vec::new(),
        transmogs: Default::default(),
    }
}

fn bag(slot: u8) -> ItemLocation {
    ItemLocation::Bag { bag: 0, slot }
}

fn offer(guid: u64, count: u16) -> TradeOffer {
    let mut offer = TradeOffer::default();
    offer.slots[0] = Some(TradeItem {
        item_guid: guid,
        count,
    });
    offer
}

#[test]
fn owned_sources_trade_reopen_validation_equipment_and_emitted_stacks() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    save_inventory_record(&db, 1, &record(95, 101, 2947, 1, Forever70205));
    save_inventory_record(&db, 2, &record(1, 202, 2947, 1, Retail));
    let templates = catalog();
    super::trade_support::execute_completed_trade(
        &db,
        &templates,
        &CompletedTrade {
            initiator: 1,
            target: 2,
            initiator_offer: offer(101, 1),
            target_offer: offer(202, 1),
        },
        &mut GoldUpdateQueue::default(),
    )
    .unwrap();
    drop(db);
    let db = Database::open(file.path()).unwrap();
    let mut retail = load_model(&db, &templates, 1).unwrap();
    let mut forever = load_model(&db, &templates, 2).unwrap();
    assert_eq!(retail.view()[&bag(0)].definition_source, Retail);
    assert_eq!(forever.view()[&bag(0)].definition_source, Forever70205);
    assert_eq!(
        retail.auto_equip(bag(0)).unwrap_err().reason,
        InventoryErrorReason::NotEquippable
    );
    forever.auto_equip(bag(0)).unwrap();
    let equipment = forever.equipment_snapshot();
    assert_eq!(equipment.items[0].item.definition_source, Forever70205);
    assert_eq!(
        forever.appearance().entries[0].definition_source,
        Some(Forever70205)
    );
    let snapshot = super::trade_support::build_trade_party_snapshot(
        &db,
        2,
        "Recipient".into(),
        false,
        &offer(101, 1),
    );
    assert_eq!(
        snapshot.slots[0].as_ref().unwrap().definition_source,
        Forever70205
    );

    // Deliberately different source-local DPS tables detect accidental Retail scaling.
    let mut retail_tables = ItemScalingTables::default();
    retail_tables.damage[0].insert(1, [99.0; 7]);
    let mut forever_tables = ItemScalingTables::default();
    forever_tables.damage[0].insert(1, [7.0; 7]);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(templates);
    app.insert_resource(retail_tables);
    app.insert_resource(ForeverItemScalingTables(forever_tables));
    app.add_plugins(StatsPlugin);
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "RetailRecipient".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            Level(1),
            forever.equipped_items(),
            UnitStats::default(),
            CombatRatings::default(),
            DerivedStats::default(),
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    app.update();
    let stats = app.world().get::<CharacterStats>(player).unwrap();
    assert_eq!((stats.main_hand.min, stats.main_hand.max), (14.0, 14.0));
    save_inventory_record(&db, 2, &forever.into_record());
    drop(db);
    let db = Database::open(file.path()).unwrap();
    let model = load_model(&db, app.world().resource::<ItemTemplates>(), 2).unwrap();
    assert_eq!(model.equipment_snapshot(), equipment);
    assert_eq!(load_item_definition_source(&db, 101), Forever70205);
    assert_eq!(load_item_definition_source(&db, 202), Retail);
}

#[test]
fn owned_sources_merge_split_and_partial_trade_keep_explicit_origin() {
    let db = persistence::temp_database();
    let templates = catalog();
    let mut mixed = record(1, 301, 2512, 10, Retail);
    mixed.inventory.items.push(InventoryItemData {
        item_guid: 302,
        stack_count: 8,
        ..mixed.inventory.items[0].clone()
    });
    mixed.definition_sources.insert(302, Forever70205);
    save_inventory_record(&db, 1, &mixed);
    save_inventory_record(&db, 2, &record(95, 303, 2512, 1, Retail));
    let mut model = load_model(&db, &templates, 1).unwrap();
    assert_eq!(
        model.split(bag(1), bag(0), 2, 999).unwrap_err().reason,
        InventoryErrorReason::CantStack
    );
    model.swap(bag(0), bag(1)).unwrap();
    assert_eq!(model.view().len(), 2);
    model.split(bag(0), bag(2), 3, 304).unwrap();
    assert_eq!(model.view()[&bag(2)].definition_source, Forever70205);
    model.swap(bag(2), bag(0)).unwrap();
    assert_eq!(model.view()[&bag(0)].count, 8);
    save_inventory_record(&db, 1, &model.into_record());
    super::trade_support::execute_completed_trade(
        &db,
        &templates,
        &CompletedTrade {
            initiator: 1,
            target: 2,
            initiator_offer: offer(302, 2),
            target_offer: TradeOffer::default(),
        },
        &mut GoldUpdateQueue::default(),
    )
    .unwrap();
    let recipient = load_model(&db, &templates, 2).unwrap();
    let stacks = recipient.view();
    assert_eq!(stacks.len(), 2);
    let incoming = stacks
        .values()
        .find(|item| item.definition_source == Forever70205)
        .unwrap();
    assert_eq!(incoming.count, 2);
    assert_ne!(incoming.item_guid, 302);
    assert_eq!(
        load_item_definition_source(&db, incoming.item_guid),
        Forever70205
    );
}

#[test]
fn owned_sources_missing_trade_guid_refuses_without_mutation() {
    let db = persistence::temp_database();
    let templates = catalog();
    save_inventory_record(&db, 1, &record(95, 501, 2947, 1, Forever70205));
    save_inventory_record(&db, 2, &record(1, 502, 2947, 1, Retail));
    let before = persistence::load_inventory(&db, 1);
    let result = super::trade_support::execute_completed_trade(
        &db,
        &templates,
        &CompletedTrade {
            initiator: 1,
            target: 2,
            initiator_offer: offer(999, 1),
            target_offer: TradeOffer::default(),
        },
        &mut GoldUpdateQueue::default(),
    );
    assert_eq!(
        result,
        Err(super::trade_support::ExchangeRefusal::OfferGone(1))
    );
    assert_eq!(persistence::load_inventory(&db, 1), before);
}

#[test]
fn owned_sources_missing_forever_catalog_is_error_not_retail_fallback() {
    let db = persistence::temp_database();
    save_inventory_record(&db, 1, &record(1, 401, 2947, 1, Forever70205));
    let mut templates = catalog();
    templates.forever.clear();
    let error = load_model(&db, &templates, 1)
        .err()
        .expect("missing selected catalog must fail");
    assert_eq!(error.reason, InventoryErrorReason::ItemNotFound);
    assert!(error.detail.unwrap().contains("Forever70205"));
}

#[test]
fn source_requirements_bag_and_owned_counts_ignore_forever_collisions() {
    for id in [2947, 2512, 2101] {
        let db = persistence::temp_database();
        let mut templates = catalog();
        let plain = templates.retail[&2512];
        templates.retail.insert(id, plain);
        templates.forever.insert(id, plain);
        save_inventory_record(&db, 1, &record(1, 801, id, 7, Forever70205));
        let model = load_model(&db, &templates, 1).unwrap();
        assert_eq!(
            model.bag_count(Retail, id),
            0,
            "Retail bag requirement for {id}"
        );
        assert_eq!(
            model.owned_count(Retail, id),
            0,
            "Retail unique limit for {id}"
        );
        assert_eq!(model.bag_count(Forever70205, id), 7);
        assert_eq!(model.owned_count(Forever70205, id), 7);
    }
}

#[test]
fn source_requirements_removal_refuses_forever_only_without_mutation() {
    for id in [2947, 2512, 2101] {
        let db = persistence::temp_database();
        let mut templates = catalog();
        let plain = templates.retail[&2512];
        templates.retail.insert(id, plain);
        templates.forever.insert(id, plain);
        save_inventory_record(&db, 1, &record(95, 811, id, 7, Forever70205));
        let mut model = load_model(&db, &templates, 1).unwrap();
        let before = model.view();
        assert!(
            model.destroy_bag_items(Retail, id, 2).is_err(),
            "Retail removal for {id}"
        );
        assert_eq!(model.view(), before);
    }
}

#[test]
fn source_requirements_removal_skips_forever_first_stack() {
    for id in [2947, 2512, 2101] {
        let db = persistence::temp_database();
        let mut templates = catalog();
        let plain = templates.retail[&2512];
        templates.retail.insert(id, plain);
        templates.forever.insert(id, plain);
        let mut mixed = record(95, 821, id, 7, Forever70205);
        mixed.inventory.items.push(InventoryItemData {
            item_guid: 822,
            stack_count: 3,
            ..mixed.inventory.items[0].clone()
        });
        mixed.definition_sources.insert(822, Retail);
        save_inventory_record(&db, 1, &mixed);
        let mut model = load_model(&db, &templates, 1).unwrap();
        model.destroy_bag_items(Retail, id, 2).unwrap();
        let view = model.view();
        assert_eq!(view[&bag(0)].count, 7, "Forever stack {id} remains");
        assert_eq!(view[&bag(1)].count, 1);
        model.destroy_bag_items(Forever70205, id, 7).unwrap();
        assert_eq!(model.bag_count(Forever70205, id), 0);
        assert_eq!(model.bag_count(Retail, id), 1);
    }
}

#[test]
fn source_requirements_quest_removal_skips_forever_first_stack() {
    for id in [2947, 2512, 2101] {
        let db = persistence::temp_database();
        let mut mixed = record(1, 831, id, 7, Forever70205);
        mixed.inventory.items.push(InventoryItemData {
            item_guid: 832,
            stack_count: 3,
            ..mixed.inventory.items[0].clone()
        });
        mixed.definition_sources.insert(832, Retail);
        save_inventory_record(&db, 1, &mixed);
        let mut inventory = persistence::load_inventory(&db, 1);
        crate::quest_view::remove_items(&db, &mut inventory, id, 2);
        persistence::save_inventory(&db, 1, &inventory);
        let inventory = persistence::load_inventory(&db, 1);
        assert_eq!(
            inventory.items[0].stack_count, 7,
            "Forever quest item {id} remains"
        );
        assert_eq!(inventory.items[1].stack_count, 1);
    }
}
