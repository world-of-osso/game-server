use super::*;
use shared::components::{
    CharacterAppearance, CustomizationChoiceSelection, EquipmentVisualSlot, EquippedAppearanceEntry,
};
use shared::item_data::ItemDefinitionSource::{Forever70205, Retail};

// Independent historical writer: no source field existed in V7 equipment.
#[derive(bitcode::Encode, bitcode::Decode, Clone)]
pub(super) struct PreSourceEquipment {
    entries: Vec<PreSourceEntry>,
}
#[derive(bitcode::Encode, bitcode::Decode, Clone)]
struct PreSourceEntry {
    slot: EquipmentVisualSlot,
    item_id: Option<u32>,
    display_info_id: Option<u32>,
    inventory_type: u8,
    hidden: bool,
}
impl From<shared::components::EquipmentAppearance> for PreSourceEquipment {
    fn from(appearance: shared::components::EquipmentAppearance) -> Self {
        Self {
            entries: appearance
                .entries
                .into_iter()
                .map(|entry| PreSourceEntry {
                    slot: entry.slot,
                    item_id: entry.item_id,
                    display_info_id: entry.display_info_id,
                    inventory_type: entry.inventory_type,
                    hidden: entry.hidden,
                })
                .collect(),
        }
    }
}
impl From<PreSourceEquipment> for shared::components::EquipmentAppearance {
    fn from(appearance: PreSourceEquipment) -> Self {
        Self {
            entries: appearance
                .entries
                .into_iter()
                .map(|entry| EquippedAppearanceEntry {
                    slot: entry.slot,
                    item_id: entry.item_id,
                    definition_source: entry.item_id.map(|_| Retail),
                    display_info_id: entry.display_info_id,
                    inventory_type: entry.inventory_type,
                    hidden: entry.hidden,
                })
                .collect(),
        }
    }
}
#[derive(bitcode::Encode, bitcode::Decode)]
struct PreSourceV7 {
    name: String,
    x: f32,
    y: f32,
    z: f32,
    health: f32,
    max_health: f32,
    race: u8,
    class: u8,
    level: u16,
    appearance: CharacterAppearance,
    equipment_appearance: PreSourceEquipment,
    durability: Vec<DurabilityEntryData>,
}
fn fixture() -> PreSourceV7 {
    PreSourceV7 {
        name: "Frozen".into(),
        x: 11.25,
        y: -22.5,
        z: 33.75,
        health: 77.0,
        max_health: 105.0,
        race: 95,
        class: 8,
        level: 42,
        appearance: CharacterAppearance {
            sex: 1,
            skin_color: 2,
            face: 3,
            eye_color: 4,
            hair_style: 5,
            hair_color: 6,
            facial_style: 7,
            customization_choices: vec![CustomizationChoiceSelection {
                option_id: 48,
                choice_id: 790,
            }],
            visage: Some(shared::components::FormAppearance::default()),
        },
        equipment_appearance: PreSourceEquipment {
            entries: vec![PreSourceEntry {
                slot: EquipmentVisualSlot::Feet,
                item_id: Some(2947),
                display_info_id: Some(154620),
                inventory_type: 8,
                hidden: true,
            }],
        },
        durability: vec![DurabilityEntryData {
            slot: EquipmentVisualSlot::Feet,
            current: 37,
            max: 80,
        }],
    }
}
#[test]
fn character_disk_abi_v7_nonempty_equipment_survives_load_save_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v7.redb");
    let original = bitcode::encode(&fixture());
    {
        let db = Database::create(&path).unwrap();
        create_tables(&db);
        let txn = db.begin_write().unwrap();
        txn.open_table(CHARACTERS)
            .unwrap()
            .insert(41, original.as_slice())
            .unwrap();
        txn.commit().unwrap();
    }
    {
        let db = Database::open(&path).unwrap();
        create_tables(&db);
        let loaded = load_character(&db, 41).unwrap();
        assert_eq!((loaded.x, loaded.y, loaded.z), (11.25, -22.5, 33.75));
        assert_eq!(loaded.appearance, fixture().appearance);
        assert_eq!(loaded.durability, fixture().durability);
        assert_eq!(
            loaded.equipment_appearance.entries[0].definition_source,
            Some(Retail)
        );
        save_character(&db, 41, &loaded);
    }
    let db = Database::open(&path).unwrap();
    create_tables(&db);
    let txn = db.begin_read().unwrap();
    let table = txn.open_table(CHARACTERS).unwrap();
    let bytes = table.get(41).unwrap().unwrap();
    assert_eq!(
        bytes.value(),
        original.as_slice(),
        "writes must preserve exact pre-source ABI"
    );
    let loaded = load_character(&db, 41).unwrap();
    assert_eq!(loaded.appearance, fixture().appearance);
    assert_eq!(loaded.durability, fixture().durability);
}
#[test]
fn character_disk_abi_owned_sources_hydrate_by_guid_not_race_after_reopen() {
    use super::equipment::EquippedItem;
    use super::inventory_store::{InventoryRecord, save_inventory_record};
    use std::collections::HashMap;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sources.redb");
    {
        let db = Database::create(&path).unwrap();
        create_tables(&db);
        for (id, race, source) in [(1, 1, Forever70205), (2, 95, Retail), (3, 95, Retail)] {
            let mut character =
                CharacterData::decode_compatible(&bitcode::encode(&fixture())).unwrap();
            character.race = race;
            character.equipment_appearance.entries[0].definition_source = Some(source);
            character
                .equipment_appearance
                .entries
                .push(EquippedAppearanceEntry {
                    slot: EquipmentVisualSlot::Head,
                    item_id: None,
                    definition_source: None,
                    display_info_id: Some(99),
                    inventory_type: 1,
                    hidden: false,
                });
            let record = InventoryRecord {
                character,
                inventory: InventoryData::default(),
                placements: vec![],
                bag_slots: vec![],
                nonvisual_equipped: Vec::new(),
                instances: HashMap::new(),
                crafting_origins: HashMap::new(),
                item_modifiers: HashMap::new(),
                equipped: vec![EquippedItem {
                    slot: EquipmentVisualSlot::Feet,
                    item: InventoryItemData {
                        item_guid: id * 100,
                        item_id: 2947,
                        name: "Owned".into(),
                        quality: 1,
                        required_level: 1,
                        stack_count: 1,
                        vendor_sell_price: 0,
                        unique_auction: false,
                    },
                }],
                item_durability: HashMap::new(),
                definition_sources: if id == 3 {
                    HashMap::new()
                } else {
                    HashMap::from([(id * 100, source)])
                },
                appearance_account: None,
                appearance_grants: Vec::new(),
                transmogs: Default::default(),
            };
            save_inventory_record(&db, id, &record);
        }
    }
    let db = Database::open(&path).unwrap();
    create_tables(&db);
    for (id, source) in [(1, Forever70205), (2, Retail), (3, Retail)] {
        let loaded = load_character(&db, id).unwrap();
        assert_eq!(
            loaded.equipment_appearance.entries[0].definition_source,
            Some(source)
        );
        assert_eq!(
            loaded.equipment_appearance.entries[1].definition_source,
            None
        );
        let wire: shared::components::EquipmentAppearance =
            bitcode::decode(&bitcode::encode(&loaded.equipment_appearance)).unwrap();
        assert_eq!(wire.entries[0].definition_source, Some(source));
        assert_eq!(wire.entries[1].definition_source, None);
        save_character(&db, id, &loaded);
        assert_eq!(load_character(&db, id).unwrap(), loaded);
    }
}

#[test]
fn character_disk_abi_rejects_unowned_forever_cache_before_save() {
    let db = temp_database();
    let mut character = CharacterData::decode_compatible(&bitcode::encode(&fixture())).unwrap();
    character.equipment_appearance.entries[0].definition_source = Some(Forever70205);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        save_character(&db, 99, &character)
    }));
    assert!(result.is_err());
    assert!(
        load_character(&db, 99).is_none(),
        "rejected cache must not create a character row"
    );
}

#[derive(bitcode::Encode)]
struct PreVisageAppearance {
    sex: u8,
    skin_color: u8,
    face: u8,
    eye_color: u8,
    hair_style: u8,
    hair_color: u8,
    facial_style: u8,
    customization_choices: Vec<CustomizationChoiceSelection>,
}
#[derive(bitcode::Encode)]
struct PreSourceV6 {
    name: String,
    x: f32,
    y: f32,
    z: f32,
    health: f32,
    max_health: f32,
    race: u8,
    class: u8,
    level: u16,
    appearance: PreVisageAppearance,
    equipment_appearance: PreSourceEquipment,
    durability: Vec<DurabilityEntryData>,
}
#[test]
fn character_disk_abi_v6_nonempty_gear_migrates_once_and_reopens() {
    let v7 = fixture();
    let old = PreSourceV6 {
        name: v7.name,
        x: v7.x,
        y: v7.y,
        z: v7.z,
        health: v7.health,
        max_health: v7.max_health,
        race: v7.race,
        class: v7.class,
        level: v7.level,
        appearance: PreVisageAppearance {
            sex: 1,
            skin_color: 2,
            face: 3,
            eye_color: 4,
            hair_style: 5,
            hair_color: 6,
            facial_style: 7,
            customization_choices: v7.appearance.customization_choices,
        },
        equipment_appearance: v7.equipment_appearance,
        durability: v7.durability,
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v6.redb");
    {
        let db = Database::create(&path).unwrap();
        create_tables(&db);
        let txn = db.begin_write().unwrap();
        txn.open_table(CHARACTERS)
            .unwrap()
            .insert(41, bitcode::encode(&old).as_slice())
            .unwrap();
        txn.commit().unwrap();
    }
    let migrated = {
        let db = Database::open(&path).unwrap();
        create_tables(&db);
        let loaded = load_character(&db, 41).unwrap();
        let mut expected = CharacterData::decode_compatible(&bitcode::encode(&fixture())).unwrap();
        expected.appearance.visage = None;
        assert_eq!(loaded, expected);
        save_character(&db, 41, &loaded);
        let txn = db.begin_read().unwrap();
        let table = txn.open_table(CHARACTERS).unwrap();
        let bytes = table.get(41).unwrap().unwrap().value().to_vec();
        bytes
    };
    let db = Database::open(&path).unwrap();
    create_tables(&db);
    let txn = db.begin_read().unwrap();
    let table = txn.open_table(CHARACTERS).unwrap();
    assert_eq!(table.get(41).unwrap().unwrap().value(), migrated.as_slice());
    assert_eq!(
        load_character(&db, 41).unwrap().durability,
        fixture().durability
    );
}
