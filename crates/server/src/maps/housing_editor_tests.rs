use super::*;

fn collect_editor_replies(
    h: &mut Harness,
    owner: &Traveler,
    visitor: &Traveler,
) -> (Vec<HousingReply>, Vec<HousingReply>) {
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let mut replies = (Vec::new(), Vec::new());
    let mut frames = 0;
    loop {
        h.advance(1);
        frames += 1;
        for (person, received) in [(owner, &mut replies.0), (visitor, &mut replies.1)] {
            received.extend(
                h.app
                    .world_mut()
                    .get_mut::<MessageReceiver<HousingReply>>(person.connection)
                    .unwrap()
                    .receive(),
            );
        }
        if frames >= 6
            && !crate::housing::persistence_pending(h.app.world())
            && !replies.0.is_empty()
            && !replies.1.is_empty()
        {
            return replies;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "owner/visitor edit publication timed out"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn housing_editor_wire_updates_a_same_house_visitor_without_leaking_owner_storage() {
    use crate::persistence::{
        GameDatabase, InventoryData, InventoryItemData, housing_store as store,
    };
    use shared::protocol::DecorTransform;
    let dir = tempfile::tempdir().unwrap();
    let db = redb::Database::create(dir.path().join("editor.redb")).unwrap();
    crate::persistence::create_tables(&db);
    crate::persistence::save_inventory(
        &db,
        30,
        &InventoryData {
            gold: 20_000_000,
            items: vec![InventoryItemData {
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
    let source = crate::housing_acquisition::tests::collection_fixture();
    h.app
        .insert_resource(crate::housing::HousingAvailability(Ok(
            crate::housing_catalog::HousingCatalog::read(&source).unwrap(),
        )));
    let actor = store::Actor {
        account_id: 30,
        character_id: 30,
        neighborhood_id: Some(1),
        inside_house: None,
    };
    let plot = h
        .app
        .world()
        .resource::<crate::housing::HousingAvailability>()
        .0
        .as_ref()
        .unwrap()
        .plot(417)
        .unwrap();
    let house = store::purchase(&h.app.world().resource::<GameDatabase>().db, actor, plot).unwrap();
    let house_id = house.house_id;
    store::redeem(
        &h.app.world().resource::<GameDatabase>().db,
        actor,
        100,
        &store::RedemptionRule {
            item_id: 245259,
            decor_id: 1693,
        },
    )
    .unwrap();
    let owner = h.spawn(30, InMap::continent(0), STOCKADE_ENTRANCE_TRIGGER);
    let visitor = h.spawn(31, InMap::continent(0), STOCKADE_ENTRANCE_TRIGGER);
    for person in [&owner, &visitor] {
        h.app
            .world_mut()
            .entity_mut(person.connection)
            .insert((pair::<HousingRequest>(), pair::<HousingReply>()));
    }
    assert_eq!(
        h.housing_request(&owner, HousingCommand::EnterHouse { house_id })
            .error,
        None
    );
    h.ack(&owner);
    assert!(
        h.housing_request(&visitor, HousingCommand::EnterHouse { house_id })
            .error
            .is_some()
    );
    assert_eq!(
        h.housing_request(
            &owner,
            HousingCommand::SaveHouseAccess {
                house_id,
                access: shared::protocol::HouseAccess {
                    anyone: true,
                    ..Default::default()
                }
            }
        )
        .error,
        None
    );
    assert_eq!(
        h.housing_request(&visitor, HousingCommand::EnterHouse { house_id })
            .error,
        None
    );
    h.ack(&visitor);
    assert_eq!(h.map_of(&visitor), h.map_of(&owner));
    h.app
        .world_mut()
        .get_mut::<MessageSender<HousingRequest>>(owner.connection)
        .unwrap()
        .send::<HousingChannel>(HousingRequest {
            request_id: 41,
            command: HousingCommand::PlaceDecor {
                variant_id: None,
                house_id,
                decor_id: 1693,
                transform: DecorTransform {
                    position: [-996.0, -2.0, 1000.0],
                    yaw: 0.0,
                    pitch: 0.0,
                    roll: 0.0,
                    scale: 1.0,
                },
            },
        });
    let (owner_replies, visitor_replies) = collect_editor_replies(&mut h, &owner, &visitor);
    assert_eq!(owner_replies.len(), 1);
    assert_eq!(
        visitor_replies.len(),
        1,
        "authoritative accepted edit must publish to the actual same-house observer"
    );
    let owned = &owner_replies[0].state;
    let visited = &visitor_replies[0].state;
    assert_eq!(
        owned.layout.as_ref().unwrap().decor,
        visited.layout.as_ref().unwrap().decor
    );
    assert!(owned.layout.as_ref().unwrap().can_edit);
    assert!(!visited.layout.as_ref().unwrap().can_edit);
    assert!(visited.stored.is_empty() && visited.owned_houses.is_empty());
    let instance_id = visited.layout.as_ref().unwrap().decor[0].instance_id;
    let transform = DecorTransform {
        position: [-995.0, -1.5, 1001.0],
        yaw: 0.3,
        pitch: 0.2,
        roll: -0.4,
        scale: 1.25,
    };
    h.app
        .world_mut()
        .get_mut::<MessageSender<HousingRequest>>(owner.connection)
        .unwrap()
        .send::<HousingChannel>(HousingRequest {
            request_id: 42,
            command: HousingCommand::TransformDecor {
                house_id,
                instance_id,
                transform,
            },
        });
    let (updated, observed) = collect_editor_replies(&mut h, &owner, &visitor);
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].error, None);
    assert_eq!(observed.len(), 1);
    assert_eq!(
        observed[0].state.layout.as_ref().unwrap().decor[0].transform,
        transform
    );
    assert!(!observed[0].state.layout.as_ref().unwrap().can_edit);
    let refused_precision = h.housing_request(
        &visitor,
        HousingCommand::TransformDecor {
            house_id,
            instance_id,
            transform: DecorTransform {
                scale: 2.0,
                ..transform
            },
        },
    );
    assert!(refused_precision.error.unwrap().contains("Only the owner"));
    assert_eq!(
        refused_precision.state.layout.unwrap().decor[0].transform,
        transform
    );
    let refused = h.housing_request(
        &visitor,
        HousingCommand::RemoveDecor {
            house_id,
            instance_id,
        },
    );
    assert!(refused.error.unwrap().contains("Only the owner"));
    assert_eq!(refused.state.layout.as_ref().unwrap().decor.len(), 1);
}
