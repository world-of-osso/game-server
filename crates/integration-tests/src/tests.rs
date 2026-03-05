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

