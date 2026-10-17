//! Eleven independent real-UDP apps for private-server profiling; no token files.
use bevy::app::AppExit;
use bevy::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use shared::protocol::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[path = "tick_perf_peers/combat.rs"]
mod combat;

const PEER_COUNT: usize = 11;
const MEASUREMENT: Duration = Duration::from_secs(90);

#[derive(Resource)]
struct Run {
    server: SocketAddr,
    username: String,
    started: Instant,
    sent: bool,
    entered: bool,
    count: Arc<AtomicUsize>,
    ready: mpsc::Sender<usize>,
    index: usize,
    all_entered: Option<Instant>,
}

fn connect(mut commands: Commands, run: Res<Run>) {
    let client_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos() as u64;
    let auth = Authentication::Manual {
        server_addr: run.server,
        client_id,
        private_key: [0; 32],
        protocol_id: 0,
    };
    let netcode =
        NetcodeClient::new(auth, NetcodeConfig::default()).expect("private netcode client");
    let entity = commands
        .spawn((
            LocalAddr(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)),
            PeerAddr(run.server),
            UdpIo::default(),
            netcode,
        ))
        .id();
    commands.trigger(Connect { entity });
}

fn connected(event: On<Add, Connected>, mut commands: Commands) {
    commands.entity(event.entity).insert((
        ReplicationReceiver,
        MessageSender::<LoginRequest>::default(),
        MessageSender::<SelectCharacter>::default(),
        MessageReceiver::<LoginResponse>::default(),
        MessageReceiver::<EnterWorldResponse>::default(),
    ));
}

fn login(
    mut run: ResMut<Run>,
    mut senders: Query<&mut MessageSender<LoginRequest>, With<Connected>>,
) {
    if run.sent {
        return;
    }
    for mut sender in &mut senders {
        sender.send::<AuthChannel>(LoginRequest {
            token: None,
            username: run.username.clone(),
            password: "fbtest".into(),
        });
        run.sent = true;
        println!("LOGIN_SENT {}", run.username);
    }
}

fn select(
    mut peers: Query<(
        &mut MessageReceiver<LoginResponse>,
        &mut MessageSender<SelectCharacter>,
    )>,
) {
    for (mut receiver, mut sender) in &mut peers {
        for reply in receiver.receive() {
            assert!(reply.success, "login rejected: {:?}", reply.error);
            let character = reply
                .characters
                .first()
                .expect("prepared private character");
            sender.send::<AuthChannel>(SelectCharacter {
                character_id: character.character_id,
            });
        }
    }
}

fn entered(
    mut run: ResMut<Run>,
    mut replies: Query<&mut MessageReceiver<EnterWorldResponse>>,
    mut combat: Option<ResMut<combat::Combat>>,
) {
    for mut receiver in &mut replies {
        for reply in receiver.receive() {
            assert!(
                reply.success,
                "{} admission rejected: {:?}",
                run.username, reply.error
            );
            assert!(!run.entered, "duplicate world entry");
            run.entered = true;
            if let Some(combat) = combat.as_mut() {
                combat.owner = reply.player_entity;
            }
            run.count.fetch_add(1, Ordering::SeqCst);
            run.ready.send(run.index).expect("profiling supervisor");
            println!(
                "ENTERED {} elapsed={:.3}",
                run.username,
                run.started.elapsed().as_secs_f64()
            );
        }
    }
    if run.count.load(Ordering::SeqCst) == PEER_COUNT && run.all_entered.is_none() {
        run.all_entered = Some(Instant::now());
        println!(
            "MEASUREMENT_STARTED {} unix_ms={}",
            run.username,
            read_unix_millis()
        );
    }
}

fn read_unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("wall clock before Unix epoch")
        .as_millis()
}

fn finish(
    run: Res<Run>,
    connected: Query<(), With<Connected>>,
    combat: Option<Res<combat::Combat>>,
    mut exit: MessageWriter<AppExit>,
) {
    assert!(
        run.started.elapsed() < Duration::from_secs(600),
        "peer readiness deadline exceeded"
    );
    assert!(
        !run.entered || !connected.is_empty(),
        "{} disconnected after entry",
        run.username
    );
    if run
        .all_entered
        .is_some_and(|start| start.elapsed() >= MEASUREMENT)
    {
        let completed_unix_ms = read_unix_millis();
        if let Some(combat) = combat {
            combat.assert_complete(&run.username);
        }
        println!("PEER_COMPLETE {} unix_ms={completed_unix_ms}", run.username);
        exit.write(AppExit::Success);
    }
}

fn run_peer(run: Run) -> bool {
    // Replicon's client receive resources are app-global: each peer needs its own App.
    let mut app = App::new();
    app.add_plugins(
        MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
            Duration::from_millis(10),
        )),
    );
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(ClientPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(shared::ProtocolPlugin);
    app.insert_resource(run);
    if let Some(combat) = combat::Combat::read_args() {
        app.insert_resource(combat);
    }
    app.add_observer(connected);
    app.add_observer(combat::connect);
    app.add_systems(Startup, connect);
    app.add_systems(FixedUpdate, login);
    app.add_systems(
        Update,
        (select, entered, combat::receive, combat::fight, finish).chain(),
    );
    app.run().is_success()
}

fn read_private_endpoint() -> SocketAddr {
    let server: SocketAddr = std::env::args()
        .nth(1)
        .expect("private endpoint")
        .parse()
        .expect("socket address");
    assert!(
        server.ip().is_loopback() && server.port() != 5000,
        "private endpoint required"
    );
    server
}

fn spawn_profile_peers(server: SocketAddr) -> Vec<std::thread::JoinHandle<bool>> {
    let count = Arc::new(AtomicUsize::new(0));
    let (sender, receiver) = mpsc::channel();
    let mut workers = Vec::new();
    for index in 1..=PEER_COUNT {
        let run = Run {
            server,
            username: format!("fb_perf_live{index}"),
            started: Instant::now(),
            sent: false,
            entered: false,
            count: count.clone(),
            ready: sender.clone(),
            index,
            all_entered: None,
        };
        workers.push(std::thread::spawn(move || run_peer(run)));
        assert_eq!(
            receiver
                .recv_timeout(Duration::from_secs(60))
                .expect("peer entry"),
            index
        );
        if index < PEER_COUNT {
            std::thread::sleep(Duration::from_secs(10));
        }
    }
    workers
}

fn main() {
    let server = read_private_endpoint();
    let mut logging = App::new();
    logging.add_plugins(bevy::log::LogPlugin::default());
    let workers = spawn_profile_peers(server);
    println!("ALL_ENTERED {PEER_COUNT}");
    for worker in workers {
        assert!(worker.join().expect("peer panicked"), "peer failed");
    }
    println!(
        "MEASUREMENT_COMPLETE peers={PEER_COUNT} seconds={}",
        MEASUREMENT.as_secs()
    );
}
