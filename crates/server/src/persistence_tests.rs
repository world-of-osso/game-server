use super::*;
use shared::components::{
    CharacterAppearance, EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry,
};

#[derive(bitcode::Encode)]
struct LegacyCharacterData {
    name: String,
    x: f32,
    y: f32,
    z: f32,
    health: f32,
    max_health: f32,
    race: u8,
    class: u8,
    level: u16,
}

fn sample_data() -> CharacterData {
    CharacterData {
        name: "TestPlayer".into(),
        x: -8949.0,
        y: -132.0,
        z: 83.0,
        health: 75.0,
        max_health: 100.0,
        race: 1,
        class: 1,
        level: 1,
        appearance: CharacterAppearance::default(),
        equipment_appearance: EquipmentAppearance::default(),
    }
}

#[test]
fn character_data_round_trip() {
    let db = temp_database();
    let data = sample_data();
    save_character(&db, 42, &data);
    let loaded = load_character(&db, 42).expect("should find saved character");
    assert_eq!(loaded, data);
}

#[test]
fn load_missing_character_returns_none() {
    let db = temp_database();
    assert!(load_character(&db, 999).is_none());
}

#[test]
fn save_overwrites_existing() {
    let db = temp_database();
    let mut data = sample_data();
    save_character(&db, 1, &data);
    data.x = 100.0;
    data.health = 50.0;
    save_character(&db, 1, &data);
    let loaded = load_character(&db, 1).unwrap();
    assert_eq!(loaded.x, 100.0);
    assert_eq!(loaded.health, 50.0);
}

#[test]
fn load_character_decodes_legacy_records_with_default_appearance() {
    let db = temp_database();
    let legacy = LegacyCharacterData {
        name: "Legacy".into(),
        x: 1.0,
        y: 2.0,
        z: 3.0,
        health: 90.0,
        max_health: 100.0,
        race: 1,
        class: 1,
        level: 7,
    };

    let bytes = bitcode::encode(&legacy);
    let txn = db.begin_write().expect("begin write");
    {
        let mut table = txn.open_table(CHARACTERS).expect("open characters");
        table.insert(99, bytes.as_slice()).expect("insert legacy");
    }
    txn.commit().expect("commit");

    let loaded = load_character(&db, 99).expect("legacy character should load");
    assert_eq!(loaded.name, "Legacy");
    assert_eq!(loaded.level, 7);
    assert_eq!(loaded.appearance, CharacterAppearance::default());
}

#[test]
fn character_data_round_trip_preserves_equipment_appearance() {
    let db = temp_database();
    let mut data = sample_data();
    data.equipment_appearance = EquipmentAppearance {
        entries: vec![EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Chest,
            item_id: Some(6123),
            display_info_id: Some(777),
            inventory_type: 5,
            hidden: false,
        }],
    };

    save_character(&db, 42, &data);
    let loaded = load_character(&db, 42).expect("should find saved character");

    assert_eq!(loaded.equipment_appearance, data.equipment_appearance);
}
