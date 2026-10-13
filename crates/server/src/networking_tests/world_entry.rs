use super::*;
use lightyear::prelude::client::{ClientPlugins, Connect, NetcodeClient};
use shared::protocol::{AuthChannel, LoginRequest, LoginResponse};
use std::net::UdpSocket;
use std::time::Duration;

fn spawn_network_npc(app: &mut App, template_id: u32, x: f32, dormant: bool) {
    let entity = app
        .world_mut()
        .spawn((
            Position { x, y: 0.0, z: 0.0 },
            crate::maps::InMap::continent(0),
            Npc {
                template_id,
                name: "Fixture wolf".into(),
            },
            NetworkVisibility::default(),
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    if dormant {
        app.world_mut()
            .entity_mut(entity)
            .insert(crate::grid_activity::GridDormant);
    }
}

fn reply_to_test_login(
    mut requests: Query<(
        &mut MessageReceiver<LoginRequest>,
        &mut MessageSender<LoginResponse>,
    )>,
) {
    for (mut receiver, mut sender) in &mut requests {
        for request in receiver.receive() {
            assert_eq!(request.username, "world-entry-fixture");
            sender.send::<AuthChannel>(LoginResponse {
                success: true,
                token: "fixture-response".into(),
                characters: vec![],
                error: None,
            });
        }
    }
}

#[derive(Resource, Default)]
struct LoginReplies(Vec<LoginResponse>);

fn collect_login_replies(
    mut receivers: Query<&mut MessageReceiver<LoginResponse>>,
    mut replies: ResMut<LoginReplies>,
) {
    for mut receiver in &mut receivers {
        replies.0.extend(receiver.receive());
    }
}

#[derive(Resource, Default)]
struct ServerFrameWork(Duration);

fn advance_network(server: &mut App, client: &mut App, frames: usize) {
    for _ in 0..frames {
        let started = std::time::Instant::now();
        server.update();
        let elapsed = started.elapsed();
        let mut work = server.world_mut().resource_mut::<ServerFrameWork>();
        work.0 = work.0.max(elapsed);
        drop(work);
        client.update();
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn received_npcs(client: &mut App) -> Vec<u32> {
    let mut result = client
        .world_mut()
        .query::<&Npc>()
        .iter(client.world())
        .map(|npc| npc.template_id)
        .collect::<Vec<_>>();
    result.sort_unstable();
    result
}

#[test]
fn pre_world_client_exchanges_auth_without_receiving_npcs_then_receives_nearby_npcs() {
    run_world_entry_case(0);
}

#[test]
fn pre_world_large_population_bootstrap_keeps_auth_frames_in_budget() {
    run_world_entry_case(100_000);
}

fn run_world_entry_case(hidden_population: usize) {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let address = socket.local_addr().unwrap();
    drop(socket);

    let mut server = App::new();
    server.add_plugins((
        bevy::MinimalPlugins,
        bevy::state::app::StatesPlugin,
        ServerPlugins {
            tick_duration: Duration::from_millis(50),
        },
        shared::ProtocolPlugin,
        crate::interest::InterestPlugin,
    ));
    server.init_resource::<ServerFrameWork>();
    server.add_observer(on_new_link);
    server.add_observer(on_player_link_inserted);
    server.add_systems(Update, reply_to_test_login);
    let server_entity = server
        .world_mut()
        .spawn((
            LocalAddr(address),
            ServerUdpIo::default(),
            NetcodeServer::new(NetcodeConfig::default()),
        ))
        .id();
    server.world_mut().trigger(Start {
        entity: server_entity,
    });
    spawn_network_npc(&mut server, 1001, 20.0, false);
    spawn_network_npc(&mut server, 1002, 1000.0, false);
    for index in 0..hidden_population {
        spawn_network_npc(&mut server, 100_000 + index as u32, 10_000.0, true);
    }

    let mut client = App::new();
    client.add_plugins((
        bevy::MinimalPlugins,
        bevy::state::app::StatesPlugin,
        ClientPlugins {
            tick_duration: Duration::from_millis(50),
        },
        shared::ProtocolPlugin,
    ));
    client.init_resource::<LoginReplies>();
    client.add_systems(Update, collect_login_replies);
    client.add_observer(|event: On<Add, Connected>, mut commands: Commands| {
        commands.entity(event.entity).insert(ReplicationReceiver);
    });
    let client_entity = client
        .world_mut()
        .spawn((
            LocalAddr("127.0.0.1:0".parse().unwrap()),
            PeerAddr(address),
            UdpIo::default(),
            NetcodeClient::new(
                Authentication::Manual {
                    server_addr: address,
                    client_id: 901,
                    private_key: [0; 32],
                    protocol_id: 0,
                },
                default(),
            )
            .unwrap(),
        ))
        .id();
    client.world_mut().trigger(Connect {
        entity: client_entity,
    });
    for app in [&mut server, &mut client] {
        app.finish();
        app.cleanup();
    }
    server.update();
    server.world_mut().run_schedule(FixedUpdate);
    advance_network(&mut server, &mut client, 250);
    assert!(client.world().get::<Connected>(client_entity).is_some());
    client
        .world_mut()
        .get_mut::<MessageSender<LoginRequest>>(client_entity)
        .unwrap()
        .send::<AuthChannel>(LoginRequest {
            token: None,
            username: "world-entry-fixture".into(),
            password: String::new(),
        });
    advance_network(&mut server, &mut client, 500);
    let replies = &client.world().resource::<LoginReplies>().0;
    assert_eq!(
        replies.len(),
        1,
        "auth replies must not require world replication"
    );
    assert!(replies[0].success);
    assert_eq!(replies[0].token, "fixture-response");
    assert_eq!(
        received_npcs(&mut client),
        Vec::<u32>::new(),
        "existing NPCs must not arrive before world entry"
    );

    spawn_network_npc(&mut server, 1003, 30.0, false);
    advance_network(&mut server, &mut client, 100);
    assert_eq!(
        received_npcs(&mut client),
        Vec::<u32>::new(),
        "new NPCs must not arrive before world entry"
    );

    let connection = server
        .world_mut()
        .query_filtered::<Entity, With<ClientOf>>()
        .single(server.world())
        .unwrap();
    let player = server
        .world_mut()
        .spawn((
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            crate::maps::InMap::continent(0),
        ))
        .id();
    server
        .world_mut()
        .entity_mut(connection)
        .insert(PlayerLink(player));
    advance_network(&mut server, &mut client, 150);
    assert_eq!(
        received_npcs(&mut client),
        vec![1001, 1003],
        "world entry must send nearby NPCs, not the distant NPC"
    );

    server.world_mut().entity_mut(player).insert(Position {
        x: 1000.0,
        y: 0.0,
        z: 0.0,
    });
    server.world().resource::<InterestDirtyQueue>().mark(player);
    advance_network(&mut server, &mut client, 150);
    assert_eq!(
        received_npcs(&mut client),
        vec![1002],
        "moving must replace the client's visible NPC set"
    );
    if hidden_population != 0 {
        let max = server.world().resource::<ServerFrameWork>().0;
        println!("{hidden_population} hidden NPCs: maximum auth/entry frame {max:?}");
        assert!(
            max < Duration::from_millis(50),
            "hidden population blocks connection bootstrap: {max:?}"
        );
    }
}
