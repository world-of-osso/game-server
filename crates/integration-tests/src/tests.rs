//! Lightyear integration tests using in-process crossbeam transport.

use crate::stepper::Stepper;
use bevy::prelude::*;
use lightyear::prelude::client::Connected;
use lightyear::prelude::server::Started;
use lightyear::prelude::*;
use lightyear_crossbeam::CrossbeamIo;
use lightyear_replication::test_diagnostics;
use shared::components::{Health, Position, WorldArrival};
use shared::protocol::InputChannel;
use shared::protocol::PlayerInput;
use std::sync::{Mutex, MutexGuard};

static INTEGRATION_TEST_LOCK: Mutex<()> = Mutex::new(());

fn integration_test_guard() -> MutexGuard<'static, ()> {
    INTEGRATION_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

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
    let _guard = integration_test_guard();
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
    let _guard = integration_test_guard();
    let mut stepper = Stepper::single_client();

    let pos = Position {
        x: 100.0,
        y: 200.0,
        z: 300.0,
    };
    stepper
        .server_world_mut()
        .spawn((pos, Replicate::to_clients(NetworkTarget::All)));

    // Tick enough times for replication to propagate
    stepper.tick_step(20);

    let found = find_replicated_position(stepper.client_world_mut(0));
    assert!(
        found.is_some(),
        "Position component was not replicated to client"
    );
    let replicated_pos = found.unwrap();
    assert_eq!(replicated_pos.x, 100.0);
    assert_eq!(replicated_pos.y, 200.0);
    assert_eq!(replicated_pos.z, 300.0);
}

#[test]
fn dirty_position_change_visits_only_the_changed_root() {
    let _guard = integration_test_guard();
    let mut stepper = Stepper::single_client();
    let changed = (0..32)
        .map(|x| {
            stepper
                .server_world_mut()
                .spawn((
                    Position {
                        x: x as f32,
                        y: 0.0,
                        z: 0.0,
                    },
                    Replicate::to_clients(NetworkTarget::All),
                ))
                .id()
        })
        .next()
        .unwrap();

    stepper.tick_step(20);
    test_diagnostics::reset();
    stepper
        .server_world_mut()
        .entity_mut(changed)
        .get_mut::<Position>()
        .unwrap()
        .x = 999.0;
    stepper
        .server_app
        .world()
        .resource::<ReplicationDirtyQueue>()
        .mark(changed);

    stepper.tick_step(1);
    let snapshot = test_diagnostics::snapshot();
    assert_eq!(snapshot.root_candidates, 1);
    assert_eq!(snapshot.replicate_entity_visits, 1);

    stepper.tick_step(5);
    assert!(client_has_position(stepper.client_world_mut(0), 999.0));
}

#[test]
fn unacked_dirty_root_remains_candidate_until_ack() {
    let _guard = integration_test_guard();
    let mut stepper = Stepper::single_client();
    let entity = stepper
        .server_world_mut()
        .spawn((
            Position {
                x: 12.0,
                y: 0.0,
                z: 0.0,
            },
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    stepper.tick_step(20);

    stepper
        .server_world_mut()
        .entity_mut(entity)
        .get_mut::<Position>()
        .unwrap()
        .x = 13.0;
    stepper
        .server_app
        .world()
        .resource::<ReplicationDirtyQueue>()
        .mark(entity);
    test_diagnostics::reset();
    stepper.server_tick_only(3);
    assert_eq!(test_diagnostics::snapshot().root_candidates, 3);

    stepper.tick_step(10);
    test_diagnostics::reset();
    stepper.server_tick_only(3);
    assert_eq!(test_diagnostics::snapshot().root_candidates, 0);
}

#[test]
fn targeted_visibility_change_does_not_visit_other_senders() {
    let _guard = integration_test_guard();
    let mut stepper = Stepper::two_clients();
    let entity = stepper
        .server_world_mut()
        .spawn((
            Position {
                x: 42.0,
                y: 0.0,
                z: 0.0,
            },
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    let senders = stepper.client_of_entities.clone();
    for sender in senders {
        stepper.server_world_mut().lose_visibility(entity, sender);
    }
    stepper.tick_step(5);

    let first_sender = stepper.client_of_entities[0];
    stepper
        .server_world_mut()
        .gain_visibility(entity, first_sender);
    stepper
        .server_app
        .world()
        .resource::<ReplicationDirtyQueue>()
        .mark_for_sender(entity, first_sender);
    test_diagnostics::reset();

    stepper.tick_step(1);
    let snapshot = test_diagnostics::snapshot();
    assert_eq!(snapshot.root_candidates, 1);
    assert_eq!(snapshot.replicate_entity_visits, 1);

    stepper.tick_step(5);
    assert!(client_has_position(stepper.client_world_mut(0), 42.0));
    assert!(!client_has_position(stepper.client_world_mut(1), 42.0));

    stepper
        .server_world_mut()
        .lose_visibility(entity, first_sender);
    stepper
        .server_app
        .world()
        .resource::<ReplicationDirtyQueue>()
        .mark_for_sender(entity, first_sender);
    test_diagnostics::reset();
    stepper.tick_step(1);
    assert_eq!(test_diagnostics::snapshot().root_candidates, 1);
    stepper.tick_step(5);
    assert!(!client_has_position(stepper.client_world_mut(0), 42.0));
}

/// A dead NPC's corpse decays and later respawns on the same server entity: losing
/// visibility (marked dirty for the sender, as `interest` does) removes the client's
/// copy, and regaining it shows the entity again.
#[test]
fn a_hidden_entity_leaves_the_client_and_returns_when_shown_again() {
    let _guard = integration_test_guard();
    let mut stepper = Stepper::single_client();
    let sender = stepper.client_of_entities[0];
    let corpse = stepper
        .server_world_mut()
        .spawn((
            Position {
                x: 7.0,
                y: 0.0,
                z: 0.0,
            },
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    stepper.tick_step(10);
    assert!(client_has_position(stepper.client_world_mut(0), 7.0));

    stepper.server_world_mut().lose_visibility(corpse, sender);
    stepper
        .server_app
        .world()
        .resource::<ReplicationDirtyQueue>()
        .mark_for_sender(corpse, sender);
    stepper.tick_step(10);
    assert!(
        !client_has_position(stepper.client_world_mut(0), 7.0),
        "a corpse hidden at decay leaves the client"
    );

    stepper.server_world_mut().gain_visibility(corpse, sender);
    stepper
        .server_app
        .world()
        .resource::<ReplicationDirtyQueue>()
        .mark_for_sender(corpse, sender);
    stepper.tick_step(10);
    assert!(
        client_has_position(stepper.client_world_mut(0), 7.0),
        "the respawned NPC is shown again"
    );
}

/// A map arrival: the destination's creatures become visible and the player's
/// `WorldArrival` rises in one server frame. The client never sees the new count
/// before the creatures, so it can end its loading screen on the count.
#[test]
fn a_world_arrival_count_reaches_the_client_no_earlier_than_the_objects_shown_with_it() {
    let _guard = integration_test_guard();
    let mut stepper = Stepper::single_client();
    let sender = stepper.client_of_entities[0];
    let player = stepper
        .server_world_mut()
        .spawn((
            Position {
                x: -1.0,
                y: 0.0,
                z: 0.0,
            },
            WorldArrival(0),
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    let creatures: Vec<Entity> = (0..94)
        .map(|index| {
            stepper
                .server_world_mut()
                .spawn((
                    Position {
                        x: 100.0 + index as f32,
                        y: 0.0,
                        z: 0.0,
                    },
                    Replicate::to_clients(NetworkTarget::All),
                ))
                .id()
        })
        .collect();
    for &creature in &creatures {
        stepper.server_world_mut().lose_visibility(creature, sender);
    }
    stepper.tick_step(10);
    assert_eq!(client_arrivals(stepper.client_world_mut(0)), Some(0));

    for &creature in &creatures {
        stepper.server_world_mut().gain_visibility(creature, sender);
        stepper
            .server_app
            .world()
            .resource::<ReplicationDirtyQueue>()
            .mark_for_sender(creature, sender);
    }
    stepper
        .server_world_mut()
        .get_mut::<WorldArrival>(player)
        .unwrap()
        .0 = 1;
    stepper
        .server_app
        .world()
        .resource::<ReplicationDirtyQueue>()
        .mark(player);

    let mut arrived = false;
    for _ in 0..20 {
        stepper.tick_step(1);
        let world = stepper.client_world_mut(0);
        let creatures_seen = (0..94)
            .filter(|&index| client_has_position(world, 100.0 + index as f32))
            .count();
        if client_arrivals(world) == Some(1) {
            assert_eq!(creatures_seen, 94, "arrival count before its creatures");
            arrived = true;
        }
    }
    assert!(arrived, "the arrival count reaches the client");
}

fn client_arrivals(world: &mut World) -> Option<u32> {
    let mut query = world.query_filtered::<&WorldArrival, With<Replicated>>();
    query.iter(world).next().map(|arrival| arrival.0)
}

#[test]
fn replicated_component_insert_and_remove_preserve_actions() {
    let _guard = integration_test_guard();
    let mut stepper = Stepper::single_client();
    let entity = stepper
        .server_world_mut()
        .spawn((
            Position {
                x: 77.0,
                y: 0.0,
                z: 0.0,
            },
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    stepper.tick_step(20);

    test_diagnostics::reset();
    stepper
        .server_world_mut()
        .entity_mut(entity)
        .insert(Health {
            current: 75.0,
            max: 100.0,
        });
    stepper.tick_step(1);
    assert_eq!(test_diagnostics::snapshot().root_candidates, 1);
    stepper.tick_step(5);
    assert_eq!(
        client_health_at_x(stepper.client_world_mut(0), 77.0),
        Some(75.0)
    );

    stepper
        .server_world_mut()
        .entity_mut(entity)
        .remove::<Health>();
    stepper.tick_step(5);
    assert_eq!(client_health_at_x(stepper.client_world_mut(0), 77.0), None);
}

#[test]
fn unchanged_replicated_roots_are_not_revisited_after_initial_sync() {
    let _guard = integration_test_guard();
    let mut stepper = Stepper::single_client();

    for x in 0..32 {
        stepper.server_world_mut().spawn((
            Position {
                x: x as f32,
                y: 0.0,
                z: 0.0,
            },
            Replicate::to_clients(NetworkTarget::All),
        ));
    }

    stepper.tick_step(20);
    test_diagnostics::reset();
    stepper.tick_step(5);

    let snapshot = test_diagnostics::snapshot();
    assert_eq!(
        snapshot.root_candidates, 0,
        "unchanged roots must not enter the sender candidate traversal"
    );
    assert_eq!(
        snapshot.replicate_entity_visits, 0,
        "unchanged roots must not reach replicate_entity"
    );
}

/// Send a PlayerInput message from client to server and verify it is received.
#[test]
fn client_message_received_by_server() {
    let _guard = integration_test_guard();
    let mut stepper = Stepper::single_client();

    let input = PlayerInput {
        direction: [1.0, 0.0, 0.0],
        facing_yaw: 0.0,
        jumping: false,
        running: false,
        swimming: false,
        flying: false,
        position: [0.0, 0.0, 0.0],
        transport: None,
        epoch: 0,
    };
    // Register a system to collect messages before they get cleared in Last schedule
    stepper.server_app.init_resource::<ReceivedInputs>();
    stepper
        .server_app
        .add_systems(Update, collect_player_inputs);

    send_player_input(&mut stepper, 0, input);
    stepper.tick_step(30);

    let received = stepper.server_app.world().resource::<ReceivedInputs>();
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
    let _guard = integration_test_guard();
    let mut stepper = Stepper::two_clients();
    let (player_a, player_b) = setup_two_player_entities(&mut stepper);
    stepper
        .server_app
        .add_systems(Update, process_player_inputs);
    send_player_input(&mut stepper, 0, input_direction([1.0, 0.0, 0.0]));
    send_player_input(&mut stepper, 1, input_direction([0.0, 1.0, 0.0]));
    stepper.tick_step(30);
    assert_player_moved_in_x(&stepper, player_a);
    assert_player_moved_in_y(&stepper, player_b);
}

fn setup_two_player_entities(stepper: &mut Stepper) -> (Entity, Entity) {
    let player_a = stepper
        .server_world_mut()
        .spawn(Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        })
        .id();
    let player_b = stepper
        .server_world_mut()
        .spawn(Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        })
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
    (player_a, player_b)
}

fn input_direction(direction: [f32; 3]) -> PlayerInput {
    PlayerInput {
        direction,
        facing_yaw: 0.0,
        jumping: false,
        running: false,
        swimming: false,
        flying: false,
        position: [0.0, 0.0, 0.0],
        transport: None,
        epoch: 0,
    }
}

fn assert_player_moved_in_x(stepper: &Stepper, player: Entity) {
    let pos = get_server_position(stepper, player);
    assert!(
        pos.x > 0.0,
        "player should have moved in +X, got x={}",
        pos.x
    );
    assert!(
        pos.y.abs() < f32::EPSILON,
        "player should not have moved in Y, got y={}",
        pos.y
    );
}

fn assert_player_moved_in_y(stepper: &Stepper, player: Entity) {
    let pos = get_server_position(stepper, player);
    assert!(
        pos.y > 0.0,
        "player should have moved in +Y, got y={}",
        pos.y
    );
    assert!(
        pos.x.abs() < f32::EPSILON,
        "player should not have moved in X, got x={}",
        pos.x
    );
}

fn get_server_position(stepper: &Stepper, entity: Entity) -> Position {
    *stepper
        .server_app
        .world()
        .entity(entity)
        .get::<Position>()
        .expect("entity should have Position")
}

/// Query the client world for a Position component on a Replicated entity.
fn find_replicated_position(world: &mut bevy::prelude::World) -> Option<Position> {
    let mut query = world.query_filtered::<&Position, bevy::prelude::With<Replicated>>();
    query.iter(world).next().copied()
}

fn client_has_position(world: &mut World, x: f32) -> bool {
    let mut query = world.query_filtered::<&Position, With<Replicated>>();
    query.iter(world).any(|position| position.x == x)
}

fn client_health_at_x(world: &mut World, x: f32) -> Option<f32> {
    let mut query = world.query_filtered::<(&Position, Option<&Health>), With<Replicated>>();
    query
        .iter(world)
        .find(|(position, _)| position.x == x)
        .and_then(|(_, health)| health.map(|health| health.current))
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
