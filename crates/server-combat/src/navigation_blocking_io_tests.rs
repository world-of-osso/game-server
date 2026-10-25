//! Linux FIFO holds the actual std::fs::read boundary, not a mocked query.
use super::*;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const IO_RESCUE: Duration = Duration::from_secs(5);
const PROGRESS_DEADLINE: Duration = Duration::from_secs(1);
const LINUX_O_NONBLOCK: i32 = 0o4000;
const LINUX_ENXIO: i32 = 6;

#[derive(Clone, Resource)]
struct TickProbe {
    ticks: Arc<AtomicUsize>,
    x: Arc<AtomicU32>,
    stop: Arc<AtomicBool>,
}

#[derive(Component)]
struct ProbeActor;

fn step_probe(
    probe: Res<TickProbe>,
    mut nav: ResMut<Navigation>,
    mut actors: Query<(Entity, &mut Position), With<ProbeActor>>,
) {
    probe.ticks.fetch_add(1, Ordering::SeqCst);
    for (entity, mut at) in &mut actors {
        if let Step::Moved { position, .. } = nav.advance(
            entity,
            MoveRequest {
                map_id: 0,
                goal: Goal::Point,
                from: *at,
                to: Position {
                    x: 120.0,
                    y: 0.2,
                    z: 100.0,
                },
                distance: 0.5,
                reach: 0.0,
            },
        ) {
            *at = position;
            probe.x.store(at.x.to_bits(), Ordering::SeqCst);
        }
    }
}

fn run_probe(root: PathBuf, probe: TickProbe) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, NavigationPlugin));
    app.insert_resource(Time::<Fixed>::from_duration(Duration::from_millis(50)));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(50),
    ));
    app.update();
    app.insert_resource(Navigation::new(&root));
    app.insert_resource(probe.clone());
    app.world_mut().spawn((
        ProbeActor,
        Position {
            x: 100.0,
            y: 0.2,
            z: 100.0,
        },
        crate::creature_ai::CreatureState::Idle,
    ));
    app.add_systems(FixedUpdate, step_probe);
    let deadline = Instant::now() + IO_RESCUE * 2;
    while !probe.stop.load(Ordering::SeqCst) && Instant::now() < deadline {
        app.update();
        std::thread::yield_now();
    }
}

pub(crate) fn write_held_tile(
    path: PathBuf,
    bytes: Vec<u8>,
    reached: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
) -> Result<bool, String> {
    let mut writer = open_tile_writer(&path)?;
    reached.send(()).map_err(|error| error.to_string())?;
    let rescued = release.recv_timeout(IO_RESCUE).is_err();
    write_tile_bytes(&mut writer, &bytes)?;
    Ok(rescued)
}

fn open_tile_writer(path: &Path) -> Result<std::fs::File, String> {
    let deadline = Instant::now() + IO_RESCUE;
    loop {
        match std::fs::OpenOptions::new()
            .write(true)
            .custom_flags(LINUX_O_NONBLOCK)
            .open(&path)
        {
            Ok(writer) => return Ok(writer),
            Err(error)
                if error.raw_os_error() == Some(LINUX_ENXIO) && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(error) => {
                return Err(format!(
                    "open held navigation tile {}: {error}",
                    path.display()
                ));
            }
        }
    }
}

fn write_tile_bytes(writer: &mut std::fs::File, bytes: &[u8]) -> Result<(), String> {
    // The FIFO remains nonblocking after the reader-open handshake. Deliver all
    // baked packets, including packets larger than pipe capacity, within rescue.
    let deadline = Instant::now() + IO_RESCUE;
    let mut remaining = bytes;
    while !remaining.is_empty() {
        match writer.write(remaining) {
            Ok(0) => return Err("held tile writer made no progress".into()),
            Ok(written) => remaining = &remaining[written..],
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                std::thread::yield_now();
            }
            Err(error) => return Err(format!("write held navigation tile: {error}")),
        }
    }
    Ok(())
}

pub(super) fn baked_floor() -> Vec<u8> {
    let a = Vec3::new(10.0, 0.0, 10.0);
    let b = Vec3::new(10.0, 0.0, 190.0);
    let c = Vec3::new(190.0, 0.0, 190.0);
    let d = Vec3::new(190.0, 0.0, 10.0);
    let bytes = navigation::bake::bake_tile(&[[a, b, c], [a, c, d]], (32, 31)).unwrap();
    assert!(!bytes.is_empty());
    bytes
}

#[test]
fn navigation_held_tile_read_keeps_fixed_ticks_and_publishes_after_release() {
    let dir = tempfile::tempdir().unwrap();
    let map = dir.path().join("0");
    std::fs::create_dir(&map).unwrap();
    let path = map.join("32_31.nav");
    // coreutils mkfifo is a Linux test fixture dependency, never a production path.
    let created = std::process::Command::new("mkfifo")
        .arg(&path)
        .status()
        .unwrap();
    assert!(created.success(), "could not create held-read FIFO");
    let bytes = baked_floor();
    let probe = TickProbe {
        ticks: Arc::new(AtomicUsize::new(0)),
        x: Arc::new(AtomicU32::new(100.0f32.to_bits())),
        stop: Arc::new(AtomicBool::new(false)),
    };
    let (reached_tx, reached_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let writer = std::thread::spawn(move || write_held_tile(path, bytes, reached_tx, release_rx));
    let root = dir.path().to_path_buf();
    let running = probe.clone();
    let simulation = std::thread::spawn(move || run_probe(root, running));
    let reached = reached_rx.recv_timeout(IO_RESCUE);
    let before = probe.ticks.load(Ordering::SeqCst);
    let deadline = Instant::now() + PROGRESS_DEADLINE;
    while probe.ticks.load(Ordering::SeqCst) < before + 8 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    let while_held = probe.ticks.load(Ordering::SeqCst);
    let held_x = f32::from_bits(probe.x.load(Ordering::SeqCst));
    let _ = release_tx.send(());
    let write_result = writer.join().expect("held tile writer panicked");
    let deadline = Instant::now() + IO_RESCUE;
    while f32::from_bits(probe.x.load(Ordering::SeqCst)) <= 100.1 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    let released_x = f32::from_bits(probe.x.load(Ordering::SeqCst));
    probe.stop.store(true, Ordering::SeqCst);
    simulation.join().expect("simulation panicked");
    assert!(
        reached.is_ok(),
        "real tile reader never reached FIFO: {reached:?}"
    );
    assert_eq!(
        write_result.unwrap(),
        false,
        "read required watchdog rescue"
    );
    assert_eq!(held_x, 100.0, "path published while tile read was held");
    assert!(
        released_x > 100.1,
        "real path did not publish after release: x={released_x}"
    );
    assert!(
        while_held >= before + 8,
        "fixed ticks stalled during actual tile read: before={before}, held={while_held}"
    );
}
