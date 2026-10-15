use super::*;
use crate::persistence;
use crate::spell_cast::test_harness::{Harness, harness};
use shared::protocol::PvpChannel;
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
enum CompletionContext {
    City,
    Departed,
    Instance,
}

#[test]
fn war_mode_toggle_waits_for_fifo_and_preserves_prior_honor() {
    assert_toggle_fifo(CompletionContext::City);
}

#[test]
fn war_mode_departed_session_receives_no_committed_preference_or_aura() {
    assert_toggle_fifo(CompletionContext::Departed);
}

#[test]
fn war_mode_committed_preference_uses_current_instance_before_applying_enlisted() {
    assert_toggle_fifo(CompletionContext::Instance);
}

fn city_harness() -> (Harness, u64) {
    let mut h = harness(Vec::new(), &[282559, 269083], 2).unwrap();
    let db = persistence::temp_database();
    let (character, _) = persistence::create_character(&db, 41, "Warmodeproof", 1, 2).unwrap();
    let source = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let maps = crate::maps::store::load_map_store(&source).unwrap();
    let wdts = maps
        .maps
        .iter()
        .map(|(&id, map)| (id, map.wdt_file_data_id))
        .collect();
    let mut ground = crate::ground::GroundMap::new(&crate::ground::ground_data_dir(), wdts);
    // Trade District MCNK5148, parent Stormwind1519; same authored terrain as city-rest proof.
    let position = Position {
        x: -8850.0,
        y: 60.0,
        z: -650.0,
    };
    assert_eq!(
        ground.area_at(0, Vec3::new(position.x, position.y, position.z)),
        Some(5148)
    );
    h.app.insert_resource(ground);
    h.app
        .insert_resource(crate::area_table::load_area_table(&source).unwrap());
    h.app
        .insert_resource(crate::faction::load_faction_templates(&source).unwrap());
    h.app.insert_resource(GameDatabase::new(db));
    h.app.world_mut().entity_mut(h.caster).insert((
        position,
        crate::maps::InMap::continent(0),
        persistence::ClientId(character),
        PlayerPvpState::default(),
        CombatStatus(false),
    ));
    h.app.world_mut().get_mut::<Player>(h.caster).unwrap().race = 1;
    h.app.world_mut().entity_mut(h.connection).insert((
        Authenticated { account_id: 41 },
        InWorld {
            character_id: character,
        },
        MessageSender::<SetWarModeDesired>::default(),
        MessageReceiver::<SetWarModeDesired>::default(),
        MessageSender::<PvpStateUpdate>::default(),
        MessageReceiver::<PvpStateUpdate>::default(),
    ));
    h.app.init_resource::<crate::world_data::WorldData>();
    Plugin::build(&crate::rest::RestPlugin, &mut h.app);
    h.advance(2);
    assert!(
        h.app
            .world()
            .get::<crate::rest::PlayerRestState>(h.caster)
            .unwrap()
            .is_resting()
    );
    install_persistence(&mut h.app);
    h.app.add_systems(Update, receive_requests);
    (h, character)
}

fn assert_toggle_fifo(completion: CompletionContext) {
    let (mut h, character) = city_harness();
    let db = h.app.world().resource::<GameDatabase>();
    let held = db.db.begin_write().unwrap();
    let prior = db.writer.enqueue("earlier war mode honor", move |db| {
        let mut progress = persistence::load_pvp(db, character);
        progress.honor = 777;
        persistence::save_pvp(db, character, &progress);
        Ok(())
    });
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        held.abort().unwrap();
    });
    h.app
        .world_mut()
        .get_mut::<MessageSender<SetWarModeDesired>>(h.connection)
        .unwrap()
        .send::<PvpChannel>(SetWarModeDesired { enabled: true });
    let started = Instant::now();
    h.advance(3);
    let elapsed = started.elapsed();
    let premature: Vec<_> = h
        .app
        .world_mut()
        .get_mut::<MessageReceiver<PvpStateUpdate>>(h.connection)
        .unwrap()
        .receive()
        .collect();
    let premature_preference = h
        .app
        .world()
        .get::<PlayerPvpState>(h.caster)
        .unwrap()
        .progress
        .war_mode_desired;
    let premature_aura = crate::auras::has_aura(h.app.world(), h.caster, ENLISTED_INSIDE);
    match completion {
        CompletionContext::City => {}
        CompletionContext::Departed => {
            h.app
                .world_mut()
                .entity_mut(h.connection)
                .remove::<InWorld>();
        }
        CompletionContext::Instance => {
            h.app
                .world_mut()
                .entity_mut(h.caster)
                .insert(crate::maps::InMap {
                    map_id: 489,
                    instance_id: 7,
                });
        }
    }
    let fence = h
        .app
        .world()
        .resource::<GameDatabase>()
        .writer
        .enqueue("war mode proof fence", |_| Ok(()));
    release.join().unwrap();
    assert!(
        elapsed < Duration::from_millis(50),
        "War Mode tick waited: {elapsed:?}"
    );
    assert!(premature.is_empty(), "War Mode reply preceded commit");
    assert!(
        !premature_preference && !premature_aura,
        "War Mode live effect preceded commit"
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut replies = Vec::new();
    let mut frames = 0;
    loop {
        h.advance(1);
        frames += 1;
        replies.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<PvpStateUpdate>>(h.connection)
                .unwrap()
                .receive(),
        );
        let received_expected_reply =
            matches!(completion, CompletionContext::Departed) || !replies.is_empty();
        let committed = fence.result().is_some();
        if frames >= 6 && committed && received_expected_reply {
            break;
        }
        assert!(Instant::now() < deadline, "War Mode FIFO did not complete");
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(prior.result(), Some(Ok(())));
    let durable = persistence::load_pvp(&h.app.world().resource::<GameDatabase>().db, character);
    assert_eq!(durable.honor, 777);
    assert!(durable.war_mode_desired);
    let state = h.app.world().get::<PlayerPvpState>(h.caster).unwrap();
    if matches!(completion, CompletionContext::Departed) {
        assert!(replies.is_empty());
        assert!(!state.progress.war_mode_desired);
    } else {
        assert_eq!(state.progress.honor, 777);
        assert!(state.progress.war_mode_desired);
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].error, None);
        assert!(
            replies[0]
                .snapshot
                .as_ref()
                .unwrap()
                .war_mode
                .as_ref()
                .unwrap()
                .desired
        );
    }
    assert_eq!(
        crate::auras::has_aura(h.app.world(), h.caster, ENLISTED_INSIDE),
        matches!(completion, CompletionContext::City)
    );
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.caster,
        ENLISTED_OUTSIDE
    ));
}
