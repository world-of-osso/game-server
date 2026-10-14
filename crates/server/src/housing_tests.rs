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
    world.insert_resource(GameDatabase::new(db));
    world.insert_resource(manager);
    world.insert_resource(maps);
    world.insert_resource(HousingAvailability(Ok(catalog.clone())));
    world.init_resource::<crate::gold::GoldUpdateQueue>();
    world.insert_resource(crate::game_objects::GameObjectSpawns::default());
    crate::maps::transfer::spawn_map_content(&mut world, copy);
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
    world.entity_mut(player).remove::<InMap>();
    assert!(
        execute(&mut world, actor, &HousingCommand::EnterHouse { house_id })
            .unwrap_err()
            .contains("entering the world")
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

#[test]
fn housing_finder_relationships_use_persisted_friends_and_account_ownership() {
    use shared::protocol::HousingPlotOwnerType;
    let dir = tempfile::tempdir().unwrap();
    let db = redb::Database::create(dir.path().join("friend-housing.redb")).unwrap();
    persistence::create_tables(&db);
    let (owner, _) = persistence::create_character(&db, 1, "Friendowner", 1, 2).unwrap();
    let (viewer, _) = persistence::create_character(&db, 2, "Friendviewer", 1, 2).unwrap();
    persistence::save_inventory(
        &db,
        owner,
        &InventoryData {
            gold: 20_000_000,
            items: vec![],
        },
    );
    let source = crate::housing_catalog::HousingCatalog::read(
        &crate::housing_catalog::tests::catalog_fixture(),
    )
    .unwrap();
    store::purchase(
        &db,
        store::Actor {
            account_id: 1,
            character_id: owner,
            neighborhood_id: Some(1),
            inside_house: None,
        },
        source.plot(417).unwrap(),
    )
    .unwrap();
    let mut world = World::new();
    world.insert_resource(GameDatabase::new(db));
    world.insert_resource(InstanceManager::default());
    world.insert_resource(HousingAvailability(Ok(source)));
    let player = world.spawn(InMap::continent(0)).id();
    let actor = Actor {
        connection: Entity::PLACEHOLDER,
        player,
        account: 2,
        character: viewer,
    };
    assert_eq!(
        read_snapshot(&world, actor, Some(1), None).unwrap().plots[0].owner_type,
        HousingPlotOwnerType::Stranger
    );
    persistence::save_friends(
        &world.resource::<GameDatabase>().db,
        viewer,
        &crate::friends::PlayerFriendsState {
            entries: vec![crate::friends::FriendRecord {
                name: "Friendowner".into(),
                note: String::new(),
            }],
        },
    );
    assert_eq!(
        read_snapshot(&world, actor, Some(1), None).unwrap().plots[0].owner_type,
        HousingPlotOwnerType::Friend
    );
    let (alt, _) =
        persistence::create_character(&world.resource::<GameDatabase>().db, 1, "Friendalt", 1, 2)
            .unwrap();
    let self_actor = Actor {
        account: 1,
        character: alt,
        ..actor
    };
    assert_eq!(
        read_snapshot(&world, self_actor, Some(1), None)
            .unwrap()
            .plots[0]
            .owner_type,
        HousingPlotOwnerType::SelfOwned
    );
    persistence::save_friends(
        &world.resource::<GameDatabase>().db,
        viewer,
        &crate::friends::PlayerFriendsState::default(),
    );
    assert_eq!(
        read_snapshot(&world, actor, Some(1), None).unwrap().plots[0].owner_type,
        HousingPlotOwnerType::Stranger
    );
}
