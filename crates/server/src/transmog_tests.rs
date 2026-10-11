//! Authored NPC gossip + real host-message + durable item application boundary.
use super::*;
use crate::auth::Authenticated;
use crate::persistence::{self, GameDatabase};
use shared::components::{
    EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry, Player,
};
use shared::item_data::ItemDefinitionSource;
use shared::protocol::{
    ApplyTransmog, EquipmentSlot, TransmogChange, TransmogChannel, TransmogChoice, TransmogFailed,
    TransmogState,
};

const TARGET: u32 = 2850; // Bronze Shortsword, explicit gold-cost override1500.
const SOURCE: u32 = 821; // Gladius2488 -> ItemAppearance154 -> display1542.
const TARGET_GUID: u64 = 91001;

fn application_harness() -> Harness {
    application_harness_db(persistence::temp_database())
}

fn application_harness_db(db: redb::Database) -> Harness {
    let mut h = harness_with(HUMAN_TEMPLATE, |app| {
        app.insert_resource(GameDatabase { db });
        add_inbox::<TransmogFailed>(app);
        add_inbox::<TransmogState>(app);
        let path = crate::test_data::world_db_path();
        let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        app.insert_resource(crate::transmog::TransmogCatalog::load(&conn).unwrap());
        app.insert_resource(crate::item_templates::load_runtime_item_templates(&path).unwrap());
        app.add_plugins(crate::transmog::TransmogPlugin);
    })
    .unwrap();
    let db = &h.app.world().resource::<GameDatabase>().db;
    let (character_id, _) =
        persistence::create_character_with_test_bags(db, 1, "Transmogger", 1, 1).unwrap();
    let mut record = persistence::inventory_store::load_inventory_record(db, character_id).unwrap();
    record.character.level = 20;
    record.inventory.gold = 5000;
    record.equipped = vec![persistence::equipment::EquippedItem {
        slot: EquipmentVisualSlot::MainHand,
        item: persistence::InventoryItemData {
            item_guid: TARGET_GUID,
            item_id: TARGET,
            name: "Bronze Shortsword".into(),
            quality: 2,
            required_level: 9,
            stack_count: 1,
            vendor_sell_price: 1500,
            unique_auction: false,
        },
    }];
    record
        .definition_sources
        .insert(TARGET_GUID, ItemDefinitionSource::Retail);
    record.character.equipment_appearance = EquipmentAppearance {
        entries: vec![EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::MainHand,
            item_id: Some(TARGET),
            definition_source: Some(ItemDefinitionSource::Retail),
            display_info_id: None,
            inventory_type: 13,
            hidden: false,
        }],
    };
    persistence::inventory_store::save_inventory_record(db, character_id, &record);
    let mut collection = shared::transmog::AppearanceCollection::default();
    collection.learn(154);
    persistence::appearance_store::save_account_appearances(db, 1, &collection);
    h.app.world_mut().entity_mut(h.player).insert((
        Player {
            name: "Transmogger".into(),
            race: 1,
            class: 1,
            appearance: Default::default(),
        },
        record.character.equipment_appearance,
    ));
    h.app.world_mut().entity_mut(h.connection).insert((
        InWorld { character_id },
        Authenticated { account_id: 1 },
        pair::<ApplyTransmog>(),
        pair::<TransmogState>(),
        pair::<TransmogFailed>(),
    ));
    h
}

fn apply(h: &mut Harness, npc: Entity, source: u32) {
    h.send::<_, TransmogChannel>(ApplyTransmog {
        npc: npc.to_bits(),
        changes: vec![TransmogChange {
            slot: EquipmentSlot::MainHand,
            item_guid: TARGET_GUID,
            choice: TransmogChoice::Source(source),
        }],
    });
}

#[test]
fn transmog_apply_authored_service_updates_display_and_gold_atomically() {
    let mut h = application_harness();
    let npc = h.spawn_real_npc(54442, 3.0);
    h.interact(npc);
    h.select(npc, 0);
    assert_eq!(
        h.app
            .world()
            .get::<ActiveInteraction>(h.connection)
            .unwrap()
            .role,
        Some(NpcRole::Transmogrifier)
    );
    apply(&mut h, npc, SOURCE);
    let db = &h.app.world().resource::<GameDatabase>().db;
    let character_id = h
        .app
        .world()
        .get::<InWorld>(h.connection)
        .unwrap()
        .character_id;
    let persisted = persistence::load_character(db, character_id).unwrap();
    assert_eq!(
        persisted.equipment_appearance.entries[0].display_info_id,
        Some(1542)
    );
    assert_eq!(persistence::load_inventory(db, character_id).gold, 3500);
    assert_eq!(
        h.app.world().get::<EquipmentAppearance>(h.player).unwrap(),
        &persisted.equipment_appearance
    );
    assert!(
        persistence::inventory_store::soulbound_guids(db, [TARGET_GUID].into_iter())
            .contains(&TARGET_GUID)
    );
}

fn open_service(h: &mut Harness) -> Entity {
    let npc = h.spawn_real_npc(54442, 3.0);
    h.interact(npc);
    h.select(npc, 0);
    npc
}

fn character_id(h: &Harness) -> u64 {
    h.app
        .world()
        .get::<InWorld>(h.connection)
        .unwrap()
        .character_id
}

#[test]
fn transmog_rejected_batches_preserve_money_equipment_and_binding() {
    use shared::protocol::TransmogError;
    let cases = [
        (
            TransmogChoice::Source(730),
            TARGET_GUID,
            false,
            5000,
            TransmogError::AppearanceNotUsable,
        ),
        (
            TransmogChoice::Hidden,
            TARGET_GUID,
            false,
            5000,
            TransmogError::AppearanceNotUsable,
        ),
        (
            TransmogChoice::Source(SOURCE),
            TARGET_GUID + 1,
            false,
            5000,
            TransmogError::ItemChanged,
        ),
        (
            TransmogChoice::Source(SOURCE),
            TARGET_GUID,
            true,
            5000,
            TransmogError::InvalidSlot,
        ),
        (
            TransmogChoice::Source(SOURCE),
            TARGET_GUID,
            false,
            1499,
            TransmogError::NotEnoughMoney,
        ),
    ];
    for (choice, item_guid, duplicate, gold, expected) in cases {
        let mut h = application_harness();
        let npc = open_service(&mut h);
        let id = character_id(&h);
        let db = &h.app.world().resource::<GameDatabase>().db;
        let mut before = persistence::inventory_store::load_inventory_record(db, id).unwrap();
        before.inventory.gold = gold;
        persistence::inventory_store::save_inventory_record(db, id, &before);
        let change = TransmogChange {
            slot: EquipmentSlot::MainHand,
            item_guid,
            choice,
        };
        let changes = if duplicate {
            vec![change.clone(), change]
        } else {
            vec![change]
        };
        h.send::<_, TransmogChannel>(ApplyTransmog {
            npc: npc.to_bits(),
            changes,
        });
        let after = persistence::inventory_store::load_inventory_record(
            &h.app.world().resource::<GameDatabase>().db,
            id,
        )
        .unwrap();
        assert_eq!(after, before);
        assert_eq!(
            h.app
                .world()
                .resource::<Inbox<TransmogFailed>>()
                .0
                .last()
                .unwrap()
                .error,
            expected
        );
    }
}

#[test]
fn transmog_close_and_out_of_range_refuse_late_apply() {
    for offset in [3.0, 100.0] {
        let mut h = application_harness();
        let npc = open_service(&mut h);
        if offset == 3.0 {
            h.send::<_, InteractionChannel>(CloseInteraction { npc: npc.to_bits() });
        } else {
            h.place_player(npc, offset);
        }
        apply(&mut h, npc, SOURCE);
        let db = &h.app.world().resource::<GameDatabase>().db;
        assert_eq!(persistence::load_inventory(db, character_id(&h)).gold, 5000);
        assert_eq!(
            h.app
                .world()
                .resource::<Inbox<TransmogFailed>>()
                .0
                .last()
                .unwrap()
                .error,
            shared::protocol::TransmogError::NotInteracting
        );
    }
}

#[test]
fn transmog_applied_guid_survives_unequip_reequip_and_database_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("transmog.redb");
    let db = redb::Database::create(&path).unwrap();
    persistence::create_tables(&db);
    let mut h = application_harness_db(db);
    let npc = open_service(&mut h);
    let id = character_id(&h);
    apply(&mut h, npc, SOURCE);
    let db = &h.app.world().resource::<GameDatabase>().db;
    let templates = h
        .app
        .world()
        .resource::<crate::item_templates::ItemTemplates>();
    let mut model = crate::inventory::load_model(db, templates, id).unwrap();
    let equipped = shared::protocol::ItemLocation::Equipment(EquipmentSlot::MainHand);
    let bag = shared::protocol::ItemLocation::Bag { bag: 0, slot: 15 };
    model.swap(equipped, bag).unwrap();
    assert!(
        model
            .appearance()
            .entries
            .iter()
            .all(|entry| entry.slot != EquipmentVisualSlot::MainHand)
    );
    persistence::inventory_store::save_inventory_record(db, id, &model.into_record());
    let mut model = crate::inventory::load_model(db, templates, id).unwrap();
    model.swap(bag, equipped).unwrap();
    assert_eq!(
        model
            .appearance()
            .entries
            .iter()
            .find(|entry| entry.slot == EquipmentVisualSlot::MainHand)
            .unwrap()
            .display_info_id,
        Some(1542)
    );
    persistence::inventory_store::save_inventory_record(db, id, &model.into_record());
    drop(h);
    let reopened = redb::Database::open(&path).unwrap();
    let record = persistence::inventory_store::load_inventory_record(&reopened, id).unwrap();
    assert_eq!(record.inventory.gold, 3500);
    assert_eq!(record.transmogs[&TARGET_GUID].source_id, SOURCE);
    assert_eq!(
        record
            .character
            .equipment_appearance
            .entries
            .iter()
            .find(|entry| entry.slot == EquipmentVisualSlot::MainHand)
            .unwrap()
            .display_info_id,
        Some(1542)
    );
}

#[test]
fn transmog_same_choice_is_free_and_revert_restores_physical_item() {
    let mut h = application_harness();
    let npc = open_service(&mut h);
    apply(&mut h, npc, SOURCE);
    apply(&mut h, npc, SOURCE);
    h.send::<_, TransmogChannel>(ApplyTransmog {
        npc: npc.to_bits(),
        changes: vec![TransmogChange {
            slot: EquipmentSlot::MainHand,
            item_guid: TARGET_GUID,
            choice: TransmogChoice::Original,
        }],
    });
    let record = persistence::inventory_store::load_inventory_record(
        &h.app.world().resource::<GameDatabase>().db,
        character_id(&h),
    )
    .unwrap();
    assert_eq!(record.inventory.gold, 3500);
    assert!(!record.transmogs.contains_key(&TARGET_GUID));
    let entry = record
        .character
        .equipment_appearance
        .entries
        .iter()
        .find(|entry| entry.slot == EquipmentVisualSlot::MainHand)
        .unwrap();
    assert_eq!(entry.item_id, Some(TARGET));
    assert_eq!(entry.display_info_id, None);
}

#[test]
fn transmog_quote_uses_collected_visuals_but_filters_source_class_and_slot() {
    let mut h = application_harness();
    let db = &h.app.world().resource::<GameDatabase>().db;
    let mut collection = persistence::appearance_store::load_account_appearances(db, 1);
    collection.learn_many([2360, 791]); // Rogue's Blade of Cunning; leather vest.
    persistence::appearance_store::save_account_appearances(db, 1, &collection);
    open_service(&mut h);
    let states = &h.app.world().resource::<Inbox<TransmogState>>().0;
    let sources = &states.last().unwrap().slots[0].sources;
    assert!(sources.iter().any(|source| source.source_id == SOURCE));
    assert!(
        sources
            .iter()
            .all(|source| ![2740, 13791, 730].contains(&source.source_id))
    );
    assert_eq!(states.last().unwrap().slots[0].cost, Some(1500));
}

#[test]
fn transmog_unknown_dynamic_price_is_not_replaced_by_raw_sell_price() {
    let mut h = application_harness();
    let id = character_id(&h);
    let db = &h.app.world().resource::<GameDatabase>().db;
    let mut record = persistence::inventory_store::load_inventory_record(db, id).unwrap();
    record.equipped[0].item.item_id = 25; // Raw SellPrice3, no explicit-price override.
    record.character.equipment_appearance.entries[0].item_id = Some(25);
    persistence::inventory_store::save_inventory_record(db, id, &record);
    let npc = open_service(&mut h);
    assert_eq!(
        h.app
            .world()
            .resource::<Inbox<TransmogState>>()
            .0
            .last()
            .unwrap()
            .slots[0]
            .cost,
        None
    );
    apply(&mut h, npc, SOURCE);
    assert_eq!(
        h.app
            .world()
            .resource::<Inbox<TransmogFailed>>()
            .0
            .last()
            .unwrap()
            .error,
        shared::protocol::TransmogError::CostUnavailable
    );
    assert_eq!(
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, id).gold,
        5000
    );
}
