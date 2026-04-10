use super::*;
use shared::components::{
    CharacterAppearance, EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry, Health,
    Player, Position,
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

#[derive(bitcode::Encode)]
struct LegacyCharacterDataV2 {
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
}

#[derive(bitcode::Encode)]
struct LegacyCharacterAppearanceV1 {
    sex: u8,
    skin_color: u8,
    face: u8,
    hair_style: u8,
    hair_color: u8,
    facial_style: u8,
}

#[derive(bitcode::Encode)]
struct LegacyCharacterDataV2AppearanceV1 {
    name: String,
    x: f32,
    y: f32,
    z: f32,
    health: f32,
    max_health: f32,
    race: u8,
    class: u8,
    level: u16,
    appearance: LegacyCharacterAppearanceV1,
}

#[derive(bitcode::Encode)]
struct LegacyCharacterDataV3AppearanceV1 {
    name: String,
    x: f32,
    y: f32,
    z: f32,
    health: f32,
    max_health: f32,
    race: u8,
    class: u8,
    level: u16,
    appearance: LegacyCharacterAppearanceV1,
    equipment_appearance: EquipmentAppearance,
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
        durability: Vec::new(),
    }
}

fn insert_encoded_character(db: &redb::Database, character_id: u64, bytes: Vec<u8>) {
    let txn = db.begin_write().expect("begin write");
    {
        let mut table = txn.open_table(CHARACTERS).expect("open characters");
        table
            .insert(character_id, bytes.as_slice())
            .expect("insert legacy");
    }
    txn.commit().expect("commit");
}

fn legacy_appearance_v1_record() -> LegacyCharacterDataV2AppearanceV1 {
    LegacyCharacterDataV2AppearanceV1 {
        name: "LegacyV2AppearanceV1".into(),
        x: 1.0,
        y: 2.0,
        z: 3.0,
        health: 90.0,
        max_health: 100.0,
        race: 1,
        class: 1,
        level: 7,
        appearance: LegacyCharacterAppearanceV1 {
            sex: 0,
            skin_color: 2,
            face: 3,
            hair_style: 4,
            hair_color: 5,
            facial_style: 6,
        },
    }
}

fn legacy_appearance_v1_with_equipment_record() -> LegacyCharacterDataV3AppearanceV1 {
    LegacyCharacterDataV3AppearanceV1 {
        name: "LegacyV3AppearanceV1".into(),
        x: 1.0,
        y: 2.0,
        z: 3.0,
        health: 90.0,
        max_health: 100.0,
        race: 1,
        class: 1,
        level: 7,
        appearance: LegacyCharacterAppearanceV1 {
            sex: 0,
            skin_color: 2,
            face: 3,
            hair_style: 4,
            hair_color: 5,
            facial_style: 6,
        },
        equipment_appearance: EquipmentAppearance {
            entries: vec![EquippedAppearanceEntry {
                slot: EquipmentVisualSlot::Feet,
                item_id: None,
                display_info_id: Some(154620),
                inventory_type: 0,
                hidden: false,
            }],
        },
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
fn load_character_decodes_previous_appearance_records_without_equipment() {
    let db = temp_database();
    let legacy = LegacyCharacterDataV2 {
        name: "LegacyV2".into(),
        x: 1.0,
        y: 2.0,
        z: 3.0,
        health: 90.0,
        max_health: 100.0,
        race: 1,
        class: 1,
        level: 7,
        appearance: CharacterAppearance {
            sex: 0,
            skin_color: 2,
            face: 3,
            eye_color: 0,
            hair_style: 4,
            hair_color: 5,
            facial_style: 6,
        },
    };

    let bytes = bitcode::encode(&legacy);
    let txn = db.begin_write().expect("begin write");
    {
        let mut table = txn.open_table(CHARACTERS).expect("open characters");
        table.insert(100, bytes.as_slice()).expect("insert legacy");
    }
    txn.commit().expect("commit");

    let loaded = load_character(&db, 100).expect("legacy v2 character should load");
    assert_eq!(loaded.name, "LegacyV2");
    assert_eq!(loaded.level, 7);
    assert_eq!(loaded.appearance, legacy.appearance);
    assert_eq!(loaded.equipment_appearance, EquipmentAppearance::default());
}

#[test]
fn load_character_decodes_previous_appearance_records_without_eye_color() {
    let db = temp_database();
    insert_encoded_character(&db, 101, bitcode::encode(&legacy_appearance_v1_record()));

    let loaded = load_character(&db, 101).expect("legacy v2 appearance v1 should load");
    assert_eq!(loaded.name, "LegacyV2AppearanceV1");
    assert_eq!(loaded.level, 7);
    assert_eq!(loaded.appearance.eye_color, 0);
    assert_eq!(loaded.appearance.hair_style, 4);
    assert_eq!(loaded.equipment_appearance, EquipmentAppearance::default());
}

#[test]
fn load_character_decodes_previous_appearance_records_without_eye_color_but_with_equipment() {
    let db = temp_database();
    insert_encoded_character(
        &db,
        102,
        bitcode::encode(&legacy_appearance_v1_with_equipment_record()),
    );

    let loaded = load_character(&db, 102).expect("legacy v3 appearance v1 should load");
    assert_eq!(loaded.name, "LegacyV3AppearanceV1");
    assert_eq!(loaded.appearance.eye_color, 0);
    assert_eq!(loaded.equipment_appearance.entries.len(), 1);
    assert_eq!(
        loaded.equipment_appearance.entries[0].slot,
        EquipmentVisualSlot::Feet
    );
    assert_eq!(
        loaded.equipment_appearance.entries[0].display_info_id,
        Some(154620)
    );
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

#[test]
fn sync_live_components_preserves_existing_equipment_appearance() {
    let mut data = sample_data();
    data.equipment_appearance = EquipmentAppearance {
        entries: vec![EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Chest,
            item_id: None,
            display_info_id: Some(175942),
            inventory_type: 0,
            hidden: false,
        }],
    };

    let player = Player {
        name: "Theron".into(),
        race: 1,
        class: 1,
        appearance: CharacterAppearance {
            sex: 0,
            skin_color: 2,
            face: 3,
            eye_color: 4,
            hair_style: 5,
            hair_color: 6,
            facial_style: 7,
        },
    };
    let pos = Position {
        x: 12.0,
        y: 34.0,
        z: 56.0,
    };
    let health = Health {
        current: 80.0,
        max: 100.0,
    };

    data.sync_live_components(&player, &pos, &health);

    assert_eq!(data.name, "Theron");
    assert_eq!(data.health, 80.0);
    assert_eq!(data.max_health, 100.0);
    assert_eq!(data.appearance, player.appearance);
    assert_eq!(data.equipment_appearance.entries.len(), 1);
    assert_eq!(
        data.equipment_appearance.entries[0].slot,
        EquipmentVisualSlot::Chest
    );
    assert_eq!(
        data.equipment_appearance.entries[0].display_info_id,
        Some(175942)
    );
}
