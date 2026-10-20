//! Map transfers and dungeon copies on real world.db data: the Stormwind Stockade (map 34), its entrance area trigger 101 in
//! Stormwind City and its exit trigger 503.

use std::sync::OnceLock;
use std::time::Duration;

use bevy::ecs::system::RunSystemOnce;
use lightyear::connection::host::HostClient;
use lightyear::prelude::*;
use lightyear::prelude::{Connected, PeerId, RemoteId};
use rusqlite::{Connection, OpenFlags};
use shared::components::{Health, MovementControl, Player, Position, Rotation};
use shared::death::{Corpse, DeathState};
use shared::protocol::*;

use super::instances::{INSTANCE_UNLOAD_DELAY_SECS, InstanceEntrant};
use super::store::MapStore;
use super::*;
use crate::auth::{Authenticated, InWorld, SentTiles};
use crate::death::PlayerDeathState;
use crate::group::{GroupRegistry, OnlineMember, OnlineView};
use crate::networking::{CombatTarget, PlayerLink};
use crate::persistence::ClientId;

#[test]
fn world_population_expansion_maps_admit_players_without_dungeon_copies() {
    let store = map_store().expect("authentic map metadata");
    let mut manager = InstanceManager::default();
    let player = entrant(9820, None);
    for map_id in [2444, 2552, 2601, 2222, 1643, 1642, 530, 571, 646, 860] {
        assert_eq!(
            manager.player_cannot_enter(store, map_id, player, 80, SUNDAY_NOON),
            None,
            "open-world map {map_id} refused"
        );
        let resolved = manager
            .create_map(store, map_id, player, at(0.0))
            .expect("hosted open-world map");
        assert_eq!(resolved.map, InMap::continent(map_id));
        assert!(!resolved.created);
    }
    // An unselected development/unused map remains refused.
    assert!(
        manager
            .player_cannot_enter(store, 13, player, 80, SUNDAY_NOON)
            .is_some()
    );
}

#[test]
fn arena_roster_reservation_admits_only_its_own_map_copy() {
    let store = map_store().expect("pinned Map DB2");
    let mut manager = InstanceManager::default();
    let copy = manager
        .create_battleground_map(store, 572, &[1, 2, 3, 4], 4, at(0.0))
        .expect("arena reservation");
    for id in 1..=4 {
        let resolved = manager
            .create_map(store, 572, entrant(id, None), at(0.0))
            .expect("reserved arena entrant");
        assert_eq!(resolved.map, copy);
    }
    assert!(
        manager
            .create_map(store, 572, entrant(5, None), at(0.0))
            .is_err()
    );
    assert!(
        manager
            .create_map(store, 617, entrant(1, None), at(0.0))
            .is_err()
    );
}

const STOCKADE: u32 = 34;
/// `areatrigger_teleport` 101 "Stormwind Jail Entrance Target": its AreaTrigger box
/// center in Stormwind City.
const STOCKADE_ENTRANCE_TRIGGER: Vec3 = Vec3::new(-8761.85, 848.557, 87.8052);
/// `world_safe_locs` 3599 inside the Stockade.
const STOCKADE_ARRIVAL: Vec3 = Vec3::new(56.6821, 0.62376, -19.2691);
/// `world_safe_locs` 3618 "Stormwind Low Prison Exit Target".
const STOCKADE_EXIT_TARGET: Vec3 = Vec3::new(-8766.11, 845.499, 87.9952);
/// Area trigger 503's box center inside the Stockade.
const STOCKADE_EXIT_TRIGGER: Vec3 = Vec3::new(48.0937, 0.933267, -13.7769);
/// `world_safe_locs` 1854 "Stormwind Stockade, GY", the graveyard of zone 717.
const STOCKADE_GRAVEYARD: Vec3 = Vec3::new(-8786.88, 829.083, 97.6504);
/// Ratchet, Northern Barrens (Kalimdor).
const RATCHET: Vec3 = Vec3::new(-956.0, -3754.0, 5.0);

#[test]
fn housing_map_rows_keep_authored_identity_without_public_admission() {
    let store = map_store().expect("authentic world map catalog");
    let mut manager = InstanceManager::default();
    let entrant = InstanceEntrant {
        character_id: 30,
        leader: 30,
        group: None,
        difficulty_id: 0,
        ..Default::default()
    };
    let now = instances::Now {
        elapsed: 0.0,
        unix: 0,
    };
    for (id, name, wdt) in [
        (2735, "Founder's Point", 5660542),
        (2736, "Razorwind Shores", 5660543),
        (2783, "Home Interior", 5890928),
    ] {
        let entry = store.map(id).expect("housing map row must load");
        assert_eq!(entry.id, id);
        assert_eq!(entry.name, name);
        assert_eq!(entry.wdt_file_data_id, wdt);
        assert!(!entry.is_dungeon());
        assert_eq!(
            manager.create_map(store, id, entrant, now),
            Err(TransferAbortReason::MapNotAllowed)
        );
    }
}

#[test]
fn housing_copies_require_authority_and_isolate_properties_without_difficulty() {
    let store = map_store().expect("authentic world map catalog");
    let mut manager = InstanceManager::default();
    let owner = entrant(30, None);
    let visitor = entrant(31, None);
    let stranger = entrant(32, None);
    let first = manager
        .authorize_housing_entry(store, 2783, 1, owner.character_id)
        .unwrap();
    let second = manager
        .authorize_housing_entry(store, 2783, 2, visitor.character_id)
        .unwrap();
    assert_ne!(first.map, second.map);
    assert!(first.created && second.created);
    assert!(manager.copy_difficulty(first.map).is_none());
    assert!(manager.lock_copies_in_use().is_empty());
    assert_eq!(
        manager.create_map(store, 2783, owner, at(0.0)).unwrap().map,
        first.map
    );
    assert_eq!(
        manager
            .create_map(store, 2783, visitor, at(0.0))
            .unwrap()
            .map,
        second.map
    );
    assert_eq!(
        manager.create_map(store, 2783, stranger, at(0.0)),
        Err(TransferAbortReason::MapNotAllowed)
    );
    assert_eq!(
        manager.cannot_enter(first.map, stranger, SUNDAY_NOON),
        Some(TransferAbortReason::MapNotAllowed)
    );
    assert_eq!(
        manager.cannot_enter(first.map, visitor, SUNDAY_NOON),
        Some(TransferAbortReason::MapNotAllowed)
    );
    let visit = manager
        .authorize_housing_entry(store, 2783, 1, visitor.character_id)
        .unwrap();
    assert_eq!(visit.map, first.map);
    assert!(!visit.created);
    assert_eq!(manager.cannot_enter(first.map, visitor, SUNDAY_NOON), None);
    assert_eq!(
        manager.player_cannot_enter(store, 2783, owner, 80, SUNDAY_NOON),
        None
    );
    assert_eq!(
        manager.player_cannot_enter(store, 2783, stranger, 80, SUNDAY_NOON),
        Some(TransferAbortReason::MapNotAllowed)
    );
    manager.revoke_housing_entry(owner.character_id);
    assert_eq!(
        manager.create_map(store, 2783, owner, at(0.0)),
        Err(TransferAbortReason::MapNotAllowed)
    );
    assert_eq!(
        manager.authorize_housing_entry(store, STOCKADE, 1, owner.character_id),
        Err(TransferAbortReason::MapNotAllowed)
    );
}

#[test]
fn housing_wire_purchase_waits_for_arrival_and_restores_admission_after_database_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("housing.redb");
    let db = redb::Database::create(&path).unwrap();
    crate::persistence::create_tables(&db);
    crate::persistence::save_inventory(
        &db,
        30,
        &crate::persistence::InventoryData {
            gold: 20_000_000,
            items: vec![],
        },
    );
    let mut h = housing_harness(db);
    let owner = h.spawn(30, InMap::continent(0), STOCKADE_ENTRANCE_TRIGGER);
    h.app
        .world_mut()
        .entity_mut(owner.connection)
        .insert((pair::<HousingRequest>(), pair::<HousingReply>()));
    let refused = h.housing_request(&owner, HousingCommand::PurchasePlot { plot_id: 417 });
    assert!(refused.error.unwrap().contains("Enter this neighborhood"));
    let entering = h.housing_request(
        &owner,
        HousingCommand::EnterNeighborhood { neighborhood_id: 1 },
    );
    assert_eq!(entering.error, None);
    assert_eq!(entering.state.plots.len(), 1);
    assert_eq!(h.map_of(&owner), None);
    assert_eq!(h.new_worlds(&owner).last().unwrap().map_id, 2735);
    h.ack(&owner);
    let neighborhood_map = h.map_of(&owner).unwrap();
    assert_eq!(neighborhood_map.map_id, 2735);
    assert_eq!(cornerstones_in(h.app.world_mut(), neighborhood_map), 1);
    let visitor = h.spawn(31, InMap::continent(0), STOCKADE_ENTRANCE_TRIGGER);
    h.app
        .world_mut()
        .entity_mut(visitor.connection)
        .insert((pair::<HousingRequest>(), pair::<HousingReply>()));
    assert_eq!(
        h.housing_request(
            &visitor,
            HousingCommand::EnterNeighborhood { neighborhood_id: 1 }
        )
        .error,
        None
    );
    h.ack(&visitor);
    assert_eq!(h.map_of(&visitor), Some(neighborhood_map));
    assert_eq!(cornerstones_in(h.app.world_mut(), neighborhood_map), 1);
    let distant = h.housing_request(&owner, HousingCommand::PurchasePlot { plot_id: 417 });
    assert!(distant.error.unwrap().contains("cornerstone"));
    let cornerstone = Vec3::new(2942.744, -214.26563, 113.11285);
    h.walk_to(&owner, cornerstone);
    let bought = h.housing_request(&owner, HousingCommand::PurchasePlot { plot_id: 417 });
    assert_eq!(bought.error, None);
    assert_eq!(bought.state.owned_houses.len(), 1);
    assert_eq!(
        bought.state.plots[0].house_id,
        Some(bought.state.owned_houses[0].house_id)
    );
    let position = bevy(cornerstone);
    drop(h);
    let db = redb::Database::open(&path).unwrap();
    assert_eq!(crate::persistence::load_inventory(&db, 30).gold, 10_000_000);
    let mut restarted = housing_harness(db);
    let place = restarted
        .app
        .world_mut()
        .run_system_once(move |mut commands: Commands, mut maps: LoginMaps| {
            maps.resolve(
                &mut commands,
                30,
                1,
                RaidDifficulty::default(),
                2735,
                position,
            )
        })
        .unwrap()
        .expect("persisted housing admission must survive a fresh manager and reopened database");
    assert_eq!(place.map.map_id, 2735);
    assert_ne!(place.map.instance_id, 0);
    assert_eq!(place.position, position);
    assert_eq!(cornerstones_in(restarted.app.world_mut(), place.map), 1);
}

#[test]
fn housing_wire_home_map_admission_restores_visited_identity_without_component_grants() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("visited-house.redb");
    let db = redb::Database::create(&path).unwrap();
    crate::persistence::create_tables(&db);
    crate::persistence::save_inventory(
        &db,
        30,
        &crate::persistence::InventoryData {
            gold: 20_000_000,
            items: vec![],
        },
    );
    let mut h = housing_harness(db);
    let owner = h.spawn(30, InMap::continent(0), STOCKADE_ENTRANCE_TRIGGER);
    h.app
        .world_mut()
        .entity_mut(owner.connection)
        .insert((pair::<HousingRequest>(), pair::<HousingReply>()));
    assert_eq!(
        h.housing_request(
            &owner,
            HousingCommand::EnterNeighborhood { neighborhood_id: 1 }
        )
        .error,
        None
    );
    h.ack(&owner);
    h.walk_to(&owner, Vec3::new(2942.744, -214.26563, 113.11285));
    let bought = h.housing_request(&owner, HousingCommand::PurchasePlot { plot_id: 417 });
    let house = &bought.state.owned_houses[0];
    let house_id = house.house_id;
    assert!(!house.interior_grant_available && !house.exterior_grant_available);
    let entered = h.housing_request(&owner, HousingCommand::EnterHouse { house_id });
    assert_eq!(
        entered.error, None,
        "raw authored Home Interior map admission is not a component grant"
    );
    assert_eq!(h.map_of(&owner), None);
    assert_eq!(h.new_worlds(&owner).last().unwrap().map_id, 2783);
    h.ack(&owner);
    let map = h.map_of(&owner).unwrap();
    assert_eq!(map.map_id, 2783);
    let position = bevy(Vec3::new(-1000.0, -1000.0, 0.1));
    assert_eq!(
        *h.app.world().get::<Position>(owner.player).unwrap(),
        position
    );
    let layout = h.housing_request(&owner, HousingCommand::ReloadLayout { house_id });
    assert!(layout.state.layout.unwrap().can_edit);
    drop(h);
    let mut restarted = housing_harness(redb::Database::open(&path).unwrap());
    let place = restarted
        .app
        .world_mut()
        .run_system_once(move |mut commands: Commands, mut maps: LoginMaps| {
            maps.resolve(
                &mut commands,
                30,
                1,
                RaidDifficulty::default(),
                2783,
                position,
            )
        })
        .unwrap()
        .expect("persisted visited-house identity must survive restart");
    assert_eq!(place.map.map_id, 2783);
    assert_eq!(
        restarted
            .app
            .world()
            .resource::<InstanceManager>()
            .housing_property(place.map),
        Some(house_id)
    );
    assert_eq!(place.position, position);
}

#[path = "housing_dye_wire_tests.rs"]
mod housing_dye_wire_tests;
#[path = "housing_editor_tests.rs"]
mod housing_editor_tests;

fn cornerstones_in(world: &mut World, map: InMap) -> usize {
    let mut objects = world.query::<(&InMap, &GameObjectInfo, &Position)>();
    objects
        .iter(world)
        .filter(|(copy, info, position)| {
            **copy == map
                && info.entry == 527268
                && crate::game_objects::wow_position(position)
                    .distance(Vec3::new(2941.2317, -214.5868, 113.15639))
                    < 0.001
        })
        .count()
}

#[test]
fn housing_normal_bag_use_collects_the_authentic_bookcase_once_into_account_storage() {
    let dir = tempfile::tempdir().unwrap();
    let db = redb::Database::create(dir.path().join("collection.redb")).unwrap();
    crate::persistence::create_tables(&db);
    let (character_id, _) =
        crate::persistence::create_character(&db, 30, "Bookcaseuser", 1, 1).unwrap();
    crate::persistence::save_inventory(
        &db,
        character_id,
        &crate::persistence::InventoryData {
            gold: 500_000,
            items: vec![crate::persistence::InventoryItemData {
                item_guid: 100,
                item_id: 245259,
                name: "Small Val'sharah Bookcase".into(),
                quality: 2,
                required_level: 0,
                stack_count: 1,
                vendor_sell_price: 10_000,
                unique_auction: false,
            }],
        },
    );
    let mut h = housing_harness(db);
    h.app.insert_resource(
        crate::item_templates::load_item_templates(&crate::test_data::world_db_path()).unwrap(),
    );
    h.app
        .insert_resource(crate::housing_acquisition::HousingCollectionRules(Ok(
            crate::housing_acquisition::read_collection_rules(
                &crate::housing_acquisition::tests::collection_fixture(),
            )
            .unwrap(),
        )));
    h.app
        .insert_resource(crate::spell_info::SpellStore::fixture(vec![
            crate::spell_info::SpellInfo {
                id: 1238340,
                effects: vec![crate::spell_info::SpellEffect {
                    effect: 324,
                    misc_values: [1693, 0],
                    ..Default::default()
                }],
                ..Default::default()
            },
        ]));
    h.app
        .add_systems(Update, crate::inventory::process_use_requests);
    let owner = h.spawn(character_id, InMap::continent(0), STOCKADE_ENTRANCE_TRIGGER);
    h.app.world_mut().entity_mut(owner.connection).insert((
        Authenticated { account_id: 30 },
        pair::<UseItem>(),
        pair::<HousingReply>(),
        pair::<InventoryError>(),
    ));
    let use_item = UseItem {
        location: ItemLocation::Bag { bag: 0, slot: 0 },
        target: None,
    };
    h.app
        .world_mut()
        .get_mut::<MessageSender<UseItem>>(owner.connection)
        .unwrap()
        .send::<InventoryChannel>(use_item.clone());
    let mut errors = Vec::new();
    for _ in 0..5 {
        h.advance(1);
        errors.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<InventoryError>>(owner.connection)
                .unwrap()
                .receive(),
        );
    }
    assert!(errors.is_empty(), "normal bookcase use refused: {errors:?}");
    let db = &h
        .app
        .world()
        .resource::<crate::persistence::GameDatabase>()
        .db;
    assert!(
        crate::persistence::load_inventory(db, character_id)
            .items
            .is_empty()
    );
    assert_eq!(
        crate::persistence::load_inventory(db, character_id).gold,
        500_000
    );
    let actor = crate::persistence::housing_store::Actor {
        account_id: 30,
        character_id,
        neighborhood_id: None,
        inside_house: None,
    };
    assert_eq!(
        crate::persistence::housing_store::snapshot(db, actor, None, None)
            .unwrap()
            .stored,
        vec![StoredDecor {
            decor_id: 1693,
            quantity: 1
        }]
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<UseItem>>(owner.connection)
        .unwrap()
        .send::<InventoryChannel>(use_item);
    h.advance(5);
    let db = &h
        .app
        .world()
        .resource::<crate::persistence::GameDatabase>()
        .db;
    assert_eq!(
        crate::persistence::housing_store::snapshot(db, actor, None, None)
            .unwrap()
            .stored[0]
            .quantity,
        1
    );
}

fn housing_harness(db: redb::Database) -> Harness {
    let catalog = crate::housing_catalog::HousingCatalog::read(
        &crate::housing_catalog::tests::catalog_fixture(),
    )
    .unwrap();
    harness_with(move |app| {
        app.insert_resource(crate::persistence::GameDatabase::new(db));
        app.insert_resource(crate::housing::HousingAvailability(Ok(catalog)));
        app.init_resource::<crate::gold::GoldUpdateQueue>();
        // Production world startup replaces the generic population through Commands.
        // Housing must survive that replacement without fixture-injected map objects.
        app.add_systems(Startup, |mut commands: Commands| {
            commands.insert_resource(crate::game_objects::GameObjectSpawns::default());
        });
        app.add_systems(Update, crate::housing::process_requests);
    })
    .unwrap()
}

fn map_store() -> Option<&'static MapStore> {
    static STORE: OnceLock<Option<MapStore>> = OnceLock::new();
    STORE
        .get_or_init(|| {
            let path = crate::test_data::world_db_path();
            let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .expect("open world.db");
            Some(
                store::load_map_store(&conn)
                    .expect("map tables in world.db; run scripts/import-azerothcore.sh"),
            )
        })
        .as_ref()
}

#[path = "tests_battleground_nodes.rs"]
mod battleground_nodes;

fn source_warsong_flags() -> crate::game_objects::GameObjectSpawns {
    source_battleground_objects(2106)
}

fn source_battleground_objects(map_id: u32) -> crate::game_objects::GameObjectSpawns {
    let source = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    // Objective entries plus every authored flag/capture type, and Alterac/Isle doors
    // and goobers (types 1/10) their opening and banner scripts use.
    let mut entries = shared::battleground::objective_object_entries(map_id);
    let authored: std::collections::BTreeSet<u32> = source
        .prepare("SELECT DISTINCT g.id FROM tdb_gameobject g JOIN tdb_gameobject_template t ON t.entry=g.id
            WHERE g.map=?1 AND (t.type IN (36,42) OR (g.map IN (30,628) AND t.type IN (1,10)))")
        .unwrap()
        .query_map([map_id], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    let objectives: std::collections::BTreeSet<u32> = entries.iter().copied().collect();
    entries.extend(authored.difference(&objectives));
    source_map_objects(map_id, &entries)
}

fn source_map_objects(map_id: u32, entries: &[u32]) -> crate::game_objects::GameObjectSpawns {
    let source = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    // The spawn loader reads the authored locks beside its database file, so the fixture
    // database lives in a temp directory next to a copy of the canonical locks.
    let dir = tempfile::tempdir().unwrap();
    std::fs::copy(
        crate::test_data::world_db_path().with_file_name("game_object_locks.json"),
        dir.path().join("game_object_locks.json"),
    )
    .expect("game_object_locks.json beside world.db; run scripts/import_game_object_locks.py");
    let target = Connection::open(dir.path().join("world.db")).unwrap();
    target.execute_batch("CREATE TABLE arena_map(map_id INTEGER);
        CREATE TABLE content_gameobject (guid INTEGER,id INTEGER,map INTEGER,position_x REAL,position_y REAL,position_z REAL,orientation REAL,spawnDifficulties TEXT,spawntimesecs INTEGER,state INTEGER);
        CREATE TABLE content_gameobject_template (entry INTEGER PRIMARY KEY,type INTEGER,displayId INTEGER,name TEXT,size REAL,Data0 INTEGER,Data1 INTEGER,Data3 INTEGER,Data5 INTEGER,Data6 INTEGER,Data10 INTEGER,Data23 INTEGER,ScriptName TEXT DEFAULT '');
        CREATE TABLE game_object_display_info (ID INTEGER PRIMARY KEY,GeoBox_0 REAL,GeoBox_1 REAL,GeoBox_2 REAL,GeoBox_3 REAL,GeoBox_4 REAL,GeoBox_5 REAL);").unwrap();
    target
        .execute("INSERT INTO arena_map VALUES(?)", [map_id])
        .unwrap();
    let mut query = source.prepare("SELECT g.guid+20000000,g.id,g.map,g.position_x,g.position_y,g.position_z,g.orientation,g.spawnDifficulties,
        t.type,t.displayId,t.name,t.size,t.Data0,t.Data1,d.GeoBox_0,d.GeoBox_1,d.GeoBox_2,d.GeoBox_3,d.GeoBox_4,d.GeoBox_5,
        g.spawntimesecs,t.Data3,t.Data5,t.Data6,g.state,t.Data10,t.Data23
        FROM tdb_gameobject g JOIN tdb_gameobject_template t ON t.entry=g.id
        JOIN game_object_display_info d ON d.ID=t.displayId WHERE g.map=? ORDER BY g.guid").unwrap();
    let mut rows = query.query([map_id]).unwrap();
    while let Some(row) = rows.next().unwrap() {
        let entry: u32 = row.get(1).unwrap();
        if !entries.contains(&entry) {
            continue;
        }
        let values: Vec<rusqlite::types::Value> =
            (0..27).map(|index| row.get(index).unwrap()).collect();
        let mut spawn = values[..8].to_vec();
        spawn.push(values[20].clone());
        spawn.push(values[24].clone());
        target
            .execute(
                "INSERT INTO content_gameobject VALUES (?,?,?,?,?,?,?,?,?,?)",
                rusqlite::params_from_iter(&spawn),
            )
            .unwrap();
        let mut template = vec![values[1].clone()];
        template.extend_from_slice(&values[8..14]);
        template.extend_from_slice(&values[21..24]);
        template.extend_from_slice(&values[25..27]);
        target
            .execute(
                "INSERT OR IGNORE INTO content_gameobject_template (entry,type,displayId,name,size,Data0,Data1,Data3,Data5,Data6,Data10,Data23) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)",
                rusqlite::params_from_iter(&template),
            )
            .unwrap();
        let mut display = vec![values[9].clone()];
        display.extend_from_slice(&values[14..20]);
        target
            .execute(
                "INSERT OR IGNORE INTO game_object_display_info VALUES (?,?,?,?,?,?,?)",
                rusqlite::params_from_iter(&display),
            )
            .unwrap();
    }
    crate::game_objects::load_game_object_spawns(&target).unwrap()
}

#[test]
fn arena_source_doors_and_shadow_sight_are_loaded_for_zero_difficulty_copy() {
    let objects = source_map_objects(572, &[185918, 185917, 184663, 184664]);
    assert_eq!(objects.0.len(), 4);
    assert_eq!(
        objects
            .0
            .iter()
            .filter(|spawn| spawn.info.go_type == 0)
            .count(),
        2
    );
    assert_eq!(
        objects
            .0
            .iter()
            .filter(|spawn| spawn.info.go_type == 6)
            .count(),
        2
    );
    assert!(
        objects
            .0
            .iter()
            .all(|spawn| spawn.spawn_difficulties == vec![0])
    );
}

#[test]
fn authored_warsong_new_flags_are_loaded_as_usable_world_objects() {
    let flags = source_warsong_flags();
    assert_eq!(flags.0.len(), 2);
    for flag in flags.0 {
        assert!(flag.info.is_usable());
        assert_eq!(flag.map_id, 2106);
    }
}

#[test]
fn arathi_authored_capture_points_are_loadable_and_usable() {
    let points = source_battleground_objects(2107);
    let mut entries: Vec<_> = points.0.iter().map(|point| point.info.entry).collect();
    entries.sort_unstable();
    assert_eq!(entries, [227420, 227522, 227536, 227538, 227544]);
    assert!(points.0.iter().all(|point| point.info.is_usable()));
}

fn close(a: Vec3, b: Vec3) -> bool {
    a.distance(b) < 0.01
}

fn wow(position: &Position) -> Vec3 {
    let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
    Vec3::new(x, y, z)
}

fn bevy(wow: Vec3) -> Position {
    let (x, y, z) = crate::terrain_height::wow_to_bevy(wow.x, wow.y, wow.z);
    Position { x, y, z }
}

#[test]
fn stockade_entrance_trigger_leads_to_its_arrival_point_and_the_exit_back() {
    let Some(store) = map_store() else { return };
    let map = store.map(STOCKADE).expect("map 34");
    assert_eq!(map.directory, "stormwindjail");
    assert!(map.is_dungeon());
    assert_eq!(map.area_table_id, 717);

    let entrance = store
        .area_triggers_on(0)
        .iter()
        .find(|trigger| trigger.id == 101)
        .expect("area trigger 101 on map 0");
    assert!(entrance.contains(STOCKADE_ENTRANCE_TRIGGER));
    // Box 4.97 x 9.69 x 7.44 turned 0.663 rad: 6 yd off-center along X is outside.
    assert!(!entrance.contains(STOCKADE_ENTRANCE_TRIGGER + Vec3::new(6.0, 0.0, 0.0)));
    assert!(entrance.allow_while_ghost);
    assert_eq!(entrance.target.map_id, STOCKADE);
    assert!(close(entrance.target.position, STOCKADE_ARRIVAL));

    let back = store
        .go_back_location(STOCKADE)
        .expect("go-back trigger 503");
    assert_eq!(back.map_id, 0);
    assert!(close(back.position, STOCKADE_EXIT_TARGET));
    let inside = store.entrance_location(STOCKADE).expect("entrance trigger");
    assert!(close(inside.position, STOCKADE_ARRIVAL));
}

#[test]
fn a_stockade_death_releases_to_the_stockade_graveyard_in_stormwind() {
    let Some(store) = map_store() else { return };
    let inside = WorldLocation {
        map_id: STOCKADE,
        position: STOCKADE_ARRIVAL,
        orientation: 0.0,
    };
    let grave = graveyard::closest_graveyard_in_zone(store, &inside, 717).expect("graveyard");
    assert_eq!(grave.map_id, 0);
    assert!(close(grave.position, STOCKADE_GRAVEYARD));
}

/// 2026-09-27 12:00 UTC (a Sunday): the instance-lock clock of these tests.
const SUNDAY_NOON: i64 = 1_790_510_400;

/// A Normal-difficulty entrant; a grouped one's leader is character 1.
fn entrant(character_id: u64, group: Option<u64>) -> InstanceEntrant {
    InstanceEntrant {
        character_id,
        group,
        leader: if group.is_some() { 1 } else { character_id },
        difficulty_id: store::DIFFICULTY_NORMAL,
        ..Default::default()
    }
}

/// Elapsed seconds `elapsed` on Sunday noon.
fn at(elapsed: f64) -> instances::Now {
    instances::Now {
        elapsed,
        unix: SUNDAY_NOON,
    }
}

#[test]
fn group_members_share_a_stockade_copy_and_a_solo_player_gets_its_own() {
    let Some(store) = map_store() else { return };
    let mut instances = InstanceManager::default();
    let first = instances
        .create_map(store, STOCKADE, entrant(1, Some(7)), at(0.0))
        .unwrap();
    let second = instances
        .create_map(store, STOCKADE, entrant(2, Some(7)), at(1.0))
        .unwrap();
    let solo = instances
        .create_map(store, STOCKADE, entrant(3, None), at(2.0))
        .unwrap();

    assert!(first.created);
    assert_eq!(
        second,
        instances::ResolvedMap {
            map: first.map,
            created: false,
        }
    );
    assert!(solo.created);
    assert_ne!(solo.map.instance_id, first.map.instance_id);
    // The solo player's copy is not the group's to enter, and the other way round.
    assert_eq!(
        instances.find_instance(store, STOCKADE, entrant(3, Some(7)), SUNDAY_NOON),
        Some(first.map)
    );
    assert_eq!(
        instances.find_instance(store, STOCKADE, entrant(1, None), SUNDAY_NOON),
        None
    );
}

#[test]
fn warsong_rosters_share_only_their_reserved_copy_without_dungeon_locks() {
    let store = map_store().expect("local map catalog");
    let template = shared::battleground::warsong_gulch();
    let mut instances = InstanceManager::default();
    let first_roster: Vec<u64> = (1..=10).collect();
    let second_roster: Vec<u64> = (11..=20).collect();
    let capacity = u32::from(template.team_size) * 2;
    assert_eq!(
        instances.create_map(store, template.map_id, entrant(1, None), at(0.0)),
        Err(TransferAbortReason::MapNotAllowed)
    );
    let first = instances
        .create_battleground_map(store, template.map_id, &first_roster, capacity, at(0.0))
        .unwrap();
    let second = instances
        .create_battleground_map(store, template.map_id, &second_roster, capacity, at(0.0))
        .unwrap();
    assert_ne!(first, second);
    for (roster, expected) in [(&first_roster, first), (&second_roster, second)] {
        for character_id in roster {
            let resolved = instances
                .create_map(
                    store,
                    template.map_id,
                    entrant(*character_id, Some(7)),
                    at(1.0),
                )
                .unwrap();
            assert_eq!(resolved.map, expected);
            assert!(!resolved.created);
            assert_eq!(
                instances.player_cannot_enter(
                    store,
                    template.map_id,
                    entrant(*character_id, None),
                    90,
                    SUNDAY_NOON
                ),
                None
            );
        }
    }
    assert_eq!(
        instances.create_map(store, template.map_id, entrant(21, None), at(1.0)),
        Err(TransferAbortReason::MapNotAllowed)
    );
    assert_eq!(
        instances.cannot_enter(first, entrant(11, None), SUNDAY_NOON),
        Some(TransferAbortReason::MapNotAllowed)
    );
    assert!(instances.lock_copies_in_use().is_empty());
    assert!(
        instances
            .take_expired(INSTANCE_UNLOAD_DELAY_SECS + 1.0)
            .is_empty(),
        "no invented battleground reservation expiry"
    );
}

#[test]
fn a_full_stockade_copy_refuses_the_sixth_player_and_a_stranger_to_its_group() {
    let Some(store) = map_store() else { return };
    let mut world = World::new();
    let mut instances = InstanceManager::default();
    let copy = instances
        .create_map(store, STOCKADE, entrant(1, Some(7)), at(0.0))
        .unwrap()
        .map;
    // MapDifficulty 8 (Normal) holds 5 players.
    for _ in 0..5 {
        instances.add_player(copy, world.spawn_empty().id());
    }
    assert_eq!(
        instances.player_cannot_enter(store, STOCKADE, entrant(6, Some(7)), 80, SUNDAY_NOON),
        Some(TransferAbortReason::MaxPlayers)
    );
    assert_eq!(
        instances.cannot_enter(copy, entrant(9, Some(8)), SUNDAY_NOON),
        Some(TransferAbortReason::MaxPlayers)
    );
}

#[test]
fn an_empty_copy_unloads_after_thirty_minutes_and_comes_back_fresh() {
    let Some(store) = map_store() else { return };
    let mut world = World::new();
    let player = world.spawn_empty().id();
    let mut instances = InstanceManager::default();
    let copy = instances
        .create_map(store, STOCKADE, entrant(1, None), at(0.0))
        .unwrap()
        .map;
    instances.add_player(copy, player);
    instances.remove_player(copy, player, at(100.0));

    assert!(
        instances
            .take_expired(100.0 + INSTANCE_UNLOAD_DELAY_SECS - 1.0)
            .is_empty()
    );
    assert_eq!(
        instances.take_expired(100.0 + INSTANCE_UNLOAD_DELAY_SECS),
        vec![copy]
    );
    // The player re-enters its recent instance id as a new copy.
    let again = instances
        .create_map(store, STOCKADE, entrant(1, None), at(2000.0))
        .unwrap();
    assert_eq!(again.map, copy);
    assert!(again.created);
}

#[test]
fn world_maps_without_npc_data_are_not_hosted() {
    let Some(store) = map_store() else { return };
    let instances = InstanceManager::default();
    // Map.csv: Alliance PVP Barracks (449) is a world map without population.
    // Outland (530) now has promoted Retail NPCs and is explicitly hosted.
    let conn = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let (instance_type, spawns): (u32, u32) = conn
        .query_row(
            "SELECT InstanceType, (SELECT COUNT(*) FROM content_creature WHERE map=map.ID)
             FROM map WHERE ID=449",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((instance_type, spawns), (0, 0));
    for map_id in [449] {
        assert_eq!(
            instances.player_cannot_enter(store, map_id, entrant(1, None), 80, SUNDAY_NOON),
            Some(TransferAbortReason::MapNotAllowed),
            "map {map_id}"
        );
    }
    assert_eq!(
        instances.player_cannot_enter(store, 1, entrant(1, None), 1, SUNDAY_NOON),
        None
    );
}

/// The Culling of Stratholme (595) Normal: MapDifficulty 91's conditions (40817, 43808)
/// carry no ContentTuning, so no level is required.
const CULLING_OF_STRATHOLME: u32 = 595;

#[test]
fn a_level_one_player_may_enter_a_dungeon_without_a_level_condition() {
    let Some(store) = map_store() else { return };
    let instances = InstanceManager::default();
    assert_eq!(
        instances.player_cannot_enter(
            store,
            CULLING_OF_STRATHOLME,
            entrant(1, None),
            1,
            SUNDAY_NOON
        ),
        None
    );
}

// --- App-level transfers ---

struct Harness {
    app: App,
    /// Messages each connection received, drained every frame (a `MessageReceiver`
    /// only holds the current frame's messages).
    new_worlds: std::collections::HashMap<Entity, Vec<NewWorld>>,
    aborts: std::collections::HashMap<Entity, Vec<TransferAborted>>,
    saves: std::collections::HashMap<Entity, usize>,
}

struct Traveler {
    player: Entity,
    connection: Entity,
}

fn pair<M: lightyear::prelude::Message>() -> (MessageSender<M>, MessageReceiver<M>) {
    (MessageSender::default(), MessageReceiver::default())
}

fn harness() -> Option<Harness> {
    harness_with(|_| {})
}

/// A harness whose copies spawn world.db's creatures and game objects.
fn harness_with_content() -> Option<Harness> {
    harness_with_content_and(|_| {})
}

fn harness_with_content_and(extra: impl FnOnce(&mut App)) -> Option<Harness> {
    let path = crate::test_data::world_db_path();
    let world_data = crate::world_data::load_world_data(&path);
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("open world.db");
    let objects = crate::game_objects::load_game_object_spawns(&conn).expect("game objects");
    let bossmod = crate::scripted_ai::bossmod::load_catalog(&conn).expect("boss-mod catalog");
    harness_with(move |app| {
        app.insert_resource(world_data);
        app.insert_resource(crate::test_data::ground_map());
        app.insert_resource(objects);
        app.insert_resource(bossmod);
        extra(app);
    })
}

fn harness_with(setup: impl FnOnce(&mut App)) -> Option<Harness> {
    let store = map_store()?;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(ProtocolPlugin);
    app.add_plugins(MapPlugin);
    app.add_message::<crate::death::PlayerDeathTransition>();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(50),
    ));
    app.insert_resource(store.clone());
    app.insert_resource(ServerClock {
        offset_secs: 0,
        fixed_unix: Some(SUNDAY_NOON),
    });
    app.init_resource::<GroupRegistry>();
    app.insert_resource(
        crate::conditions::load_spell_target_conditions(&crate::test_data::world_db_path())
            .expect("authored conditions in map-lifetime fixture"),
    );
    setup(&mut app);
    app.finish();
    app.cleanup();
    app.update();
    Some(Harness {
        app,
        new_worlds: Default::default(),
        aborts: Default::default(),
        saves: Default::default(),
    })
}

/// The level harness players have: every hosted dungeon's level condition admits it
/// (Mythic Grim Batol, PlayerCondition 115171, needs 80).
const TRAVELER_LEVEL: u8 = 80;

impl Harness {
    fn housing_request(&mut self, who: &Traveler, command: HousingCommand) -> HousingReply {
        self.app
            .world_mut()
            .get_mut::<MessageSender<HousingRequest>>(who.connection)
            .unwrap()
            .send::<HousingChannel>(HousingRequest {
                request_id: 1,
                command,
            });
        let mut replies = Vec::new();
        for _ in 0..6 {
            self.advance(1);
            replies.extend(
                self.app
                    .world_mut()
                    .get_mut::<MessageReceiver<HousingReply>>(who.connection)
                    .unwrap()
                    .receive(),
            );
        }
        assert_eq!(
            replies.len(),
            1,
            "exactly one authoritative reply per request"
        );
        replies.remove(0)
    }

    fn spawn(&mut self, character_id: u64, map: InMap, at: Vec3) -> Traveler {
        self.spawn_at_level(character_id, TRAVELER_LEVEL, map, at)
    }

    fn spawn_at_level(&mut self, character_id: u64, level: u8, map: InMap, at: Vec3) -> Traveler {
        let player = self
            .app
            .world_mut()
            .spawn((
                Player {
                    name: format!("Traveler{character_id}"),
                    race: 1,
                    class: 1,
                    appearance: Default::default(),
                },
                ClientId(character_id),
                bevy(at),
                Rotation {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                MovementControl::default(),
                CombatTarget(None),
                shared::components::Level(level),
                map,
            ))
            .id();
        let connection = self
            .app
            .world_mut()
            .spawn((
                RemoteId(PeerId::Local(character_id)),
                Connected,
                HostClient { buffer: Vec::new() },
                Authenticated {
                    account_id: character_id,
                },
                InWorld { character_id },
                PlayerLink(player),
                SentTiles {
                    tiles: Default::default(),
                    last_tile: Some((32, 48)),
                },
                (
                    pair::<NewWorld>(),
                    pair::<WorldPortAck>(),
                    pair::<TransferAborted>(),
                    pair::<DeathStateUpdate>(),
                    pair::<InstanceSaveCreated>(),
                    pair::<WorldServerInfo>(),
                    pair::<SetDungeonDifficulty>(),
                    pair::<DungeonDifficultySet>(),
                    pair::<SetRaidDifficulty>(),
                    pair::<RaidDifficultySet>(),
                ),
            ))
            .id();
        self.app.update();
        Traveler { player, connection }
    }

    fn advance(&mut self, frames: usize) {
        for _ in 0..frames {
            self.app.update();
            let world = self.app.world_mut();
            let mut connections = world.query::<(
                Entity,
                &mut MessageReceiver<NewWorld>,
                &mut MessageReceiver<TransferAborted>,
                &mut MessageReceiver<InstanceSaveCreated>,
            )>();
            for (connection, mut new_worlds, mut aborts, mut saves) in connections.iter_mut(world) {
                *self.saves.entry(connection).or_default() += saves.receive().count();
                self.new_worlds
                    .entry(connection)
                    .or_default()
                    .extend(new_worlds.receive());
                self.aborts
                    .entry(connection)
                    .or_default()
                    .extend(aborts.receive());
            }
        }
    }

    fn walk_to(&mut self, who: &Traveler, at: Vec3) {
        self.app.world_mut().entity_mut(who.player).insert(bevy(at));
        self.advance(2);
    }

    fn new_worlds(&mut self, who: &Traveler) -> Vec<NewWorld> {
        self.advance(2);
        self.new_worlds.remove(&who.connection).unwrap_or_default()
    }

    fn aborts(&mut self, who: &Traveler) -> Vec<TransferAborted> {
        self.advance(2);
        self.aborts.remove(&who.connection).unwrap_or_default()
    }

    fn ack(&mut self, who: &Traveler) {
        self.app
            .world_mut()
            .get_mut::<MessageSender<WorldPortAck>>(who.connection)
            .unwrap()
            .send::<TransferChannel>(WorldPortAck);
        self.advance(3);
    }

    fn receive_pvp_updates(
        &mut self,
        frames: usize,
    ) -> std::collections::HashMap<Entity, Vec<PvpStateUpdate>> {
        let mut received = std::collections::HashMap::<Entity, Vec<PvpStateUpdate>>::new();
        for _ in 0..frames {
            self.advance(1);
            let world = self.app.world_mut();
            let mut receivers = world.query::<(Entity, &mut MessageReceiver<PvpStateUpdate>)>();
            for (connection, mut receiver) in receivers.iter_mut(world) {
                received
                    .entry(connection)
                    .or_default()
                    .extend(receiver.receive());
            }
        }
        received
    }

    fn map_of(&self, who: &Traveler) -> Option<InMap> {
        self.app.world().get::<InMap>(who.player).copied()
    }

    fn position(&self, who: &Traveler) -> Vec3 {
        wow(self.app.world().get::<Position>(who.player).unwrap())
    }

    fn group(&mut self, members: &[&Traveler]) {
        let online: OnlineView = members
            .iter()
            .map(|member| {
                let id = self.app.world().get::<ClientId>(member.player).unwrap().0;
                (
                    id,
                    OnlineMember {
                        entity: member.player.to_bits(),
                        name: format!("Traveler{id}"),
                        class: 1,
                        level: 20,
                        portrait: Default::default(),
                        dungeon_difficulty: crate::maps::store::DIFFICULTY_NORMAL,
                        raid_difficulty: crate::maps::RaidDifficulty::default(),
                    },
                )
            })
            .collect();
        let ids: Vec<u64> = members
            .iter()
            .map(|member| self.app.world().get::<ClientId>(member.player).unwrap().0)
            .collect();
        let mut registry = self.app.world_mut().resource_mut::<GroupRegistry>();
        registry.sync_presence(&online);
        for (index, &id) in ids.iter().enumerate().skip(1) {
            // A party holds five: a sixth member joins a raid.
            if index == 5 {
                registry.convert_to_raid(ids[0]);
            }
            registry.invite(ids[0], &format!("Traveler{id}"), &online);
            registry.respond_invite(id, true);
        }
        assert_eq!(registry.group_members(ids[0]).len(), ids.len());
    }

    fn arrivals(&self, who: &Traveler) -> u32 {
        self.app
            .world()
            .get::<shared::components::WorldArrival>(who.player)
            .unwrap()
            .0
    }

    /// Stockade creatures `who`'s connection sees.
    fn visible_stockade_creatures(&mut self, who: &Traveler) -> usize {
        let world = self.app.world_mut();
        let mut query = world.query_filtered::<
            (&InMap, &crate::interest::NetworkVisibility),
            With<shared::components::Npc>,
        >();
        query
            .iter(world)
            .filter(|(map, visibility)| {
                map.map_id == STOCKADE && visibility.is_visible(who.connection)
            })
            .count()
    }

    /// (template, name, level) of every creature in `map`, sorted.
    fn creatures_in(&mut self, map: InMap) -> Vec<(u32, String, u8)> {
        let world = self.app.world_mut();
        let mut query = world.query::<(&shared::components::Npc, &UnitLevelOf, &InMap)>();
        let mut creatures: Vec<_> = query
            .iter(world)
            .filter(|(_, _, in_map)| **in_map == map)
            .map(|(npc, level, _)| (npc.template_id, npc.name.clone(), level.0))
            .collect();
        creatures.sort();
        creatures
    }

    /// Walk from beside the entrance into trigger 101 and answer the `NewWorld`.
    fn enter_stockade(&mut self, who: &Traveler) -> NewWorld {
        self.walk_to(who, STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0));
        self.walk_to(who, STOCKADE_ENTRANCE_TRIGGER);
        let new_world = self.new_worlds(who).pop().expect("NewWorld for map 34");
        self.ack(who);
        new_world
    }
}

const STORMWIND: InMap = InMap::continent(0);
type UnitLevelOf = shared::components::UnitLevel;
/// Retail Stockade (TDB1210 map 34, DEFAULT_PHASE 169, Normal): 143 creatures, one Hogger (46254)
/// on ContentTuning 215 (levels 10-30) with LevelScalingDelta 2.
const STOCKADE_CREATURES: usize = 143;
const HOGGER: u32 = 46254;

#[test]
fn each_stockade_copy_spawns_its_own_retail_creatures_and_unloads_them() {
    let Some(mut h) = harness_with_content() else {
        return;
    };
    let start = STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0);
    let first = h.spawn(1, STORMWIND, start);
    let second = h.spawn(2, STORMWIND, start);
    h.enter_stockade(&first);
    h.enter_stockade(&second);
    let (a, b) = (h.map_of(&first).unwrap(), h.map_of(&second).unwrap());
    assert_ne!(a, b);
    for copy in [a, b] {
        let creatures = h.creatures_in(copy);
        assert_eq!(creatures.len(), STOCKADE_CREATURES, "{copy:?}");
        let hogger: Vec<_> = creatures.iter().filter(|c| c.0 == HOGGER).collect();
        assert_eq!(hogger.len(), 1, "{copy:?}");
        assert_eq!(hogger[0].1, "Hogger");
        assert_eq!(
            hogger[0].2, 32,
            "ContentTuning 215 top level 30 + Normal delta 2"
        );
    }

    // `first` leaves; after Instance.UnloadDelay its copy goes with its creatures.
    h.walk_to(&first, STOCKADE_EXIT_TRIGGER);
    h.new_worlds(&first);
    h.ack(&first);
    let delay = Duration::from_secs_f64(INSTANCE_UNLOAD_DELAY_SECS + 1.0);
    h.app
        .world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(delay);
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(delay));
    h.advance(2);
    assert!(h.creatures_in(a).is_empty());
    assert_eq!(
        h.creatures_in(b).len(),
        STOCKADE_CREATURES,
        "the occupied copy stays"
    );
}

#[test]
fn four_arena_players_invite_ack_prepare_pick_shadow_sight_end_and_return_over_channel() {
    exercise_arena_channel(ArenaQueueMode::Skirmish2v2, false, 572);
}
#[test]
fn rated_arena_groups_play_then_persist_reference_rating_without_queue_only_awards() {
    exercise_arena_channel(ArenaQueueMode::Rated2v2, false, 572);
}
#[test]
fn six_arena_players_complete_skirmish_three_v_three_over_real_channels() {
    exercise_arena_channel(ArenaQueueMode::Skirmish3v3, false, 572);
}
#[test]
fn six_arena_players_complete_rated_three_v_three_and_persist_played_results() {
    exercise_arena_channel(ArenaQueueMode::Rated3v3, false, 572);
}
#[test]
fn arena_alive_forfeits_end_the_match_without_fabricating_scoreboard_deaths() {
    exercise_arena_channel(ArenaQueueMode::Skirmish2v2, true, 572);
}

#[test]
fn arena_authored_cage_gate_is_visible_immediately_after_worldport_without_movement() {
    let mut h = harness_with_content_and(|app| {
        app.add_plugins(crate::interest::InterestPlugin);
    })
    .unwrap();
    let actor = h.spawn_at_level(1, 90, STORMWIND, STOCKADE_ENTRANCE_TRIGGER);
    h.app
        .world_mut()
        .entity_mut(actor.player)
        .insert(crate::interest::NetworkVisibility::default());
    let copy = h
        .app
        .world_mut()
        .resource_scope(|world, mut manager: Mut<InstanceManager>| {
            manager.create_battleground_map(
                world.resource::<MapStore>(),
                2759,
                &[1, 2, 3, 4],
                4,
                at(0.0),
            )
        })
        .unwrap();
    super::transfer::spawn_map_content(h.app.world_mut(), copy);
    let destination = h.app.world().resource::<MapStore>().safe_locs[&100101];
    assert!(teleport_to(
        h.app.world_mut(),
        actor.player,
        destination,
        TeleportOptions::default()
    ));
    h.new_worlds(&actor).pop().expect("source Cage NewWorld");
    h.ack(&actor);
    let mut query = h.app.world_mut().query::<(
        &InMap,
        &GameObjectInfo,
        &crate::interest::NetworkVisibility,
        &GameObjectCollision,
    )>();
    let gate = query
        .iter(h.app.world())
        .find(|(map, info, ..)| **map == copy && info.entry == 505683)
        .unwrap();
    assert!(
        gate.2.is_visible(actor.connection),
        "source nearby trapdoor must be shown without waiting for opening"
    );
    assert!(
        gate.3.0,
        "source READY1 collision must be present during preparation"
    );
}

#[test]
fn arena_all_sixteen_active_source_maps_play_authored_starts_gates_orbs_and_return() {
    let source = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let maps: Vec<u32> = source
        .prepare("SELECT map_id FROM arena_map WHERE min_level>0 AND max_level>=90 ORDER BY map_id")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(
        maps.len(),
        16,
        "local BattlemasterList/Map/WDT source census"
    );
    const MAX_WORKERS: usize = 4;
    std::thread::scope(|scope| {
        let workers: Vec<_> = maps.chunks(maps.len().div_ceil(MAX_WORKERS))
            .map(|batch| scope.spawn(move || {
                for &map_id in batch {
                    exercise_arena_channel(ArenaQueueMode::Skirmish2v2, false, map_id);
                    println!(
                        "SOURCE_ARENA_MAP_PASS {map_id}: authored starts/gates, preparation, physical orb, scored elimination, exact returns"
                    );
                }
            }))
            .collect();
        for worker in workers {
            worker.join().expect("every source arena fixture must pass");
        }
    });
}

#[test]
fn arena_cage_preparation_keeps_authored_gate_in_observer_visibility() {
    exercise_arena_channel(ArenaQueueMode::Skirmish2v2, false, 2759);
}

#[test]
fn arena_dalaran_authored_waterfall_and_pipe_hazards_follow_actual_lifetime() {
    exercise_arena_channel(ArenaQueueMode::Skirmish2v2, false, 617);
}

fn exercise_arena_channel(mode: ArenaQueueMode, forfeit: bool, map_id: u32) {
    exercise_arena_channel_with_reconnect(mode, forfeit, map_id, None);
}

fn exercise_pvp_reconnect(
    h: &mut Harness,
    players: &[Traveler],
    origins: &[Vec3],
    case: ReconnectCase,
    database_path: &std::path::Path,
) {
    let teams: Vec<_> = players
        .iter()
        .map(|who| reconnect_snapshot(h, who).team)
        .collect();
    let index = if reconnect_snapshot(h, &players[0]).arena.is_none() {
        teams
            .iter()
            .position(|team| *team == shared::battleground::BgTeam::B)
            .unwrap()
    } else {
        0
    };
    let who = &players[index];
    let character_id = h.app.world().get::<ClientId>(who.player).unwrap().0;
    let old_map = h.map_of(who).unwrap();
    let original = origins[index];
    let prior_faction = h
        .app
        .world()
        .get::<shared::components::UnitFactionTemplate>(who.player)
        .copied();
    let prior = reconnect_snapshot(h, who).clone();
    assert_eq!(prior.phase, shared::battleground::BgPhase::InProgress);
    if matches!(case, ReconnectCase::NearTeleportWithinFullMap) {
        let current = wow(h.app.world().get::<Position>(who.player).unwrap());
        let destination = WorldLocation {
            map_id: old_map.map_id,
            position: current + Vec3::new(1.0, 0.0, 0.0),
            orientation: 0.7,
        };
        assert!(
            teleport_to(
                h.app.world_mut(),
                who.player,
                destination,
                TeleportOptions::default()
            ),
            "an already-admitted arena player can move within its full reserved copy"
        );
        assert_eq!(h.map_of(who), Some(old_map));
        assert!(
            h.new_worlds(who).is_empty(),
            "near teleport must not re-enter a full copy"
        );
        assert!(close(
            wow(h.app.world().get::<Position>(who.player).unwrap()),
            destination.position
        ));
        return;
    }
    if matches!(case, ReconnectCase::LastLivingLogout) {
        let ally = players
            .iter()
            .zip(&teams)
            .enumerate()
            .find(|(other, (_, team))| *other != index && **team == prior.team)
            .map(|(_, (who, _))| who)
            .unwrap();
        h.app
            .world_mut()
            .entity_mut(ally.connection)
            .insert(lightyear::prelude::server::ClientOf)
            .remove::<Connected>();
        h.advance(3);
        let state = h
            .app
            .world()
            .get::<crate::pvp::PlayerPvpState>(who.player)
            .unwrap();
        assert_eq!(
            state.battleground.as_ref().unwrap().phase,
            shared::battleground::BgPhase::InProgress
        );
    }
    h.app
        .world_mut()
        .entity_mut(who.connection)
        .insert(lightyear::prelude::server::ClientOf)
        .remove::<Connected>();
    h.advance(3);
    assert!(
        h.app.world().get_entity(who.player).is_err(),
        "the actual disconnect cleanup despawned the old player"
    );
    if matches!(case, ReconnectCase::LastLivingLogout) {
        let opponent = players
            .iter()
            .zip(&teams)
            .find(|(_, team)| **team != prior.team)
            .unwrap()
            .0;
        let ended = h
            .app
            .world()
            .get::<crate::pvp::PlayerPvpState>(opponent.player)
            .unwrap()
            .battleground
            .as_ref()
            .unwrap();
        assert_eq!(ended.phase, shared::battleground::BgPhase::Ended);
        assert_eq!(
            ended.outcome,
            Some(shared::battleground::BgOutcome::Winner(
                *teams.iter().find(|team| **team != prior.team).unwrap()
            ))
        );
        assert!(ended.players.iter().all(|score| score.deaths == 0));
        return;
    }
    if matches!(case, ReconnectCase::MissingMatch) {
        crate::pvp::forget_matches_for_reconnect_fixture(h.app.world_mut());
        drop(
            h.app
                .world_mut()
                .remove_resource::<crate::persistence::GameDatabase>(),
        );
        h.app.insert_resource(crate::persistence::GameDatabase::new(
            redb::Database::open(database_path).expect("reopen durable battleground entry point"),
        ));
    }
    if matches!(case, ReconnectCase::ExpiredMatch) {
        h.advance(6001);
    }
    let player = relog_saved_character(h, character_id);
    let state = h
        .app
        .world()
        .get::<crate::pvp::PlayerPvpState>(player)
        .unwrap();
    match case {
        ReconnectCase::SurvivingMatch => {
            assert_eq!(h.app.world().get::<InMap>(player), Some(&old_map));
            let joined = state
                .battleground
                .as_ref()
                .expect("rejoin must restore match membership, not only arena coordinates");
            assert_eq!(joined.match_id, prior.match_id);
            assert_eq!(joined.team, prior.team);
            assert!(joined.accepted);
            assert_eq!(joined.phase, shared::battleground::BgPhase::InProgress);
            if let Some(faction) = prior_faction {
                assert_eq!(
                    h.app
                        .world()
                        .get::<shared::components::UnitFactionTemplate>(player),
                    Some(&faction)
                );
            }
        }
        ReconnectCase::MissingMatch | ReconnectCase::ExpiredMatch => {
            assert_eq!(h.app.world().get::<InMap>(player), Some(&STORMWIND));
            assert!(state.battleground.is_none());
            assert!(
                (wow(h.app.world().get::<Position>(player).unwrap()) - original).length() < 0.001
            );
        }
        ReconnectCase::LastLivingLogout | ReconnectCase::NearTeleportWithinFullMap => {
            unreachable!()
        }
    }
}

fn reconnect_snapshot<'a>(
    h: &'a Harness,
    who: &Traveler,
) -> &'a shared::protocol::BattlegroundSnapshot {
    h.app
        .world()
        .get::<crate::pvp::PlayerPvpState>(who.player)
        .unwrap()
        .battleground
        .as_ref()
        .unwrap()
}

fn relog_saved_character(h: &mut Harness, character_id: u64) -> Entity {
    let connection = h
        .app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(1001)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated {
                account_id: character_id,
            },
            pair::<shared::protocol::SelectCharacter>(),
            pair::<shared::protocol::EnterWorldResponse>(),
            pair::<shared::protocol::LoadTerrain>(),
            pair::<NewWorld>(),
            pair::<WorldPortAck>(),
            pair::<TransferAborted>(),
            pair::<InstanceSaveCreated>(),
            pair::<PvpStateUpdate>(),
        ))
        .id();
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::SelectCharacter>>(connection)
        .unwrap()
        .send::<shared::protocol::AuthChannel>(shared::protocol::SelectCharacter { character_id });
    h.advance(2);
    crate::test_data::drain_persistence(
        h.app.world().resource::<crate::persistence::GameDatabase>(),
    );
    h.advance(8);
    h.app
        .world()
        .get::<PlayerLink>(connection)
        .expect("real SelectCharacter restores the saved character")
        .0
}

#[derive(Clone, Copy)]
enum ReconnectCase {
    SurvivingMatch,
    MissingMatch,
    LastLivingLogout,
    ExpiredMatch,
    NearTeleportWithinFullMap,
}

#[test]
fn arena_disconnect_rejoins_surviving_match_over_real_channels() {
    exercise_arena_channel_with_reconnect(
        ArenaQueueMode::Skirmish2v2,
        false,
        572,
        Some(ReconnectCase::SurvivingMatch),
    );
}

#[test]
fn arena_disconnect_returns_to_captured_origin_when_match_is_missing() {
    exercise_arena_channel_with_reconnect(
        ArenaQueueMode::Skirmish2v2,
        false,
        572,
        Some(ReconnectCase::MissingMatch),
    );
}

#[test]
fn arena_near_teleport_within_full_reserved_copy_does_not_fail_capacity_admission() {
    exercise_arena_channel_with_reconnect(
        ArenaQueueMode::Skirmish2v2,
        false,
        572,
        Some(ReconnectCase::NearTeleportWithinFullMap),
    );
}

#[test]
fn arena_disconnect_expires_after_source_three_hundred_seconds_then_returns_to_origin() {
    exercise_arena_channel_with_reconnect(
        ArenaQueueMode::Skirmish2v2,
        false,
        572,
        Some(ReconnectCase::ExpiredMatch),
    );
}

#[test]
fn arena_disconnect_last_living_logout_ends_the_arena_without_false_deaths() {
    exercise_arena_channel_with_reconnect(
        ArenaQueueMode::Skirmish2v2,
        false,
        572,
        Some(ReconnectCase::LastLivingLogout),
    );
}

#[test]
fn solo_shuffle_six_rounds_restore_source_gates_and_return_all_origins() {
    for map in [1134, 1505, 617, 2759] {
        exercise_arena_channel(ArenaQueueMode::SoloShuffle, false, map);
    }
}

type PvpCompletionCheck = fn(&mut Harness, &[Traveler], &[Vec3], u64);

fn exercise_arena_channel_with_reconnect(
    mode: ArenaQueueMode,
    forfeit: bool,
    map_id: u32,
    reconnect: Option<ReconnectCase>,
) {
    exercise_arena_channel_with_completion(mode, forfeit, map_id, reconnect, None);
}

fn exercise_arena_channel_with_completion(
    mode: ArenaQueueMode,
    forfeit: bool,
    map_id: u32,
    reconnect: Option<ReconnectCase>,
    after_complete: Option<PvpCompletionCheck>,
) {
    use shared::protocol::{ArenaQueueMode, QueueForArena};
    let rated = matches!(mode, ArenaQueueMode::Rated2v2 | ArenaQueueMode::Rated3v3);
    let team_size = match mode {
        ArenaQueueMode::Skirmish2v2 | ArenaQueueMode::Rated2v2 => 2,
        ArenaQueueMode::Skirmish3v3 | ArenaQueueMode::Rated3v3 => 3,
        ArenaQueueMode::SoloShuffle => 3,
    };
    let bracket = if team_size == 2 {
        PvpBracketSnapshot::Arena2v2
    } else {
        PvpBracketSnapshot::Arena3v3
    };
    let source = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let catalog_db = Connection::open_in_memory().unwrap();
    catalog_db
        .execute(
            "ATTACH DATABASE ? AS source",
            [crate::test_data::world_db_path().to_string_lossy().as_ref()],
        )
        .unwrap();
    catalog_db
        .execute(
            "CREATE TABLE arena_map AS SELECT * FROM source.arena_map WHERE map_id=?",
            [map_id],
        )
        .unwrap();
    let (start_a, start_b): (u32, u32) = catalog_db
        .query_row("SELECT start_a,start_b FROM arena_map", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    let catalog = crate::pvp::arena::Catalog::load(&catalog_db).unwrap();
    let gates = crate::pvp::arena_objects::GateCatalog::load(&source).unwrap();
    let dalaran = crate::pvp::arena_dalaran::Catalog::load(&source).unwrap();
    let cage = crate::pvp::arena_cage::Catalog::load(&source).unwrap();
    let gate_actions: Vec<(u32, String, u32)> = source
        .prepare("SELECT entry,action,despawn_secs FROM arena_gate WHERE map_id=?")
        .unwrap()
        .query_map([map_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert!(!gate_actions.is_empty(), "map{map_id} source gates");
    let mut entries: Vec<_> = gate_actions
        .iter()
        .map(|(entry, _, _)| *entry)
        .chain([184663, 184664])
        .collect();
    if map_id == 617 {
        entries.extend([191877, 194395]);
    }
    let objects = source_map_objects(map_id, &entries);
    let sight = crate::pvp::arena_objects::SightCatalog::load(&source, &objects).unwrap();
    let source_orb = objects
        .0
        .iter()
        .find(|spawn| spawn.info.entry == 184663)
        .unwrap()
        .clone();
    let store = crate::spell_cast::test_harness::world_db().unwrap();
    let spells = crate::spell_info::SpellStore::fixture(
        [
            32727, 34709, 57405, 96538, 96539, 58873, 254013, 48018, 228212, 1214676, 371319,
        ]
        .iter()
        .map(|id| store.store.get(*id).unwrap().clone())
        .collect(),
    );
    let database_file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(database_file.path()).unwrap();
    crate::persistence::create_tables(&db);
    let world_data = crate::test_data::world_data().clone();
    let classes = (mode == ArenaQueueMode::SoloShuffle).then(|| {
        crate::spell_cast::test_harness::world_db()
            .unwrap()
            .classes
            .clone()
    });
    let achievements = crate::test_data::achievement_catalog();
    let mut h = harness_with(move |app| {
        if let Some(classes) = classes {
            app.insert_resource(classes);
        }
        app.insert_resource(crate::persistence::GameDatabase::new(db));
        if reconnect.is_some() {
            crate::networking::install_disconnect_fixture(app);
            app.add_systems(Update, crate::auth_character::process_select_character);
        }
        app.insert_resource(achievements);
        app.insert_resource(objects);
        app.insert_resource(world_data);
        app.insert_resource(spells);
        app.insert_resource(crate::faction::world_db_faction_templates().unwrap());
        app.add_plugins((
            crate::interest::InterestPlugin,
            crate::pvp::PvpPlugin,
            crate::spell_cast::SpellCastPlugin,
            crate::auras::AuraPlugin,
            crate::combat::CombatPlugin,
            crate::death::DeathPlugin,
            crate::achievement::AchievementPlugin,
        ));
        app.insert_resource(catalog);
        app.insert_resource(gates);
        app.insert_resource(sight);
        app.insert_resource(dalaran);
        app.insert_resource(cage);
    })
    .unwrap();
    let origin = STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0);
    let players: Vec<_> = (1..=(team_size * 2) as u64)
        .map(|id| h.spawn_at_level(id, 90, STORMWIND, origin + Vec3::new(id as f32, 0.0, 0.0)))
        .collect();
    let origins: Vec<_> = players
        .iter()
        .map(|who| wow(h.app.world().get::<Position>(who.player).unwrap()))
        .collect();
    for (index, who) in players.iter().enumerate() {
        if reconnect.is_some() {
            let id = index as u64 + 1;
            let db = &h
                .app
                .world()
                .resource::<crate::persistence::GameDatabase>()
                .db;
            let name = ["Reconone", "Recontwo", "Reconthree", "Reconfour"][index];
            let (created_id, mut data) =
                crate::persistence::create_character(db, id, name, 1, 1).unwrap();
            assert_eq!(created_id, id);
            data.level = 90;
            crate::persistence::save_character(db, id, &data);
            h.app
                .world_mut()
                .get_mut::<Player>(who.player)
                .unwrap()
                .name = name.into();
        }
        h.app.world_mut().entity_mut(who.player).insert((
            crate::pvp::PlayerPvpState::default(),
            crate::death::PlayerDeathState::default(),
            shared::components::CurrentPvpFaction(2),
            crate::achievement::PlayerAchievementState::default(),
            crate::spell_cast::powers::PowerPools {
                order: vec![shared::components::PowerType::Rage],
                pools: vec![crate::spell_cast::powers::PowerPool {
                    power: shared::components::PowerType::Rage,
                    current: 70.0,
                    max: 100.0,
                    regen_per_sec: 0.0,
                }],
                ..Default::default()
            },
        ));
        h.app.world_mut().entity_mut(who.connection).insert((
            pair::<QueueForArena>(),
            pair::<shared::protocol::QueueForRatedPvp>(),
            pair::<RespondToBattlegroundInvite>(),
            pair::<LeaveBattleground>(),
            pair::<PvpStateUpdate>(),
            pair::<shared::protocol::AttackSwing>(),
        ));
        if mode == ArenaQueueMode::SoloShuffle {
            let healer = index < 2;
            h.app
                .world_mut()
                .get_mut::<Player>(who.player)
                .unwrap()
                .class = if healer { 5 } else { 1 };
            h.app
                .world_mut()
                .entity_mut(who.player)
                .insert(shared::components::ActiveSpec(if healer {
                    257
                } else {
                    71
                }));
        }
        if !rated {
            h.app
                .world_mut()
                .get_mut::<MessageSender<QueueForArena>>(who.connection)
                .unwrap()
                .send::<PvpChannel>(QueueForArena { mode });
        }
    }
    if rated {
        let online: OnlineView = players
            .iter()
            .enumerate()
            .map(|(index, who)| {
                (
                    (index + 1) as u64,
                    OnlineMember {
                        entity: who.player.to_bits(),
                        name: format!("Traveler{}", index + 1),
                        class: 1,
                        level: 90,
                        portrait: Default::default(),
                        dungeon_difficulty: 1,
                        raid_difficulty: Default::default(),
                    },
                )
            })
            .collect();
        let mut groups = h.app.world_mut().resource_mut::<GroupRegistry>();
        groups.sync_presence(&online);
        for member in 3..=(team_size * 2) as u64 {
            let leader = if member % 2 == 1 { 1 } else { 2 };
            groups.invite(leader, &format!("Traveler{member}"), &online);
            groups.respond_invite(member, true);
        }
        assert_eq!(groups.group_members(1).len(), team_size);
        assert_eq!(groups.group_members(2).len(), team_size);
        for who in &players[..2] {
            h.app
                .world_mut()
                .get_mut::<MessageSender<shared::protocol::QueueForRatedPvp>>(who.connection)
                .unwrap()
                .send::<PvpChannel>(shared::protocol::QueueForRatedPvp {
                    bracket: bracket.clone(),
                });
        }
    }
    // Dead entrants are resurrected by the source battlefield-port flow.
    let dead_entrant = players.last().unwrap().player;
    h.app
        .world_mut()
        .get_mut::<Health>(dead_entrant)
        .unwrap()
        .current = 0.0;
    let updates = h.receive_pvp_updates(5);
    let invitations: Vec<_> = players
        .iter()
        .map(|who| {
            updates[&who.connection]
                .iter()
                .filter_map(|u| u.snapshot.as_ref()?.battleground.clone())
                .last()
                .unwrap_or_else(|| {
                    panic!("missing arena invitation: {:?}", updates[&who.connection])
                })
        })
        .collect();
    let match_id = invitations[0].match_id;
    for (who, invite) in players.iter().zip(&invitations) {
        assert_eq!(invite.map_id, map_id);
        assert_eq!(invite.match_id, match_id);
        assert_eq!(
            h.app
                .world()
                .get::<shared::components::CurrentPvpFaction>(who.player),
            Some(&shared::components::CurrentPvpFaction(2)),
            "arena invitation keeps racial faction"
        );
        h.app
            .world_mut()
            .get_mut::<MessageSender<RespondToBattlegroundInvite>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(RespondToBattlegroundInvite {
                match_id,
                accepted: true,
            });
    }
    h.receive_pvp_updates(5);
    for (who, invite) in players.iter().zip(&invitations) {
        let expected = if invite.team == shared::battleground::BgTeam::A {
            2
        } else {
            1
        };
        assert_eq!(
            h.app
                .world()
                .get::<shared::components::CurrentPvpFaction>(who.player),
            Some(&shared::components::CurrentPvpFaction(expected)),
            "arena uses entered team, not race"
        );
        assert_eq!(h.map_of(who), None);
        let travel = h.new_worlds(who);
        assert_eq!(travel.len(), 1);
        assert_eq!(travel[0].map_id, map_id);
        let source_id = if invite.team == shared::battleground::BgTeam::A {
            start_a
        } else {
            start_b
        };
        let source_start = map_store().unwrap().safe_locs[&source_id];
        assert!(close(
            wow(&Position {
                x: travel[0].position[0],
                y: travel[0].position[1],
                z: travel[0].position[2]
            }),
            source_start.position
        ));
        h.ack(who);
    }
    h.receive_pvp_updates(5);
    let map = h.map_of(&players[0]).unwrap();
    assert!(players.iter().all(|who| h.map_of(who) == Some(map)));
    let revived = h.app.world().get::<Health>(dead_entrant).unwrap();
    assert_eq!(
        revived.current, revived.max,
        "source battlefield-port full resurrection before preparation"
    );
    assert_eq!(
        h.app
            .world()
            .get::<crate::death::PlayerDeathState>(dead_entrant)
            .unwrap()
            .state,
        DeathState::Alive
    );
    let object_count = |world: &mut World, kind: u8| {
        world
            .query::<(
                &InMap,
                &shared::protocol::GameObjectInfo,
                Option<&shared::protocol::GameObjectVisible>,
            )>()
            .iter(world)
            .filter(|(copy, info, visible)| {
                **copy == map && info.go_type == kind && visible.is_none_or(|visible| visible.0)
            })
            .count()
    };
    assert_eq!(
        object_count(h.app.world_mut(), 6),
        0,
        "orbs hidden during preparation"
    );
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(players[0].player)
            .unwrap()
            .iter()
            .any(|a| a.spell_id == 32727)
    );
    let source_gates: Vec<_> = h
        .app
        .world_mut()
        .query::<(Entity, &InMap, &shared::protocol::GameObjectInfo)>()
        .iter(h.app.world())
        .filter(|(_, copy, info)| {
            **copy == map
                && gate_actions
                    .iter()
                    .any(|(entry, _, _)| *entry == info.entry)
        })
        .map(|(entity, _, info)| (entity, info.entry))
        .collect();
    for (entry, _, _) in &gate_actions {
        assert!(
            source_gates.iter().any(|(_, actual)| actual == entry),
            "map{map_id} missing source gate{entry}"
        );
    }
    if map_id == 2759 {
        let position = h.app.world().get::<Position>(players[0].player).unwrap();
        let gate = source_gates
            .iter()
            .min_by(|(a, _), (b, _)| {
                let distance = |entity| {
                    let gate = h.app.world().get::<Position>(entity).unwrap();
                    Vec3::new(
                        gate.x - position.x,
                        gate.y - position.y,
                        gate.z - position.z,
                    )
                    .length_squared()
                };
                distance(*a).total_cmp(&distance(*b))
            })
            .unwrap()
            .0;
        assert!(
            h.app
                .world()
                .get::<crate::interest::NetworkVisibility>(gate)
                .unwrap()
                .is_visible(players[0].connection),
            "nearby Cage gate must remain visible through actual32727 preparation"
        );
        for who in &players {
            assert!(
                crate::auras::has_aura(h.app.world(), who.player, 228212),
                "source entrance areatrigger marks every Cage entrant during preparation"
            );
        }
    }
    for (gate, entry) in &source_gates {
        let (_, action, _) = gate_actions
            .iter()
            .find(|(source, _, _)| source == entry)
            .unwrap();
        assert_eq!(
            h.app
                .world()
                .get::<shared::protocol::GameObjectCollision>(*gate),
            Some(&shared::protocol::GameObjectCollision(true)),
            "source gates and collision walls block during preparation"
        );
        if action == "delete" {
            continue; // Empyrean collision walls exist without a door-animation contract.
        }
        assert_eq!(
            h.app
                .world()
                .get::<shared::protocol::GameObjectAnimation>(*gate)
                .unwrap()
                .animation_id,
            147
        );
    }
    if mode == ArenaQueueMode::SoloShuffle {
        exercise_shuffle_rounds(
            &mut h,
            &players,
            &origins,
            match_id,
            map,
            &gate_actions,
            after_complete,
        );
        return;
    }
    let preparing = players[0].player;
    assert_eq!(
        crate::spell_cast::powers::power_value(
            h.app.world(),
            preparing,
            shared::components::PowerType::Rage
        )
        .unwrap()
        .0,
        0.0
    );
    h.app
        .world_mut()
        .get_mut::<Health>(preparing)
        .unwrap()
        .current = 25.0;
    crate::spell_cast::powers::change_power(
        h.app.world_mut(),
        preparing,
        shared::components::PowerType::Rage,
        60.0,
    );
    h.receive_pvp_updates(110);
    let reset_health = h.app.world().get::<Health>(preparing).unwrap();
    assert_eq!(
        reset_health.current, reset_health.max,
        "source preparation periodic full health"
    );
    assert_eq!(
        crate::spell_cast::powers::power_value(
            h.app.world(),
            preparing,
            shared::components::PowerType::Rage
        )
        .unwrap()
        .0,
        0.0,
        "source preparation resets primary rage"
    );
    h.receive_pvp_updates(1091);
    assert_eq!(
        h.app
            .world()
            .get::<crate::pvp::PlayerPvpState>(players[0].player)
            .unwrap()
            .battleground
            .as_ref()
            .unwrap()
            .phase,
        shared::battleground::BgPhase::InProgress
    );
    assert!(
        !h.app
            .world()
            .get::<crate::auras::ActiveAuras>(players[0].player)
            .unwrap()
            .iter()
            .any(|a| a.spell_id == 32727)
    );
    for (gate, entry) in &source_gates {
        let (_, action, _) = gate_actions
            .iter()
            .find(|(source, _, _)| source == entry)
            .unwrap();
        if action == "delete" {
            assert!(
                h.app.world().get_entity(*gate).is_err(),
                "map{map_id} source wall removed at start"
            );
        } else {
            assert_eq!(
                h.app
                    .world()
                    .get::<shared::protocol::GameObjectCollision>(*gate),
                Some(&shared::protocol::GameObjectCollision(false)),
                "source open gate disables model collision"
            );
            assert_eq!(
                h.app
                    .world()
                    .get::<shared::protocol::GameObjectAnimation>(*gate)
                    .unwrap()
                    .animation_id,
                148,
                "map{map_id} source door open"
            );
        }
    }
    if map_id == 617 {
        h.receive_pvp_updates(700);
        let collision = h
            .app
            .world_mut()
            .query::<(
                &InMap,
                &shared::protocol::GameObjectInfo,
                &shared::protocol::GameObjectCollision,
            )>()
            .iter(h.app.world())
            .find(|(copy, info, _)| **copy == map && info.entry == 194395)
            .map(|(_, _, state)| state.0)
            .unwrap();
        assert!(
            collision,
            "TC waterfall collision activates after30s warning+5s"
        );
        h.receive_pvp_updates(600);
        let collision = h
            .app
            .world_mut()
            .query::<(
                &InMap,
                &shared::protocol::GameObjectInfo,
                &shared::protocol::GameObjectCollision,
            )>()
            .iter(h.app.world())
            .find(|(copy, info, _)| **copy == map && info.entry == 194395)
            .map(|(_, _, state)| state.0)
            .unwrap();
        assert!(!collision, "TC waterfall resets after30s active duration");
        h.receive_pvp_updates(400);
    } else {
        h.receive_pvp_updates(1700);
    }
    assert_eq!(
        object_count(h.app.world_mut(), 6),
        0,
        "before source90second activation"
    );
    if let Some(case) = reconnect {
        exercise_pvp_reconnect(&mut h, &players, &origins, case, database_file.path());
        return;
    }
    h.receive_pvp_updates(101);
    assert_eq!(object_count(h.app.world_mut(), 6), 2);
    for (gate, entry) in &source_gates {
        let (_, action, despawn) = gate_actions
            .iter()
            .find(|(source, _, _)| source == entry)
            .unwrap();
        assert_eq!(
            h.app.world().get_entity(*gate).is_ok(),
            action != "delete" && *despawn == 0,
            "map{map_id} gate{entry} source lifetime"
        );
    }
    h.app
        .world_mut()
        .entity_mut(players[0].player)
        .insert(bevy(source_orb.position));
    h.receive_pvp_updates(5);
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(players[0].player)
            .unwrap()
            .iter()
            .any(|a| a.spell_id == 34709)
    );
    assert_eq!(
        object_count(h.app.world_mut(), 6),
        1,
        "physical authored orb consumed"
    );
    let caster = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(players[0].player)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 34709)
        .unwrap()
        .caster;
    assert!(
        h.app.world().get_entity(caster).is_ok(),
        "hidden orb retains aura ownership"
    );
    if forfeit {
        exercise_arena_forfeit(&mut h, &players, &origins, match_id);
        return;
    }
    for who in &players {
        assert!(
            !h.app
                .world()
                .get::<crate::achievement::PlayerAchievementState>(who.player)
                .unwrap()
                .criteria_progress
                .keys()
                .any(|id| [306, 8613, 8617, 8605, 8609].contains(id)),
            "queue, invitation and preparation cannot grant a rated result"
        );
    }
    let attacker = players[0].player;
    for target in players.iter().skip(1).step_by(2).map(|who| who.player) {
        h.app
            .world_mut()
            .get_mut::<Position>(target)
            .unwrap()
            .clone_from(&bevy(source_orb.position + Vec3::new(1.0, 0.0, 0.0)));
        h.app.world_mut().get_mut::<Health>(target).unwrap().current = 1.0;
        h.app
            .world_mut()
            .get_mut::<MessageSender<shared::protocol::AttackSwing>>(players[0].connection)
            .unwrap()
            .send::<shared::protocol::CombatChannel>(shared::protocol::AttackSwing {
                target: target.to_bits(),
            });
        h.receive_pvp_updates(50);
    }
    let ended = h
        .app
        .world()
        .get::<crate::pvp::PlayerPvpState>(attacker)
        .unwrap()
        .battleground
        .as_ref()
        .unwrap()
        .clone();
    assert_eq!(
        ended.outcome,
        Some(shared::battleground::BgOutcome::Winner(
            shared::battleground::BgTeam::A
        ))
    );
    assert_eq!(ended.players.len(), team_size * 2);
    let killer = ended
        .players
        .iter()
        .find(|player| player.character_id == 1)
        .unwrap();
    assert_eq!(killer.killing_blows, team_size as u32);
    assert!(
        killer.damage_done > 0,
        "real combat events populate arena damage scoreboard"
    );
    // TC Arena.cpp249: only online rated winners receive WinAnyRankedArena37.
    for (index, who) in players.iter().enumerate() {
        let state = h
            .app
            .world()
            .get::<crate::achievement::PlayerAchievementState>(who.player)
            .unwrap();
        let expected = (rated && index % 2 == 0).then_some(1);
        assert_eq!(state.criteria_progress.get(&306).copied(), expected);
        assert_eq!(state.earned_ids.contains(&397), expected.is_some());
        assert_rated_arena_map_credits(state, rated && map_id == 572, index % 2 == 0, team_size);
        assert_rated_arena_combat_credits(state, rated, index, team_size);
        assert_arena_map_type_credits(state, index, team_size);
    }
    h.receive_pvp_updates(5);
    for (index, who) in players.iter().enumerate() {
        let state = h
            .app
            .world()
            .get::<crate::achievement::PlayerAchievementState>(who.player)
            .unwrap();
        let expected = (rated && index % 2 == 0).then_some(1);
        assert_eq!(
            state.criteria_progress.get(&306).copied(),
            expected,
            "idle replay"
        );
        assert_rated_arena_map_credits(state, rated && map_id == 572, index % 2 == 0, team_size);
        assert_rated_arena_combat_credits(state, rated, index, team_size);
        assert_arena_map_type_credits(state, index, team_size);
    }
    if rated {
        assert_eq!(ended.arena.as_ref().unwrap().mode, mode);
        assert_eq!(
            ended.arena.as_ref().unwrap().rating_changes.len(),
            team_size * 2
        );
        for (index, who) in players.iter().enumerate() {
            let progress = &h
                .app
                .world()
                .get::<crate::pvp::PlayerPvpState>(who.player)
                .unwrap()
                .progress;
            let entry = progress
                .brackets
                .iter()
                .find(|entry| entry.bracket == bracket)
                .unwrap();
            let won = index % 2 == 0;
            assert_eq!(entry.rating, Some(if won { 48 } else { 0 }));
            assert_eq!(entry.matchmaker_rating, Some(if won { 1512 } else { 1488 }));
            assert_eq!(entry.season_wins, u32::from(won));
            assert_eq!(entry.season_losses, u32::from(!won));
            assert_eq!((progress.honor, progress.conquest), (0, 0));
            let saved = crate::persistence::load_pvp(
                &h.app
                    .world()
                    .resource::<crate::persistence::GameDatabase>()
                    .db,
                (index + 1) as u64,
            );
            assert_eq!(saved, *progress);
        }
    }
    if let Some(check) = after_complete {
        check(&mut h, &players, &origins, match_id);
        return;
    }
    for who in &players {
        assert_eq!(
            h.app.world().get::<Health>(who.player).unwrap().current,
            h.app.world().get::<Health>(who.player).unwrap().max,
            "source full resurrection on match completion"
        );
        h.app
            .world_mut()
            .get_mut::<MessageSender<LeaveBattleground>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(LeaveBattleground { match_id });
    }
    h.receive_pvp_updates(5);
    for (who, origin) in players.iter().zip(&origins) {
        assert_eq!(
            h.app
                .world()
                .get::<shared::components::CurrentPvpFaction>(who.player),
            Some(&shared::components::CurrentPvpFaction(2)),
            "arena return restores race"
        );
        h.ack(who);
        assert_eq!(h.map_of(who), Some(STORMWIND));
        assert!(close(
            wow(h.app.world().get::<Position>(who.player).unwrap()),
            *origin
        ));
        assert!(
            h.app
                .world()
                .get::<crate::pvp::PlayerPvpState>(who.player)
                .unwrap()
                .battleground
                .is_none()
        );
    }
    drop(h);
    let reopened = redb::Database::open(database_file.path()).unwrap();
    for index in 0..team_size * 2 {
        let saved = crate::persistence::load_achievements(&reopened, (index + 1) as u64);
        let expected = (rated && index % 2 == 0).then_some(1);
        assert_eq!(
            saved.criteria_progress.get(&306).copied(),
            expected,
            "redb reopen"
        );
        assert_rated_arena_map_credits(&saved, rated && map_id == 572, index % 2 == 0, team_size);
        assert_rated_arena_combat_credits(&saved, rated, index, team_size);
        assert_arena_map_type_credits(&saved, index, team_size);
    }
}

fn exercise_shuffle_rounds(
    h: &mut Harness,
    players: &[Traveler],
    origins: &[Vec3],
    match_id: u64,
    map: InMap,
    gates: &[(u32, String, u32)],
    after_complete: Option<PvpCompletionCheck>,
) {
    use shared::battleground::{BgPhase, BgTeam};
    let mut combinations = std::collections::BTreeSet::new();
    for round in 1..=6 {
        let matched = h
            .app
            .world()
            .get::<crate::pvp::PlayerPvpState>(players[0].player)
            .unwrap()
            .battleground
            .as_ref()
            .unwrap()
            .clone();
        let arena = matched.arena.as_ref().unwrap();
        let shuffle = arena.shuffle.as_ref().expect("six-round state");
        assert_eq!(shuffle.round, round);
        assert_eq!(matched.phase, BgPhase::Waiting);
        let team_a: Vec<_> = arena
            .units
            .iter()
            .filter(|unit| unit.team == BgTeam::A)
            .map(|unit| unit.character_id)
            .collect();
        assert_eq!(team_a.len(), 3);
        assert_eq!(
            arena
                .units
                .iter()
                .filter(|unit| unit.team == BgTeam::B)
                .count(),
            3
        );
        combinations.insert(team_a);
        for who in players {
            assert_eq!(h.map_of(who), Some(map));
            let health = h.app.world().get::<Health>(who.player).unwrap();
            assert_eq!(health.current, health.max, "every round resets health");
            assert!(crate::auras::has_aura(h.app.world(), who.player, 32727));
        }
        let present: Vec<_> = h
            .app
            .world_mut()
            .query::<(
                &InMap,
                &shared::protocol::GameObjectInfo,
                &shared::protocol::GameObjectCollision,
            )>()
            .iter(h.app.world())
            .filter(|(copy, _, _)| **copy == map)
            .map(|(_, info, collision)| (info.entry, collision.0))
            .collect();
        for (entry, _, _) in gates {
            assert!(
                present.contains(&(*entry, true)),
                "deleted/open gate{entry} restored in round{round}"
            );
        }
        h.receive_pvp_updates(1205);
        assert_eq!(
            h.app
                .world()
                .get::<crate::pvp::PlayerPvpState>(players[0].player)
                .unwrap()
                .battleground
                .as_ref()
                .unwrap()
                .phase,
            BgPhase::InProgress
        );
        h.receive_pvp_updates(110); // source5s door despawns must actually occur before reset
        let victim_id = matched
            .arena
            .as_ref()
            .unwrap()
            .units
            .iter()
            .find(|unit| unit.team == BgTeam::B)
            .unwrap()
            .character_id;
        let victim = players
            .iter()
            .find(|who| {
                h.app
                    .world()
                    .get::<crate::persistence::ClientId>(who.player)
                    .unwrap()
                    .0
                    == victim_id
            })
            .unwrap();
        h.app
            .world_mut()
            .get_mut::<Health>(victim.player)
            .unwrap()
            .current = 0.0;
        h.receive_pvp_updates(5);
        let matched = h
            .app
            .world()
            .get::<crate::pvp::PlayerPvpState>(players[0].player)
            .unwrap()
            .battleground
            .as_ref()
            .unwrap()
            .clone();
        assert_eq!(
            matched.phase,
            if round == 6 {
                BgPhase::Ended
            } else {
                BgPhase::Waiting
            },
            "first kill finishes a round, not team elimination"
        );
        assert!(matched.arena.as_ref().unwrap().rating_changes.is_empty());
        assert_eq!(
            matched
                .arena
                .as_ref()
                .unwrap()
                .shuffle
                .as_ref()
                .unwrap()
                .rounds_won
                .iter()
                .map(|(_, wins)| u32::from(*wins))
                .sum::<u32>(),
            u32::from(round) * 3
        );
        for who in players {
            let snapshot = h
                .app
                .world()
                .get::<crate::pvp::PlayerPvpState>(who.player)
                .unwrap()
                .battleground
                .as_ref()
                .unwrap();
            let id = h
                .app
                .world()
                .get::<crate::persistence::ClientId>(who.player)
                .unwrap()
                .0;
            let expected = matched
                .arena
                .as_ref()
                .unwrap()
                .units
                .iter()
                .find(|unit| unit.character_id == id)
                .unwrap()
                .team;
            assert_eq!(snapshot.team, expected, "replicated team follows rotation");
            assert!(crate::auras::has_aura(h.app.world(), who.player, 32727) == (round < 6));
        }
    }
    assert_eq!(combinations.len(), 6);
    let snapshot = h
        .app
        .world()
        .get::<crate::pvp::PlayerPvpState>(players[0].player)
        .unwrap()
        .battleground
        .as_ref()
        .unwrap();
    assert_eq!(
        snapshot
            .arena
            .as_ref()
            .unwrap()
            .shuffle
            .as_ref()
            .unwrap()
            .rounds_won,
        vec![(1, 6), (2, 0), (3, 3), (4, 3), (5, 3), (6, 3)]
    );
    if let Some(check) = after_complete {
        check(h, players, origins, match_id);
        return;
    }
    for (index, who) in players.iter().enumerate() {
        let state = h
            .app
            .world()
            .get::<crate::achievement::PlayerAchievementState>(who.player)
            .unwrap();
        assert_rated_arena_combat_credits(state, false, index, 3);
        h.app
            .world_mut()
            .get_mut::<MessageSender<LeaveBattleground>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(LeaveBattleground { match_id });
    }
    h.receive_pvp_updates(5);
    for who in players {
        h.ack(who);
    }
    h.receive_pvp_updates(5);
    for (who, origin) in players.iter().zip(origins) {
        assert_eq!(h.map_of(who), Some(STORMWIND));
        assert!(close(
            wow(h.app.world().get::<Position>(who.player).unwrap()),
            *origin
        ));
        assert!(
            h.app
                .world()
                .get::<crate::pvp::PlayerPvpState>(who.player)
                .unwrap()
                .battleground
                .is_none()
        );
    }
}

fn assert_arena_map_type_credits(
    state: &crate::achievement::PlayerAchievementState,
    index: usize,
    team_size: usize,
) {
    let expected = (index == 0).then_some(team_size as u64);
    assert_eq!(
        state.criteria_progress.get(&50457).copied(),
        expected,
        "arena killing-blow map type"
    );
    if index == 0 {
        assert_eq!(
            state.criteria_progress.get(&50456).copied(),
            expected,
            "arena kill reward map type"
        );
    }
    assert!(
        !state.criteria_progress.contains_key(&50458),
        "arena is not battleground map type3"
    );
    assert!(
        !state.criteria_progress.contains_key(&50460),
        "arena deaths are not battleground map type3"
    );
}

fn assert_rated_arena_combat_credits(
    state: &crate::achievement::PlayerAchievementState,
    rated: bool,
    index: usize,
    team_size: usize,
) {
    for (size, blow, death, kill) in [(2, 5541, 4962, 4959), (3, 5542, 4963, 4960)] {
        let expected_blow = (rated && team_size == size && index == 0).then_some(team_size as u64);
        let expected_death = (rated && team_size == size && index % 2 == 1).then_some(1);
        assert_eq!(
            state.criteria_progress.get(&blow).copied(),
            expected_blow,
            "rated{size} killing blows"
        );
        assert_eq!(
            state.criteria_progress.get(&death).copied(),
            expected_death,
            "rated{size} deaths"
        );
        if index == 0 {
            assert_eq!(
                state.criteria_progress.get(&kill).copied(),
                expected_blow,
                "rated{size} native kill rewards"
            );
        }
    }
}

fn assert_rated_arena_map_credits(
    state: &crate::achievement::PlayerAchievementState,
    rated: bool,
    won: bool,
    team_size: usize,
) {
    for (size, participate, win) in [(2, 8613, 8617), (3, 8605, 8609), (5, 8597, 8601)] {
        assert_eq!(
            state.criteria_progress.get(&participate).copied(),
            (rated && team_size == size).then_some(1),
            "rated Ruins participation/{size}"
        );
        assert_eq!(
            state.criteria_progress.get(&win).copied(),
            (rated && won && team_size == size).then_some(1),
            "rated Ruins win/{size}"
        );
    }
    assert!(
        !state.criteria_progress.contains_key(&5728),
        "Ruins cannot credit Blade's Edge"
    );
}

fn exercise_arena_forfeit(h: &mut Harness, players: &[Traveler], origins: &[Vec3], match_id: u64) {
    for who in players.iter().skip(1).step_by(2) {
        h.app
            .world_mut()
            .get_mut::<MessageSender<LeaveBattleground>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(LeaveBattleground { match_id });
    }
    h.receive_pvp_updates(5);
    for who in players.iter().skip(1).step_by(2) {
        h.ack(who);
    }
    h.receive_pvp_updates(5);
    let ended = h
        .app
        .world()
        .get::<crate::pvp::PlayerPvpState>(players[0].player)
        .unwrap()
        .battleground
        .as_ref()
        .unwrap();
    assert_eq!(
        ended.outcome,
        Some(shared::battleground::BgOutcome::Winner(
            shared::battleground::BgTeam::A
        ))
    );
    assert_eq!(ended.players.len(), 4);
    assert!(
        ended.players.iter().all(|score| score.deaths == 0),
        "leaving alive is not a combat death: {:?}",
        ended.players
    );
    for who in players.iter().step_by(2) {
        h.app
            .world_mut()
            .get_mut::<MessageSender<LeaveBattleground>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(LeaveBattleground { match_id });
    }
    h.receive_pvp_updates(5);
    for who in players.iter().step_by(2) {
        h.ack(who);
    }
    for (who, origin) in players.iter().zip(origins) {
        assert_eq!(h.map_of(who), Some(STORMWIND));
        assert!(close(
            wow(h.app.world().get::<Position>(who.player).unwrap()),
            *origin
        ));
        assert!(
            h.app
                .world()
                .get::<crate::pvp::PlayerPvpState>(who.player)
                .unwrap()
                .battleground
                .is_none()
        );
    }
}

#[test]
fn ten_warsong_players_accept_over_the_channel_then_ack_one_reserved_map() {
    flag_match_over_the_channel(shared::battleground::warsong_gulch(), "warsonggulch2");
}

#[test]
fn ten_twin_peaks_players_capture_three_source_flags_over_the_channel() {
    flag_match_over_the_channel(shared::battleground::twin_peaks(), "cataclysmctf");
}

#[test]
fn battleground_disconnect_rejoins_reserved_copy_and_restores_mercenary_team() {
    flag_match_with_reconnect(
        shared::battleground::warsong_gulch(),
        "warsonggulch2",
        Some(ReconnectCase::SurvivingMatch),
    );
}

/// Source-minimum roster queue/invite/entry, three physical captures and origin return.
fn flag_match_over_the_channel(template: shared::battleground::BgTemplate, directory: &str) {
    flag_match_with_reconnect(template, directory, None);
}

fn flag_match_with_reconnect(
    template: shared::battleground::BgTemplate,
    directory: &str,
    reconnect: Option<ReconnectCase>,
) {
    flag_match_with_completion(template, directory, reconnect, None);
}

fn flag_match_with_completion(
    template: shared::battleground::BgTemplate,
    directory: &str,
    reconnect: Option<ReconnectCase>,
    after_complete: Option<PvpCompletionCheck>,
) {
    let rules = shared::battleground::flag_rules(template.map_id).unwrap();
    let flags = source_battleground_objects(template.map_id);
    assert_eq!(flags.0.len(), 2, "both authored team flags");
    let source_flags = flags.clone();
    let database_file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(database_file.path()).unwrap();
    crate::persistence::create_tables(&db);
    let mut h = harness_with(|app| {
        app.insert_resource(crate::persistence::GameDatabase::new(db));
        app.insert_resource(flags);
        if reconnect.is_some() {
            crate::networking::install_disconnect_fixture(app);
            app.add_systems(Update, crate::auth_character::process_select_character);
        }
        app.insert_resource(crate::faction::FactionTemplates::default());
        app.add_plugins((
            crate::pvp::PvpPlugin,
            crate::npc_interaction::NpcInteractionPlugin,
        ));
    })
    .unwrap();
    let origin = STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0);
    let roster = 2 * u64::from(template.min_players_per_team);
    let players: Vec<_> = (1..=roster)
        .map(|id| h.spawn(id, STORMWIND, origin))
        .collect();
    let origins: Vec<_> = players
        .iter()
        .map(|who| wow(h.app.world().get::<Position>(who.player).unwrap()))
        .collect();
    for (index, who) in players.iter().enumerate() {
        if reconnect.is_some() {
            let id = index as u64 + 1;
            let names = [
                "Wsgalpha",
                "Wsgbravo",
                "Wsgcharlie",
                "Wsgdelta",
                "Wsgecho",
                "Wsgfoxtrot",
                "Wsggolf",
                "Wsghotel",
                "Wsgindia",
                "Wsgjuliet",
            ];
            let db = &h
                .app
                .world()
                .resource::<crate::persistence::GameDatabase>()
                .db;
            let (created_id, mut data) =
                crate::persistence::create_character(db, id, names[index], 1, 1).unwrap();
            assert_eq!(created_id, id);
            data.level = u16::from(TRAVELER_LEVEL);
            crate::persistence::save_character(db, id, &data);
            h.app
                .world_mut()
                .get_mut::<Player>(who.player)
                .unwrap()
                .name = names[index].into();
        }
        h.app.world_mut().entity_mut(who.player).insert((
            crate::pvp::PlayerPvpState::default(),
            crate::currency::PlayerCurrencies::default(),
            shared::components::UnitFactionTemplate(crate::faction::faction_template_for_race(1)),
        ));
        h.app.world_mut().entity_mut(who.connection).insert((
            pair::<QueueForBattleground>(),
            pair::<RespondToBattlegroundInvite>(),
            pair::<LeaveBattleground>(),
            pair::<UseGameObject>(),
            pair::<PvpStateUpdate>(),
        ));
        h.app
            .world_mut()
            .get_mut::<MessageSender<QueueForBattleground>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(QueueForBattleground {
                battleground_id: template.id,
            });
    }
    let updates = h.receive_pvp_updates(5);
    let invitations: Vec<_> = players
        .iter()
        .map(|who| {
            updates[&who.connection]
                .iter()
                .filter_map(|update| update.snapshot.as_ref())
                .filter_map(|snapshot| snapshot.battleground.clone())
                .last()
                .expect("typed invitation")
        })
        .collect();
    let match_id = invitations[0].match_id;
    for (who, invitation) in players.iter().zip(&invitations) {
        assert_eq!(invitation.match_id, match_id);
        assert_eq!(invitation.map_id, template.map_id);
        assert_eq!(invitation.invitation_time_left_secs, None);
        assert!(!invitation.accepted);
        assert_eq!(h.map_of(who), Some(STORMWIND));
        h.app
            .world_mut()
            .get_mut::<MessageSender<RespondToBattlegroundInvite>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(RespondToBattlegroundInvite {
                match_id,
                accepted: true,
            });
    }
    h.receive_pvp_updates(5);
    for (who, invitation) in players.iter().zip(&invitations) {
        assert_eq!(
            h.map_of(who),
            None,
            "accepted player waits for WorldPortAck"
        );
        let messages = h.new_worlds(who);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].map_id, template.map_id);
        assert_eq!(messages[0].map_directory, directory);
        let safe_id = match invitation.team {
            shared::battleground::BgTeam::A => rules.starts[0],
            shared::battleground::BgTeam::B => rules.starts[1],
        };
        let destination = map_store().unwrap().safe_locs[&safe_id];
        assert!(close(
            wow(&Position {
                x: messages[0].position[0],
                y: messages[0].position[1],
                z: messages[0].position[2]
            }),
            destination.position
        ));
    }
    for who in &players {
        h.ack(who);
    }
    let shared_map = h.map_of(&players[0]).unwrap();
    assert_eq!(shared_map.map_id, template.map_id);
    assert_ne!(shared_map.instance_id, 0);
    for who in &players {
        assert_eq!(h.map_of(who), Some(shared_map));
    }
    assert!(
        h.app
            .world()
            .resource::<InstanceManager>()
            .lock_copies_in_use()
            .is_empty()
    );
    battleground_nodes::advance_source_preparation(&mut h);
    if let Some(case) = reconnect {
        exercise_pvp_reconnect(&mut h, &players, &origins, case, database_file.path());
        return;
    }
    let alliance = source_flags
        .0
        .iter()
        .find(|spawn| spawn.info.entry == WARSONG_ALLIANCE_FLAG_ENTRY)
        .unwrap();
    let horde = source_flags
        .0
        .iter()
        .find(|spawn| spawn.info.entry == WARSONG_HORDE_FLAG_ENTRY)
        .unwrap();
    let carrier = &players[0];
    for capture in 1..=3 {
        let flag_entity = {
            let world = h.app.world_mut();
            let mut objects = world.query::<(Entity, &GameObjectInfo, &InMap)>();
            objects
                .iter(world)
                .find_map(|(entity, info, map)| {
                    (info.entry == WARSONG_HORDE_FLAG_ENTRY && *map == shared_map).then_some(entity)
                })
                .expect("authored enemy flag spawned")
        };
        h.walk_to(carrier, horde.position);
        h.app
            .world_mut()
            .get_mut::<MessageSender<UseGameObject>>(carrier.connection)
            .unwrap()
            .send::<InteractionChannel>(UseGameObject {
                object: flag_entity.to_bits(),
            });
        h.receive_pvp_updates(5);
        let carried = h
            .app
            .world()
            .get::<crate::pvp::PlayerPvpState>(carrier.player)
            .unwrap()
            .battleground
            .as_ref()
            .unwrap();
        assert_eq!(
            carried.flags.flag_b,
            shared::battleground::FlagState::Carried { carrier: 1 }
        );
        h.walk_to(carrier, alliance.position);
        h.receive_pvp_updates(5);
        let scored = h
            .app
            .world()
            .get::<crate::pvp::PlayerPvpState>(carrier.player)
            .unwrap()
            .battleground
            .as_ref()
            .unwrap();
        assert_eq!(scored.score_a, capture);
        assert_eq!(scored.score_b, 0);
        assert_eq!(scored.flags.flag_b, shared::battleground::FlagState::AtBase);
        assert_eq!(
            scored.honor_reward, None,
            "never apply synthetic legacy rewards"
        );
        assert_eq!(scored.players.len() as u64, roster);
        let row = scored
            .players
            .iter()
            .find(|row| row.character_id == 1)
            .unwrap();
        assert!(
            row.objective_scores
                .contains(&(rules.capture_stat, capture))
        );
        assert!(row.objective_scores.contains(&(rules.return_stat, 0)));
        assert!(scored.world_states.contains(&(1581, capture as i32)));
    }
    for who in &players {
        let result = h
            .app
            .world()
            .get::<crate::pvp::PlayerPvpState>(who.player)
            .unwrap()
            .battleground
            .as_ref()
            .unwrap();
        assert_eq!(result.phase, shared::battleground::BgPhase::Ended);
        assert_eq!(
            result.outcome,
            Some(shared::battleground::BgOutcome::Winner(
                shared::battleground::BgTeam::A
            ))
        );
    }
    if let Some(check) = after_complete {
        check(&mut h, &players, &origins, match_id);
        return;
    }
    for who in &players {
        h.app
            .world_mut()
            .get_mut::<MessageSender<LeaveBattleground>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(LeaveBattleground { match_id });
    }
    h.receive_pvp_updates(5);
    for who in &players {
        assert_eq!(h.map_of(who), None, "return waits for WorldPortAck");
        let returns = h.new_worlds(who);
        assert_eq!(returns.len(), 1);
        assert_eq!(returns[0].map_id, 0);
        assert!(close(
            wow(&Position {
                x: returns[0].position[0],
                y: returns[0].position[1],
                z: returns[0].position[2]
            }),
            origin
        ));
        h.ack(who);
        assert_eq!(h.map_of(who), Some(STORMWIND));
        assert!(close(h.position(who), origin));
        assert!(
            h.app
                .world()
                .get::<crate::pvp::PlayerPvpState>(who.player)
                .unwrap()
                .battleground
                .is_none()
        );
    }
    assert!(
        h.app
            .world()
            .resource::<InstanceManager>()
            .copy_difficulty(shared_map)
            .is_none()
    );
}

#[test]
fn wailing_caverns_world_port_spawns_selected_gathering_nodes_in_each_copy() {
    let path = crate::test_data::world_db_path();
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut h = harness_with_content_and(|app| {
        app.init_resource::<crate::spawn_pools::PoolSpawns>();
        app.insert_resource(crate::gathering::load_data(&conn).unwrap());
    })
    .expect("authentic Wailing Caverns world data");
    let destination = WorldLocation {
        map_id: 43,
        position: Vec3::new(-151.139, 414.367, -72.6294),
        orientation: 0.0,
    };
    let pooled_guids: Vec<_> = h
        .app
        .world()
        .resource::<crate::game_objects::GameObjectSpawns>()
        .0
        .iter()
        .filter(|spawn| spawn.map_id == 43 && spawn.info.go_type == GAMEOBJECT_TYPE_GATHERING_NODE)
        .filter(|spawn| {
            h.app
                .world()
                .resource::<crate::world_data::WorldData>()
                .pools
                .gameobject_pool(spawn.guid)
                .is_some()
        })
        .map(|spawn| spawn.guid)
        .collect();
    assert!(
        !pooled_guids.is_empty(),
        "source map43 must contain pooled gathering nodes"
    );
    let mut copies = Vec::new();
    for character in [101, 102] {
        let player = h.spawn(character, STORMWIND, STOCKADE_EXIT_TARGET);
        assert!(transfer::teleport_to(
            h.app.world_mut(),
            player.player,
            destination,
            Default::default()
        ));
        assert_eq!(h.new_worlds(&player).pop().unwrap().map_id, 43);
        assert_eq!(h.map_of(&player), None);
        h.ack(&player);
        let map = h.map_of(&player).expect("ack enters Wailing Caverns copy");
        assert_eq!(map.map_id, 43);
        assert_ne!(map.instance_id, 0);
        let selected: std::collections::BTreeSet<_> = pooled_guids
            .iter()
            .copied()
            .filter(|guid| {
                h.app.world().resource::<crate::spawn_pools::PoolSpawns>().0[&map]
                    .spawns
                    .gameobject_spawned(*guid)
            })
            .collect();
        // Source pools may select no nodes; their candidates must be rejected after
        // initialization rather than crash before inspecting that selection.
        let world = h.app.world_mut();
        let spawned: std::collections::BTreeSet<_> = world
            .query::<(&InMap, &crate::gathering::NodeSource)>()
            .iter(world)
            .filter(|(copy, node)| **copy == map && pooled_guids.contains(&node.0.guid))
            .map(|(_, node)| node.0.guid)
            .collect();
        assert_eq!(
            spawned, selected,
            "only selected real nodes spawn into their own copy"
        );
        copies.push(map);
    }
    assert_ne!(
        copies[0], copies[1],
        "unrelated entrants get isolated copies"
    );
}

#[test]
fn walking_into_the_stockade_entrance_transfers_the_player_into_a_copy_of_map_34() {
    let Some(mut h) = harness() else { return };
    let player = h.spawn(
        1,
        STORMWIND,
        STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0),
    );
    h.walk_to(&player, STOCKADE_ENTRANCE_TRIGGER);

    // Out of map 0, waiting for the client.
    assert_eq!(h.map_of(&player), None);
    let new_world = h.new_worlds(&player).pop().expect("NewWorld");
    assert_eq!(new_world.map_id, STOCKADE);
    assert_eq!(new_world.map_directory, "stormwindjail");
    assert!(close(
        wow(&Position {
            x: new_world.position[0],
            y: new_world.position[1],
            z: new_world.position[2],
        }),
        STOCKADE_ARRIVAL
    ));

    h.ack(&player);
    let map = h.map_of(&player).expect("in map 34");
    assert_eq!(map.map_id, STOCKADE);
    assert_ne!(map.instance_id, 0);
    assert!(close(h.position(&player), STOCKADE_ARRIVAL));
    let epoch = h
        .app
        .world()
        .get::<MovementControl>(player.player)
        .unwrap()
        .epoch;
    assert_eq!(epoch, 1, "the client adopts the arrival position");
}

/// `HandleMoveWorldportAck` sends the arriving player the destination's objects; the
/// `WorldArrival` count rises in the frame that makes them visible, so both go out in
/// one replication tick and the client knows its arrival is complete.
#[test]
fn the_arrival_count_rises_in_the_frame_the_stockade_creatures_become_visible() {
    let Some(mut h) = harness_with_content_and(|app| {
        app.add_plugins(crate::interest::InterestPlugin);
    }) else {
        return;
    };
    let player = h.spawn(
        1,
        STORMWIND,
        STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0),
    );
    h.walk_to(&player, STOCKADE_ENTRANCE_TRIGGER);
    h.new_worlds(&player).pop().expect("NewWorld for map 34");
    assert_eq!(h.arrivals(&player), 0);
    assert_eq!(h.visible_stockade_creatures(&player), 0);

    h.app
        .world_mut()
        .get_mut::<MessageSender<WorldPortAck>>(player.connection)
        .unwrap()
        .send::<TransferChannel>(WorldPortAck);
    let mut frames = Vec::new();
    for _ in 0..6 {
        h.app.update();
        frames.push((h.arrivals(&player), h.visible_stockade_creatures(&player)));
    }
    let arrived = frames
        .iter()
        .position(|&(arrivals, _)| arrivals == 1)
        .unwrap_or_else(|| panic!("no arrival in {frames:?}"));
    assert!(frames[arrived].1 > 0, "{frames:?}");
    assert!(
        frames[..arrived].iter().all(|&frame| frame == (0, 0)),
        "{frames:?}"
    );
    assert!(frames[arrived..].iter().all(|&(arrivals, _)| arrivals == 1));
}

#[test]
fn the_exit_trigger_returns_the_player_to_stormwind() {
    let Some(mut h) = harness() else { return };
    let player = h.spawn(
        1,
        STORMWIND,
        STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0),
    );
    h.enter_stockade(&player);

    h.walk_to(&player, STOCKADE_EXIT_TRIGGER);
    let new_world = h.new_worlds(&player).pop().expect("NewWorld for map 0");
    assert_eq!(new_world.map_directory, "azeroth");
    h.ack(&player);

    assert_eq!(h.map_of(&player), Some(STORMWIND));
    assert!(close(h.position(&player), STOCKADE_EXIT_TARGET));
}

#[test]
fn grouped_players_enter_one_stockade_copy_and_an_outsider_another() {
    let Some(mut h) = harness() else { return };
    let start = STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0);
    let leader = h.spawn(1, STORMWIND, start);
    let member = h.spawn(2, STORMWIND, start);
    let outsider = h.spawn(3, STORMWIND, start);
    h.group(&[&leader, &member]);

    h.enter_stockade(&leader);
    h.enter_stockade(&member);
    h.enter_stockade(&outsider);

    let copy = h.map_of(&leader).unwrap();
    assert_eq!(h.map_of(&member), Some(copy));
    let other = h.map_of(&outsider).unwrap();
    assert_eq!(other.map_id, STOCKADE);
    assert_ne!(other.instance_id, copy.instance_id);
}

#[test]
fn a_sixth_raid_member_is_refused_at_the_entrance_and_stays_in_stormwind() {
    let Some(mut h) = harness() else { return };
    let start = STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0);
    let members: Vec<Traveler> = (1..=6).map(|id| h.spawn(id, STORMWIND, start)).collect();
    h.group(&members.iter().collect::<Vec<_>>());
    for member in &members[..5] {
        h.enter_stockade(member);
    }

    let sixth = &members[5];
    h.walk_to(sixth, STOCKADE_ENTRANCE_TRIGGER);
    assert_eq!(
        h.aborts(sixth),
        vec![TransferAborted {
            map_id: STOCKADE,
            reason: TransferAbortReason::MaxPlayers,
        }]
    );
    assert!(h.new_worlds(sixth).is_empty());
    assert_eq!(h.map_of(sixth), Some(STORMWIND));
}

/// Stockade MapDifficulty 8 (Normal) → MapDifficultyXCondition 132 → PlayerCondition 74354:
/// ContentTuning 215 (levels 10-30), Flags 0x60 (`WithinOrAboveRecord` | `UseEffectiveLevel`).
fn stockade_level_refusal() -> TransferAborted {
    TransferAborted {
        map_id: STOCKADE,
        reason: TransferAbortReason::Condition(
            "You must be at least level 10 to enter Stormwind Stockade.".into(),
        ),
    }
}

#[test]
fn a_level_one_player_is_refused_at_the_stockade_entrance_and_stays_in_stormwind() {
    let Some(mut h) = harness() else { return };
    let player = h.spawn_at_level(
        1,
        1,
        STORMWIND,
        STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0),
    );
    h.walk_to(&player, STOCKADE_ENTRANCE_TRIGGER);

    assert_eq!(h.aborts(&player), vec![stockade_level_refusal()]);
    assert!(h.new_worlds(&player).is_empty());
    assert_eq!(h.map_of(&player), Some(STORMWIND));
    assert!(close(h.position(&player), STOCKADE_ENTRANCE_TRIGGER));
}

#[test]
fn level_nine_is_refused_and_level_ten_enters_the_stockade() {
    let Some(mut h) = harness() else { return };
    let start = STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0);
    let nine = h.spawn_at_level(1, 9, STORMWIND, start);
    let ten = h.spawn_at_level(2, 10, STORMWIND, start);

    h.walk_to(&nine, STOCKADE_ENTRANCE_TRIGGER);
    assert_eq!(h.aborts(&nine), vec![stockade_level_refusal()]);
    assert_eq!(h.map_of(&nine), Some(STORMWIND));

    let new_world = h.enter_stockade(&ten);
    assert_eq!(new_world.map_id, STOCKADE);
    assert!(h.aborts(&ten).is_empty());
    assert_eq!(h.map_of(&ten).map(|map| map.map_id), Some(STOCKADE));
}

#[test]
fn follower_admission_from_normal_ruby_transfers_to_a_distinct_205_copy() {
    let mut h = harness().unwrap();
    let player = h.spawn_at_level(30, 80, STORMWIND, STOCKADE_EXIT_TARGET);
    let destination = WorldLocation {
        map_id: 2521,
        position: Vec3::new(1386.2, -169.719, 131.0),
        orientation: 0.0,
    };
    assert!(teleport_to(
        h.app.world_mut(),
        player.player,
        destination,
        TeleportOptions::default()
    ));
    h.new_worlds(&player);
    h.ack(&player);
    let normal = h.map_of(&player).unwrap();
    assert_eq!(
        h.app
            .world()
            .resource::<InstanceManager>()
            .copy_difficulty(normal)
            .unwrap()
            .difficulty_id,
        1
    );
    // Follower begin_run sets205 before teleporting to the authored entrance.
    h.app
        .world_mut()
        .entity_mut(player.player)
        .insert(DungeonDifficulty(205));
    assert!(teleport_to(
        h.app.world_mut(),
        player.player,
        destination,
        TeleportOptions::default()
    ));
    assert!(
        h.app.world().get::<MapTransfer>(player.player).is_some(),
        "Follower admission must not near-teleport within the Normal copy"
    );
    assert_eq!(h.new_worlds(&player).pop().unwrap().map_id, 2521);
    h.ack(&player);
    let follower = h.map_of(&player).unwrap();
    assert_ne!(normal, follower);
    assert_eq!(
        h.app
            .world()
            .resource::<InstanceManager>()
            .copy_difficulty(follower)
            .unwrap()
            .difficulty_id,
        205
    );
}

#[test]
fn teleporting_across_continents_moves_the_player_to_kalimdor() {
    let Some(mut h) = harness() else { return };
    let player = h.spawn(1, STORMWIND, STOCKADE_EXIT_TARGET);
    let destination = WorldLocation {
        map_id: 1,
        position: RATCHET,
        orientation: 1.0,
    };
    let player_entity = player.player;
    let accepted = h
        .app
        .world_mut()
        .run_system_once(move |world: &mut World| {
            teleport_to(
                world,
                player_entity,
                destination,
                TeleportOptions::default(),
            )
        })
        .unwrap();
    assert!(accepted);
    assert_eq!(
        h.new_worlds(&player).pop().unwrap().map_directory,
        "kalimdor"
    );
    h.ack(&player);

    assert_eq!(h.map_of(&player), Some(InMap::continent(1)));
    assert!(close(h.position(&player), RATCHET));
}

#[test]
fn spell_teleport_hearthstone_cross_map_uses_new_world_and_world_port_ack() {
    let mut h = harness().unwrap();
    let player = h.spawn(1, STORMWIND, STOCKADE_EXIT_TARGET);
    let destination = WorldLocation {
        map_id: 1,
        position: RATCHET,
        orientation: 1.0,
    };
    let player_entity = player.player;
    let spell = crate::spell_cast::test_harness::world_db()
        .unwrap()
        .store
        .get(8690)
        .unwrap()
        .clone();
    h.app
        .world_mut()
        .entity_mut(player_entity)
        .insert(crate::spell_cast::homebind::HomeBind(destination));
    h.app
        .world_mut()
        .run_system_once(move |world: &mut World| {
            let effect = spell
                .effects
                .iter()
                .find(|effect| effect.effect == 252)
                .unwrap();
            let context = crate::spell_cast::effects::EffectCtx {
                caster: player_entity,
                spell: &spell,
                depth: 0,
                dest: None,
                chain_multiplier: 1.0,
            };
            crate::spell_cast::effects::handler(effect.effect).unwrap()(
                world,
                &context,
                effect,
                player_entity,
            );
        })
        .unwrap();
    assert!(h.app.world().get::<MapTransfer>(player_entity).is_some());
    assert_eq!(
        h.new_worlds(&player).pop().unwrap().map_directory,
        "kalimdor"
    );
    h.ack(&player);
    assert_eq!(h.map_of(&player), Some(InMap::continent(1)));
    assert!(close(h.position(&player), RATCHET));
}

#[test]
fn releasing_in_the_stockade_resurrects_the_player_at_the_stockade_graveyard() {
    let Some(mut h) = harness() else { return };
    let player = h.spawn(
        1,
        STORMWIND,
        STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0),
    );
    h.enter_stockade(&player);
    let map = h.map_of(&player).unwrap();
    let corpse = Corpse {
        player: 1,
        map_id: STOCKADE as u16,
        x: 0.0,
        y: 0.0,
        z: 0.0,
        created_at: 0,
    };
    h.app.world_mut().entity_mut(player.player).insert((
        Health {
            current: 0.0,
            max: 100.0,
        },
        PlayerDeathState {
            state: DeathState::Dead,
            corpse: Some(corpse),
            graveyard: None,
        },
    ));
    let player_entity = player.player;
    let store = map_store().unwrap().clone();
    h.app
        .world_mut()
        .run_system_once(
            move |mut commands: Commands, mut deaths: Query<&mut PlayerDeathState>| {
                let entry = store.map(map.map_id).unwrap().clone();
                let position = Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                };
                crate::death::release_spirit_in_dungeon(
                    player_entity,
                    &position,
                    &mut deaths.get_mut(player_entity).unwrap(),
                    &entry,
                    &store,
                    &mut commands,
                )
                .unwrap();
            },
        )
        .unwrap();
    h.advance(1);
    let new_world = h.new_worlds(&player).pop().expect("NewWorld to map 0");
    assert_eq!(new_world.map_id, 0);
    h.ack(&player);

    assert_eq!(h.map_of(&player), Some(STORMWIND));
    assert!(close(h.position(&player), STOCKADE_GRAVEYARD));
    let world = h.app.world();
    assert_eq!(
        world.get::<PlayerDeathState>(player.player).unwrap().state,
        DeathState::Alive
    );
    assert_eq!(world.get::<Health>(player.player).unwrap().current, 50.0);
}

// --- Difficulties and instance locks (Grim Batol, map 670) ---

const GRIM_BATOL: u32 = 670;
/// `Difficulty.db2` Heroic and (five-player) Mythic.
const DIFFICULTY_HEROIC: u32 = 2;
const DIFFICULTY_MYTHIC: u32 = 23;
/// `areatrigger_teleport` 6179 "Grim Batol Entrance" box center in the Twilight Highlands.
const GRIM_BATOL_ENTRANCE_TRIGGER: Vec3 = Vec3::new(-4042.39, -3444.45, 295.0);
/// `world_safe_locs` 3957 inside Grim Batol.
const GRIM_BATOL_ARRIVAL: Vec3 = Vec3::new(-624.155, -189.365, 272.04);
/// General Umbriss (boss 0, DungeonEncounter 1051, Bit 3).
const UMBRISS: u32 = 39625;
const TROGG_DWELLER: u32 = 39450;
const EASTERN_KINGDOMS: InMap = InMap::continent(0);
/// 2026-09-27 15:00 UTC: the daily reset after `SUNDAY_NOON`.
const SUNDAY_RESET: i64 = 1_790_521_200;

fn entrant_at(character_id: u64, difficulty_id: u32) -> InstanceEntrant {
    InstanceEntrant {
        difficulty_id,
        ..entrant(character_id, None)
    }
}

#[test]
fn grim_batol_offers_three_difficulties_and_the_stockade_downscales_to_normal() {
    let Some(store) = map_store() else { return };
    let heroic = store
        .downscaled_map_difficulty(GRIM_BATOL, DIFFICULTY_HEROIC)
        .unwrap();
    assert_eq!(
        (
            heroic.difficulty_id,
            heroic.reset_interval,
            heroic.is_instance_id_bound()
        ),
        (2, store::ResetInterval::Daily, true)
    );
    let mythic = store
        .downscaled_map_difficulty(GRIM_BATOL, DIFFICULTY_MYTHIC)
        .unwrap();
    assert_eq!(
        (
            mythic.difficulty_id,
            mythic.reset_interval,
            mythic.is_using_encounter_locks()
        ),
        (23, store::ResetInterval::Weekly, true)
    );
    let normal = store
        .downscaled_map_difficulty(GRIM_BATOL, store::DIFFICULTY_NORMAL)
        .unwrap();
    assert!(!normal.has_reset_schedule());
    // The Stockade has Normal only: Heroic and Mythic fall back (Mythic → Heroic → Normal).
    for difficulty in [DIFFICULTY_HEROIC, DIFFICULTY_MYTHIC] {
        assert_eq!(
            store
                .downscaled_map_difficulty(STOCKADE, difficulty)
                .unwrap()
                .difficulty_id,
            store::DIFFICULTY_NORMAL
        );
    }
    assert_eq!(store.difficulty_entries[&DIFFICULTY_MYTHIC].name, "Mythic");
    assert!(
        !store.difficulty_entries[&8].can_select(),
        "Mythic Keystone is not selectable"
    );
    assert_eq!(
        store
            .dungeon_encounter(1051, GRIM_BATOL, DIFFICULTY_HEROIC)
            .map(|e| e.bit),
        Some(3)
    );
}

#[test]
fn a_heroic_copy_is_created_at_heroic_and_normal_is_another_copy() {
    let Some(store) = map_store() else { return };
    let mut instances = InstanceManager::default();
    let heroic = instances
        .create_map(store, GRIM_BATOL, entrant_at(1, DIFFICULTY_HEROIC), at(0.0))
        .unwrap();
    // A change to Normal forgets the (unlocked) Heroic copy; even remembered, it is not entered.
    let normal = instances
        .create_map(
            store,
            GRIM_BATOL,
            entrant_at(1, store::DIFFICULTY_NORMAL),
            at(1.0),
        )
        .unwrap();
    assert_ne!(heroic.map, normal.map);
    assert_eq!(
        instances.copy_difficulty(heroic.map).unwrap().difficulty_id,
        2
    );
    assert_eq!(
        instances.copy_difficulty(normal.map).unwrap().difficulty_id,
        1
    );
    // The Stockade at Heroic is a Normal copy.
    let stockade = instances
        .create_map(store, STOCKADE, entrant_at(1, DIFFICULTY_HEROIC), at(2.0))
        .unwrap();
    assert_eq!(
        instances
            .copy_difficulty(stockade.map)
            .unwrap()
            .difficulty_id,
        1
    );
}

#[test]
fn a_heroic_kill_locks_the_player_to_its_copy_which_comes_back_with_the_boss_dead() {
    let Some(store) = map_store() else { return };
    let mut world = World::new();
    let player = world.spawn_empty().id();
    let mut instances = InstanceManager::default();
    let heroic = entrant_at(1, DIFFICULTY_HEROIC);
    let copy = instances
        .create_map(store, GRIM_BATOL, heroic, at(0.0))
        .unwrap()
        .map;
    instances.add_player(copy, player);
    let saved = instances.update_instance_lock(copy, &[(player, 1)], 1, Some(3), SUNDAY_NOON);
    assert_eq!(saved, vec![player]);
    instances.remove_player(copy, player, at(10.0));
    assert_eq!(
        instances.take_expired(10.0 + INSTANCE_UNLOAD_DELAY_SECS),
        vec![copy]
    );

    // Re-entering creates the same instance id with Umbriss done; he does not spawn.
    let again = instances
        .create_map(store, GRIM_BATOL, heroic, at(2000.0))
        .unwrap();
    assert_eq!((again.map, again.created), (copy, true));
    assert_eq!(instances.initial_done_bosses(copy), 1);
    assert_eq!(
        crate::scripted_ai::instance_script::done_boss_entries(store, GRIM_BATOL, 1),
        vec![UMBRISS]
    );
    // Saved to that copy, the player may not join another group's Heroic copy...
    let other = instances
        .create_map(
            store,
            GRIM_BATOL,
            entrant_at(2, DIFFICULTY_HEROIC),
            at(2001.0),
        )
        .unwrap()
        .map;
    assert_eq!(
        instances.cannot_enter(other, entrant_at(1, DIFFICULTY_HEROIC), SUNDAY_NOON),
        Some(TransferAbortReason::LockedToDifferentInstance)
    );
    // ...but may after the daily reset, and Normal never locks.
    assert_eq!(
        instances.cannot_enter(other, entrant_at(1, DIFFICULTY_HEROIC), SUNDAY_RESET + 1),
        None
    );
    let normal = instances
        .create_map(
            store,
            GRIM_BATOL,
            entrant_at(1, store::DIFFICULTY_NORMAL),
            at(2002.0),
        )
        .unwrap();
    assert!(normal.created);
    assert_eq!(instances.initial_done_bosses(normal.map), 0);
}

#[test]
fn an_expired_lock_resets_its_empty_copy() {
    let Some(store) = map_store() else { return };
    let mut world = World::new();
    let player = world.spawn_empty().id();
    let mut instances = InstanceManager::default();
    let heroic = entrant_at(1, DIFFICULTY_HEROIC);
    let copy = instances
        .create_map(store, GRIM_BATOL, heroic, at(0.0))
        .unwrap()
        .map;
    instances.add_player(copy, player);
    instances.update_instance_lock(copy, &[(player, 1)], 1, Some(3), SUNDAY_NOON);
    let after = instances::Now {
        elapsed: 100.0,
        unix: SUNDAY_RESET + 1,
    };
    // Occupied: the players are told, the copy stays.
    assert_eq!(instances.expire_locked_copies(after), vec![copy]);
    assert!(instances.take_expired(200.0).is_empty());
    // Empty after the reset: it unloads at once and the next entry is a fresh copy.
    instances.remove_player(copy, player, after);
    assert_eq!(instances.take_expired(101.0), vec![copy]);
    let fresh = instances
        .create_map(store, GRIM_BATOL, heroic, after)
        .unwrap();
    assert!(fresh.created);
    assert_ne!(fresh.map, copy);
    assert_eq!(instances.initial_done_bosses(fresh.map), 0);
}

#[test]
fn a_difficulty_change_resets_and_forgets_the_players_unlocked_copies() {
    let Some(store) = map_store() else { return };
    let mut instances = InstanceManager::default();
    let normal = instances
        .create_map(
            store,
            GRIM_BATOL,
            entrant_at(1, store::DIFFICULTY_NORMAL),
            at(0.0),
        )
        .unwrap()
        .map;
    let results = instances.reset_instances(entrant_at(1, store::DIFFICULTY_NORMAL), true, at(5.0));
    assert_eq!(
        results,
        vec![(GRIM_BATOL, instances::InstanceResetResult::Success)]
    );
    assert_eq!(instances.take_expired(6.0), vec![normal]);
    assert!(
        instances
            .create_map(
                store,
                GRIM_BATOL,
                entrant_at(1, store::DIFFICULTY_NORMAL),
                at(7.0)
            )
            .unwrap()
            .created
    );
}

impl Harness {
    fn enter_grim_batol(&mut self, who: &Traveler) {
        self.walk_to(who, GRIM_BATOL_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 40.0));
        self.walk_to(who, GRIM_BATOL_ENTRANCE_TRIGGER);
        let new_world = self.new_worlds(who).pop().expect("NewWorld for map 670");
        assert_eq!(new_world.map_id, GRIM_BATOL);
        self.ack(who);
    }

    fn leave_map(&mut self, who: &Traveler) {
        let player = who.player;
        self.app
            .world_mut()
            .run_system_once(move |world: &mut World| {
                let back = WorldLocation {
                    map_id: 0,
                    position: GRIM_BATOL_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 40.0),
                    orientation: 0.0,
                };
                teleport_to(world, player, back, TeleportOptions::default())
            })
            .unwrap();
        self.new_worlds(who);
        self.ack(who);
    }

    /// (entity, max health) of the creatures of `entry` in `map`.
    fn creature_health(&mut self, map: InMap, entry: u32) -> Vec<(Entity, f32)> {
        let world = self.app.world_mut();
        let mut query = world.query::<(Entity, &shared::components::Npc, &Health, &InMap)>();
        query
            .iter(world)
            .filter(|(_, npc, _, in_map)| **in_map == map && npc.template_id == entry)
            .map(|(entity, _, health, _)| (entity, health.max))
            .collect()
    }
}

#[test]
fn heroic_grim_batol_spawns_heroic_creatures_and_a_boss_kill_keeps_its_copy() {
    let Some(mut h) = harness_with_content_and(|app| {
        app.add_plugins(crate::scripted_ai::ScriptedAiPlugin);
    }) else {
        return;
    };
    let start = GRIM_BATOL_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 40.0);
    let hero = h.spawn(1, EASTERN_KINGDOMS, start);
    let casual = h.spawn(2, EASTERN_KINGDOMS, start);
    h.app
        .world_mut()
        .entity_mut(hero.player)
        .insert(DungeonDifficulty(DIFFICULTY_HEROIC));
    h.enter_grim_batol(&hero);
    h.enter_grim_batol(&casual);
    let (heroic, normal) = (h.map_of(&hero).unwrap(), h.map_of(&casual).unwrap());
    assert_ne!(heroic, normal);
    assert!(close(h.position(&hero), GRIM_BATOL_ARRIVAL));
    // Trogg Dweller: HealthModifier 1.125 at Heroic, 0.4 at Normal (creature_template_difficulty).
    let heroic_troggs = h.creature_health(heroic, TROGG_DWELLER);
    let normal_troggs = h.creature_health(normal, TROGG_DWELLER);
    assert_eq!((heroic_troggs.len(), normal_troggs.len()), (94, 94));
    let ratio = heroic_troggs[0].1 / normal_troggs[0].1;
    assert!((ratio - 1.125 / 0.4).abs() < 0.01, "{ratio}");
    let heroic_umbriss = h.creature_health(heroic, UMBRISS);
    assert_eq!(heroic_umbriss.len(), 1);

    // Umbriss dies (BossAI::_JustDied): the hero is saved.
    let umbriss = heroic_umbriss[0].0;
    h.advance(2);
    h.app
        .world_mut()
        .run_system_once(move |world: &mut World| {
            crate::scripted_ai::dispatch(world, umbriss, crate::scripted_ai::Hook::JustDied(None));
        })
        .unwrap();
    h.advance(2);
    assert_eq!(h.saves.get(&hero.connection).copied(), Some(1));
    let lock = h
        .app
        .world()
        .resource::<InstanceManager>()
        .locks
        .lock(1, (GRIM_BATOL, 0))
        .cloned()
        .expect("heroic lock");
    assert_eq!(
        (
            lock.instance_id,
            lock.difficulty_id,
            lock.data.completed_encounters
        ),
        (heroic.instance_id, 2, 1 << 3)
    );

    // Out, the copy unloads; back in, the same copy without Umbriss.
    h.leave_map(&hero);
    let delay = Duration::from_secs_f64(INSTANCE_UNLOAD_DELAY_SECS + 1.0);
    h.app
        .world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(delay);
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(delay));
    h.advance(2);
    assert!(h.creature_health(heroic, UMBRISS).is_empty());
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(50),
        ));
    h.enter_grim_batol(&hero);
    assert_eq!(h.map_of(&hero), Some(heroic));
    assert!(
        h.creature_health(heroic, UMBRISS).is_empty(),
        "Umbriss stays dead"
    );
    assert!(
        !h.creatures_in(heroic).is_empty(),
        "the rest of the copy respawned"
    );
}

impl Harness {
    fn request_difficulty(&mut self, who: &Traveler, difficulty_id: u32) {
        self.app
            .world_mut()
            .get_mut::<MessageSender<SetDungeonDifficulty>>(who.connection)
            .unwrap()
            .send::<InstanceChannel>(SetDungeonDifficulty { difficulty_id });
    }

    /// The `DungeonDifficultySet`s `who` received over the next frames.
    fn difficulty_messages(&mut self, who: &Traveler) -> Vec<u32> {
        let mut received = Vec::new();
        for _ in 0..3 {
            self.app.update();
            received.extend(
                self.app
                    .world_mut()
                    .get_mut::<MessageReceiver<DungeonDifficultySet>>(who.connection)
                    .unwrap()
                    .receive()
                    .map(|set| set.difficulty_id),
            );
        }
        received
    }

    fn difficulty_of(&self, who: &Traveler) -> Option<u32> {
        self.app
            .world()
            .get::<DungeonDifficulty>(who.player)
            .map(|d| d.0)
    }
}

#[test]
fn the_player_picks_its_dungeon_difficulty_outside_instances_only() {
    let Some(mut h) = harness() else { return };
    let start = GRIM_BATOL_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 40.0);
    let player = h.spawn(1, EASTERN_KINGDOMS, start);
    h.request_difficulty(&player, DIFFICULTY_HEROIC);
    assert_eq!(h.difficulty_messages(&player), vec![2]);
    assert_eq!(h.difficulty_of(&player), Some(2));
    // Mythic Keystone (8) is not selectable; nothing changes.
    h.request_difficulty(&player, 8);
    assert!(h.difficulty_messages(&player).is_empty());
    // Inside Grim Batol (Heroic copy) the difficulty cannot change.
    h.enter_grim_batol(&player);
    assert_eq!(
        h.app
            .world()
            .resource::<InstanceManager>()
            .copy_difficulty(h.map_of(&player).unwrap())
            .unwrap()
            .difficulty_id,
        2
    );
    h.request_difficulty(&player, DIFFICULTY_MYTHIC);
    assert!(h.difficulty_messages(&player).is_empty());
    assert_eq!(h.difficulty_of(&player), Some(2));
}

#[test]
fn only_a_group_leader_sets_the_groups_difficulty_for_every_member() {
    let Some(mut h) = harness() else { return };
    let start = GRIM_BATOL_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 40.0);
    let leader = h.spawn(1, EASTERN_KINGDOMS, start);
    let member = h.spawn(2, EASTERN_KINGDOMS, start);
    h.group(&[&leader, &member]);
    h.request_difficulty(&member, DIFFICULTY_HEROIC);
    assert!(h.difficulty_messages(&member).is_empty());
    h.request_difficulty(&leader, DIFFICULTY_MYTHIC);
    h.app.update();
    assert_eq!(h.difficulty_messages(&member), vec![23]);
    assert_eq!(
        (h.difficulty_of(&leader), h.difficulty_of(&member)),
        (Some(23), Some(23))
    );
    // Both enter the group's Mythic copy.
    h.enter_grim_batol(&leader);
    h.enter_grim_batol(&member);
    let copy = h.map_of(&leader).unwrap();
    assert_eq!(h.map_of(&member), Some(copy));
    assert_eq!(
        h.app
            .world()
            .resource::<InstanceManager>()
            .copy_difficulty(copy)
            .unwrap()
            .difficulty_id,
        23
    );
}

#[test]
fn raid_entry_legacy_raid_copy_and_weekly_bind_resume() {
    let store = map_store().expect("canonical map data");
    let mut instances = InstanceManager::default();
    let mut raider = entrant(71, None);
    raider.difficulty_id = 9; // Difficulty.db2: Molten Core 40 Player.
    assert_eq!(
        instances.player_cannot_enter(store, 409, raider, 80, SUNDAY_NOON),
        None
    );
    let copy = instances
        .create_map(store, 409, raider, at(0.0))
        .unwrap()
        .map;
    assert_eq!(instances.copy_difficulty(copy).unwrap().difficulty_id, 9);
    let entries = *store.map_difficulty(409, 9).unwrap();
    assert_eq!(entries.reset_interval, store::ResetInterval::Weekly);
    let player = World::new().spawn_empty().id();
    instances.add_player(copy, player);
    assert_eq!(
        instances.update_instance_lock(copy, &[(player, 71)], 1, Some(0), SUNDAY_NOON),
        vec![player]
    );
    let saved = instances.locks.saved();
    let mut reloaded = InstanceManager::with_locks(locks::InstanceLocks::from_saved(saved));
    let resumed = reloaded
        .create_map(store, 409, raider, at(1.0))
        .unwrap()
        .map;
    assert_eq!(resumed.instance_id, copy.instance_id);
    assert_eq!(reloaded.initial_done_bosses(resumed), 1);
}

#[test]
fn raid_entry_current_requires_raid_but_legacy_allows_solo_and_resolves_separate_selections() {
    let store = map_store().unwrap();
    let instances = InstanceManager::default();
    let mut player = entrant(71, Some(7));
    assert_eq!(
        instances.player_cannot_enter(store, 2939, player, 90, SUNDAY_NOON),
        Some(TransferAbortReason::NeedGroup)
    );
    player.raid_group = true;
    // Actual pinned map condition, not a made-up map/level.
    assert_eq!(
        instances.player_cannot_enter(store, 2939, player, 90, SUNDAY_NOON),
        None
    );
    let mut instances = instances;
    player.raid_difficulty.primary = 15;
    player.raid_difficulty.legacy = 4;
    let modern = instances
        .create_map(store, 2522, player, at(0.0))
        .unwrap()
        .map;
    assert_eq!(instances.copy_difficulty(modern).unwrap().difficulty_id, 15);
    let legacy = instances
        .create_map(store, 533, player, at(0.0))
        .unwrap()
        .map;
    assert_eq!(instances.copy_difficulty(legacy).unwrap().difficulty_id, 4);
    let dungeon = instances
        .create_map(store, 34, player, at(0.0))
        .unwrap()
        .map;
    assert_eq!(instances.copy_difficulty(dungeon).unwrap().difficulty_id, 1);
}

#[test]
fn raid_entry_weekly_reset_is_expired_at_tuesday_1500_not_a_second_later() {
    let store = map_store().unwrap();
    let entries = store.map_difficulty(409, 9).unwrap();
    let reset = locks::next_reset_time(entries.reset_interval, SUNDAY_NOON);
    let mut locks = locks::InstanceLocks::default();
    locks.create_for_new_instance(71, entries, 8, SUNDAY_NOON);
    locks.update_for_player(
        71,
        entries,
        &locks::LockUpdate {
            instance_id: 8,
            done_bosses: 1,
            instance_completed_encounters: 1,
            completed_encounter_bit: Some(0),
        },
        SUNDAY_NOON,
    );
    assert!(locks.find_active_lock(71, entries, reset - 1).is_some());
    assert!(locks.find_active_lock(71, entries, reset).is_none());
    locks.set_extended(71, entries, true, reset);
    let lock = locks.find_active_lock(71, entries, reset).unwrap();
    assert_eq!(lock.effective_expiry(entries, reset), reset + 604800);
    locks.set_extended(71, entries, false, reset);
    assert!(locks.find_active_lock(71, entries, reset).is_none());
}

#[test]
#[ignore = "needs RAID_WORLD_DB: a prepared raid world.db copy (docs/specs, raid-entry)"]
fn raid_entry_authentic_lucifron_death_saves_only_players_of_his_copy() {
    let path = std::env::var("RAID_WORLD_DB").expect("prepared read-only raid copy");
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let store = store::load_map_store(&conn).unwrap();
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    world.insert_resource(ServerClock {
        fixed_unix: Some(SUNDAY_NOON),
        offset_secs: 0,
    });
    let mut instances = InstanceManager::default();
    let copy = instances
        .create_map(&store, 409, entrant(71, Some(7)), at(0.0))
        .unwrap()
        .map;
    let mut outsider = entrant(73, Some(8));
    outsider.leader = 73;
    let other = instances
        .create_map(&store, 409, outsider, at(0.0))
        .unwrap()
        .map;
    let mut players = Vec::new();
    for (id, map) in [(71, copy), (72, copy), (73, other)] {
        let player = world
            .spawn((
                Player {
                    name: format!("Raider{id}"),
                    race: 1,
                    class: 1,
                    appearance: Default::default(),
                },
                ClientId(id),
                map,
            ))
            .id();
        instances.add_player(map, player);
        players.push(player);
    }
    world.insert_resource(store);
    world.insert_resource(instances);
    let lucifron = world
        .spawn((
            shared::components::Npc {
                template_id: 12118,
                name: "Lucifron".into(),
            },
            copy,
            Health {
                current: 0.0,
                max: 100.0,
            },
        ))
        .id();
    let history_file = tempfile::NamedTempFile::new().unwrap();
    world.insert_resource(crate::persistence::GameDatabase::new(
        redb::Database::create(history_file.path()).unwrap(),
    ));
    crate::combat::mark_npc_dead_world(&mut world, lucifron);
    {
        use redb::ReadableDatabase;
        let txn = world
            .resource::<crate::persistence::GameDatabase>()
            .db
            .begin_read()
            .unwrap();
        let table = txn
            .open_table(::server::raid_history_store::FIRST_KILLS)
            .unwrap();
        for id in [71, 72] {
            let key = format!("character:{id}:409:9:663");
            let record: ::server::raid_history_store::FirstKill =
                serde_json::from_slice(table.get(key.as_str()).unwrap().unwrap().value()).unwrap();
            assert_eq!(record.first_kill_at, SUNDAY_NOON);
            assert_eq!(
                record
                    .participants
                    .iter()
                    .map(|p| p.character_id.as_str())
                    .collect::<Vec<_>>(),
                vec!["71", "72"]
            );
        }
        assert!(table.get("character:73:409:9:663").unwrap().is_none());
    }
    let instances = world.resource::<InstanceManager>();
    for id in [71, 72] {
        let lock = instances
            .locks
            .lock(id, (409, 0))
            .expect("boss death permanently saved both present raiders");
        assert_eq!(lock.data.completed_encounters, 1);
        assert_eq!(lock.instance_id, copy.instance_id);
    }
    assert!(instances.locks.lock(73, (409, 0)).is_none());
    let entries = crate::scripted_ai::instance_script::done_boss_entries(
        world.resource::<MapStore>(),
        409,
        1,
    );
    assert_eq!(entries, vec![12118]);
}

#[test]
fn raid_entry_legacy_flexible_lock_uses_encounters_not_one_instance_id() {
    let store = map_store().unwrap();
    assert!(!store.map_difficulty(533, 3).unwrap().is_instance_id_bound());
    assert!(
        store
            .map_difficulty(2522, 16)
            .unwrap()
            .is_instance_id_bound()
    );
}
#[test]
fn raid_entry_extended_copy_does_not_expire_at_its_original_weekly_reset() {
    let store = map_store().unwrap();
    let mut instances = InstanceManager::default();
    let copy = instances
        .create_map(store, 409, entrant(71, None), at(0.0))
        .unwrap()
        .map;
    let player = World::new().spawn_empty().id();
    instances.add_player(copy, player);
    instances.update_instance_lock(copy, &[(player, 71)], 1, Some(0), SUNDAY_NOON);
    let entries = store.map_difficulty(409, 9).unwrap();
    let reset = locks::next_reset_time(entries.reset_interval, SUNDAY_NOON);
    instances.locks.set_extended(71, entries, true, SUNDAY_NOON);
    assert!(
        instances
            .expire_locked_copies(instances::Now {
                elapsed: 100.0,
                unix: reset
            })
            .is_empty()
    );
}

#[test]
fn raid_entry_flexible_saved_player_can_join_equal_progress_but_not_unkilled_encounters() {
    let store = map_store().unwrap();
    let entries = store.map_difficulty(533, 3).unwrap();
    let mut locks = locks::InstanceLocks::default();
    locks.create_for_new_instance(71, entries, 8, SUNDAY_NOON);
    locks.update_for_player(
        71,
        entries,
        &locks::LockUpdate {
            instance_id: 8,
            done_bosses: 1,
            instance_completed_encounters: 1,
            completed_encounter_bit: Some(0),
        },
        SUNDAY_NOON,
    );
    assert_eq!(locks.can_join(71, entries, 9, SUNDAY_NOON, 1), None);
    assert_eq!(locks.can_join(71, entries, 9, SUNDAY_NOON, 3), None);
    assert_eq!(
        locks.can_join(71, entries, 9, SUNDAY_NOON, 0),
        Some(TransferAbortReason::AlreadyCompletedEncounter)
    );
}

#[test]
fn raid_entry_only_leader_sets_raid_selections_over_the_real_message_receiver_outside_instances() {
    let mut h = harness().unwrap();
    let leader = h.spawn(1, InMap::continent(0), Vec3::ZERO);
    let member = h.spawn(2, InMap::continent(0), Vec3::ZERO);
    h.group(&[&leader, &member]);
    let send = |h: &mut Harness, who: &Traveler, difficulty_id: u32, legacy: bool| {
        h.app
            .world_mut()
            .get_mut::<MessageSender<SetRaidDifficulty>>(who.connection)
            .unwrap()
            .send::<InstanceChannel>(SetRaidDifficulty {
                difficulty_id,
                legacy,
            });
        h.advance(3);
    };
    send(&mut h, &member, 15, false);
    assert_eq!(
        h.app
            .world()
            .resource::<GroupRegistry>()
            .raid_difficulty_of(2)
            .unwrap(),
        RaidDifficulty::default()
    );
    send(&mut h, &leader, 15, false);
    send(&mut h, &leader, 6, true);
    for who in [&leader, &member] {
        assert_eq!(
            *h.app.world().get::<RaidDifficulty>(who.player).unwrap(),
            RaidDifficulty {
                primary: 15,
                legacy: 6
            }
        );
    }
    let store = h.app.world().resource::<MapStore>().clone();
    let copy = h
        .app
        .world_mut()
        .resource_mut::<InstanceManager>()
        .create_map(&store, 409, entrant(1, None), at(0.0))
        .unwrap()
        .map;
    h.app.world_mut().entity_mut(leader.player).insert(copy);
    send(&mut h, &leader, 14, false);
    send(&mut h, &leader, 3, true);
    assert_eq!(
        *h.app.world().get::<RaidDifficulty>(leader.player).unwrap(),
        RaidDifficulty {
            primary: 15,
            legacy: 6
        }
    );
}

#[test]
fn housing_finder_visit_ports_to_authored_plot_without_payment_or_grant() {
    let dir = tempfile::tempdir().unwrap();
    let db = redb::Database::create(dir.path().join("plot-visit.redb")).unwrap();
    crate::persistence::create_tables(&db);
    crate::persistence::save_inventory(
        &db,
        30,
        &crate::persistence::InventoryData {
            gold: 20_000_000,
            items: vec![],
        },
    );
    let mut h = housing_harness(db);
    let viewer = h.spawn(30, InMap::continent(0), STOCKADE_ENTRANCE_TRIGGER);
    h.app
        .world_mut()
        .entity_mut(viewer.connection)
        .insert((pair::<HousingRequest>(), pair::<HousingReply>()));
    let plot = h
        .app
        .world()
        .resource::<crate::housing::HousingAvailability>()
        .0
        .as_ref()
        .unwrap()
        .plot(417)
        .unwrap()
        .clone();
    let reply = h.housing_request(&viewer, HousingCommand::VisitPlot { plot_id: 417 });
    assert_eq!(reply.error, None);
    assert!(reply.state.owned_houses.is_empty());
    assert!(reply.state.stored.is_empty());
    h.ack(&viewer);
    assert_eq!(h.map_of(&viewer).unwrap().map_id, 2735);
    let [x, y, z] = plot.teleport;
    let expected = crate::terrain_height::wow_to_bevy(x, y, z);
    let actual = h.app.world().get::<Position>(viewer.player).unwrap();
    assert_eq!((actual.x, actual.y, actual.z), expected);
    assert_eq!(
        crate::persistence::load_inventory(
            &h.app
                .world()
                .resource::<crate::persistence::GameDatabase>()
                .db,
            30
        )
        .gold,
        20_000_000
    );
}
