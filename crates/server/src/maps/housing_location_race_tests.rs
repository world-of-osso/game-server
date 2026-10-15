use super::*;
use crate::persistence::{self, GameDatabase, housing_store as store};
use std::time::Instant;

#[derive(Clone, Copy)]
enum Race {
    TwoRequests,
    Departure,
    SourceChange,
}

#[test]
fn housing_location_two_requests_keep_character_and_relog_property_coherent() {
    assert_committed_destination(Race::TwoRequests);
}

#[test]
fn housing_location_departure_snapshot_keeps_committed_relog_property() {
    assert_committed_destination(Race::Departure);
}

#[test]
fn housing_location_source_change_snapshot_keeps_committed_relog_property() {
    assert_committed_destination(Race::SourceChange);
}

fn assert_committed_destination(race: Race) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("location-race.redb");
    let db = redb::Database::create(&path).unwrap();
    persistence::create_tables(&db);
    let source = crate::housing_catalog::tests::catalog_fixture();
    source.execute("INSERT INTO housing_catalog_record SELECT build_key, table_name, 2, fields_json FROM housing_catalog_record WHERE table_name='NeighborhoodMap' AND record_id=1", []).unwrap();
    let mut h = housing_harness(db);
    h.app
        .insert_resource(crate::housing::HousingAvailability(Ok(
            crate::housing_catalog::HousingCatalog::read(&source).unwrap(),
        )));
    let who = h.spawn(30, InMap::continent(0), STOCKADE_ENTRANCE_TRIGGER);
    h.app
        .world_mut()
        .entity_mut(who.connection)
        .insert((pair::<HousingRequest>(), pair::<HousingReply>()));
    let world = h.app.world();
    persistence::save_live_character(
        &world.resource::<GameDatabase>().db,
        30,
        world.get::<Player>(who.player).unwrap(),
        world.get::<Position>(who.player).unwrap(),
        world.get::<Health>(who.player).unwrap(),
        None,
        0,
    );
    let held = world.resource::<GameDatabase>().db.begin_write().unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        held.abort().unwrap();
    });
    send_request(&mut h, &who, 91, 1);
    if matches!(race, Race::TwoRequests) {
        send_request(&mut h, &who, 92, 2);
    }
    h.advance(3);
    let mut replies: Vec<_> = h
        .app
        .world_mut()
        .get_mut::<MessageReceiver<HousingReply>>(who.connection)
        .unwrap()
        .receive()
        .collect();
    let departing = if matches!(race, Race::Departure | Race::SourceChange) {
        if matches!(race, Race::Departure) {
            h.app
                .world_mut()
                .entity_mut(who.connection)
                .remove::<InWorld>();
        } else {
            h.app
                .world_mut()
                .entity_mut(who.player)
                .insert(InMap::continent(1));
        }
        let world = h.app.world();
        Some(persistence::enqueue_live_character(
            world.resource::<GameDatabase>(),
            30,
            world.get::<Player>(who.player).unwrap(),
            world.get::<Position>(who.player).unwrap(),
            world.get::<Health>(who.player).unwrap(),
            None,
            0,
            None,
        ))
    } else {
        None
    };
    release.join().unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while crate::housing::persistence_pending(h.app.world())
        || departing
            .as_ref()
            .is_some_and(|receipt| receipt.result().is_none())
    {
        h.advance(1);
        replies.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<HousingReply>>(who.connection)
                .unwrap()
                .receive(),
        );
        assert!(
            Instant::now() < deadline,
            "housing race receipts did not complete"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    if matches!(race, Race::Departure) {
        assert!(replies.is_empty());
    }
    if matches!(race, Race::TwoRequests) {
        assert_eq!(
            replies.iter().filter(|reply| reply.error.is_none()).count(),
            1
        );
    }
    drop(h);
    let db = redb::Database::open(path).unwrap();
    let character = persistence::load_character(&db, 30).unwrap();
    let map_id = crate::maps::persistence::load_character_map(&db, 30);
    let admission = store::load_neighborhood_location(&db, 30).unwrap().unwrap();
    let mut restarted = housing_harness(db);
    let position = bevy(Vec3::new(character.x, character.y, character.z));
    let place = restarted
        .app
        .world_mut()
        .run_system_once(move |mut commands: Commands, mut maps: LoginMaps| {
            maps.resolve(
                &mut commands,
                30,
                1,
                RaidDifficulty::default(),
                map_id,
                position,
            )
        })
        .unwrap()
        .expect("committed housing destination must be reloggable");
    assert_eq!(
        place.map.map_id, admission.map_id,
        "character destination and admission diverged"
    );
    assert_eq!(
        restarted
            .app
            .world()
            .resource::<InstanceManager>()
            .housing_property(place.map),
        Some(1)
    );
    assert_eq!(admission.neighborhood_id, 1);
    let source = restarted
        .app
        .world()
        .resource::<crate::housing::HousingAvailability>();
    let destination = source.0.as_ref().unwrap().neighborhood(1).unwrap().entry;
    assert_eq!(
        (character.x, character.y, character.z),
        (
            destination.position.x,
            destination.position.y,
            destination.position.z
        )
    );
}

fn send_request(h: &mut Harness, who: &Traveler, request_id: u64, neighborhood_id: u32) {
    h.app
        .world_mut()
        .get_mut::<MessageSender<HousingRequest>>(who.connection)
        .unwrap()
        .send::<HousingChannel>(HousingRequest {
            request_id,
            command: HousingCommand::EnterNeighborhood { neighborhood_id },
        });
}
