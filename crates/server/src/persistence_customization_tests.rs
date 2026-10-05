use super::character_abi_tests::PreSourceEquipment;
use super::*;
use redb::ReadableTableMetadata;
use shared::components::{
    CharacterAppearance, EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry,
};

// Frozen pre-customization appearance: field order is the historical wire format.
#[derive(bitcode::Encode, Clone)]
struct FrozenAppearance {
    sex: u8,
    skin_color: u8,
    face: u8,
    eye_color: u8,
    hair_style: u8,
    hair_color: u8,
    facial_style: u8,
}

#[derive(bitcode::Encode)]
struct FrozenCharacterWithDurability {
    name: String,
    x: f32,
    y: f32,
    z: f32,
    health: f32,
    max_health: f32,
    race: u8,
    class: u8,
    level: u16,
    appearance: FrozenAppearance,
    equipment_appearance: PreSourceEquipment,
    durability: Vec<DurabilityEntryData>,
}

#[derive(bitcode::Encode)]
struct FrozenCharacterWithoutDurability {
    name: String,
    x: f32,
    y: f32,
    z: f32,
    health: f32,
    max_health: f32,
    race: u8,
    class: u8,
    level: u16,
    appearance: FrozenAppearance,
    equipment_appearance: PreSourceEquipment,
}

fn frozen_character() -> FrozenCharacterWithDurability {
    FrozenCharacterWithDurability {
        name: "Elara".into(),
        x: 11.0,
        y: 22.0,
        z: 33.0,
        health: 77.0,
        max_health: 105.0,
        race: 1,
        class: 8,
        level: 42,
        appearance: FrozenAppearance {
            sex: 1,
            skin_color: 2,
            face: 3,
            eye_color: 4,
            hair_style: 5,
            hair_color: 6,
            facial_style: 7,
        },
        equipment_appearance: EquipmentAppearance {
            entries: vec![EquippedAppearanceEntry {
                slot: EquipmentVisualSlot::Feet,
                item_id: Some(6123),
                definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
                display_info_id: Some(154620),
                inventory_type: 8,
                hidden: false,
            }],
        }
        .into(),
        durability: vec![DurabilityEntryData {
            slot: EquipmentVisualSlot::Feet,
            current: 37,
            max: 80,
        }],
    }
}

fn expected_character() -> CharacterData {
    let frozen = frozen_character();
    CharacterData {
        name: frozen.name,
        x: frozen.x,
        y: frozen.y,
        z: frozen.z,
        health: frozen.health,
        max_health: frozen.max_health,
        race: frozen.race,
        class: frozen.class,
        level: frozen.level,
        appearance: CharacterAppearance {
            sex: 1,
            skin_color: 2,
            face: 3,
            eye_color: 4,
            hair_style: 5,
            hair_color: 6,
            facial_style: 7,
            customization_choices: Vec::new(),
            visage: None,
        },
        equipment_appearance: frozen.equipment_appearance.into(),
        durability: frozen.durability,
    }
}

#[test]
fn customization_migrates_previous_character_with_durability() {
    let bytes = bitcode::encode(&frozen_character());
    let decoded =
        CharacterData::decode_compatible(&bytes).expect("old durability record must decode");
    assert_eq!(decoded, expected_character());
}

#[test]
fn customization_migrates_previous_character_without_durability() {
    let frozen = frozen_character();
    let old = FrozenCharacterWithoutDurability {
        name: frozen.name,
        x: frozen.x,
        y: frozen.y,
        z: frozen.z,
        health: frozen.health,
        max_health: frozen.max_health,
        race: frozen.race,
        class: frozen.class,
        level: frozen.level,
        appearance: frozen.appearance,
        equipment_appearance: frozen.equipment_appearance,
    };
    let decoded = CharacterData::decode_compatible(&bitcode::encode(&old))
        .expect("old equipment record must decode");
    let mut expected = expected_character();
    expected.durability.clear();
    assert_eq!(decoded, expected);
}

#[test]
fn customization_migration_rejects_corrupt_records_without_rewriting_them() {
    let db = temp_database();
    let bytes = [255, 255, 255];
    let txn = db.begin_write().unwrap();
    {
        txn.open_table(CHARACTERS)
            .unwrap()
            .insert(55, bytes.as_slice())
            .unwrap();
    }
    txn.commit().unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| create_tables(&db)));
    assert!(result.is_err(), "unknown encoding must fail explicitly");
    let txn = db.begin_read().unwrap();
    let table = txn.open_table(CHARACTERS).unwrap();
    assert_eq!(table.get(55).unwrap().unwrap().value(), bytes.as_slice());
}

#[test]
fn customization_migration_rewrites_old_bytes_once_across_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.redb");
    {
        let db = Database::create(&path).unwrap();
        create_tables(&db);
        let bytes = bitcode::encode(&frozen_character());
        let txn = db.begin_write().unwrap();
        {
            txn.open_table(CHARACTERS)
                .unwrap()
                .insert(41, bytes.as_slice())
                .unwrap();
        }
        txn.commit().unwrap();
    }
    let migrated_bytes = {
        let db = Database::open(&path).unwrap();
        create_tables(&db);
        assert_eq!(load_character(&db, 41).unwrap(), expected_character());
        let txn = db.begin_read().unwrap();
        let table = txn.open_table(CHARACTERS).unwrap();
        let bytes = table.get(41).unwrap().unwrap().value().to_vec();
        let current = CharacterData::decode_stored(&bytes).expect("startup rewrites old schema");
        assert_eq!(current, expected_character());
        bytes
    };
    let db = Database::open(&path).unwrap();
    create_tables(&db);
    let txn = db.begin_read().unwrap();
    let table = txn.open_table(CHARACTERS).unwrap();
    assert_eq!(
        table.get(41).unwrap().unwrap().value(),
        migrated_bytes.as_slice()
    );
    assert_eq!(table.len().unwrap(), 1);
}
