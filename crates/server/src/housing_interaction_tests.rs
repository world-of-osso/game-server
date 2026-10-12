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
    let objects = crate::game_objects::GameObjectSpawns(catalog.cornerstones());
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
