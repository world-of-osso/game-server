//! Lightyear integration tests using in-process crossbeam transport.

use crate::stepper::Stepper;
use bevy::prelude::*;
use lightyear::prelude::client::Connected;
use lightyear::prelude::server::Started;
use lightyear::prelude::*;
use lightyear_crossbeam::CrossbeamIo;
use shared::components::Position;
use shared::protocol::InputChannel;
use shared::protocol::PlayerInput;

/// Resource to accumulate received PlayerInput messages across ticks.
#[derive(Resource, Default)]
struct ReceivedInputs(Vec<PlayerInput>);

/// System that drains PlayerInput from all MessageReceivers and accumulates them.
fn collect_player_inputs(
    mut receivers: Query<&mut MessageReceiver<PlayerInput>>,
    mut received: ResMut<ReceivedInputs>,
) {
    for mut receiver in receivers.iter_mut() {
        received.0.extend(receiver.receive());
    }
}

/// Verify that a client can connect to a server via crossbeam transport.
/// After connection, both sides should have the expected marker components.
#[test]
fn client_connects_to_server() {
    let stepper = Stepper::single_client();

    // Client side: Connected + CrossbeamIo
    assert!(stepper.client(0).contains::<Connected>());
    assert!(stepper.client(0).contains::<CrossbeamIo>());

    // Server: Started
    assert!(stepper.server().contains::<Started>());

    // Server-side client_of entity: Connected + CrossbeamIo
    assert!(stepper.client_of(0).contains::<Connected>());
    assert!(stepper.client_of(0).contains::<CrossbeamIo>());
}

/// Spawn a replicated entity with Position on the server and verify
/// it appears on the client after enough ticks.
#[test]
fn server_replicates_position_to_client() {
    let mut stepper = Stepper::single_client();

    let pos = Position {
        x: 100.0,
        y: 200.0,
        z: 300.0,
    };
    stepper
        .server_world_mut()
        .spawn((pos, Replicate::default()));

    // Tick enough times for replication to propagate
    stepper.tick_step(20);

    let found = find_replicated_position(stepper.client_world_mut(0));
    assert!(found.is_some(), "Position component was not replicated to client");
    let replicated_pos = found.unwrap();
    assert_eq!(replicated_pos.x, 100.0);
    assert_eq!(replicated_pos.y, 200.0);
    assert_eq!(replicated_pos.z, 300.0);
}

/// Send a PlayerInput message from client to server and verify it is received.
#[test]
fn client_message_received_by_server() {
    let mut stepper = Stepper::single_client();

    let input = PlayerInput {
        direction: [1.0, 0.0, 0.0],
        facing_yaw: 0.0,
        jumping: false,
        running: false,
    };
    // Register a system to collect messages before they get cleared in Last schedule
    stepper.server_app.init_resource::<ReceivedInputs>();
    stepper
        .server_app
        .add_systems(Update, collect_player_inputs);

    send_player_input(&mut stepper, 0, input);
    stepper.tick_step(30);

    let received = stepper
        .server_app
        .world()
        .resource::<ReceivedInputs>();
    assert!(
        !received.0.is_empty(),
        "Server did not receive PlayerInput message from client"
    );
    assert_eq!(received.0[0].direction, [1.0, 0.0, 0.0]);
}

/// Links a connection entity to its spawned player entity (mirrors server::PlayerLink).
#[derive(Component, Debug)]
struct PlayerLink(Entity);

/// System that reads PlayerInput from each connection's MessageReceiver,
/// looks up the PlayerLink, and applies movement to the linked Position.
/// Mirrors server::process_player_inputs.
fn process_player_inputs(
    mut receivers: Query<(&mut MessageReceiver<PlayerInput>, &PlayerLink)>,
    mut players: Query<&mut Position>,
) {
    for (mut receiver, link) in receivers.iter_mut() {
        for input in receiver.receive() {
            let dir_len = (input.direction[0].powi(2)
                + input.direction[1].powi(2)
                + input.direction[2].powi(2))
            .sqrt();
            if dir_len < 0.001 {
                continue;
            }
            // Apply a fixed step (no Time dependency needed for the assertion)
            let scale = 1.0 / dir_len;
            if let Ok(mut pos) = players.get_mut(link.0) {
                pos.x += input.direction[0] * scale;
                pos.y += input.direction[1] * scale;
                pos.z += input.direction[2] * scale;
            }
        }
    }
}

/// Connect two clients, send different PlayerInput from each, and verify
/// that each client's player entity moves independently.
#[test]
fn multi_client_input_routes_independently() {
    let mut stepper = Stepper::two_clients();

    // Spawn a player entity for each client on the server,
    // and link them via PlayerLink on the client_of entity.
    let player_a = stepper
        .server_world_mut()
        .spawn(Position { x: 0.0, y: 0.0, z: 0.0 })
        .id();
    let player_b = stepper
        .server_world_mut()
        .spawn(Position { x: 0.0, y: 0.0, z: 0.0 })
        .id();

    let client_of_0 = stepper.client_of_entities[0];
    let client_of_1 = stepper.client_of_entities[1];
    stepper
        .server_world_mut()
        .entity_mut(client_of_0)
        .insert(PlayerLink(player_a));
    stepper
        .server_world_mut()
        .entity_mut(client_of_1)
        .insert(PlayerLink(player_b));

    // Register the input processing system on the server
    stepper
        .server_app
        .add_systems(Update, process_player_inputs);

    // Client 0 sends input moving in +X, client 1 sends input moving in +Y
    send_player_input(
        &mut stepper,
        0,
        PlayerInput {
            direction: [1.0, 0.0, 0.0],
            facing_yaw: 0.0,
            jumping: false,
            running: false,
        },
    );
    send_player_input(
        &mut stepper,
        1,
        PlayerInput {
            direction: [0.0, 1.0, 0.0],
            facing_yaw: 0.0,
            jumping: false,
            running: false,
        },
    );

    // Tick enough for messages to arrive and be processed
    stepper.tick_step(30);

    let pos_a = *stepper
        .server_app
        .world()
        .entity(player_a)
        .get::<Position>()
        .expect("player_a should have Position");
    let pos_b = *stepper
        .server_app
        .world()
        .entity(player_b)
        .get::<Position>()
        .expect("player_b should have Position");

    // Player A moved in +X only
    assert!(
        pos_a.x > 0.0,
        "player_a should have moved in +X, got x={}",
        pos_a.x
    );
    assert!(
        pos_a.y.abs() < f32::EPSILON,
        "player_a should not have moved in Y, got y={}",
        pos_a.y
    );

    // Player B moved in +Y only
    assert!(
        pos_b.y > 0.0,
        "player_b should have moved in +Y, got y={}",
        pos_b.y
    );
    assert!(
        pos_b.x.abs() < f32::EPSILON,
        "player_b should not have moved in X, got x={}",
        pos_b.x
    );
}

/// Query the client world for a Position component on a Replicated entity.
fn find_replicated_position(world: &mut bevy::prelude::World) -> Option<Position> {
    let mut query = world.query_filtered::<&Position, bevy::prelude::With<Replicated>>();
    query.iter(world).next().copied()
}

/// Send a PlayerInput message from a client via its MessageSender.
fn send_player_input(stepper: &mut Stepper, client_id: usize, input: PlayerInput) {
    let entity = stepper.client_entities[client_id];
    let world = stepper.client_world_mut(client_id);
    let mut entity_mut = world.entity_mut(entity);
    let mut sender = entity_mut
        .get_mut::<MessageSender<PlayerInput>>()
        .expect("Client should have MessageSender<PlayerInput>");
    sender.send::<InputChannel>(input);
}

