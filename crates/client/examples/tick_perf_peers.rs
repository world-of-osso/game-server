//! Real UDP peers for bounded private-server profiling; no token files or rendering.
use bevy::app::AppExit;
use bevy::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use shared::protocol::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const PEER_COUNT: usize = 11;
const MEASUREMENT: Duration = Duration::from_secs(90);

#[derive(Resource)]
struct Run {
    server: SocketAddr,
    started: Instant,
    next_peer: usize,
    last_entry: Option<Instant>,
    all_entered: Option<Instant>,
}

#[derive(Component)]
struct Peer {
    username: String,
    sent: bool,
    entered: bool,
}

fn connect_next(mut commands: Commands, mut run: ResMut<Run>, peers: Query<&Peer>) {
    if run.next_peer > PEER_COUNT || peers.iter().any(|peer| !peer.entered) {
        return;
    }
    if run
        .last_entry
        .is_some_and(|entry| entry.elapsed() < Duration::from_secs(10))
    {
        return;
    }
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
            Peer {
                username: format!("fb_perf_live{}", run.next_peer),
                sent: false,
                entered: false,
            },
        ))
        .id();
    run.next_peer += 1;
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

fn login(mut peers: Query<(&mut Peer, &mut MessageSender<LoginRequest>), With<Connected>>) {
    for (mut peer, mut sender) in &mut peers {
        if peer.sent {
            continue;
        }
        sender.send::<AuthChannel>(LoginRequest {
            token: None,
            username: peer.username.clone(),
            password: "fbtest".into(),
        });
        peer.sent = true;
        println!("LOGIN_SENT {}", peer.username);
    }
}

fn select(
    mut peers: Query<(
        &Peer,
        &mut MessageReceiver<LoginResponse>,
        &mut MessageSender<SelectCharacter>,
    )>,
) {
    for (peer, mut receiver, mut sender) in &mut peers {
        for reply in receiver.receive() {
            assert!(
                reply.success,
                "{} login rejected: {:?}",
                peer.username, reply.error
            );
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
    mut peers: Query<(&mut Peer, &mut MessageReceiver<EnterWorldResponse>)>,
) {
    for (mut peer, mut receiver) in &mut peers {
        for reply in receiver.receive() {
            assert!(
                reply.success,
                "{} admission rejected: {:?}",
                peer.username, reply.error
            );
            peer.entered = true;
            run.last_entry = Some(Instant::now());
            println!(
                "ENTERED {} elapsed={:.3}",
                peer.username,
                run.started.elapsed().as_secs_f64()
            );
        }
    }
    let entered = peers.iter().filter(|(peer, _)| peer.entered).count();
    if entered == PEER_COUNT && run.all_entered.is_none() {
        run.all_entered = Some(Instant::now());
        println!("ALL_ENTERED {entered}");
    }
}

fn finish(run: Res<Run>, peers: Query<(&Peer, Has<Connected>)>, mut exit: MessageWriter<AppExit>) {
    assert!(
        run.started.elapsed() < Duration::from_secs(600),
        "peer readiness deadline exceeded"
    );
    for (peer, connected) in &peers {
        assert!(
            !peer.entered || connected,
            "{} disconnected after entry",
            peer.username
        );
    }
    if run
        .all_entered
        .is_some_and(|start| start.elapsed() >= MEASUREMENT)
    {
        println!(
            "MEASUREMENT_COMPLETE peers={PEER_COUNT} seconds={}",
            MEASUREMENT.as_secs()
        );
        exit.write(AppExit::Success);
    }
}

fn main() {
    let server: SocketAddr = std::env::args()
        .nth(1)
        .expect("private endpoint")
        .parse()
        .expect("socket address");
    assert!(
        server.ip().is_loopback() && server.port() != 5000,
        "private endpoint required"
    );
    let mut app = App::new();
    app.add_plugins(
        MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
            Duration::from_millis(10),
        )),
    );
    app.add_plugins((
        bevy::state::app::StatesPlugin,
        bevy::log::LogPlugin::default(),
    ));
    app.add_plugins(ClientPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(shared::ProtocolPlugin);
    app.insert_resource(Run {
        server,
        started: Instant::now(),
        next_peer: 1,
        last_entry: None,
        all_entered: None,
    });
    app.add_observer(connected);
    app.add_systems(FixedUpdate, login);
    app.add_systems(Update, (select, entered, connect_next, finish).chain());
    if !app.run().is_success() {
        std::process::exit(1);
    }
}
