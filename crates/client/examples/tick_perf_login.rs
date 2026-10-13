//! Real private-UDP login/admission peer; fixture deadline never changes server policy.
use bevy::app::AppExit;
use bevy::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use shared::protocol::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Resource)]
struct Run {
    server: SocketAddr,
    username: String,
    start: Instant,
    entered: bool,
    sent: bool,
    snapshots: usize,
}

fn connect(mut commands: Commands, run: Res<Run>) {
    let client_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
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
        MessageReceiver::<AchievementStateUpdate>::default(),
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
    mut replies: Query<(Entity, &mut MessageReceiver<LoginResponse>)>,
    mut senders: Query<&mut MessageSender<SelectCharacter>>,
) {
    for (entity, mut receiver) in &mut replies {
        for reply in receiver.receive() {
            assert!(reply.success, "login rejected: {:?}", reply.error);
            let character = reply
                .characters
                .first()
                .expect("prepared private character");
            senders
                .get_mut(entity)
                .expect("character selector")
                .send::<AuthChannel>(SelectCharacter {
                    character_id: character.character_id,
                });
            println!("SELECT_SENT {}", character.name);
        }
    }
}

fn entered(mut run: ResMut<Run>, mut replies: Query<&mut MessageReceiver<EnterWorldResponse>>) {
    for mut receiver in &mut replies {
        for reply in receiver.receive() {
            assert!(reply.success, "admission rejected: {:?}", reply.error);
            run.entered = true;
            println!(
                "ENTERED {} after {:.1} ms",
                run.username,
                run.start.elapsed().as_secs_f64() * 1000.0
            );
        }
    }
}

fn snapshots(
    mut run: ResMut<Run>,
    mut updates: Query<&mut MessageReceiver<AchievementStateUpdate>>,
) {
    for mut receiver in &mut updates {
        run.snapshots += receiver.receive().count();
    }
}

fn finish(run: Res<Run>, mut exit: MessageWriter<AppExit>) {
    if run.start.elapsed() < Duration::from_secs(60) {
        return;
    }
    println!(
        "SUMMARY entered={} achievement_updates={}",
        run.entered, run.snapshots
    );
    if run.entered {
        exit.write(AppExit::Success);
    } else {
        exit.write(AppExit::Error(1.try_into().unwrap()));
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let server: SocketAddr = args
        .next()
        .expect("private endpoint")
        .parse()
        .expect("socket address");
    let username = args.next().expect("fb_perf account");
    assert!(
        server.ip().is_loopback() && server.port() != 5000,
        "private endpoint required"
    );
    assert!(
        username.starts_with("fb_perf"),
        "private account prefix required"
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
        username,
        start: Instant::now(),
        entered: false,
        sent: false,
        snapshots: 0,
    });
    app.add_observer(connected);
    app.add_systems(Startup, connect);
    app.add_systems(FixedUpdate, login);
    app.add_systems(Update, (select, entered, snapshots, finish));
    let exit = app.run();
    if !exit.is_success() {
        std::process::exit(1);
    }
}
