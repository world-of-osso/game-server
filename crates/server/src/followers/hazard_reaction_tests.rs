//! FOLLOWER_HAZARD_REACTION_POLICY uses simulation time and the real worker.
use super::hazard_tests::{FROM, hostile_field_at};
use super::navigation_tests::stockade_fixture;
use bevy::prelude::*;
use shared::components::Position;
use std::time::Duration;

fn advance_clock(world: &mut World, millis: u64) {
    let delta = Duration::from_millis(millis);
    world.resource_mut::<Time>().advance_by(delta);
    world.resource_mut::<Time<Virtual>>().advance_by(delta);
}

fn at(world: &World, unit: Entity) -> Position {
    *world.get::<Position>(unit).unwrap()
}

#[test]
fn follower_hazard_reaction_holds_until_250ms_then_moves_on_ready_route() {
    let (mut h, unit, _) = stockade_fixture(FROM);
    hostile_field_at(&mut h, unit, FROM);
    let initial = at(h.app.world(), unit);
    assert!(super::hazards::escape(h.app.world_mut(), unit));
    // Publish every planning stage without advancing the reaction clock.
    for _ in 0..3 {
        crate::navigation::drain_fixture(&mut h.app);
        assert!(super::hazards::escape(h.app.world_mut(), unit));
        assert_eq!(
            at(h.app.world(), unit),
            initial,
            "ready routes cannot bypass reaction time"
        );
    }
    for _ in 0..4 {
        advance_clock(h.app.world_mut(), 50);
        assert!(super::hazards::escape(h.app.world_mut(), unit));
        assert_eq!(
            at(h.app.world(), unit),
            initial,
            "no movement before detection +250ms"
        );
    }
    advance_clock(h.app.world_mut(), 50);
    assert!(super::hazards::escape(h.app.world_mut(), unit));
    assert_ne!(
        at(h.app.world(), unit),
        initial,
        "ready escape starts at the 250ms boundary"
    );
}

#[test]
fn follower_hazard_reaction_held_worker_stands_inside_then_moves_at_publication() {
    let (mut h, unit, _) = stockade_fixture(FROM);
    hostile_field_at(&mut h, unit, FROM);
    let dir = tempfile::tempdir().unwrap();
    let map = dir.path().join("34");
    std::fs::create_dir(&map).unwrap();
    let original =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/ground/navmesh/34");
    let tile = navigation::tile_of(Vec3::new(FROM.x, FROM.y, FROM.z));
    let held_name = format!("{}_{}.nav", tile.0, tile.1);
    for entry in std::fs::read_dir(&original).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() != held_name.as_str() {
            std::fs::copy(entry.path(), map.join(entry.file_name())).unwrap();
        }
    }
    let path = map.join(&held_name);
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    let bytes = std::fs::read(original.join(&held_name)).unwrap();
    let (reached_tx, reached_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let writer = std::thread::spawn(move || {
        crate::navigation::blocking_io_tests::write_held_tile(path, bytes, reached_tx, release_rx)
    });
    h.app
        .insert_resource(crate::navigation::Navigation::new(dir.path()));
    let initial = at(h.app.world(), unit);
    assert!(super::hazards::escape(h.app.world_mut(), unit));
    let reached = reached_rx.recv_timeout(Duration::from_secs(5));
    for _ in 0..10 {
        advance_clock(h.app.world_mut(), 50);
        assert!(super::hazards::escape(h.app.world_mut(), unit));
        assert_eq!(
            at(h.app.world(), unit),
            initial,
            "late worker permits standing in the hazard"
        );
    }
    let _ = release_tx.send(());
    assert!(
        !writer.join().unwrap().unwrap(),
        "test must release the actual held read, not rescue it"
    );
    reached.expect("escape request must be submitted before the reaction deadline");
    crate::navigation::drain_fixture(&mut h.app);
    assert!(super::hazards::escape(h.app.world_mut(), unit));
    assert_ne!(
        at(h.app.world(), unit),
        initial,
        "late publication starts movement on its first tick without another planning roundtrip"
    );
}
