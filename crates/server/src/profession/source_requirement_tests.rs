use super::super::*;
use super::*;
use crate::item_templates::ItemTemplate;
use crate::persistence::inventory_store::{load_inventory_record, save_inventory_record};
use shared::item_data::ItemDefinitionSource::{Forever70205, Retail};
use std::collections::HashMap;

fn fixture(id: u32, tools: Option<Tools>) -> (World, Entity, u64, SpellInfo) {
    let db = persistence::temp_database();
    let (character, _) = persistence::create_character(&db, 1, "Crafter", 95, 1).unwrap();
    let mut record = load_inventory_record(&db, character).unwrap();
    record.inventory.items.clear();
    record.inventory.items.push(InventoryItemData {
        item_guid: 901,
        item_id: id,
        name: format!("Forever {id}"),
        quality: 1,
        required_level: 0,
        stack_count: 7,
        vendor_sell_price: 0,
        unique_auction: false,
    });
    record.definition_sources.insert(901, Forever70205);
    save_inventory_record(&db, character, &record);
    let template = ItemTemplate {
        max_stack: 200,
        allowable_class: -1,
        allowable_race: u64::MAX,
        ..Default::default()
    };
    let mut data = ProfessionData::default();
    if let Some(tools) = tools {
        data.tools.insert(2963, tools);
        data.item_totem_categories.insert(id, 1);
        data.totem_categories.insert(
            1,
            TotemCategory {
                name: "Retail tool".into(),
                kind: 1,
                mask: 1,
            },
        );
    } else {
        data.reagents.insert(2963, vec![(id, 2)]);
    }
    let mut world = World::new();
    world.insert_resource(GameDatabase::new(db));
    world.insert_resource(ItemTemplates {
        retail: HashMap::from([(id, template)]),
        forever: HashMap::from([(id, template)]),
        loot_need: Default::default(),
        retail_learning_spells: Default::default(),
    });
    world.insert_resource(VendorCatalog::default());
    world.insert_resource(data);
    world.insert_resource(GoldUpdateQueue::default());
    let caster = world
        .spawn((
            ClientId(character),
            Player {
                name: "SourceCrafter".into(),
                race: 95,
                class: 1,
                appearance: Default::default(),
            },
        ))
        .id();
    (
        world,
        caster,
        character,
        SpellInfo {
            id: 2963,
            ..Default::default()
        },
    )
}

#[test]
fn source_requirements_profession_reagents_check_and_take_retail_only() {
    for id in [2947, 2512, 2101] {
        let (mut world, caster, character, spell) = fixture(id, None);
        assert!(
            matches!(
                check_cast_items(&world, caster, &spell),
                Err(CastFailure::Reagents(_))
            ),
            "Forever-only reagent {id}"
        );
        take_reagents(&mut world, caster, &spell);
        let db = &world.resource::<GameDatabase>().db;
        let mut record = load_inventory_record(db, character).unwrap();
        assert_eq!(record.inventory.items[0].stack_count, 7);
        record.inventory.items.push(InventoryItemData {
            item_guid: 902,
            stack_count: 3,
            ..record.inventory.items[0].clone()
        });
        record.definition_sources.insert(902, Retail);
        save_inventory_record(db, character, &record);
        assert!(check_cast_items(&world, caster, &spell).is_ok());
        take_reagents(&mut world, caster, &spell);
        let db = &world.resource::<GameDatabase>().db;
        let inventory = persistence::load_inventory(db, character);
        assert_eq!(
            inventory
                .items
                .iter()
                .find(|item| item.item_guid == 901)
                .unwrap()
                .stack_count,
            7
        );
        assert_eq!(
            inventory
                .items
                .iter()
                .find(|item| item.item_guid == 902)
                .unwrap()
                .stack_count,
            1
        );
    }
}

#[test]
fn source_requirements_profession_explicit_tools_retail_only() {
    assert_tools_retail_only(|id| Tools {
        items: vec![id],
        categories: vec![],
    });
}

#[test]
fn source_requirements_profession_category_tools_retail_only() {
    assert_tools_retail_only(|_| Tools {
        items: vec![],
        categories: vec![1],
    });
}

fn assert_tools_retail_only(tools: impl Fn(u32) -> Tools) {
    for id in [2947, 2512, 2101] {
        let (world, caster, character, spell) = fixture(id, Some(tools(id)));
        assert!(
            matches!(
                check_cast_items(&world, caster, &spell),
                Err(CastFailure::Totems(_))
            ),
            "Forever-only tool {id}"
        );
        let db = &world.resource::<GameDatabase>().db;
        let mut record = load_inventory_record(db, character).unwrap();
        record.inventory.items.push(InventoryItemData {
            item_guid: 902,
            stack_count: 1,
            ..record.inventory.items[0].clone()
        });
        record.definition_sources.insert(902, Retail);
        save_inventory_record(db, character, &record);
        assert!(check_cast_items(&world, caster, &spell).is_ok());
    }
}
