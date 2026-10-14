//! An idle owner must receive admin travel without an unrelated player mutation.
use super::*;
use bevy::ecs::system::SystemState;
use shared::components::MovementControl;
use std::net::UdpSocket;

fn seen_position(client: &mut App) -> Option<(Position, u32)> {
    client
        .world_mut()
        .query::<(&Player, &Position, &MovementControl)>()
        .iter(client.world())
        .find(|(player, ..)| player.name == "Bgtravel")
        .map(|(_, position, control)| (*position, control.epoch))
}

#[test]
fn idle_client_receives_admin_position_and_teleport_epoch_over_udp() {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let address = socket.local_addr().unwrap();
    drop(socket);
    let mut server = super::regen::network_server(address);
    let (mut client, connection) = super::regen::network_client(address, 924);
    for app in [&mut server, &mut client] {
        app.finish();
        app.cleanup();
    }
    super::regen::advance(&mut server, &mut client, 250);
    assert!(client.world().get::<Connected>(connection).is_some());
    let db = crate::persistence::temp_database();
    let (character, _) = crate::persistence::create_character(&db, 1, "Bgtravel", 1, 1).unwrap();
    let db = crate::persistence::GameDatabase::new(db);
    let origin = Position {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let player = server
        .world_mut()
        .spawn((
            origin,
            Player {
                name: "Bgtravel".into(),
                race: 1,
                class: 1,
                appearance: default(),
            },
            crate::persistence::ClientId(character),
            MovementControl::default(),
            crate::maps::InMap::continent(0),
            NetworkVisibility::default(),
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    let owner = server
        .world_mut()
        .query_filtered::<Entity, With<RemoteId>>()
        .single(server.world())
        .unwrap();
    server
        .world_mut()
        .entity_mut(owner)
        .insert(PlayerLink(player));
    super::regen::advance(&mut server, &mut client, 350);
    assert_eq!(seen_position(&mut client), Some((origin, 0)));
    // Let initial replication settle; no movement/gravity/regen systems mutate this player.
    super::regen::advance(&mut server, &mut client, 100);
    let destination = (1441.06, 2700.15, 9.50497);
    let mut state = SystemState::<(
        Query<(
            Entity,
            &mut Position,
            &crate::persistence::ClientId,
            &mut MovementControl,
        )>,
        Res<lightyear::prelude::ReplicationDirtyQueue>,
    )>::new(server.world_mut());
    {
        let (mut players, dirty) = state.get_mut(server.world_mut()).unwrap();
        crate::admin_handlers::handle_set_position(
            &db,
            &mut players,
            None,
            Some(&dirty),
            "Bgtravel",
            destination.0,
            destination.1,
            destination.2,
        );
    }
    state.apply(server.world_mut());
    let (x, y, z) = crate::terrain_height::wow_to_bevy(destination.0, destination.1, destination.2);
    let expected = Position { x, y, z };
    assert_eq!(*server.world().get::<Position>(player).unwrap(), expected);
    super::regen::advance(&mut server, &mut client, 600);
    assert_eq!(
        seen_position(&mut client),
        Some((expected, 1)),
        "idle native owner must adopt source travel without a later unrelated dirty mark"
    );
}
