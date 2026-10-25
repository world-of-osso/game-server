//! Player specialization over real UDP: another client sees a player's replicated
//! `ActiveSpec` (TrinityCore `UF::PlayerData::CurrentSpecID`), and its update after
//! the player's `SetSpecialization`.

use super::sheet_stats::{advance, network_client, network_server};
use super::*;
use crate::auth::{Authenticated, InWorld};
use crate::class_progression::ClassProgressionPlugin;
use crate::persistence::{ClientId, GameDatabase, temp_database};
use shared::components::{ActiveSpec, Level, Player};
use shared::protocol::{SetSpecialization, TalentChannel};
use std::net::UdpSocket;

/// ChrSpecialization 71 (Arms, OrderIndex 0) and 72 (Fury, OrderIndex 1).
const SPEC_ARMS: u32 = 71;
const SPEC_FURY: u32 = 72;

/// A level 10 human warrior in the world: past the spec unlock level.
fn spawn_warrior(server: &mut App, name: &str, character_id: u64, x: f32) -> Entity {
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
            Level(10),
            ClientId(character_id),
            Health {
                current: 100.0,
                max: 100.0,
            },
            NetworkVisibility::default(),
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id()
}

/// The named player's spec as one client sees it.
fn seen_spec(client: &mut App, name: &str) -> Option<u32> {
    client
        .world_mut()
        .query::<(&Player, &ActiveSpec)>()
        .iter(client.world())
        .find(|(player, _)| player.name == name)
        .map(|(_, spec)| spec.0)
}

#[test]
fn another_client_sees_a_players_spec_and_its_change() {
    let world_db = crate::test_data::world_db_path();
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let address = socket.local_addr().unwrap();
    drop(socket);
    let mut server = network_server(address, &world_db);
    let conn = rusqlite::Connection::open_with_flags(
        &world_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    server.insert_resource(crate::spell_info::load_class_spells(&conn).unwrap());
    server.insert_resource(GameDatabase::new(temp_database()));
    server.add_plugins(ClassProgressionPlugin);
    let (alice_client, alice_entity) = network_client(address, 931);
    let (bob_client, bob_entity) = network_client(address, 932);
    let mut clients = [alice_client, bob_client];
    for app in std::iter::once(&mut server).chain(clients.iter_mut()) {
        app.finish();
        app.cleanup();
    }
    advance(&mut server, &mut clients, 250);
    assert!(clients[0].world().get::<Connected>(alice_entity).is_some());
    assert!(clients[1].world().get::<Connected>(bob_entity).is_some());

    let alice = spawn_warrior(&mut server, "Alice", 1, 0.0);
    let bob = spawn_warrior(&mut server, "Bob", 2, 5.0);
    let connections: Vec<(Entity, u64)> = server
        .world_mut()
        .query::<(Entity, &RemoteId)>()
        .iter(server.world())
        .map(|(entity, remote)| (entity, remote.0.to_bits()))
        .collect();
    for (connection, id) in connections {
        let (player, character_id) = if id == 931 { (alice, 1) } else { (bob, 2) };
        server.world_mut().entity_mut(connection).insert((
            PlayerLink(player),
            Authenticated { account_id: id },
            InWorld { character_id },
        ));
    }
    advance(&mut server, &mut clients, 150);
    // No stored spec: the first non-Initial spec by OrderIndex.
    assert_eq!(seen_spec(&mut clients[1], "Alice"), Some(SPEC_ARMS));

    clients[0]
        .world_mut()
        .get_mut::<MessageSender<SetSpecialization>>(alice_entity)
        .unwrap()
        .send::<TalentChannel>(SetSpecialization { spec_id: SPEC_FURY });
    advance(&mut server, &mut clients, 100);
    assert_eq!(
        server.world().get::<ActiveSpec>(alice),
        Some(&ActiveSpec(SPEC_FURY))
    );
    assert_eq!(seen_spec(&mut clients[1], "Alice"), Some(SPEC_FURY));
    assert_eq!(seen_spec(&mut clients[0], "Bob"), Some(SPEC_ARMS));
    server
        .world_mut()
        .entity_mut(alice)
        .insert(crate::pvp::PlayerPvpState {
            queue: Some(crate::pvp::PlayerPvpQueue::Arena {
                mode: shared::protocol::ArenaQueueMode::SoloShuffle,
            }),
            ..Default::default()
        });
    clients[0]
        .world_mut()
        .get_mut::<MessageSender<SetSpecialization>>(alice_entity)
        .unwrap()
        .send::<TalentChannel>(SetSpecialization { spec_id: SPEC_ARMS });
    advance(&mut server, &mut clients, 100);
    assert_eq!(
        seen_spec(&mut clients[1], "Alice"),
        Some(SPEC_FURY),
        "Shuffle roles cannot change while queued/in the match"
    );
}
