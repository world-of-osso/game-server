use super::*;
use crate::persistence::{self, GameDatabase, housing_store as store};
use std::path::Path;
use std::time::Instant;

#[derive(Clone, Copy)]
enum Race {
    TwoRequests,
    Departure,
    SourceChange,
}

#[test]
fn housing_location_two_requests_keep_character_and_relog_property_coherent() {
    assert_committed_destination(
        Race::TwoRequests,
        HousingCommand::EnterNeighborhood { neighborhood_id: 1 },
    );
}

#[test]
fn housing_location_departure_snapshot_keeps_committed_relog_property() {
    assert_committed_destination(
        Race::Departure,
        HousingCommand::EnterNeighborhood { neighborhood_id: 1 },
    );
}

#[test]
fn housing_location_source_change_snapshot_keeps_committed_relog_property() {
    assert_committed_destination(
        Race::SourceChange,
        HousingCommand::EnterNeighborhood { neighborhood_id: 1 },
    );
}

#[test]
fn housing_location_home_departure_snapshot_keeps_committed_relog_property() {
    assert_committed_destination(Race::Departure, HousingCommand::EnterHouse { house_id: 1 });
}

#[test]
fn housing_location_leave_departure_snapshot_keeps_committed_relog_property() {
    assert_committed_destination(Race::Departure, HousingCommand::LeaveHouse);
}

#[test]
fn housing_location_plot_departure_snapshot_keeps_committed_relog_property() {
    assert_committed_destination(Race::Departure, HousingCommand::VisitPlot { plot_id: 417 });
}

fn prepare_race(
    db: redb::Database,
    command: &HousingCommand,
) -> (Harness, Traveler, WorldLocation) {
    persistence::create_tables(&db);
    let source = crate::housing_catalog::tests::catalog_fixture();
    source.execute("INSERT INTO housing_catalog_record SELECT build_key, table_name, 2, fields_json FROM housing_catalog_record WHERE table_name='NeighborhoodMap' AND record_id=1", []).unwrap();
    let catalog = crate::housing_catalog::HousingCatalog::read(&source).unwrap();
    if matches!(
        command,
        HousingCommand::EnterHouse { .. } | HousingCommand::LeaveHouse
    ) {
        persistence::save_inventory(
            &db,
            30,
            &persistence::InventoryData {
                gold: 20_000_000,
                items: vec![],
            },
        );
        store::purchase(
            &db,
            store::Actor {
                account_id: 30,
                character_id: 30,
                neighborhood_id: Some(1),
                inside_house: None,
            },
            catalog.plot(417).unwrap(),
        )
        .unwrap();
    }
    let destination = match command {
        HousingCommand::EnterHouse { .. } => catalog.home_entry().unwrap(),
        HousingCommand::VisitPlot { .. } | HousingCommand::LeaveHouse => WorldLocation {
            position: Vec3::from_array(catalog.plot(417).unwrap().teleport),
            ..catalog.neighborhood(1).unwrap().entry
        },
        _ => catalog.neighborhood(1).unwrap().entry,
    };
    let mut h = housing_harness(db);
    h.app
        .insert_resource(crate::housing::HousingAvailability(Ok(catalog)));
    let who = h.spawn(30, InMap::continent(0), STOCKADE_ENTRANCE_TRIGGER);
    h.app
        .world_mut()
        .entity_mut(who.connection)
        .insert((pair::<HousingRequest>(), pair::<HousingReply>()));
    if matches!(command, HousingCommand::LeaveHouse) {
        assert_eq!(
            h.housing_request(&who, HousingCommand::EnterHouse { house_id: 1 })
                .error,
            None
        );
        h.ack(&who);
    }
    (h, who, destination)
}

fn assert_committed_destination(race: Race, command: HousingCommand) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("location-race.redb");
    let db = redb::Database::create(&path).unwrap();
    let (mut h, who, destination) = prepare_race(db, &command);
    let source_map = h.map_of(&who).unwrap();
    let held = h
        .app
        .world()
        .resource::<GameDatabase>()
        .db
        .begin_write()
        .unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        held.abort().unwrap();
    });
    send_request(&mut h, &who, 91, command);
    if matches!(race, Race::TwoRequests) {
        send_request(
            &mut h,
            &who,
            92,
            HousingCommand::EnterNeighborhood { neighborhood_id: 2 },
        );
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
            source_map.map_id,
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
        assert_eq!(
            h.map_of(&who),
            Some(source_map),
            "departed actor was stale-teleported"
        );
    }
    if matches!(race, Race::SourceChange) {
        assert_eq!(
            h.map_of(&who),
            Some(InMap::continent(1)),
            "changed source was stale-teleported"
        );
    }
    if matches!(race, Race::TwoRequests) {
        assert_eq!(
            replies.iter().filter(|reply| reply.error.is_none()).count(),
            1
        );
    }
    drop(h);
    assert_relogged_destination(&path, destination);
}

fn assert_relogged_destination(path: &Path, destination: WorldLocation) {
    let db = redb::Database::open(path).unwrap();
    let character = persistence::load_character(&db, 30).unwrap();
    let map_id = crate::maps::persistence::load_character_map(&db, 30);
    if destination.map_id == 2783 {
        assert_eq!(
            store::load_house_location(&db, 30, None).unwrap().house_id,
            1
        );
    } else {
        let admission = store::load_neighborhood_location(&db, 30).unwrap().unwrap();
        assert_eq!(
            (admission.map_id, admission.neighborhood_id),
            (destination.map_id, 1)
        );
    }
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
        place.map.map_id, destination.map_id,
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
    assert_eq!(
        (character.x, character.y, character.z),
        (
            destination.position.x,
            destination.position.y,
            destination.position.z
        )
    );
}

fn send_request(h: &mut Harness, who: &Traveler, request_id: u64, command: HousingCommand) {
    h.app
        .world_mut()
        .get_mut::<MessageSender<HousingRequest>>(who.connection)
        .unwrap()
        .send::<HousingChannel>(HousingRequest {
            request_id,
            command,
        });
}
