//! Player regeneration over real UDP: a damaged, out-of-combat level 10 mage's health and
//! mana climb on the server's 2 s health ticks and per-tick mana regen, and the client
//! sees the replicated values change, ending exactly at max (no 1-health shortfall).

use super::*;
use crate::spell_cast::powers::{
    PlayerHealthRegen, PowerPools, PowerTypeTable, regenerate_player_health, regenerate_powers,
    sync_unit_powers,
};
use lightyear::prelude::client::{ClientPlugins, Connect, NetcodeClient};
use shared::components::{Level, Mana, Player, PowerType, UnitPowers};
use std::net::UdpSocket;
use std::time::{Duration, Instant};

pub(super) fn network_server(address: std::net::SocketAddr) -> App {
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
    server.init_resource::<PowerTypeTable>();
    server.add_systems(
        FixedUpdate,
        (
            regenerate_powers,
            sync_unit_powers,
            regenerate_player_health,
        )
            .chain(),
    );
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

pub(super) fn advance(server: &mut App, client: &mut App, frames: usize) {
    for _ in 0..frames {
        server.update();
        client.update();
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// `(health, mana)` of the named player as the client sees it.
fn seen(client: &mut App, name: &str) -> Option<(Health, Mana, i32)> {
    client
        .world_mut()
        .query::<(&Player, &Health, &Mana, &UnitPowers)>()
        .iter(client.world())
        .find(|(player, ..)| player.name == name)
        .map(|(_, health, mana, powers)| {
            let replicated_mana = powers
                .entries
                .iter()
                .find(|entry| entry.power == PowerType::Mana)
                .map_or(-1, |entry| entry.current);
            (*health, *mana, replicated_mana)
        })
}

/// Health values the client saw while running for `duration` of real time.
fn watch_health(
    server: &mut App,
    client: &mut App,
    duration: Duration,
) -> (Vec<f32>, Option<(Health, Mana, i32)>) {
    let start = Instant::now();
    let mut values = Vec::new();
    let mut last = None;
    while start.elapsed() < duration {
        advance(server, client, 1);
        last = seen(client, "Regen");
        if let Some((health, ..)) = last
            && values.last() != Some(&health.current)
        {
            values.push(health.current);
        }
    }
    (values, last)
}

#[test]
fn client_sees_a_damaged_players_health_and_mana_regenerate_to_exactly_full() {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let address = socket.local_addr().unwrap();
    drop(socket);
    let mut server = network_server(address);
    let (mut client, client_entity) = network_client(address, 921);
    for app in [&mut server, &mut client] {
        app.finish();
        app.cleanup();
    }
    advance(&mut server, &mut client, 250);
    assert!(client.world().get::<Connected>(client_entity).is_some());

    let player = server
        .world_mut()
        .spawn((
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            crate::maps::InMap::continent(0),
            Player {
                name: "Regen".into(),
                race: 1,
                class: 8,
                appearance: default(),
            },
            Level(10),
            shared::components::CombatStatus(false),
            Health {
                current: 666.0,
                max: 1110.0,
            },
            Mana {
                current: 0.0,
                max: 1000.0,
            },
            crate::stats::CharacterStats {
                base_mana: 1000.0,
                ..default()
            },
            PowerPools {
                order: vec![PowerType::Mana],
                ..default()
            },
            UnitPowers::default(),
            PlayerHealthRegen::default(),
            NetworkVisibility::default(),
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    let connection = server
        .world_mut()
        .query_filtered::<Entity, With<RemoteId>>()
        .single(server.world())
        .unwrap();
    server
        .world_mut()
        .entity_mut(connection)
        .insert(PlayerLink(player));

    // ~5 s: two or three 2 s health ticks of 22.2 (level 10: 20% of 1110 / 10).
    let (values, last) = watch_health(&mut server, &mut client, Duration::from_millis(5_500));
    assert!(values.len() >= 3, "client health samples {values:?}");
    assert_eq!(values[0], 666.0, "{values:?}");
    for pair in values.windows(2) {
        let gain = pair[1] - pair[0];
        assert!(gain == 22.0 || gain == 23.0, "{values:?}");
    }
    let (_, mana, replicated_mana) = last.unwrap();
    // 1% of 1000 base mana per second for ~5 s; UnitPowers carries the whole part.
    assert!((40.0..=60.0).contains(&mana.current), "{mana:?}");
    assert!((40..=60).contains(&replicated_mana), "{replicated_mana}");

    // The observed 1109/1110 shortfall: the next tick fills to exactly max.
    server
        .world_mut()
        .get_mut::<Health>(player)
        .unwrap()
        .current = 1109.0;
    if let Some(dirty) = server
        .world()
        .get_resource::<lightyear::prelude::ReplicationDirtyQueue>()
    {
        dirty.mark(player);
    }
    let (values, _) = watch_health(&mut server, &mut client, Duration::from_millis(2_600));
    assert_eq!(values.last(), Some(&1110.0), "{values:?}");
    assert_eq!(
        server.world().get::<Health>(player).unwrap().current,
        1110.0
    );
}
