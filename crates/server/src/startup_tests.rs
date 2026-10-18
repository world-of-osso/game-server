use super::mail_store::MAILBOXES;
use super::*;
use super::{AUCTIONS, INVENTORIES, SERVER_SETTINGS};
use redb::ReadableDatabase;
use shared::components::{CharacterAppearance, EquipmentAppearance};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// SQLite helpers
// ---------------------------------------------------------------------------

fn create_world_schema(conn: &rusqlite::Connection) {
    conn.execute_batch(
        "CREATE TABLE zephras_row_source (table_name TEXT, row_key TEXT, kind TEXT);
        CREATE TABLE content_creature_template (
            entry INTEGER PRIMARY KEY, name TEXT, subname TEXT,
            minlevel INTEGER, maxlevel INTEGER, faction INTEGER, npcflag INTEGER,
            speed_walk REAL, speed_run REAL, scale REAL, rank INTEGER,
            unit_class INTEGER, unit_flags INTEGER, type INTEGER,
            HealthModifier REAL, ManaModifier REAL, AIName TEXT,
            MovementType INTEGER, flags_extra INTEGER, exp INTEGER, ExperienceModifier REAL,
            ScriptName TEXT NOT NULL DEFAULT '',
            detection_range REAL,
            ContentTuningID INTEGER NOT NULL DEFAULT 0, HealthScalingExpansion INTEGER NOT NULL DEFAULT 0,
            LevelScalingDeltaMin INTEGER NOT NULL DEFAULT 0, LevelScalingDeltaMax INTEGER NOT NULL DEFAULT 0,
            DamageModifier REAL NOT NULL DEFAULT 1, ArmorModifier REAL NOT NULL DEFAULT 1,
            unit_flags2 INTEGER NOT NULL DEFAULT 0, RegenHealth INTEGER NOT NULL DEFAULT 1,
            family INTEGER NOT NULL DEFAULT 0, type_flags INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE content_tuning_level (ID INTEGER PRIMARY KEY, minlevel INTEGER, maxlevel INTEGER);
        CREATE TABLE map_difficulty (MapID INTEGER, DifficultyID INTEGER, ContentTuningID INTEGER);
        CREATE TABLE area_table (ID INTEGER PRIMARY KEY, ParentAreaID INTEGER, ContentTuningID INTEGER);
        CREATE TABLE expected_stat (
            ID INTEGER PRIMARY KEY, ExpansionID INTEGER, CreatureHealth REAL,
            CreatureAutoAttackDps REAL, CreatureArmor REAL, ArmorConstant REAL, Lvl INTEGER
        );
        CREATE TABLE expected_stat_mod (
            ID INTEGER PRIMARY KEY, CreatureHealthMod REAL, CreatureAutoAttackDPSMod REAL,
            CreatureArmorMod REAL, ArmorConstantMod REAL
        );
        CREATE TABLE content_tuning_x_expected (
            ID INTEGER PRIMARY KEY, ExpectedStatModID INTEGER, MinMythicPlusSeasonID INTEGER,
            ContentTuningID INTEGER
        );
        CREATE TABLE content_creature_template_difficulty (entry INTEGER NOT NULL,
            DifficultyID INTEGER NOT NULL, minlevel INTEGER NOT NULL, maxlevel INTEGER NOT NULL, exp INTEGER NOT NULL,
            ContentTuningID INTEGER NOT NULL, HealthScalingExpansion INTEGER NOT NULL, LevelScalingDeltaMin INTEGER NOT NULL,
            LevelScalingDeltaMax INTEGER NOT NULL, HealthModifier REAL NOT NULL, ManaModifier REAL NOT NULL,
            ArmorModifier REAL NOT NULL, DamageModifier REAL NOT NULL, lootid INTEGER NOT NULL DEFAULT 0,
            mingold INTEGER NOT NULL DEFAULT 0, maxgold INTEGER NOT NULL DEFAULT 0, PRIMARY KEY (entry, DifficultyID));
        CREATE TABLE content_creature_template_model (
            CreatureID INTEGER, Idx INTEGER, CreatureDisplayID INTEGER,
            DisplayScale REAL, Probability REAL
        );
        CREATE TABLE content_creature (
            guid INTEGER PRIMARY KEY, id1 INTEGER, map INTEGER,
            zoneId INTEGER, areaId INTEGER,
            position_x REAL, position_y REAL, position_z REAL, orientation REAL,
            spawntimesecs INTEGER, wander_distance REAL, MovementType INTEGER,
            modelid INTEGER NOT NULL DEFAULT 0, unit_flags INTEGER NOT NULL DEFAULT 0,
            spawnDifficulties TEXT NOT NULL DEFAULT '0', equipment_id INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE content_creature_equip_template (
            CreatureID INTEGER, ID INTEGER, ItemID1 INTEGER, AppearanceModID1 INTEGER,
            ItemID2 INTEGER, AppearanceModID2 INTEGER, ItemID3 INTEGER, AppearanceModID3 INTEGER
        );
        CREATE TABLE item (ID INTEGER PRIMARY KEY, InventoryType INTEGER);
        CREATE TABLE item_modified_appearance (
            ItemID INTEGER, ItemAppearanceModifierID INTEGER, ItemAppearanceID INTEGER
        );
        CREATE TABLE item_appearance (ID INTEGER PRIMARY KEY, ItemDisplayInfoID INTEGER);
        CREATE TABLE creature_display_info (ID INTEGER PRIMARY KEY, ModelID INTEGER);
        CREATE TABLE creature_model_data (ID INTEGER PRIMARY KEY, FileDataID INTEGER);
        CREATE TABLE content_game_event_creature (
            eventEntry INTEGER NOT NULL, guid INTEGER NOT NULL,
            PRIMARY KEY (guid, eventEntry)
        );
        CREATE TABLE content_waypoint_data (
            id INTEGER, point INTEGER, position_x REAL, position_y REAL, position_z REAL,
            orientation REAL, delay INTEGER, move_type INTEGER
        );
        CREATE TABLE content_creature_addon (
            guid INTEGER PRIMARY KEY, path_id INTEGER, mount INTEGER, bytes1 INTEGER,
            bytes2 INTEGER, emote INTEGER, visibilityDistanceType INTEGER, auras TEXT
        );
        CREATE TABLE content_creature_template_addon (
            entry INTEGER PRIMARY KEY, path_id INTEGER, mount INTEGER, bytes1 INTEGER,
            bytes2 INTEGER, emote INTEGER, visibilityDistanceType INTEGER, auras TEXT
        );
        CREATE TABLE content_creature_formations (
            leaderGUID INTEGER, memberGUID INTEGER PRIMARY KEY, dist REAL, angle REAL,
            groupAI INTEGER, point_1 INTEGER, point_2 INTEGER
        );
        CREATE TABLE content_pool_template (entry INTEGER NOT NULL PRIMARY KEY,
            max_limit INTEGER NOT NULL, description TEXT);
        CREATE TABLE content_pool_members (type INTEGER NOT NULL, spawnId INTEGER NOT NULL,
            poolSpawnId INTEGER NOT NULL, chance REAL NOT NULL, description TEXT,
            PRIMARY KEY (type, spawnId));
        CREATE TABLE content_gameobject (guid INTEGER PRIMARY KEY, map INTEGER);
        CREATE TABLE tdb_creature_template (entry INTEGER PRIMARY KEY, VignetteID INTEGER,
            StringId TEXT);
        CREATE TABLE tdb_creature (guid INTEGER PRIMARY KEY, id INTEGER, StringId TEXT);
        CREATE TABLE tdb_conditions (SourceTypeOrReferenceId INTEGER, SourceEntry INTEGER,
            ElseGroup INTEGER, ConditionTypeOrReference INTEGER, ConditionValue1 INTEGER,
            ConditionValue2 INTEGER, ConditionValue3 INTEGER, NegativeCondition INTEGER);
        CREATE TABLE tdb_spawn_group (groupId INTEGER, spawnType INTEGER, spawnId INTEGER);
        CREATE TABLE tdb_spawn_group_template (groupId INTEGER PRIMARY KEY, groupName TEXT,
            groupFlags INTEGER);
        CREATE TABLE vignette (ID INTEGER PRIMARY KEY, Flags INTEGER);
        CREATE TABLE content_game_event_pool (eventEntry INTEGER NOT NULL,
            pool_entry INTEGER NOT NULL PRIMARY KEY);
        CREATE TABLE creature_classlevelstats (
            level INTEGER, class INTEGER, basehp0 INTEGER, basehp1 INTEGER, basehp2 INTEGER,
            basemana INTEGER NOT NULL DEFAULT 0
        );",
    )
    .expect("create world schema");
}

fn write_to_temp_sqlite(populate: impl FnOnce(&rusqlite::Connection)) -> PathBuf {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("world_test.db");
    let conn = rusqlite::Connection::open(&path).unwrap();
    create_world_schema(&conn);
    populate(&conn);
    // Keep the tempdir alive by leaking it (same pattern as temp_database).
    std::mem::forget(dir);
    path
}

// ---------------------------------------------------------------------------
// 1. World data loading tests
// ---------------------------------------------------------------------------

#[test]
fn world_data_loads_from_sqlite() {
    let path = write_to_temp_sqlite(|conn| {
        conn.execute_batch(
            "INSERT INTO content_creature_template VALUES
                (1,'Wolf',NULL,3,4,35,0, 1.0,1.4,1.0,0,1,0,1,1.0,1.0,'',1,0,0,1.0,'',20.0,0,0,0,0,1.0,1.0,0,1,0,0),
                (2,'Boar',NULL,2,3,35,0, 1.0,1.2,1.0,0,1,0,1,1.0,1.0,'',0,0,0,1.0,'',20.0,0,0,0,0,1.0,1.0,0,1,0,0);
             INSERT INTO content_creature_template_model VALUES (1,0,100,1.0,1.0),(2,0,200,1.0,1.0);
             INSERT INTO content_creature (guid, id1, map, zoneId, areaId, position_x, position_y,
                position_z, orientation, spawntimesecs, wander_distance, MovementType) VALUES (10,1,0,1,1, 0.0,0.0,0.0,0.0, 300,5.0,1);
             INSERT INTO content_creature (guid, id1, map, zoneId, areaId, position_x, position_y,
                position_z, orientation, spawntimesecs, wander_distance, MovementType) VALUES (11,2,1,2,2, 10.0,20.0,30.0,0.0, 300,0.0,0);
             INSERT INTO content_waypoint_data VALUES (10,1, 1.0,2.0,3.0,0.0,0,0);
             INSERT INTO content_waypoint_data VALUES (10,2, 4.0,5.0,6.0,0.0,0,0);",
        )
        .unwrap();
    });

    let world = crate::world_data::load_world_data(&path);

    assert_eq!(world.templates.len(), 2);
    assert_eq!(world.spawns.len(), 2);
    assert!(world.templates.contains_key(&1));
    assert!(world.templates.contains_key(&2));

    let wolf = &world.templates[&1];
    assert_eq!(wolf.name, "Wolf");
    assert_eq!(wolf.min_level, 3);
    assert_eq!(wolf.max_level, 4);

    let waypoints = &world.waypoints[&10];
    assert_eq!(waypoints.points.len(), 2);
    assert_eq!(waypoints.points[0].position_x, 1.0);
    assert_eq!(waypoints.points[1].position_x, 4.0);
}

#[test]
fn world_data_excludes_inactive_tournament_spawns_without_lowering_ordinary_npcs() {
    let path = write_to_temp_sqlite(|conn| {
        conn.execute_batch(
            "INSERT INTO content_creature (guid, id1, map, zoneId, areaId, position_x, position_y,
                position_z, orientation, spawntimesecs, wander_distance, MovementType) VALUES
                (96194,26012,0,0,0,-8918.51,-135.146,84.2136,0,180,0,0),
                (96198,26760,0,0,0,-8914.73,-133.636,84.3208,0,180,0,0),
                (96204,26075,0,0,0,-8911.02,-131.912,84.4265,0,180,0,0),
                (79968,952,0,0,0,-8902.13,-181.646,113.24,0,430,0,0),
                (76,1,0,0,0,10,20,30,0,180,0,0),
                (77,1,0,0,0,40,50,60,0,180,0,0);
             INSERT INTO content_game_event_creature VALUES
                (31,96194),(31,96198),(31,96204),(-72,76),(-72,77),(31,77);",
        )
        .unwrap();
    });
    let world = crate::world_data::load_world_data(&path);
    let mut guids = world
        .spawns
        .iter()
        .map(|spawn| spawn.guid)
        .collect::<Vec<_>>();
    guids.sort_unstable();
    assert_eq!(guids, vec![76, 77, 79968]);
    let brother_neals = world
        .spawns
        .iter()
        .find(|spawn| spawn.guid == 79968)
        .unwrap();
    assert_eq!(brother_neals.position_z, 113.24);
    assert_eq!(world.spawns_by_map[&0].len(), 3);
    let conn = rusqlite::Connection::open(&path).unwrap();
    let stored_spawns: i64 = conn
        .query_row("SELECT count(*) FROM content_creature", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(
        stored_spawns, 6,
        "event filtering must preserve source spawn rows"
    );
}

#[test]
fn world_data_empty_tables() {
    let path = write_to_temp_sqlite(|_| {});
    let world = crate::world_data::load_world_data(&path);
    assert_eq!(world.templates.len(), 0);
    assert_eq!(world.spawns.len(), 0);
    assert_eq!(world.spawns_by_map.len(), 0);
    assert_eq!(world.waypoints.len(), 0);
}

#[test]
fn world_data_spawns_indexed_by_map() {
    let path = write_to_temp_sqlite(|conn| {
        conn.execute_batch(
            "INSERT INTO content_creature_template VALUES
                (1,'Wolf',NULL,1,1,35,0,1.0,1.0,1.0,0,1,0,1,1.0,1.0,'',0,0,0,1.0,'',20.0,0,0,0,0,1.0,1.0,0,1,0,0);
             INSERT INTO content_creature (guid, id1, map, zoneId, areaId, position_x, position_y,
                position_z, orientation, spawntimesecs, wander_distance, MovementType) VALUES (1,1,0,1,1, 0.0,0.0,0.0,0.0, 300,0.0,0);
             INSERT INTO content_creature (guid, id1, map, zoneId, areaId, position_x, position_y,
                position_z, orientation, spawntimesecs, wander_distance, MovementType) VALUES (2,1,0,1,1, 1.0,0.0,0.0,0.0, 300,0.0,0);
             INSERT INTO content_creature (guid, id1, map, zoneId, areaId, position_x, position_y,
                position_z, orientation, spawntimesecs, wander_distance, MovementType) VALUES (3,1,1,1,1, 2.0,0.0,0.0,0.0, 300,0.0,0);",
        )
        .unwrap();
    });

    let world = crate::world_data::load_world_data(&path);

    assert_eq!(world.spawns_by_map[&0].len(), 2);
    assert_eq!(world.spawns_by_map[&1].len(), 1);

    // Verify indices actually point into the right spawns.
    for &idx in &world.spawns_by_map[&0] {
        assert_eq!(world.spawns[idx].map, 0);
    }
    for &idx in &world.spawns_by_map[&1] {
        assert_eq!(world.spawns[idx].map, 1);
    }
}

#[test]
fn world_data_models_attached_to_templates() {
    let path = write_to_temp_sqlite(|conn| {
        conn.execute_batch(
            "INSERT INTO content_creature_template VALUES
                (5,'Demon',NULL,10,10,35,0,1.0,1.5,1.0,0,1,0,1,2.0,1.0,'',0,0,0,1.0,'',20.0,0,0,0,0,1.0,1.0,0,1,0,0);
             INSERT INTO content_creature_template_model VALUES (5,0,301,1.0,0.7);
             INSERT INTO content_creature_template_model VALUES (5,1,302,1.0,0.3);",
        )
        .unwrap();
    });

    let world = crate::world_data::load_world_data(&path);
    let demon = &world.templates[&5];
    assert_eq!(demon.models.len(), 2);
    assert_eq!(demon.models[0].display_id, 301);
    assert_eq!(demon.models[1].display_id, 302);
}

#[test]
fn world_data_waypoints_grouped_by_path() {
    let path = write_to_temp_sqlite(|conn| {
        conn.execute_batch(
            "INSERT INTO content_waypoint_data VALUES (1,1, 0.0,0.0,0.0,0.0,500,0);
             INSERT INTO content_waypoint_data VALUES (1,2, 1.0,1.0,1.0,0.0,500,0);
             INSERT INTO content_waypoint_data VALUES (1,3, 2.0,2.0,2.0,0.0,500,0);
             INSERT INTO content_waypoint_data VALUES (2,1, 5.0,5.0,5.0,0.0,300,0);
             INSERT INTO content_waypoint_data VALUES (2,2, 6.0,6.0,6.0,0.0,300,0);",
        )
        .unwrap();
    });

    let world = crate::world_data::load_world_data(&path);

    let path1 = &world.waypoints[&1];
    assert_eq!(path1.points.len(), 3);
    assert_eq!(path1.points[0].position_x, 0.0);
    assert_eq!(path1.points[2].position_x, 2.0);

    let path2 = &world.waypoints[&2];
    assert_eq!(path2.points.len(), 2);
    assert_eq!(path2.points[0].position_x, 5.0);
}

// ---------------------------------------------------------------------------
// 2. Persistence layer tests
// ---------------------------------------------------------------------------

#[test]
fn create_tables_idempotent() {
    let db = temp_database();
    // A second call should not panic.
    create_tables(&db);
}

#[test]
fn all_14_tables_exist() {
    let db = temp_database();
    let txn = db.begin_read().expect("begin read");
    txn.open_table(CHARACTERS).expect("CHARACTERS missing");
    txn.open_table(INVENTORIES).expect("INVENTORIES missing");
    txn.open_table(AUCTIONS).expect("AUCTIONS missing");
    txn.open_table(MAILBOXES).expect("MAILBOXES missing");
    txn.open_table(ACCOUNTS).expect("ACCOUNTS missing");
    txn.open_table(ACCOUNT_CHARACTERS)
        .expect("ACCOUNT_CHARACTERS missing");
    txn.open_table(CHARACTER_NAMES)
        .expect("CHARACTER_NAMES missing");
    txn.open_table(CHARACTER_GUILDS)
        .expect("CHARACTER_GUILDS missing");
    txn.open_table(PENDING_GUILD_INVITES)
        .expect("PENDING_GUILD_INVITES missing");
    txn.open_table(PASSWORDS).expect("PASSWORDS missing");
    txn.open_table(USERNAME_ACCOUNTS)
        .expect("USERNAME_ACCOUNTS missing");
    txn.open_table(PENDING_REGISTRATIONS)
        .expect("PENDING_REGISTRATIONS missing");
    txn.open_table(NEXT_IDS).expect("NEXT_IDS missing");
    txn.open_table(SERVER_SETTINGS)
        .expect("SERVER_SETTINGS missing");
}

#[test]
fn next_id_starts_at_one() {
    let db = temp_database();
    assert_eq!(next_id(&db, "test"), 1);
}

#[test]
fn next_id_increments() {
    let db = temp_database();
    assert_eq!(next_id(&db, "test"), 1);
    assert_eq!(next_id(&db, "test"), 2);
    assert_eq!(next_id(&db, "test"), 3);
}

#[test]
fn next_id_independent_keys() {
    let db = temp_database();
    assert_eq!(next_id(&db, "characters"), 1);
    assert_eq!(next_id(&db, "accounts"), 1);
    assert_eq!(next_id(&db, "characters"), 2);
    assert_eq!(next_id(&db, "accounts"), 2);
}

#[test]
fn motd_round_trip() {
    let db = temp_database();
    let mut motd = shared::motd::Motd::default();
    motd.set("Welcome to World of Osso!", 1_000_000).unwrap();
    save_motd(&db, &motd);
    let loaded = load_motd(&db);
    assert_eq!(loaded, motd);
    assert_eq!(loaded.message, "Welcome to World of Osso!");
    assert_eq!(loaded.updated_at, 1_000_000);
}

#[test]
fn motd_default_when_missing() {
    let db = temp_database();
    let motd = load_motd(&db);
    assert_eq!(motd.message, "Welcome to World of Osso.");
}

#[test]
fn inventory_round_trip() {
    let db = temp_database();
    let inventory = InventoryData {
        gold: 50_000,
        items: vec![InventoryItemData {
            item_guid: 1,
            item_id: 2589,
            name: "Linen Cloth".to_string(),
            quality: 1,
            required_level: 1,
            stack_count: 20,
            vendor_sell_price: 13,
            unique_auction: false,
        }],
    };
    save_inventory(&db, 42, &inventory);
    let loaded = load_inventory(&db, 42);
    assert_eq!(loaded, inventory);
}

#[test]
fn inventory_default_on_missing() {
    let db = temp_database();
    let inventory = load_inventory(&db, 9999);
    assert_eq!(inventory.gold, 0);
    assert!(inventory.items.is_empty());
}

#[test]
fn auction_save_load_delete() {
    let db = temp_database();
    let auction = AuctionData {
        auction_id: 100,
        seller_character_id: 1,
        seller_name: "Trader".to_string(),
        item: InventoryItemData {
            item_guid: 10,
            item_id: 2770,
            name: "Copper Ore".to_string(),
            quality: 1,
            required_level: 1,
            stack_count: 10,
            vendor_sell_price: 25,
            unique_auction: false,
        },
        stack_count: 10,
        min_bid: 500,
        current_bid: None,
        current_bidder_character_id: None,
        buyout_price: Some(1000),
        expires_at_unix_secs: 1_700_000_000,
        faction: 1,
        deposit: 50,
    };
    save_auction(&db, &auction);
    let loaded = load_auctions(&db);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0], auction);

    delete_auction(&db, 100);
    let after_delete = load_auctions(&db);
    assert!(after_delete.is_empty());
}

// ---------------------------------------------------------------------------
// 4. Shutdown / data integrity tests
// ---------------------------------------------------------------------------

fn full_character_data() -> CharacterData {
    CharacterData {
        name: "Azeroth".to_string(),
        x: -8949.0,
        y: -132.0,
        z: 83.0,
        health: 95.5,
        max_health: 100.0,
        race: 1,
        class: 2,
        level: 60,
        appearance: CharacterAppearance {
            sex: 1,
            skin_color: 3,
            face: 5,
            eye_color: 2,
            hair_style: 4,
            hair_color: 6,
            facial_style: 1,
            customization_choices: Vec::new(),
            visage: None,
        },
        equipment_appearance: EquipmentAppearance::default(),
        durability: Vec::new(),
    }
}

#[test]
fn character_save_load_round_trip_with_all_fields() {
    let db = temp_database();
    let data = full_character_data();
    save_character(&db, 1, &data);
    let loaded = load_character(&db, 1).expect("character should be present");
    assert_eq!(loaded, data);
}

#[test]
fn multiple_characters_independent() {
    let db = temp_database();
    let mut data1 = full_character_data();
    data1.name = "Alpha".to_string();
    data1.level = 10;

    let mut data2 = full_character_data();
    data2.name = "Beta".to_string();
    data2.level = 20;

    save_character(&db, 1, &data1);
    save_character(&db, 2, &data2);

    let loaded1 = load_character(&db, 1).unwrap();
    let loaded2 = load_character(&db, 2).unwrap();

    assert_eq!(loaded1.name, "Alpha");
    assert_eq!(loaded1.level, 10);
    assert_eq!(loaded2.name, "Beta");
    assert_eq!(loaded2.level, 20);
}

#[test]
fn overwrite_preserves_latest() {
    let db = temp_database();
    let mut data = full_character_data();
    save_character(&db, 1, &data);

    data.health = 1.0;
    data.x = 999.0;
    save_character(&db, 1, &data);

    let loaded = load_character(&db, 1).unwrap();
    assert_eq!(loaded.health, 1.0);
    assert_eq!(loaded.x, 999.0);
    assert_eq!(loaded.name, data.name);
}

// ---------------------------------------------------------------------------
// 5. World data import verification (requires data/world.db)
// ---------------------------------------------------------------------------

fn world_db_path() -> PathBuf {
    crate::test_data::world_db_path()
}

fn world_db_conn() -> rusqlite::Connection {
    let path = world_db_path();
    rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap_or_else(|e| panic!("Failed to open {}: {e}", path.display()))
}

fn table_count(conn: &rusqlite::Connection, table: &str) -> u64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}

#[test]
#[ignore] // requires data/world.db (not in git)
fn world_db_creature_template_row_count() {
    let conn = world_db_conn();
    let count = table_count(&conn, "creature_template");
    assert!(
        count >= 29_000,
        "creature_template has {count} rows, expected >= 29,000"
    );
}

#[test]
#[ignore]
fn world_db_creature_row_count() {
    let conn = world_db_conn();
    let count = table_count(&conn, "creature");
    assert!(
        count >= 140_000,
        "creature has {count} rows, expected >= 140,000"
    );
}

#[test]
#[ignore]
fn world_db_creature_template_model_row_count() {
    let conn = world_db_conn();
    let count = table_count(&conn, "creature_template_model");
    assert!(
        count >= 40_000,
        "creature_template_model has {count} rows, expected >= 40,000"
    );
}

#[test]
#[ignore]
fn world_db_waypoint_data_row_count() {
    let conn = world_db_conn();
    let count = table_count(&conn, "waypoint_data");
    assert!(
        count >= 140_000,
        "waypoint_data has {count} rows, expected >= 140,000"
    );
}

#[test]
#[ignore]
fn world_db_hogger_exists() {
    let conn = world_db_conn();
    let name: String = conn
        .query_row(
            "SELECT name FROM content_creature_template WHERE entry = 448",
            [],
            |r| r.get(0),
        )
        .expect("Hogger (entry 448) missing from content_creature_template");
    assert_eq!(name, "Hogger");
}

#[test]
#[ignore]
fn world_db_hogger_has_spawn() {
    let conn = world_db_conn();
    let count: u64 = conn
        .query_row(
            "SELECT COUNT(*) FROM content_creature WHERE id1 = 448",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(count >= 1, "Hogger has no spawns in content_creature");
}

#[test]
#[ignore]
fn world_db_hogger_has_model() {
    let conn = world_db_conn();
    let count: u64 = conn
        .query_row(
            "SELECT COUNT(*) FROM creature_template_model WHERE CreatureID = 448",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        count >= 1,
        "Hogger has no models in creature_template_model"
    );
}

#[test]
#[ignore]
fn world_db_templates_mostly_have_names() {
    let conn = world_db_conn();
    let nameless: u64 = conn
        .query_row(
            "SELECT COUNT(*) FROM content_creature_template WHERE name = '' OR name IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    // AzerothCore has a few placeholder entries with empty names
    assert!(
        nameless <= 5,
        "{nameless} creature templates have empty names (expected <= 5)"
    );
}

#[test]
#[ignore]
fn world_db_spawns_reference_valid_templates() {
    let conn = world_db_conn();
    let orphans: u64 = conn
        .query_row(
            "SELECT COUNT(*) FROM content_creature c
             WHERE NOT EXISTS (SELECT 1 FROM content_creature_template t WHERE t.entry = c.id1)",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        orphans, 0,
        "{orphans} creature spawns reference non-existent templates"
    );
}

#[test]
#[ignore]
fn world_db_waypoints_have_ordered_points() {
    let conn = world_db_conn();
    // Verify waypoint points within each path are in ascending order
    // (AzerothCore paths don't always start at 1, but points must be monotonic)
    let disordered: u64 = conn
        .query_row(
            "SELECT COUNT(*) FROM (
                SELECT id, point,
                       LAG(point) OVER (PARTITION BY id ORDER BY point) AS prev
                FROM content_waypoint_data
            ) WHERE prev IS NOT NULL AND point <= prev",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        disordered, 0,
        "{disordered} waypoint points are out of order within their path"
    );
}

#[test]
#[ignore]
fn world_db_multiple_maps_present() {
    let conn = world_db_conn();
    let map_count: u64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT map) FROM content_creature",
            [],
            |r| r.get(0),
        )
        .unwrap();
    // AzerothCore has spawns across Eastern Kingdoms (0), Kalimdor (1), Outland (530), Northrend (571), etc.
    assert!(
        map_count >= 4,
        "only {map_count} distinct maps, expected >= 4"
    );
}

#[test]
#[ignore]
fn world_db_has_all_expected_tables() {
    let conn = world_db_conn();
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    let required = [
        "creature",
        "creature_template",
        "creature_template_model",
        "content_creature",
        "content_creature_template",
        "content_creature_template_model",
        "content_creature_addon",
        "content_creature_template_addon",
        "content_creature_formations",
        "content_game_event_creature",
        "content_waypoint_data",
        "content_tuning_x_expected",
        "creature_display_info",
        "expected_stat",
        "expected_stat_mod",
        "spell_cast_times",
        "spell_cooldowns",
        "spell_duration",
        "spell_effect",
        "spell_misc",
        "spell_name",
        "spell_power",
        "spell_range",
        "waypoint_data",
    ];
    for name in required {
        assert!(
            tables.contains(&name.to_string()),
            "missing table: {name} (found: {tables:?})"
        );
    }
}

#[test]
#[ignore]
fn world_db_spell_name_row_count() {
    let conn = world_db_conn();
    let count = table_count(&conn, "spell_name");
    assert!(
        count >= 300_000,
        "spell_name has {count} rows, expected >= 300,000"
    );
}

#[test]
#[ignore]
fn world_db_spell_effect_row_count() {
    let conn = world_db_conn();
    let count = table_count(&conn, "spell_effect");
    assert!(
        count >= 500_000,
        "spell_effect has {count} rows, expected >= 500,000"
    );
}

#[test]
#[ignore]
fn world_db_spell_has_known_spells() {
    let conn = world_db_conn();
    let name: String = conn
        .query_row(
            "SELECT Name_lang FROM spell_name WHERE ID = 12294",
            [],
            |r| r.get(0),
        )
        .expect("Mortal Strike (12294) missing from spell_name");
    assert_eq!(name, "Mortal Strike");
}

#[test]
#[ignore]
fn world_db_spell_effect_has_coefficients() {
    let conn = world_db_conn();
    let count: u64 = conn
        .query_row(
            "SELECT COUNT(*) FROM spell_effect
             WHERE BonusCoefficientFromAP != 0 OR EffectBonusCoefficient != 0",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        count >= 1_000,
        "only {count} spell_effect entries with non-zero coefficients"
    );
}

#[test]
#[ignore]
fn world_db_loads_through_world_data() {
    let path = world_db_path();
    let data = crate::world_data::load_world_data(&path);
    assert!(!data.templates.is_empty(), "no templates loaded");
    assert!(!data.spawns.is_empty(), "no spawns loaded");
    assert!(!data.spawns_by_map.is_empty(), "no map index built");
    assert!(!data.waypoints.is_empty(), "no waypoints loaded");
    let hogger = data.templates.get(&448).expect("Hogger template missing");
    // ContentTuning 73: Elwynn Forest scales 1-30.
    assert_eq!((hogger.min_level, hogger.max_level), (1, 30));
    assert!(!hogger.models.is_empty(), "Hogger has no models");
}
