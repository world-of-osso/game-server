use super::*;
use super::{AUCTIONS, INVENTORIES, MAILBOXES, SERVER_SETTINGS};
use redb::ReadableDatabase;
use shared::components::{CharacterAppearance, EquipmentAppearance};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// SQLite helpers
// ---------------------------------------------------------------------------

fn create_world_schema(conn: &rusqlite::Connection) {
    conn.execute_batch(
        "CREATE TABLE creature_template (
            entry INTEGER PRIMARY KEY, name TEXT, subname TEXT,
            minlevel INTEGER, maxlevel INTEGER, faction INTEGER, npcflag INTEGER,
            speed_walk REAL, speed_run REAL, scale REAL, rank INTEGER,
            unit_class INTEGER, unit_flags INTEGER, type INTEGER,
            HealthModifier REAL, ManaModifier REAL, AIName TEXT,
            MovementType INTEGER, flags_extra INTEGER
        );
        CREATE TABLE creature_template_model (
            CreatureID INTEGER, Idx INTEGER, CreatureDisplayID INTEGER,
            DisplayScale REAL, Probability REAL
        );
        CREATE TABLE creature (
            guid INTEGER PRIMARY KEY, id1 INTEGER, map INTEGER,
            zoneId INTEGER, areaId INTEGER,
            position_x REAL, position_y REAL, position_z REAL, orientation REAL,
            spawntimesecs INTEGER, wander_distance REAL, MovementType INTEGER
        );
        CREATE TABLE waypoint_data (
            id INTEGER, point INTEGER, position_x REAL, position_y REAL, position_z REAL,
            orientation REAL, delay INTEGER, move_type INTEGER
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
            "INSERT INTO creature_template VALUES
                (1,'Wolf',NULL,3,4,35,0, 1.0,1.4,1.0,0,1,0,1,1.0,1.0,'',1,0),
                (2,'Boar',NULL,2,3,35,0, 1.0,1.2,1.0,0,1,0,1,1.0,1.0,'',0,0);
             INSERT INTO creature_template_model VALUES (1,0,100,1.0,1.0),(2,0,200,1.0,1.0);
             INSERT INTO creature VALUES (10,1,0,1,1, 0.0,0.0,0.0,0.0, 300,5.0,1);
             INSERT INTO creature VALUES (11,2,1,2,2, 10.0,20.0,30.0,0.0, 300,0.0,0);
             INSERT INTO waypoint_data VALUES (10,1, 1.0,2.0,3.0,0.0,0,0);
             INSERT INTO waypoint_data VALUES (10,2, 4.0,5.0,6.0,0.0,0,0);",
        )
        .unwrap();
    });

    let world = crate::world_data::load_world_data(&path);

    assert_eq!(world.templates.len(), 2);
    assert_eq!(world.spawns.len(), 2);
    assert!(world.templates.contains_key(&1));
    assert!(world.templates.contains_key(&2));

    let wolf = &world.templates[&1];
    assert_eq!(wolf.min_level, 3);
    assert_eq!(wolf.max_level, 4);

    let waypoints = &world.waypoints[&10];
    assert_eq!(waypoints.points.len(), 2);
    assert_eq!(waypoints.points[0].position_x, 1.0);
    assert_eq!(waypoints.points[1].position_x, 4.0);
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
            "INSERT INTO creature_template VALUES
                (1,'Wolf',NULL,1,1,35,0,1.0,1.0,1.0,0,1,0,1,1.0,1.0,'',0,0);
             INSERT INTO creature VALUES (1,1,0,1,1, 0.0,0.0,0.0,0.0, 300,0.0,0);
             INSERT INTO creature VALUES (2,1,0,1,1, 1.0,0.0,0.0,0.0, 300,0.0,0);
             INSERT INTO creature VALUES (3,1,1,1,1, 2.0,0.0,0.0,0.0, 300,0.0,0);",
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
            "INSERT INTO creature_template VALUES
                (5,'Demon',NULL,10,10,35,0,1.0,1.5,1.0,0,1,0,1,2.0,1.0,'',0,0);
             INSERT INTO creature_template_model VALUES (5,0,301,1.0,0.7);
             INSERT INTO creature_template_model VALUES (5,1,302,1.0,0.3);",
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
fn world_data_waypoints_grouped_by_creature() {
    let path = write_to_temp_sqlite(|conn| {
        conn.execute_batch(
            "INSERT INTO waypoint_data VALUES (1,1, 0.0,0.0,0.0,0.0,500,0);
             INSERT INTO waypoint_data VALUES (1,2, 1.0,1.0,1.0,0.0,500,0);
             INSERT INTO waypoint_data VALUES (1,3, 2.0,2.0,2.0,0.0,500,0);
             INSERT INTO waypoint_data VALUES (2,1, 5.0,5.0,5.0,0.0,300,0);
             INSERT INTO waypoint_data VALUES (2,2, 6.0,6.0,6.0,0.0,300,0);",
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
    assert_eq!(motd, shared::motd::Motd::default());
    assert!(motd.message.is_empty());
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
    // default_inventory gives 3 starter items and 100_000 gold
    assert_eq!(inventory.gold, 100_000);
    assert_eq!(inventory.items.len(), 3);
}

#[test]
fn mailbox_empty_on_missing() {
    let db = temp_database();
    let mails = load_mailbox(&db, 9999);
    assert!(mails.is_empty());
}

#[test]
fn mailbox_append() {
    let db = temp_database();
    let mail1 = MailEntryData {
        mail_id: 1,
        subject: "Hello".to_string(),
        body: "First message".to_string(),
        attached_money: 0,
        attached_item: None,
    };
    let mail2 = MailEntryData {
        mail_id: 2,
        subject: "Package".to_string(),
        body: "Here's some gold".to_string(),
        attached_money: 1000,
        attached_item: Some(InventoryItemData {
            item_guid: 5,
            item_id: 2447,
            name: "Peacebloom".to_string(),
            quality: 1,
            required_level: 1,
            stack_count: 5,
            vendor_sell_price: 10,
            unique_auction: false,
        }),
    };
    append_mail(&db, 7, mail1.clone());
    append_mail(&db, 7, mail2.clone());
    let mails = load_mailbox(&db, 7);
    assert_eq!(mails.len(), 2);
    assert_eq!(mails[0], mail1);
    assert_eq!(mails[1], mail2);
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
        },
        equipment_appearance: EquipmentAppearance::default(),
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
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db")
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
            "SELECT name FROM creature_template WHERE entry = 448",
            [],
            |r| r.get(0),
        )
        .expect("Hogger (entry 448) missing from creature_template");
    assert_eq!(name, "Hogger");
}

#[test]
#[ignore]
fn world_db_hogger_has_spawn() {
    let conn = world_db_conn();
    let count: u64 = conn
        .query_row("SELECT COUNT(*) FROM creature WHERE id1 = 448", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(count >= 1, "Hogger has no spawns in creature table");
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
            "SELECT COUNT(*) FROM creature_template WHERE name = '' OR name IS NULL",
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
            "SELECT COUNT(*) FROM creature c
             WHERE NOT EXISTS (SELECT 1 FROM creature_template t WHERE t.entry = c.id1)",
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
                FROM waypoint_data
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
        .query_row("SELECT COUNT(DISTINCT map) FROM creature", [], |r| r.get(0))
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
        "spell_bonus_data",
        "spell_dbc",
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
fn world_db_spell_dbc_row_count() {
    let conn = world_db_conn();
    let count = table_count(&conn, "spell_dbc");
    // AzerothCore spell_dbc subset has ~4,400+ spell entries
    assert!(
        count >= 4_000,
        "spell_dbc has {count} rows, expected >= 4,000"
    );
}

#[test]
#[ignore]
fn world_db_spell_bonus_data_row_count() {
    let conn = world_db_conn();
    let count = table_count(&conn, "spell_bonus_data");
    // AzerothCore spell_bonus_data has ~1,200+ coefficient entries
    assert!(
        count >= 1_000,
        "spell_bonus_data has {count} rows, expected >= 1,000"
    );
}

#[test]
#[ignore]
fn world_db_spell_dbc_has_known_spells() {
    let conn = world_db_conn();
    // Verify at least one well-known spell exists
    let name: String = conn
        .query_row(
            "SELECT Name_Lang_enUS FROM spell_dbc WHERE ID = 84",
            [],
            |r| r.get(0),
        )
        .expect("spell ID 84 missing from spell_dbc");
    assert_eq!(name, "Low Health");
}

#[test]
#[ignore]
fn world_db_spell_bonus_data_has_coefficients() {
    let conn = world_db_conn();
    // Verify at least one entry has non-zero coefficients
    let count: u64 = conn
        .query_row(
            "SELECT COUNT(*) FROM spell_bonus_data
             WHERE direct_bonus != 0 OR dot_bonus != 0 OR ap_bonus != 0 OR ap_dot_bonus != 0",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        count >= 50,
        "only {count} spell_bonus_data entries with non-zero coefficients"
    );
}

#[test]
#[ignore]
fn world_db_loads_spells_through_spell_database() {
    let conn = world_db_conn();
    let db = crate::spell_db::load_spell_database(&conn);
    assert!(!db.is_empty(), "no spells loaded");
    assert!(db.len() >= 4_000, "only {} spells loaded", db.len());
    // Verify a known spell loaded
    let spell = db.get(84).expect("spell 84 not in SpellDatabase");
    assert_eq!(spell.name, "Low Health");
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
    assert!(hogger.min_level >= 10);
    assert!(!hogger.models.is_empty(), "Hogger has no models");
}
