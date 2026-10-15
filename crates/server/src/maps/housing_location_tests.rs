use super::*;
use crate::persistence::{GameDatabase, housing_store as store};
use std::time::Instant;

#[test]
fn housing_neighborhood_location_waits_for_fifo_before_transfer_and_reply() {
    assert_location_fifo(HousingCommand::EnterNeighborhood { neighborhood_id: 1 });
}

#[test]
fn housing_plot_location_waits_for_fifo_before_transfer_and_reply() {
    assert_location_fifo(HousingCommand::VisitPlot { plot_id: 417 });
}

#[test]
fn housing_house_location_waits_for_fifo_before_transfer_and_reply() {
    assert_location_fifo(HousingCommand::EnterHouse { house_id: 1 });
}

#[test]
fn housing_leave_location_waits_for_fifo_before_transfer_and_reply() {
    assert_location_fifo(HousingCommand::LeaveHouse);
}

fn assert_location_fifo(command: HousingCommand) {
    let dir = tempfile::tempdir().unwrap();
    let db = redb::Database::create(dir.path().join("location.redb")).unwrap();
    crate::persistence::create_tables(&db);
    let source = crate::housing_catalog::HousingCatalog::read(
        &crate::housing_catalog::tests::catalog_fixture(),
    )
    .unwrap();
    crate::persistence::save_inventory(
        &db,
        30,
        &crate::persistence::InventoryData {
            gold: 20_000_000,
            items: vec![],
        },
    );
    let actor = store::Actor {
        account_id: 30,
        character_id: 30,
        neighborhood_id: Some(1),
        inside_house: None,
    };
    assert_eq!(
        store::purchase(&db, actor, source.plot(417).unwrap())
            .unwrap()
            .house_id,
        1
    );
    let entering_house = matches!(command, HousingCommand::EnterHouse { .. });
    let leaving_house = matches!(command, HousingCommand::LeaveHouse);
    let mut h = housing_harness(db);
    let traveler = h.spawn(30, InMap::continent(0), STOCKADE_ENTRANCE_TRIGGER);
    h.app
        .world_mut()
        .entity_mut(traveler.connection)
        .insert((pair::<HousingRequest>(), pair::<HousingReply>()));
    if leaving_house {
        assert_eq!(
            h.housing_request(&traveler, HousingCommand::EnterHouse { house_id: 1 })
                .error,
            None
        );
        h.ack(&traveler);
    }
    let source_map = h.map_of(&traveler);
    let database = h.app.world().resource::<GameDatabase>();
    let held = database.db.begin_write().unwrap();
    let prior = database.writer.enqueue("earlier housing location", |db| {
        store::save_neighborhood_location(
            db,
            30,
            &store::NeighborhoodLocation {
                map_id: 2736,
                neighborhood_id: 2,
            },
        )
    });
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        held.abort().unwrap();
    });
    h.app
        .world_mut()
        .get_mut::<MessageSender<HousingRequest>>(traveler.connection)
        .unwrap()
        .send::<HousingChannel>(HousingRequest {
            request_id: 91,
            command,
        });
    let started = Instant::now();
    h.advance(3);
    let elapsed = started.elapsed();
    let premature = h
        .app
        .world_mut()
        .get_mut::<MessageReceiver<HousingReply>>(traveler.connection)
        .unwrap()
        .receive()
        .next();
    let pending_map = h.map_of(&traveler);
    let pending_transfer = h.app.world().get::<MapTransfer>(traveler.player).is_some();
    release.join().unwrap();
    assert!(
        elapsed < Duration::from_millis(50),
        "housing location tick waited: {elapsed:?}"
    );
    assert!(premature.is_none(), "location reply preceded commit");
    assert_eq!(pending_map, source_map);
    assert!(!pending_transfer, "housing transfer preceded commit");
    let deadline = Instant::now() + Duration::from_secs(3);
    let reply = loop {
        h.advance(1);
        if let Some(reply) = h
            .app
            .world_mut()
            .get_mut::<MessageReceiver<HousingReply>>(traveler.connection)
            .unwrap()
            .receive()
            .next()
        {
            break reply;
        }
        assert!(Instant::now() < deadline, "no committed location reply");
        std::thread::sleep(Duration::from_millis(1));
    };
    assert_eq!(reply.request_id, 91);
    assert_eq!(reply.error, None);
    assert_eq!(prior.result(), Some(Ok(())));
    let saved = store::load_neighborhood_location(&h.app.world().resource::<GameDatabase>().db, 30)
        .unwrap()
        .unwrap();
    if entering_house {
        assert_eq!(saved.map_id, 2736);
        assert_eq!(saved.neighborhood_id, 2);
        let saved_house =
            store::load_house_location(&h.app.world().resource::<GameDatabase>().db, 30, None)
                .unwrap();
        assert_eq!(saved_house.house_id, 1);
    } else {
        assert_eq!(saved.map_id, 2735);
        assert_eq!(saved.neighborhood_id, 1);
    }
    h.ack(&traveler);
    assert_eq!(
        h.map_of(&traveler).unwrap().map_id,
        if entering_house { 2783 } else { 2735 }
    );
}
