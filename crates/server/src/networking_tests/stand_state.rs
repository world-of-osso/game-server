//! Player stand state over real UDP: one client's `CMSG_STAND_STATE_CHANGE` changes its
//! player's replicated `PlayerStandState`, which another client sees.

use super::sheet_stats::{advance, network_client, network_server};
use super::*;
use crate::auth::{Authenticated, InWorld};
use shared::components::{Player, PlayerStandState, StandState};
use shared::protocol::{CombatChannel, StandStateIntent};
use std::net::UdpSocket;

/// A human warrior in the world, standing.
fn spawn_player(server: &mut App, name: &str, x: f32) -> Entity {
    server
        .world_mut()
        .spawn((
            Position { x, y: 0.0, z: 0.0 },
            crate::maps::InMap::continent(0),
            Player {
                name: name.into(),
                race: 1,
                class: 1,
                appearance: default(),
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            PlayerStandState::default(),
            NetworkVisibility::default(),
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id()
}

/// The named player's stand state as one client sees it.
fn seen_stand_state(client: &mut App, name: &str) -> Option<StandState> {
    client
        .world_mut()
        .query::<(&Player, &PlayerStandState)>()
        .iter(client.world())
        .find(|(player, _)| player.name == name)
        .map(|(_, state)| state.0)
}

fn request(client: &mut App, connection: Entity, state: StandState) {
    client
        .world_mut()
        .get_mut::<MessageSender<StandStateIntent>>(connection)
        .unwrap()
        .send::<CombatChannel>(StandStateIntent { state });
}

#[test]
fn another_client_sees_a_player_sit_kneel_and_stand() {
    let world_db = crate::test_data::world_db_path();
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let address = socket.local_addr().unwrap();
    drop(socket);
    let mut server = network_server(address, &world_db);
    server.add_plugins(crate::stand_state::StandStatePlugin);
    let (alice_client, alice_entity) = network_client(address, 921);
    let (bob_client, bob_entity) = network_client(address, 922);
    let mut clients = [alice_client, bob_client];
    for app in std::iter::once(&mut server).chain(clients.iter_mut()) {
        app.finish();
        app.cleanup();
    }
    advance(&mut server, &mut clients, 250);
    assert!(clients[0].world().get::<Connected>(alice_entity).is_some());
    assert!(clients[1].world().get::<Connected>(bob_entity).is_some());

    let alice = spawn_player(&mut server, "Alice", 0.0);
    let bob = spawn_player(&mut server, "Bob", 5.0);
    let connections: Vec<(Entity, u64)> = server
        .world_mut()
        .query::<(Entity, &RemoteId)>()
        .iter(server.world())
        .map(|(entity, remote)| (entity, remote.0.to_bits()))
        .collect();
    for (connection, id) in connections {
        let (player, character_id) = if id == 921 { (alice, 1) } else { (bob, 2) };
        server.world_mut().entity_mut(connection).insert((
            PlayerLink(player),
            Authenticated { account_id: id },
            InWorld { character_id },
        ));
    }
    advance(&mut server, &mut clients, 150);
    assert_eq!(
        seen_stand_state(&mut clients[1], "Alice"),
        Some(StandState::Stand)
    );

    for state in [StandState::Sit, StandState::Kneel, StandState::Stand] {
        request(&mut clients[0], alice_entity, state);
        advance(&mut server, &mut clients, 100);
        assert_eq!(
            server.world().get::<PlayerStandState>(alice),
            Some(&PlayerStandState(state))
        );
        assert_eq!(
            seen_stand_state(&mut clients[1], "Alice"),
            Some(state),
            "Bob sees Alice {state:?}"
        );
        assert_eq!(
            seen_stand_state(&mut clients[1], "Bob"),
            Some(StandState::Stand)
        );
    }
}
