use super::*;
use crate::persistence::{self, appearance_store::load_account_appearances};
use shared::protocol::GameObjectInfo;

const CORLAIN_BREASTPLATE: u32 = 154840;
const CORLAIN_APPEARANCE: u32 = 36157;
const TRASHMASTERS_MANTLE: u32 = 168970;
const WORN_SHORTSWORD: u32 = 25;

struct Harness {
    app: App,
    directory: tempfile::TempDir,
    player: Entity,
    machine: Entity,
    character: u64,
    account: u64,
}

impl Harness {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let db = redb::Database::create(directory.path().join("game.redb")).unwrap();
        persistence::create_tables(&db);
        let (account, _) =
            crate::persistence_accounts::register_account(&db, "scrap_fixture", "fixture").unwrap();
        let (character, _) = persistence::create_character(&db, account, "Scrapper", 1, 2).unwrap();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(GameDatabase { db });
        let path = crate::test_data::world_db_path();
        app.insert_resource(crate::item_templates::load_item_templates(&path).unwrap());
        app.add_plugins(ScrappingPlugin);
        app.update();
        let player = app
            .world_mut()
            .spawn((
                ClientId(character),
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                InMap::continent(1643),
            ))
            .id();
        let conn = rusqlite::Connection::open(path).unwrap();
        app.insert_resource(load_scrapping_data(&conn).unwrap());
        // Authentic TDB machine identity, positioned locally for the interaction fixture.
        let info = conn.query_row(
            "SELECT entry,type,displayId,name,size FROM tdb_gameobject_template WHERE entry=293132 AND type=48 AND Data0=2",
            [], |row| Ok(GameObjectInfo { entry: row.get(0)?, go_type: row.get(1)?,
                display_id: row.get(2)?, name: row.get(3)?, scale: row.get(4)? }),
        ).unwrap();
        let machine = app
            .world_mut()
            .spawn((
                ScrappingMachine,
                info,
                InMap::continent(1643),
                Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                GameObjectBounds {
                    min: Vec3::ZERO,
                    max: Vec3::ZERO,
                    scale: 1.0,
                    orientation: 0.0,
                },
            ))
            .id();
        Self {
            app,
            directory,
            player,
            machine,
            character,
            account,
        }
    }

    fn add_item(&mut self, item_id: u32) -> u64 {
        let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
        let catalog = crate::merchant::load_vendor_catalog(&conn).unwrap();
        let world = self.app.world();
        let db = &world.resource::<GameDatabase>().db;
        crate::inventory::grant_item(
            db,
            world.resource::<ItemTemplates>(),
            &catalog,
            self.character,
            (item_id, 1),
        )
        .unwrap();
        persistence::load_inventory(db, self.character)
            .items
            .iter()
            .filter(|item| item.item_id == item_id)
            .map(|item| item.item_guid)
            .max()
            .unwrap()
    }

    fn request(&self, items: Vec<u64>) -> ScrapRequest {
        ScrapRequest {
            player: self.player,
            machine: self.machine,
            items,
        }
    }

    fn send(&mut self, items: Vec<u64>) -> ScrapFailure {
        let request = self.request(items);
        self.app.world_mut().write_message(request);
        self.app.update();
        let responses: Vec<_> = self
            .app
            .world_mut()
            .resource_mut::<Messages<ScrapResponse>>()
            .drain()
            .collect();
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].player, self.player);
        responses.into_iter().next().unwrap().result.unwrap_err()
    }

    fn inventory(&self) -> persistence::InventoryData {
        persistence::load_inventory(
            &self.app.world().resource::<GameDatabase>().db,
            self.character,
        )
    }

    fn reopen(self) -> (tempfile::TempDir, redb::Database, u64, u64) {
        drop(self.app);
        let db = redb::Database::open(self.directory.path().join("game.redb")).unwrap();
        (self.directory, db, self.character, self.account)
    }
}

#[test]
fn scrapping_scrappable_boe_passes_preflight_but_missing_loot_keeps_item_and_appearance_unknown() {
    let mut h = Harness::new();
    let guid = h.add_item(CORLAIN_BREASTPLATE);
    let request = h.request(vec![guid]);
    assert!(check_machine(h.app.world(), &request).is_ok());
    load_and_check_bag_items(h.app.world(), &request).expect("eligible owned Corlain breastplate");
    let before = h.inventory();
    assert!(matches!(h.send(vec![guid]), ScrapFailure::LootUnavailable));
    assert_eq!(h.inventory(), before);
    let (_directory, db, character, account) = h.reopen();
    assert_eq!(persistence::load_inventory(&db, character), before);
    assert!(!load_account_appearances(&db, account).has(CORLAIN_APPEARANCE));
}

#[test]
fn scrapping_unscrappable_short_sword_is_rejected_and_kept_after_reopen() {
    let mut h = Harness::new();
    let guid = h.add_item(WORN_SHORTSWORD);
    let before = h.inventory();
    let failure = h.send(vec![guid]);
    assert!(
        matches!(failure, ScrapFailure::CantBeScrapped),
        "{failure:?}"
    );
    let (_directory, db, character, account) = h.reopen();
    assert_eq!(persistence::load_inventory(&db, character), before);
    assert!(!load_account_appearances(&db, account).has(154));
}

#[test]
fn scrapping_bound_item_preserves_bind_time_collection_across_reopen() {
    let mut h = Harness::new();
    let guid = h.add_item(TRASHMASTERS_MANTLE);
    let template = h
        .app
        .world()
        .resource::<ItemTemplates>()
        .get(Retail, TRASHMASTERS_MANTLE)
        .unwrap();
    assert!(template.binds_on_pickup());
    let appearance = template.appearance_id.unwrap();
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert!(load_account_appearances(db, h.account).has(appearance));
    assert!(matches!(h.send(vec![guid]), ScrapFailure::LootUnavailable));
    let (_directory, db, character, account) = h.reopen();
    assert!(
        persistence::load_inventory(&db, character)
            .items
            .iter()
            .any(|item| item.item_guid == guid)
    );
    assert!(load_account_appearances(&db, account).has(appearance));
}

#[test]
fn scrapping_nine_items_pass_preflight_but_empty_duplicate_and_ten_are_rejected() {
    let mut h = Harness::new();
    let guids: Vec<_> = (0..10).map(|_| h.add_item(CORLAIN_BREASTPLATE)).collect();
    assert!(matches!(
        h.send(guids[..9].to_vec()),
        ScrapFailure::LootUnavailable
    ));
    assert!(matches!(h.send(Vec::new()), ScrapFailure::InvalidRequest));
    assert!(matches!(
        h.send(vec![guids[0], guids[0]]),
        ScrapFailure::InvalidRequest
    ));
    assert!(matches!(h.send(guids), ScrapFailure::InvalidRequest));
}

#[test]
fn scrapping_requires_living_player_same_instance_and_machine_reach() {
    let mut h = Harness::new();
    let guid = h.add_item(CORLAIN_BREASTPLATE);
    h.app
        .world_mut()
        .get_mut::<Health>(h.player)
        .unwrap()
        .current = 0.0;
    assert!(matches!(h.send(vec![guid]), ScrapFailure::PlayerDead));
    h.app
        .world_mut()
        .get_mut::<Health>(h.player)
        .unwrap()
        .current = 100.0;
    h.app
        .world_mut()
        .get_mut::<InMap>(h.player)
        .unwrap()
        .instance_id = 1;
    assert!(matches!(h.send(vec![guid]), ScrapFailure::InvalidMachine));
    h.app
        .world_mut()
        .get_mut::<InMap>(h.player)
        .unwrap()
        .instance_id = 0;
    h.app.world_mut().get_mut::<Position>(h.player).unwrap().x = 6.0;
    assert!(matches!(h.send(vec![guid]), ScrapFailure::InvalidMachine));
    h.app.world_mut().get_mut::<Position>(h.player).unwrap().x = 0.0;
    h.app
        .world_mut()
        .entity_mut(h.machine)
        .remove::<ScrappingMachine>();
    assert!(matches!(h.send(vec![guid]), ScrapFailure::InvalidMachine));
}

#[test]
fn scrapping_authentic_machine_spawn_loads_and_gets_interaction_marker() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute(
        "ATTACH DATABASE ?1 AS reference",
        [crate::test_data::world_db_path().to_str().unwrap()],
    )
    .unwrap();
    // Read authentic TDB rows through private temp views, never mutate world.db.
    conn.execute_batch("CREATE TEMP VIEW content_gameobject AS SELECT * FROM reference.tdb_gameobject WHERE guid=501905;
        CREATE TEMP VIEW content_gameobject_template AS SELECT * FROM reference.tdb_gameobject_template;
        CREATE TEMP VIEW game_object_display_info AS SELECT * FROM reference.game_object_display_info;").unwrap();
    let spawns = crate::game_objects::load_game_object_spawns(&conn).unwrap();
    assert_eq!(spawns.0.len(), 1);
    assert_eq!(spawns.0[0].info.entry, 293132);
    let mut app = App::new();
    app.insert_resource(spawns);
    app.add_systems(
        Startup,
        |mut commands: Commands, spawns: Res<crate::game_objects::GameObjectSpawns>| {
            assert_eq!(
                crate::game_objects::spawn_map_game_objects(
                    &mut commands,
                    &spawns,
                    InMap::continent(1643),
                    0
                ),
                1
            );
        },
    );
    app.update();
    let world = app.world_mut();
    assert_eq!(
        world
            .query_filtered::<Entity, With<ScrappingMachine>>()
            .iter(world)
            .count(),
        1
    );
}

#[test]
fn scrapping_foreign_guid_is_rejected_before_any_inventory_change() {
    let mut h = Harness::new();
    h.add_item(CORLAIN_BREASTPLATE);
    let before = h.inventory();
    let failure = h.send(vec![u64::MAX]);
    assert!(
        matches!(&failure, ScrapFailure::Inventory(error)
        if error.reason == shared::protocol::InventoryErrorReason::ItemNotFound),
        "{failure:?}"
    );
    assert_eq!(h.inventory(), before);
}
