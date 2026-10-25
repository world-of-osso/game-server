//! Real UseGameObject transport and model-box reach, not a direct housing command.
use super::*;
use crate::maps::{InMap, InstanceManager};
use shared::protocol::{HousingReply, UseGameObject};

#[test]
fn housing_cornerstone_use_opens_only_for_authenticated_same_copy_player_in_real_reach() {
    let source = crate::housing_catalog::tests::catalog_fixture();
    let catalog = crate::housing_catalog::HousingCatalog::read(&source).unwrap();
    let maps = crate::maps::store::load_map_store(&world_db().unwrap()).unwrap();
    let mut manager = InstanceManager::default();
    let copy = manager
        .authorize_housing_entry(&maps, 2735, 1, 1)
        .unwrap()
        .map;
    let objects = crate::game_objects::GameObjectSpawns::default();
    let mut h = harness_with(HUMAN_TEMPLATE, move |app| {
        app.insert_resource(crate::housing::HousingAvailability(Ok(catalog)));
        app.insert_resource(maps);
        app.insert_resource(manager);
        app.insert_resource(objects);
        add_inbox::<HousingReply>(app);
    })
    .unwrap();
    h.app.world_mut().entity_mut(h.player).insert(copy);
    h.app.world_mut().entity_mut(h.connection).insert((
        crate::auth::Authenticated { account_id: 1 },
        pair::<UseGameObject>(),
        pair::<HousingReply>(),
    ));
    crate::maps::transfer::spawn_map_content(h.app.world_mut(), copy);
    let object = {
        let mut query = h.app.world_mut().query::<(Entity, &GameObjectInfo)>();
        query
            .iter(h.app.world())
            .find(|(_, info)| info.entry == 527268)
            .unwrap()
            .0
    };
    let origin = *h.app.world().get::<Position>(object).unwrap();
    // World-space origin is deliberately unrelated to the actor's initial0 position.
    h.send::<_, InteractionChannel>(UseGameObject {
        object: object.to_bits(),
    });
    assert!(h.take::<HousingReply>().is_empty());
    assert_eq!(
        h.take::<InteractionFailed>()[0].error,
        InteractionError::TooFarAway
    );
    h.app.world_mut().entity_mut(h.player).insert(origin);
    h.send::<_, InteractionChannel>(UseGameObject {
        object: object.to_bits(),
    });
    let replies = h.take::<HousingReply>();
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].state.opened_cornerstone, Some(417));
    assert_eq!(replies[0].state.neighborhood_id, Some(1));
    assert_eq!(replies[0].state.plots[0].plot_id, 417);
    assert_eq!(replies[0].state.plots[0].house_id, None);
    assert!(replies[0].state.owned_houses.is_empty());
    assert!(h.take::<InteractionFailed>().is_empty());
    // Identical coordinates in a different copy do not authorize inspection.
    h.app.world_mut().entity_mut(h.player).insert(InMap {
        map_id: 2735,
        instance_id: copy.instance_id + 1,
    });
    h.send::<_, InteractionChannel>(UseGameObject {
        object: object.to_bits(),
    });
    assert!(h.take::<HousingReply>().is_empty());
    assert_eq!(
        h.take::<InteractionFailed>()[0].error,
        InteractionError::InvalidTarget
    );
    h.app.world_mut().entity_mut(h.player).insert(copy);
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .remove::<crate::auth::Authenticated>();
    h.send::<_, InteractionChannel>(UseGameObject {
        object: object.to_bits(),
    });
    assert!(h.take::<HousingReply>().is_empty());
    assert_eq!(
        h.take::<InteractionFailed>()[0].error,
        InteractionError::InvalidTarget
    );
}

#[test]
fn housing_bulletin_board_use_lists_this_neighborhoods_residents_alphabetically_with_presence() {
    let source = crate::housing_catalog::tests::catalog_fixture();
    crate::housing_catalog::tests::add_bulletin_board(&source);
    // NeighborhoodPlot 418 (PlotIndex 1) as authored; its cornerstone row is not imported.
    let plot = serde_json::json!({"NeighborhoodMapID":"1", "PlotIndex":"1", "Name":"", "Cost":"10000000", "CornerstonePosition_0":"2674.317626953125", "CornerstonePosition_1":"651.8455200195312", "CornerstonePosition_2":"49.2256965637207", "TeleportPosition_0":"2675.425048828125", "TeleportPosition_1":"649.3803100585938", "TeleportPosition_2":"49.53499984741211", "CornerstoneGameObjectID":"527310"});
    source
        .execute(
            "INSERT INTO housing_catalog_record VALUES (?, 'NeighborhoodPlot', 418, ?)",
            rusqlite::params![
                crate::persistence::housing_store::BUILD_KEY,
                plot.to_string()
            ],
        )
        .unwrap();
    let catalog = crate::housing_catalog::HousingCatalog::read(&source).unwrap();
    let plots = [
        catalog.plot(417).unwrap().clone(),
        catalog.plot(418).unwrap().clone(),
    ];
    let maps = crate::maps::store::load_map_store(&world_db().unwrap()).unwrap();
    let mut manager = InstanceManager::default();
    let copy = manager
        .authorize_housing_entry(&maps, 2735, 1, 1)
        .unwrap()
        .map;
    let objects = crate::game_objects::GameObjectSpawns::default();
    let mut h = harness_with(HUMAN_TEMPLATE, move |app| {
        app.insert_resource(crate::housing::HousingAvailability(Ok(catalog)));
        app.insert_resource(maps);
        app.insert_resource(manager);
        app.insert_resource(objects);
        add_inbox::<HousingReply>(app);
    })
    .unwrap();
    h.app.world_mut().entity_mut(h.player).insert(copy);
    h.app.world_mut().entity_mut(h.connection).insert((
        crate::auth::Authenticated { account_id: 1 },
        pair::<UseGameObject>(),
        pair::<HousingReply>(),
    ));
    // The viewer owns plot 417 (PlotIndex 0); another account's offline character owns 418.
    let (viewer, resident) = {
        let db = &h
            .app
            .world()
            .resource::<crate::persistence::GameDatabase>()
            .db;
        let buy = |account: u64, name: &str, plot: &shared::protocol::HousingPlot| {
            let (character, _) =
                crate::persistence::create_character(db, account, name, 1, 1).unwrap();
            let mut inventory = crate::persistence::load_inventory(db, character);
            inventory.gold = plot.cost;
            crate::persistence::save_inventory(db, character, &inventory);
            let buyer = crate::persistence::housing_store::Actor {
                account_id: account,
                character_id: character,
                neighborhood_id: Some(1),
                inside_house: None,
            };
            crate::persistence::housing_store::purchase(db, buyer, plot).unwrap();
            character
        };
        (
            buy(1, "Housevisitor", &plots[0]),
            buy(2, "Houseproof", &plots[1]),
        )
    };
    h.app.world_mut().entity_mut(h.connection).insert(InWorld {
        character_id: viewer,
    });
    crate::maps::transfer::spawn_map_content(h.app.world_mut(), copy);
    let board = {
        let mut query = h.app.world_mut().query::<(Entity, &GameObjectInfo)>();
        query
            .iter(h.app.world())
            .find(|(_, info)| info.entry == 576398)
            .unwrap()
            .0
    };
    h.send::<_, InteractionChannel>(UseGameObject {
        object: board.to_bits(),
    });
    assert!(h.take::<HousingReply>().is_empty());
    assert_eq!(
        h.take::<InteractionFailed>()[0].error,
        InteractionError::TooFarAway
    );
    let origin = *h.app.world().get::<Position>(board).unwrap();
    h.app.world_mut().entity_mut(h.player).insert(origin);
    h.send::<_, InteractionChannel>(UseGameObject {
        object: board.to_bits(),
    });
    let replies = h.take::<HousingReply>();
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].state.opened_cornerstone, None);
    let roster = replies[0].state.bulletin_board.as_ref().unwrap();
    assert_eq!(roster.neighborhood_id, 1);
    let resident_row =
        |name: &str, online: bool, plot_index: u32| shared::protocol::NeighborhoodResident {
            name: name.into(),
            online,
            plot_index,
        };
    assert_eq!(
        roster.residents,
        vec![
            resident_row("Houseproof", false, 1),
            resident_row("Housevisitor", true, 0),
        ]
    );
    // The resident logs in: the next board use reports them online.
    h.app.world_mut().spawn((
        InWorld {
            character_id: resident,
        },
        crate::auth::Authenticated { account_id: 2 },
    ));
    h.send::<_, InteractionChannel>(UseGameObject {
        object: board.to_bits(),
    });
    let replies = h.take::<HousingReply>();
    let roster = replies[0].state.bulletin_board.as_ref().unwrap();
    assert!(roster.residents.iter().all(|resident| resident.online));
}
