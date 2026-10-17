use super::character_abi_tests::PreSourceEquipment;
use super::*;
use shared::components::{
    CharacterAppearance, EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry, Health,
    Player, Position,
};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

#[test]
fn achievements_criteria_reputation_migrates_known_delta_once_and_retains_earned_ids() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = Database::create(file.path()).unwrap();
    create_tables(&db);
    let (character, _) = create_character(&db, 1, "Repmigrate", 1, 1).unwrap();
    let legacy: CharacterReputation = serde_json::from_value(serde_json::json!({
        "values": {"72": 21250, "76": 0, "576": 12200},
        "tracked": [72], "watched": 72
    }))
    .unwrap();
    let txn = db.begin_write().unwrap();
    txn.open_table(TableDefinition::<u64, &[u8]>::new("reputations"))
        .unwrap()
        .insert(character, bitcode::encode(&legacy).as_slice())
        .unwrap();
    txn.commit().unwrap();
    let mut earned = crate::achievement::PlayerAchievementState::default();
    earned.earned_ids.insert(944);
    save_achievements(&db, character, &earned);
    let registry = shared::reputation::default_faction_registry();
    let migrated = load_reputation(&db, character, || {
        CharacterReputation::new_for_race(&registry, 1)
    });
    assert_eq!(migrated.get_value(72), 4250);
    assert_eq!(migrated.get_value(76), -42000);
    assert_eq!(migrated.get_value(576), -2700);
    assert_eq!(migrated.watched_faction(), Some(72));
    assert_eq!(
        load_achievements(&db, character).earned_ids,
        earned.earned_ids
    );
    drop(db);
    let db = Database::open(file.path()).unwrap();
    let reloaded = load_reputation(&db, character, || panic!("already migrated"));
    assert_eq!(reloaded, migrated);
}

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

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
    appearance: LegacyCharacterAppearanceV2,
}

#[derive(bitcode::Encode)]
struct LegacyCharacterAppearanceV2 {
    sex: u8,
    skin_color: u8,
    face: u8,
    eye_color: u8,
    hair_style: u8,
    hair_color: u8,
    facial_style: u8,
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
    equipment_appearance: PreSourceEquipment,
}

#[derive(bitcode::Encode)]
struct LegacyInventoryItemData {
    item_guid: u64,
    item_id: u32,
    name: String,
    quality: u8,
    required_level: u16,
    stack_count: u32,
    vendor_sell_price: u32,
}

#[derive(bitcode::Encode)]
struct LegacyInventoryData {
    gold: u32,
    items: Vec<LegacyInventoryItemData>,
}

/// The inventory layout before gold became `u64`.
#[derive(bitcode::Encode)]
struct LegacyInventoryDataU32Gold {
    gold: u32,
    items: Vec<InventoryItemData>,
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
                definition_source: None,
                display_info_id: Some(154620),
                inventory_type: 0,
                hidden: false,
            }],
        }
        .into(),
    }
}

#[test]
fn account_lookup_handles_missing_empty_and_populated_tables() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    assert_eq!(find_account_id_for_character(&db, 42), None);

    let txn = db.begin_write().unwrap();
    txn.open_table(ACCOUNT_CHARACTERS).unwrap();
    txn.commit().unwrap();
    assert_eq!(find_account_id_for_character(&db, 42), None);

    let txn = db.begin_write().unwrap();
    {
        let mut table = txn.open_table(ACCOUNT_CHARACTERS).unwrap();
        table
            .insert(7, bitcode::encode(&vec![42_u64, 43]).as_slice())
            .unwrap();
        table
            .insert(8, bitcode::encode(&vec![44_u64]).as_slice())
            .unwrap();
    }
    txn.commit().unwrap();
    drop(db);
    let db = redb::Database::open(file.path()).unwrap();
    assert_eq!(find_account_id_for_character(&db, 42), Some(7));
    assert_eq!(find_account_id_for_character(&db, 43), Some(7));
    assert_eq!(find_account_id_for_character(&db, 44), Some(8));
    assert_eq!(find_account_id_for_character(&db, 45), None);
}

#[test]
fn legacy_inventories_move_to_u64_gold_during_startup() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    let linen = |unique_auction| InventoryItemData {
        item_guid: 7,
        item_id: 2589,
        name: "Linen Cloth".into(),
        quality: 1,
        required_level: 1,
        stack_count: 20,
        vendor_sell_price: 13,
        unique_auction,
    };
    // Character 42: the first layout; character 43: the u32-gold layout at the
    // AzerothCore cap.
    let first = bitcode::encode(&LegacyInventoryData {
        gold: 42_000,
        items: vec![LegacyInventoryItemData {
            item_guid: 7,
            item_id: 2589,
            name: "Linen Cloth".into(),
            quality: 1,
            required_level: 1,
            stack_count: 20,
            vendor_sell_price: 13,
        }],
    });
    let u32_gold = bitcode::encode(&LegacyInventoryDataU32Gold {
        gold: 0x7FFF_FFFE,
        items: vec![linen(true)],
    });
    {
        let txn = db.begin_write().unwrap();
        {
            let mut table = txn.open_table(LEGACY_INVENTORIES).unwrap();
            table.insert(42, first.as_slice()).unwrap();
            table.insert(43, u32_gold.as_slice()).unwrap();
        }
        txn.commit().unwrap();
    }

    create_tables(&db);

    assert_eq!(
        load_inventory(&db, 42),
        InventoryData {
            gold: 42_000,
            items: vec![linen(false)],
        }
    );
    assert_eq!(
        load_inventory(&db, 43),
        InventoryData {
            gold: 2_147_483_646,
            items: vec![linen(true)],
        }
    );
    let txn = db.begin_read().unwrap();
    assert!(txn.open_table(LEGACY_INVENTORIES).is_err());
    drop(txn);
    // A second startup has nothing left to move.
    create_tables(&db);
    assert_eq!(load_inventory(&db, 43).gold, 2_147_483_646);
}

#[test]
fn legacy_database_migrates_missing_pvp_state_table() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    {
        let txn = db.begin_write().unwrap();
        txn.open_table(CHARACTERS).unwrap();
        txn.commit().unwrap();
    }

    create_tables(&db);

    assert_eq!(load_pvp(&db, 42), crate::pvp::PlayerPvpProgress::default());
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
fn customization_migration_reopens_all_earlier_schemas() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("earlier-characters.redb");
    let six = legacy_appearance_v1_record();
    let equipped_six = legacy_appearance_v1_with_equipment_record();
    let absent = LegacyCharacterData {
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
    let seven = LegacyCharacterDataV2 {
        name: "LegacySeven".into(),
        x: 1.0,
        y: 2.0,
        z: 3.0,
        health: 90.0,
        max_health: 100.0,
        race: 1,
        class: 1,
        level: 7,
        appearance: LegacyCharacterAppearanceV2 {
            sex: 0,
            skin_color: 2,
            face: 3,
            eye_color: 9,
            hair_style: 4,
            hair_color: 5,
            facial_style: 6,
        },
    };
    let expected_six = CharacterAppearance {
        sex: 0,
        skin_color: 2,
        face: 3,
        eye_color: 0,
        hair_style: 4,
        hair_color: 5,
        facial_style: 6,
        customization_choices: Vec::new(),
        visage: None,
    };
    let fixtures = [
        (
            bitcode::encode(&absent),
            "Legacy",
            CharacterAppearance::default(),
            EquipmentAppearance::default(),
        ),
        (
            bitcode::encode(&six),
            "LegacyV2AppearanceV1",
            expected_six.clone(),
            EquipmentAppearance::default(),
        ),
        (
            bitcode::encode(&equipped_six),
            "LegacyV3AppearanceV1",
            expected_six.clone(),
            equipped_six.equipment_appearance.clone().into(),
        ),
        (
            bitcode::encode(&seven),
            "LegacySeven",
            CharacterAppearance {
                eye_color: 9,
                ..expected_six
            },
            EquipmentAppearance::default(),
        ),
    ];
    {
        let db = redb::Database::create(&path).unwrap();
        create_tables(&db);
        for (index, (bytes, _, _, _)) in fixtures.iter().enumerate() {
            insert_encoded_character(&db, index as u64 + 1, bytes.clone());
        }
    }
    let current_bytes = {
        let db = redb::Database::open(&path).unwrap();
        create_tables(&db);
        let txn = db.begin_read().unwrap();
        let table = txn.open_table(CHARACTERS).unwrap();
        fixtures
            .iter()
            .enumerate()
            .map(|(index, (_, name, appearance, equipment))| {
                let bytes = table
                    .get(index as u64 + 1)
                    .unwrap()
                    .unwrap()
                    .value()
                    .to_vec();
                let current: CharacterData = CharacterData::decode_stored(&bytes)
                    .expect("legacy row rewritten to current schema");
                assert_eq!(
                    current,
                    CharacterData {
                        name: (*name).into(),
                        x: 1.0,
                        y: 2.0,
                        z: 3.0,
                        health: 90.0,
                        max_health: 100.0,
                        race: 1,
                        class: 1,
                        level: 7,
                        appearance: appearance.clone(),
                        equipment_appearance: equipment.clone(),
                        durability: Vec::new(),
                    }
                );
                bytes
            })
            .collect::<Vec<_>>()
    };
    let db = redb::Database::open(&path).unwrap();
    create_tables(&db);
    let txn = db.begin_read().unwrap();
    let table = txn.open_table(CHARACTERS).unwrap();
    for (index, expected) in current_bytes.iter().enumerate() {
        assert_eq!(
            table.get(index as u64 + 1).unwrap().unwrap().value(),
            expected.as_slice()
        );
    }
    assert_eq!(table.iter().unwrap().count(), fixtures.len());
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
fn game_database_path_defaults_to_project_data_file() {
    let _guard = env_lock().lock().unwrap();
    unsafe { std::env::remove_var("GAME_SERVER_DB_PATH") };
    assert_eq!(game_database_path(), PathBuf::from("data/game.redb"));
}

#[test]
fn game_database_path_uses_env_override() {
    let _guard = env_lock().lock().unwrap();
    unsafe { std::env::set_var("GAME_SERVER_DB_PATH", "/tmp/custom.redb") };
    assert_eq!(game_database_path(), PathBuf::from("/tmp/custom.redb"));
    unsafe { std::env::remove_var("GAME_SERVER_DB_PATH") };
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
        appearance: LegacyCharacterAppearanceV2 {
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
    assert_eq!(
        loaded.appearance,
        CharacterAppearance {
            sex: 0,
            skin_color: 2,
            face: 3,
            eye_color: 0,
            hair_style: 4,
            hair_color: 5,
            facial_style: 6,
            customization_choices: Vec::new(),
            visage: None,
        }
    );
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
            definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
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
            definition_source: None,
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
            customization_choices: Vec::new(),
            visage: None,
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

/// Bevy `(x, y, z)` stored as WoW `(x, -z, y)`.
fn stored_position(db: &Database, character_id: u64) -> (f32, f32, f32) {
    let data = load_character(db, character_id).expect("saved character");
    (data.x, data.y, data.z)
}

#[test]
fn a_player_who_stops_moving_before_the_save_tick_is_still_saved() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(FixedUpdate, persistence_runtime::save_dirty_characters);
    app.insert_resource(GameDatabase::new(temp_database()));
    let db = &app.world().resource::<GameDatabase>().db;
    let (character_id, _) = create_character(db, 1, "Walker", 1, 1).unwrap();
    let player = app
        .world_mut()
        .spawn((
            crate::maps::InMap::continent(0),
            Player {
                name: "Walker".into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            ClientId(character_id),
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().get_mut::<Position>(player).unwrap().x = 25.0;
    // Moves on tick 2, then stands still through the 300-tick save.
    for _ in 0..300 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    let db = app
        .world_mut()
        .remove_resource::<GameDatabase>()
        .unwrap()
        .into_database();
    assert_eq!(stored_position(&db, character_id).0, 25.0);
}

fn spawn_saved_player(app: &mut App, name: &str, map_id: u32, x: f32) -> u64 {
    let db = &app.world().resource::<GameDatabase>().db;
    let (character_id, _) = create_character(db, 1, name, 1, 1).unwrap();
    app.world_mut().spawn((
        crate::maps::InMap::continent(map_id),
        Player {
            name: name.into(),
            race: 1,
            class: 1,
            appearance: CharacterAppearance::default(),
        },
        ClientId(character_id),
        Position { x, y: 0.0, z: 0.0 },
        Health {
            current: 100.0,
            max: 100.0,
        },
    ));
    character_id
}

#[test]
fn periodic_save_never_waits_for_a_busy_durable_writer() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(FixedUpdate, persistence_runtime::save_dirty_characters);
    app.insert_resource(GameDatabase::new(temp_database()));
    let character = spawn_saved_player(&mut app, "Queued", 0, 25.0);
    for _ in 0..299 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    let held = app
        .world()
        .resource::<GameDatabase>()
        .db
        .begin_write()
        .unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        held.abort().unwrap();
    });
    let started = std::time::Instant::now();
    app.world_mut().run_schedule(FixedUpdate);
    let elapsed = started.elapsed();
    let entity = app
        .world_mut()
        .query_filtered::<Entity, With<ClientId>>()
        .single(app.world())
        .unwrap();
    app.world_mut().get_mut::<Position>(entity).unwrap().x = 50.0;
    app.world_mut().get_mut::<Health>(entity).unwrap().current = 33.0;
    release.join().unwrap();
    let saved = app
        .world_mut()
        .remove_resource::<GameDatabase>()
        .unwrap()
        .into_database();
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "save tick waited for writer: {elapsed:?}"
    );
    assert_eq!(stored_position(&saved, character).0, 25.0);
    assert_eq!(load_character(&saved, character).unwrap().health, 100.0);
}

#[derive(Resource, Default)]
struct TransferBurstFrames {
    frame: u8,
    arrivals: Vec<u8>,
}

fn record_transfer_burst_frame(
    event: On<Insert, crate::maps::InMap>,
    maps: Query<&crate::maps::InMap, With<Player>>,
    mut frames: ResMut<TransferBurstFrames>,
) {
    if maps.get(event.entity).is_ok_and(|map| map.map_id == 2106) {
        let frame = frames.frame;
        frames.arrivals.push(frame);
    }
}

#[test]
fn twenty_world_port_acks_and_due_save_never_wait_for_a_busy_writer_and_survive_reopen() {
    use crate::auth::{Authenticated, InWorld};
    use crate::maps::{
        InMap, InstanceManager, MapStore, MapTransfer, ServerClock, TeleportOptions,
    };
    use crate::networking::PlayerLink;
    use lightyear::connection::host::HostClient;
    use lightyear::prelude::*;
    use shared::components::{Level, MovementControl, Rotation};
    use shared::protocol::{TransferChannel, WorldPortAck};

    let file = tempfile::NamedTempFile::new().unwrap();
    let database = Database::create(file.path()).unwrap();
    create_tables(&database);
    let source = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let maps = crate::maps::store::load_map_store(&source).unwrap();
    // Active Retail Warsong, not retired map489.
    let start = shared::battleground::flag_rules(2106).unwrap().starts[0];
    let destination = maps.safe_locs[&start];
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: std::time::Duration::from_millis(50),
    });
    app.add_plugins(shared::ProtocolPlugin);
    // Advance the real fixed-save system explicitly; wire delivery must not consume
    // its 300-tick interval while arranging the burst.
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::ZERO,
    ));
    app.insert_resource(GameDatabase::new(database));
    app.insert_resource(maps);
    app.init_resource::<InstanceManager>();
    app.insert_resource(ServerClock {
        fixed_unix: Some(1790812900),
        offset_secs: 0,
    });
    app.init_resource::<TransferBurstFrames>();
    app.add_observer(record_transfer_burst_frame);
    app.add_systems(Update, crate::maps::transfer::receive_world_port_acks);
    app.add_systems(FixedUpdate, persistence_runtime::save_dirty_characters);
    app.finish();
    app.cleanup();
    let mut players = Vec::new();
    let mut connections = Vec::new();
    let mut roster = Vec::new();
    for index in 0..20_u8 {
        let name = format!("Port{}", char::from(b'a' + index));
        let (id, _) =
            create_character(&app.world().resource::<GameDatabase>().db, 1, &name, 1, 1).unwrap();
        roster.push(id);
        let player = app
            .world_mut()
            .spawn((
                Player {
                    name,
                    race: 1,
                    class: 1,
                    appearance: Default::default(),
                },
                ClientId(id),
                Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                Level(80),
                Rotation {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                MovementControl::default(),
                InMap::continent(0),
            ))
            .id();
        let connection = app
            .world_mut()
            .spawn((
                RemoteId(PeerId::Local(id)),
                Connected,
                HostClient { buffer: Vec::new() },
                Authenticated { account_id: 1 },
                InWorld { character_id: id },
                PlayerLink(player),
                MessageSender::<WorldPortAck>::default(),
                MessageReceiver::<WorldPortAck>::default(),
            ))
            .id();
        players.push((id, player));
        connections.push(connection);
    }
    let copy = app
        .world_mut()
        .resource_scope(|world, mut manager: Mut<InstanceManager>| {
            manager.create_battleground_map(
                world.resource::<MapStore>(),
                destination.map_id,
                &roster,
                20,
                crate::maps::instances::Now {
                    elapsed: 0.0,
                    unix: 1790812900,
                },
            )
        })
        .unwrap();
    app.update();
    for (_, player) in &players {
        let origin = crate::maps::transfer::current_location(app.world(), *player, 0);
        app.world_mut()
            .entity_mut(*player)
            .remove::<InMap>()
            .insert((
                destination.bevy_position(),
                MapTransfer {
                    map_id: destination.map_id,
                    destination,
                    origin,
                    options: TeleportOptions::default(),
                },
            ));
    }
    for _ in 0..299 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    for connection in &connections {
        app.world_mut()
            .get_mut::<MessageSender<WorldPortAck>>(*connection)
            .unwrap()
            .send::<TransferChannel>(WorldPortAck);
    }
    let held = app
        .world()
        .resource::<GameDatabase>()
        .db
        .begin_write()
        .unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        held.abort().unwrap();
    });
    let started = std::time::Instant::now();
    for frame in 1..=3 {
        app.world_mut().resource_mut::<TransferBurstFrames>().frame = frame;
        app.update();
    }
    app.world_mut().run_schedule(FixedUpdate);
    let elapsed = started.elapsed();
    let arrivals = &app.world().resource::<TransferBurstFrames>().arrivals;
    assert_eq!(arrivals.len(), 20);
    assert!(
        arrivals.iter().all(|frame| *frame == arrivals[0]),
        "ACKs must arrive in one frame: {arrivals:?}"
    );
    for (_, player) in &players {
        assert_eq!(app.world().get::<InMap>(*player), Some(&copy));
        assert!(app.world().get::<MapTransfer>(*player).is_none());
        app.world_mut().get_mut::<Position>(*player).unwrap().x += 50.0;
        app.world_mut().get_mut::<Health>(*player).unwrap().current = 33.0;
    }
    release.join().unwrap();
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "20 WorldPortAck/save burst blocked: {elapsed:?}"
    );
    drop(app); // FIFO drain must retain submission-time destinations, not later live state.
    let db = Database::open(file.path()).unwrap();
    let expected = destination.position;
    for id in roster {
        assert_eq!(
            crate::maps::persistence::load_character_map(&db, id),
            destination.map_id
        );
        assert_eq!(
            stored_position(&db, id),
            (expected.x, expected.y, expected.z)
        );
        assert_eq!(load_character(&db, id).unwrap().health, 100.0);
    }
}

#[test]
fn periodic_save_writes_every_changed_player_with_map_and_corpse() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(FixedUpdate, persistence_runtime::save_dirty_characters);
    app.insert_resource(GameDatabase::new(temp_database()));
    let walker = spawn_saved_player(&mut app, "Walker", 0, 25.0);
    let ghost = spawn_saved_player(&mut app, "Spirit", 1, -40.0);
    let corpse = shared::death::Corpse {
        player: ghost,
        map_id: 1,
        x: 1.0,
        y: 2.0,
        z: 3.0,
        created_at: 1234,
    };
    let ghost_entity = app
        .world_mut()
        .query::<(Entity, &ClientId)>()
        .iter(app.world())
        .find(|(_, id)| id.0 == ghost)
        .map(|(entity, _)| entity)
        .unwrap();
    app.world_mut()
        .entity_mut(ghost_entity)
        .insert(crate::death::PlayerDeathState {
            state: shared::death::DeathState::Ghost,
            corpse: Some(corpse.clone()),
            graveyard: None,
        });
    for _ in 0..300 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    let saved_db = app
        .world_mut()
        .remove_resource::<GameDatabase>()
        .unwrap()
        .into_database();
    let db = &saved_db;
    assert_eq!(stored_position(db, walker).0, 25.0);
    assert_eq!(stored_position(db, ghost).0, -40.0);
    assert_eq!(crate::maps::persistence::load_character_map(db, walker), 0);
    assert_eq!(crate::maps::persistence::load_character_map(db, ghost), 1);
    assert!(load_corpse(db, walker).is_none());
    let saved = load_corpse(db, ghost).expect("ghost keeps its corpse");
    assert_eq!(
        (saved.map_id, saved.x, saved.y, saved.z, saved.created_at),
        (1, 1.0, 2.0, 3.0, 1234)
    );
}

fn peacebloom(item_guid: u64) -> InventoryItemData {
    InventoryItemData {
        item_guid,
        item_id: 2447,
        name: "Peacebloom".into(),
        quality: 1,
        required_level: 0,
        stack_count: 5,
        vendor_sell_price: 5,
        unique_auction: false,
    }
}

#[test]
fn legacy_u32_money_auctions_and_auction_mail_move_to_u64_tables() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    let auction = LegacyAuctionDataU32Money {
        auction_id: 9,
        seller_character_id: 42,
        seller_name: "Seller".into(),
        item: peacebloom(70),
        stack_count: 5,
        min_bid: 1_000,
        current_bid: Some(4_000_000_000),
        current_bidder_character_id: Some(43),
        buyout_price: Some(4_294_967_295),
        expires_at_unix_secs: 1_800_000_000,
        faction: 1,
        deposit: 60,
    };
    let mails = vec![mail_store::LegacyAuctionMail {
        mail_id: 3,
        subject: "Auction won".into(),
        body: "You won Peacebloom.".into(),
        attached_money: 4_000_000_000,
        attached_item: Some(peacebloom(71)),
    }];
    {
        let txn = db.begin_write().unwrap();
        {
            let mut table = txn.open_table(LEGACY_AUCTIONS).unwrap();
            table
                .insert(9, bitcode::encode(&auction).as_slice())
                .unwrap();
            let mut table = txn.open_table(mail_store::LEGACY_MAILBOXES).unwrap();
            table
                .insert(43, bitcode::encode(&mails).as_slice())
                .unwrap();
        }
        txn.commit().unwrap();
    }

    create_tables(&db);

    let auctions = load_auctions(&db);
    assert_eq!(auctions.len(), 1);
    assert_eq!(auctions[0].current_bid, Some(4_000_000_000));
    assert_eq!(auctions[0].buyout_price, Some(4_294_967_295));
    assert_eq!(auctions[0].deposit, 60);
    assert_eq!(auctions[0].item, peacebloom(70));
    let mailbox = mail_store::load_mailbox(&db, 43);
    assert_eq!(mailbox.len(), 1);
    assert_eq!(mailbox[0].sender, mail_store::MailSender::AuctionHouse);
    assert_eq!(mailbox[0].subject, "Auction won");
    assert_eq!(mailbox[0].money, 4_000_000_000);
    assert_eq!(mailbox[0].items[0].item, peacebloom(71));
    assert_eq!(
        mailbox[0].expires_at,
        mailbox[0].delivers_at + shared::mail::MAIL_EXPIRY_SECS
    );
    let txn = db.begin_read().unwrap();
    assert!(txn.open_table(LEGACY_AUCTIONS).is_err());
    assert!(txn.open_table(mail_store::LEGACY_MAILBOXES).is_err());
    drop(txn);
    // A second startup keeps the migrated rows.
    create_tables(&db);
    assert_eq!(load_auctions(&db)[0].min_bid, 1_000);
}
