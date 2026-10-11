use super::*;
use crate::maps::{InMap, InstanceManager};
use crate::persistence::{self, GameDatabase, InventoryData};
use shared::components::Position;
use shared::protocol::HousingCommand;

#[test]
fn housing_runtime_requires_real_cornerstone_presence_and_reports_durable_ownership() {
    let dir = tempfile::tempdir().unwrap();
    let db = redb::Database::create(dir.path().join("housing.redb")).unwrap();
    persistence::create_tables(&db);
    persistence::save_inventory(
        &db,
        30,
        &InventoryData {
            gold: 20_000_000,
            items: vec![],
        },
    );
    let catalog = crate::housing_catalog::HousingCatalog::read(
        &crate::housing_catalog::tests::catalog_fixture(),
    )
    .unwrap();
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let maps = crate::maps::store::load_map_store(&conn).unwrap();
    let mut manager = InstanceManager::default();
    let copy = manager
        .authorize_housing_entry(&maps, 2735, 1, 30)
        .unwrap()
        .map;
    let mut world = World::new();
    world.insert_resource(GameDatabase { db });
    world.insert_resource(manager);
    world.insert_resource(maps);
    world.insert_resource(HousingAvailability(Ok(catalog.clone())));
    world.init_resource::<crate::gold::GoldUpdateQueue>();
    let player = world
        .spawn((
            copy,
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        ))
        .id();
    let actor = Actor {
        connection: Entity::PLACEHOLDER,
        player,
        account: 1,
        character: 30,
    };
    let command = HousingCommand::PurchasePlot { plot_id: 417 };
    assert!(
        execute(&mut world, actor, &command)
            .unwrap_err()
            .contains("cornerstone")
    );
    assert_eq!(
        persistence::load_inventory(&world.resource::<GameDatabase>().db, 30).gold,
        20_000_000
    );
    world.entity_mut(player).insert(Position {
        x: f32::NAN,
        y: 0.0,
        z: 0.0,
    });
    assert!(execute(&mut world, actor, &command).is_err());
    let plot = catalog.plot(417).unwrap();
    let [x, y, z] = plot.cornerstone;
    let (x, y, z) = crate::terrain_height::wow_to_bevy(x, y, z);
    world.entity_mut(player).insert(Position { x, y, z });
    execute(&mut world, actor, &command).unwrap();
    let snapshot = read_snapshot(&world, actor, Some(1), None).unwrap();
    assert_eq!(snapshot.owned_houses.len(), 1);
    assert_eq!(
        snapshot.plots[0].house_id,
        Some(snapshot.owned_houses[0].house_id)
    );
    assert_eq!(
        persistence::load_inventory(&world.resource::<GameDatabase>().db, 30).gold,
        10_000_000
    );
    assert!(!world.resource::<crate::gold::GoldUpdateQueue>().is_empty());
    let house_id = snapshot.owned_houses[0].house_id;
    assert!(
        execute(&mut world, actor, &HousingCommand::EnterHouse { house_id })
            .unwrap_err()
            .contains("grant")
    );
    assert!(world.get::<crate::maps::MapTransfer>(player).is_none());
    world.entity_mut(player).insert(InMap::continent(0));
    assert!(execute(&mut world, actor, &command).is_err());
    assert_eq!(
        persistence::load_inventory(&world.resource::<GameDatabase>().db, 30).gold,
        10_000_000
    );
    assert!(read_snapshot(&world, actor, Some(4), None).is_err());
}
