//! Real filesystem boundaries of startup's metadata-only queries.
use super::*;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::symlink;
use std::sync::mpsc;
use std::time::Duration;

fn fifo(path: &Path) {
    assert!(
        std::process::Command::new("mkfifo")
            .arg(path)
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn spawn_latency_area_lookup_does_not_wait_for_collision_io_and_later_floor_is_unchanged() {
    let source = ground_data_dir();
    let private = tempfile::tempdir().unwrap();
    let terrain = private.path().join("terrain");
    std::fs::create_dir(&terrain).unwrap();
    symlink(source.join("models"), private.path().join("models")).unwrap();
    let wdt = std::fs::read(source.join("terrain/775971.wdt")).unwrap();
    let feet = Vec3::new(-8820.0, 98.1, -660.0);
    let (row, col) = bevy_to_tile(feet.x, feet.z);
    let tile = parse_maid(&wdt).unwrap()[col as usize * MAP_TILES + row as usize];
    assert_ne!(tile.obj0, 0);
    std::fs::write(terrain.join("775971.wdt"), wdt).unwrap();
    symlink(
        source.join(format!("terrain/{}.adt", tile.root)),
        terrain.join(format!("{}.adt", tile.root)),
    )
    .unwrap();
    let objects = terrain.join(format!("{}.adt", tile.obj0));
    fifo(&objects);
    // Keep the FIFO open, but provide no collision bytes. Eager area queries block here.
    let hold = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&objects)
        .unwrap();
    let mut reference = GroundMap::new(&source, HashMap::from([(0, 775971)]));
    let expected_area = reference
        .area_at(0, feet)
        .expect("authored auction house area");
    let expected_floor = reference.ground_at(0, feet);
    assert!(
        matches!(expected_floor, GroundProbe::Supported(height) if (height - 98.06).abs() < 0.05),
        "authored auction house floor: {expected_floor:?}"
    );
    let root = private.path().to_path_buf();
    let (send, receive) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let mut ground = GroundMap::new(&root, HashMap::from([(0, 775971)]));
        let area = ground.area_at(0, feet);
        send.send(area).unwrap();
        ground
    });
    let immediate = receive.recv_timeout(Duration::from_secs(2));
    // Always release and join before asserting, including the RED path.
    drop(hold);
    let mut ground = worker.join().unwrap();
    std::fs::remove_file(&objects).unwrap();
    symlink(source.join(format!("terrain/{}.adt", tile.obj0)), &objects).unwrap();
    assert_eq!(
        immediate,
        Ok(Some(expected_area)),
        "area-only level tuning waited for collision I/O"
    );
    assert_eq!(ground.ground_at(0, feet), expected_floor);
    assert_eq!(
        ground.ground_at(0, feet),
        expected_floor,
        "cached collision changed the floor"
    );
}

#[test]
fn spawn_latency_map_validation_overlaps_io_and_preserves_sorted_refusals() {
    let private = tempfile::tempdir().unwrap();
    let terrain = private.path().join("terrain");
    std::fs::create_dir(&terrain).unwrap();
    let (entered, receive) = mpsc::channel();
    let mut writers = Vec::new();
    let mut releases = Vec::new();
    for id in [101, 102] {
        let path = terrain.join(format!("{id}.wdt"));
        fifo(&path);
        let entered = entered.clone();
        let (release, wait) = mpsc::channel();
        releases.push(release);
        writers.push(std::thread::spawn(move || {
            let mut file = OpenOptions::new().write(true).open(path).unwrap();
            entered.send(id).unwrap();
            wait.recv().unwrap();
            // Valid MPHD, missing MAID: exact parse refusal must survive concurrency.
            file.write_all(b"DHPM\x04\0\0\0\0\0\0\0").unwrap();
        }));
    }
    let root = private.path().to_path_buf();
    let check = std::thread::spawn(move || {
        let ground = GroundMap::new(&root, HashMap::from([(1, 101), (2, 102)]));
        check_spawned_ground(&ground, [2, 1], &HashSet::new())
    });
    let first = receive.recv_timeout(Duration::from_secs(5));
    let second = receive.recv_timeout(Duration::from_secs(2));
    for release in releases {
        release.send(()).unwrap();
    }
    for writer in writers {
        writer.join().unwrap();
    }
    let check = check.join().unwrap();
    assert!(
        first.is_ok() && second.is_ok(),
        "independent map I/O serialized: {first:?}, {second:?}"
    );
    assert_eq!(
        check,
        SpawnedGroundCheck {
            missing: vec![
                "map 1: terrain/101.wdt: WDT has no MAID".into(),
                "map 2: terrain/102.wdt: WDT has no MAID".into(),
            ],
            known_gaps: vec![],
            known_gap_files: 0,
        }
    );
}
