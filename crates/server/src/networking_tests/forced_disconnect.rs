//! A kick over real Netcode/UDP: the kicked client reads the reason and is disconnected
//! well inside the netcode timeout, while another client on the same server stays connected.

use super::*;
use lightyear::prelude::client::{ClientPlugins, Connect, NetcodeClient};
use shared::protocol::ForcedDisconnect;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(10);

#[derive(Resource, Default)]
struct ReceivedNotices(Vec<ForcedDisconnect>);

fn collect_notices(
    mut receivers: Query<&mut MessageReceiver<ForcedDisconnect>>,
    mut notices: ResMut<ReceivedNotices>,
) {
    for mut receiver in &mut receivers {
        notices.0.extend(receiver.receive());
    }
}

fn kick_server(address: SocketAddr) -> App {
    let mut server = App::new();
    server.add_plugins((
        bevy::MinimalPlugins,
        bevy::state::app::StatesPlugin,
        ServerPlugins {
            tick_duration: TICK,
        },
        shared::ProtocolPlugin,
    ));
    server.add_systems(Update, auth::deliver_forced_disconnects);
    let entity = server
        .world_mut()
        .spawn((
            LocalAddr(address),
            ServerUdpIo::default(),
            NetcodeServer::new(NetcodeConfig::default()),
        ))
        .id();
    server.world_mut().trigger(Start { entity });
    server.finish();
    server.cleanup();
    server
}

fn connecting_client(address: SocketAddr, client_id: u64) -> (App, Entity) {
    let mut client = App::new();
    client.add_plugins((
        bevy::MinimalPlugins,
        bevy::state::app::StatesPlugin,
        ClientPlugins {
            tick_duration: TICK,
        },
        shared::ProtocolPlugin,
    ));
    client.init_resource::<ReceivedNotices>();
    client.add_systems(Update, collect_notices);
    let entity = client
        .world_mut()
        .spawn((
            LocalAddr("127.0.0.1:0".parse().unwrap()),
            PeerAddr(address),
            UdpIo::default(),
            NetcodeClient::new(
                Authentication::Manual {
                    server_addr: address,
                    client_id,
                    private_key: [0; 32],
                    protocol_id: 0,
                },
                default(),
            )
            .unwrap(),
        ))
        .id();
    client.world_mut().trigger(Connect { entity });
    client.finish();
    client.cleanup();
    (client, entity)
}

fn step(server: &mut App, clients: &mut [(App, Entity)]) {
    server.update();
    for (client, _) in clients.iter_mut() {
        client.update();
    }
    std::thread::sleep(Duration::from_millis(2));
}

fn is_connected(client: &(App, Entity)) -> bool {
    client.0.world().get::<Connected>(client.1).is_some()
}

fn is_disconnected(client: &(App, Entity)) -> bool {
    client.0.world().get::<Disconnected>(client.1).is_some()
}

/// The server's `ClientOf` for the netcode client id.
fn server_link(server: &mut App, client_id: u64) -> Option<Entity> {
    server
        .world_mut()
        .query_filtered::<(Entity, &RemoteId), (With<ClientOf>, With<Connected>)>()
        .iter(server.world())
        .find(|(_, remote)| remote.0 == PeerId::Netcode(client_id))
        .map(|(entity, _)| entity)
}

#[test]
fn kicked_client_is_disconnected_with_reason_and_other_client_stays_connected() {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let address = socket.local_addr().unwrap();
    drop(socket);
    let mut server = kick_server(address);
    let mut clients = [
        connecting_client(address, 901),
        connecting_client(address, 902),
    ];
    let deadline = Instant::now() + Duration::from_secs(5);
    while !clients.iter().all(is_connected) {
        assert!(Instant::now() < deadline, "clients never connected");
        step(&mut server, &mut clients);
    }

    let kicked = server_link(&mut server, 901).expect("server link of client 901");
    let mut commands = server.world_mut().commands();
    auth::queue_forced_disconnect(&mut commands, kicked, "You have been kicked.", false);
    server.world_mut().flush();
    let kicked_at = Instant::now();
    while !is_disconnected(&clients[0]) {
        assert!(
            kicked_at.elapsed() < Duration::from_secs(1),
            "kicked client still connected after 1 s"
        );
        step(&mut server, &mut clients);
    }

    let notices = &clients[0].0.world().resource::<ReceivedNotices>().0;
    assert_eq!(notices.len(), 1);
    assert_eq!(notices[0].message, "You have been kicked.");
    assert!(!notices[0].reconnect_allowed);
    // Past the 3 s netcode timeout, so a silently dropped link would have shown up.
    let settle = Instant::now() + Duration::from_secs(4);
    while Instant::now() < settle {
        step(&mut server, &mut clients);
    }
    assert!(is_connected(&clients[1]), "the other client was dropped");
    assert!(server_link(&mut server, 902).is_some());
    assert_eq!(server_link(&mut server, 901), None);
    assert!(
        clients[1]
            .0
            .world()
            .resource::<ReceivedNotices>()
            .0
            .is_empty()
    );
}
