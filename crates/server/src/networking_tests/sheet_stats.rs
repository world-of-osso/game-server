//! Character sheet stats over real UDP: a player receives its own `UnitStats`,
//! `CombatRatings` and `DerivedStats`, never another player's, and equipping gear
//! updates them.

use super::*;
use crate::item_stats::load_item_scaling_tables;
use crate::sheet_visibility::SheetVisibilityPlugin;
use crate::stats::{EquippedItems, StatsPlugin};
use lightyear::prelude::client::{ClientPlugins, Connect, NetcodeClient};
use shared::components::{CombatRatings, DerivedStats, Level, Player, UnitStats};
use shared::protocol::EquipmentSlot;
use std::net::UdpSocket;
use std::time::Duration;

/// Notched Shortsword (ilvl 5): +1 Agility, +1 Stamina.
const NOTCHED_SHORTSWORD: u32 = 727;

pub(super) fn network_server(address: std::net::SocketAddr, world_db: &std::path::Path) -> App {
    let mut server = App::new();
    server.add_plugins((
        bevy::MinimalPlugins,
        bevy::state::app::StatesPlugin,
        ServerPlugins {
            tick_duration: Duration::from_millis(50),
        },
        shared::ProtocolPlugin,
        crate::interest::InterestPlugin,
        SheetVisibilityPlugin,
        StatsPlugin,
    ));
    server.insert_resource(crate::item_templates::load_item_templates(world_db).unwrap());
    server.insert_resource(load_item_scaling_tables(world_db).unwrap());
    server.add_observer(on_new_link);
    server.add_observer(on_player_link_inserted);
    let entity = server
        .world_mut()
        .spawn((
            LocalAddr(address),
            ServerUdpIo::default(),
            NetcodeServer::new(NetcodeConfig::default()),
        ))
        .id();
    server.world_mut().trigger(Start { entity });
    server
}

pub(super) fn network_client(address: std::net::SocketAddr, client_id: u64) -> (App, Entity) {
    let mut client = App::new();
    client.add_plugins((
        bevy::MinimalPlugins,
        bevy::state::app::StatesPlugin,
        ClientPlugins {
            tick_duration: Duration::from_millis(50),
        },
        shared::ProtocolPlugin,
    ));
    client.add_observer(|event: On<Add, Connected>, mut commands: Commands| {
        commands.entity(event.entity).insert(ReplicationReceiver);
    });
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
    (client, entity)
}

pub(super) fn advance(server: &mut App, clients: &mut [App], frames: usize) {
    for _ in 0..frames {
        server.update();
        for client in clients.iter_mut() {
            client.update();
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// A level 5 human warrior in the world, without gear.
fn spawn_warrior(server: &mut App, name: &str, x: f32) -> Entity {
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
            Level(5),
            UnitStats::default(),
            CombatRatings::default(),
            DerivedStats::default(),
            Health {
                current: 1.0,
                max: 1.0,
            },
            EquippedItems::default(),
            NetworkVisibility::default(),
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id()
}

/// The named player as one client sees it: present, and its stats if replicated.
fn seen_player(client: &mut App, name: &str) -> Option<Option<UnitStats>> {
    client
        .world_mut()
        .query::<(&Player, Option<&UnitStats>)>()
        .iter(client.world())
        .find(|(player, _)| player.name == name)
        .map(|(_, stats)| stats.copied())
}

/// The named player's `DerivedStats` as one client sees them, if replicated.
fn seen_derived(client: &mut App, name: &str) -> Option<Option<DerivedStats>> {
    client
        .world_mut()
        .query::<(&Player, Option<&DerivedStats>)>()
        .iter(client.world())
        .find(|(player, _)| player.name == name)
        .map(|(_, derived)| derived.copied())
}

#[test]
fn each_player_receives_only_its_own_sheet_stats_and_equipping_updates_them() {
    let world_db = crate::test_data::world_db_path();
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let address = socket.local_addr().unwrap();
    drop(socket);
    let mut server = network_server(address, &world_db);
    let (alice_client, alice_entity) = network_client(address, 911);
    let (bob_client, bob_entity) = network_client(address, 912);
    let mut clients = [alice_client, bob_client];
    for app in std::iter::once(&mut server).chain(clients.iter_mut()) {
        app.finish();
        app.cleanup();
    }
    advance(&mut server, &mut clients, 250);
    assert!(clients[0].world().get::<Connected>(alice_entity).is_some());
    assert!(clients[1].world().get::<Connected>(bob_entity).is_some());

    let alice = spawn_warrior(&mut server, "Alice", 0.0);
    let bob = spawn_warrior(&mut server, "Bob", 5.0);
    let connections: Vec<(Entity, u64)> = server
        .world_mut()
        .query::<(Entity, &RemoteId)>()
        .iter(server.world())
        .map(|(entity, remote)| (entity, remote.0.to_bits()))
        .collect();
    for (connection, id) in connections {
        let player = if id == 911 { alice } else { bob };
        server
            .world_mut()
            .entity_mut(connection)
            .insert(PlayerLink(player));
    }
    advance(&mut server, &mut clients, 150);

    let alice_sheet = seen_player(&mut clients[0], "Alice")
        .expect("Alice sees herself")
        .expect("Alice receives her own sheet stats");
    let server_sheet = *server.world().get::<UnitStats>(alice).unwrap();
    assert!(server_sheet.stamina > 0.0, "level 5 warrior base stats");
    assert_eq!(alice_sheet, server_sheet);
    assert_eq!(
        seen_player(&mut clients[0], "Bob"),
        Some(None),
        "Alice sees Bob without his sheet stats"
    );
    assert_eq!(
        seen_player(&mut clients[1], "Alice"),
        Some(None),
        "Bob sees Alice without her sheet stats"
    );
    assert!(seen_player(&mut clients[1], "Bob").unwrap().is_some());
    let alice_derived = seen_derived(&mut clients[0], "Alice")
        .unwrap()
        .expect("Alice receives her own derived stats");
    assert_eq!(
        alice_derived,
        *server.world().get::<DerivedStats>(alice).unwrap()
    );
    // Every player's 5% base crit.
    assert_eq!(alice_derived.crit_pct, 5.0);
    assert_eq!(seen_derived(&mut clients[0], "Bob"), Some(None));
    assert_eq!(seen_derived(&mut clients[1], "Alice"), Some(None));

    server.world_mut().entity_mut(alice).insert(EquippedItems {
        items: vec![(
            EquipmentSlot::MainHand,
            shared::item_data::ItemDefinitionSource::Retail,
            NOTCHED_SHORTSWORD,
        )],
    });
    advance(&mut server, &mut clients, 100);

    let equipped = seen_player(&mut clients[0], "Alice").unwrap().unwrap();
    assert_eq!(equipped.agility, alice_sheet.agility + 1.0);
    assert_eq!(equipped.stamina, alice_sheet.stamina + 1.0);
    assert_eq!(
        seen_player(&mut clients[1], "Alice"),
        Some(None),
        "equipping does not leak Alice's stats to Bob"
    );
    assert_eq!(seen_derived(&mut clients[1], "Alice"), Some(None));
}
