//! FOLLOWER_HAZARD_REACTION_POLICY uses simulation time and the real worker.
use super::hazard_tests::{FROM, hostile_field_at};
use super::navigation_tests::stockade_fixture;
use bevy::prelude::*;
use shared::components::Position;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn advance_clock(world: &mut World, millis: u64) {
    let delta = Duration::from_millis(millis);
    world.resource_mut::<Time>().advance_by(delta);
    world.resource_mut::<Time<Virtual>>().advance_by(delta);
}

fn at(world: &World, unit: Entity) -> Position {
    *world.get::<Position>(unit).unwrap()
}

#[derive(Clone, Default)]
struct TraceBuffer(Arc<Mutex<Vec<u8>>>);

impl Write for TraceBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> bevy::log::tracing_subscriber::fmt::MakeWriter<'a> for TraceBuffer {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[test]
#[ignore = "follower hazard live trace RED: enable after implementation"]
fn follower_hazard_trace_correlates_native_detection_worker_and_escape() {
    let (mut h, unit, _) = stockade_fixture(FROM);
    hostile_field_at(&mut h, unit, FROM);
    let buffer = TraceBuffer::default();
    let subscriber = bevy::log::tracing_subscriber::fmt()
        .with_max_level(bevy::log::tracing::Level::DEBUG)
        .with_writer(buffer.clone())
        .without_time()
        .with_ansi(false)
        .finish();
    bevy::log::tracing::subscriber::with_default(subscriber, || {
        assert!(super::hazards::escape(h.app.world_mut(), unit));
        crate::navigation::drain_fixture(&mut h.app);
        assert!(super::hazards::escape(h.app.world_mut(), unit));
        for _ in 0..100 {
            advance_clock(h.app.world_mut(), 50);
            if !super::hazards::escape(h.app.world_mut(), unit) {
                break;
            }
            crate::navigation::drain_fixture(&mut h.app);
        }
    });
    let text = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
    let records: Vec<serde_json::Value> = text
        .lines()
        .filter_map(|line| {
            line.split_once("FOLLOWER_HAZARD_TRACE ")
                .map(|(_, json)| serde_json::from_str(json).unwrap())
        })
        .filter(|record: &serde_json::Value| record["actor"] == unit.to_bits())
        .collect();
    let event = |name: &str| {
        records
            .iter()
            .find(|record| record["event"] == name)
            .unwrap_or_else(|| panic!("missing {name} in native trace: {text}"))
    };
    let detection = event("detected");
    let submitted = event("query_submitted");
    let published = event("query_published");
    let adopted = event("corridor_adopted");
    let moved = event("move");
    let exit = event("exit");
    assert_eq!(detection["schema"], 1);
    assert!(detection["simulation_s"].as_f64().is_some());
    assert!(
        records
            .windows(2)
            .all(|pair| pair[0]["monotonic_ns"].as_u64().unwrap()
                <= pair[1]["monotonic_ns"].as_u64().unwrap())
    );
    assert_eq!(
        submitted["data"]["request_id"],
        published["data"]["request_id"]
    );
    assert_eq!(
        submitted["data"]["request_id"],
        adopted["data"]["request_id"]
    );
    assert!(
        moved["simulation_s"].as_f64().unwrap() >= detection["data"]["react_at"].as_f64().unwrap()
    );
    assert_ne!(moved["data"]["from"], moved["data"]["to"]);
    assert_eq!(exit["data"]["physical_clear"], true);
    assert!(
        records
            .iter()
            .any(|record| record["event"] == "hold" && record["data"]["reason"] == "reaction")
    );
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
