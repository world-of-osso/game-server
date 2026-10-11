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
    let mut h = harness_with(HUMAN_TEMPLATE, |_| {}).unwrap();
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
