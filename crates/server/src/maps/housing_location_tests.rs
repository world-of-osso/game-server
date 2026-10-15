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

fn assert_location_fifo(command: HousingCommand) {
    let dir = tempfile::tempdir().unwrap();
    let db = redb::Database::create(dir.path().join("location.redb")).unwrap();
    crate::persistence::create_tables(&db);
    let mut h = housing_harness(db);
    let traveler = h.spawn(30, InMap::continent(0), STOCKADE_ENTRANCE_TRIGGER);
    h.app
        .world_mut()
        .entity_mut(traveler.connection)
        .insert((pair::<HousingRequest>(), pair::<HousingReply>()));
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
    assert_eq!(pending_map, Some(InMap::continent(0)));
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
    assert_eq!(saved.map_id, 2735);
    assert_eq!(saved.neighborhood_id, 1);
    h.ack(&traveler);
    assert_eq!(h.map_of(&traveler).unwrap().map_id, 2735);
}
