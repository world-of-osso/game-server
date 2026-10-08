//! Remote life state and visibility through the production UDP replication boundary.
use super::*;
use crate::death::{DeathPlugin, PlayerDeathState, PlayerDeathTransition};
use shared::death::{DeathState, Graveyard};
use shared::protocol::{DeathChannel, ReleaseSpirit, ResurrectAtCorpse};
use std::net::UdpSocket;
use std::time::{Duration, Instant};

fn tick(server: &mut App, clients: &mut [App], duration: Duration) {
    let start = Instant::now();
    while start.elapsed() < duration {
        server.update();
        for client in clients.iter_mut() {
            client.update();
        }
        assert_ne!(
            seen(&mut clients[1]),
            Some(DeathState::Ghost),
            "living observer must never receive a ghost-state frame"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn seen(client: &mut App) -> Option<DeathState> {
    client
        .world_mut()
        .query::<(&Player, Option<&DeathState>)>()
        .iter(client.world())
        .find(|(player, _)| player.name == "Elara")
        .and_then(|(_, state)| state.copied())
}

#[test]
fn ghoststate_other_clients_see_death_release_resurrection_over_udp() {
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
        DeathPlugin,
    ));
    server.add_observer(on_new_link);
    server.add_observer(on_player_link_inserted);
    let transport = server
        .world_mut()
        .spawn((
            LocalAddr(address),
            ServerUdpIo::default(),
            NetcodeServer::new(NetcodeConfig::default()),
        ))
        .id();
    server.world_mut().trigger(Start { entity: transport });
    let (ghost, _) = super::regen::network_client(address, 931);
    let (living, _) = super::regen::network_client(address, 932);
    let (owner, owner_link) = super::regen::network_client(address, 933);
    let mut clients = [ghost, living, owner];
    server.finish();
    server.cleanup();
    for client in &mut clients {
        client.finish();
        client.cleanup();
    }
    tick(&mut server, &mut clients, Duration::from_millis(400));
    let mut players = Vec::new();
    for (id, name, state) in [
        (931, "GhostObserver", DeathState::Ghost),
        (932, "LivingObserver", DeathState::Alive),
        (933, "Elara", DeathState::Alive),
    ] {
        let player = server
            .world_mut()
            .spawn((
                Player {
                    name: name.into(),
                    race: 1,
                    class: 1,
                    appearance: default(),
                },
                Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                crate::maps::InMap::continent(0),
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                shared::components::MovementControl::default(),
                crate::persistence::ClientId(id),
                PlayerDeathState { state, ..default() },
                NetworkVisibility::default(),
                Replicate::to_clients(NetworkTarget::All),
            ))
            .id();
        let connection = server
            .world_mut()
            .query::<(Entity, &RemoteId)>()
            .iter(server.world())
            .find(|(_, remote)| remote.0 == PeerId::Netcode(id))
            .map(|(entity, _)| entity)
            .unwrap();
        server.world_mut().entity_mut(connection).insert((
            PlayerLink(player),
            crate::auth::Authenticated { account_id: id },
        ));
        players.push(player);
    }
    tick(&mut server, &mut clients, Duration::from_millis(400));
    assert_eq!(seen(&mut clients[0]), Some(DeathState::Alive));
    assert_eq!(seen(&mut clients[1]), Some(DeathState::Alive));
    let subject = players[2];
    server
        .world_mut()
        .get_mut::<Health>(subject)
        .unwrap()
        .current = 0.0;
    server
        .world_mut()
        .write_message(PlayerDeathTransition { player: subject });
    tick(&mut server, &mut clients, Duration::from_millis(400));
    assert_eq!(seen(&mut clients[0]), Some(DeathState::Dead));
    assert_eq!(seen(&mut clients[1]), Some(DeathState::Dead));
    // The fixture's graveyard is in corpse-recovery range, avoiding movement dependence.
    server
        .world_mut()
        .get_mut::<PlayerDeathState>(subject)
        .unwrap()
        .graveyard = Some(Graveyard {
        id: 1,
        zone_id: 1,
        faction: 0,
        map_id: 0,
        x: 5.0,
        y: 0.0,
        z: 0.0,
    });
    clients[2]
        .world_mut()
        .get_mut::<MessageSender<ReleaseSpirit>>(owner_link)
        .unwrap()
        .send::<DeathChannel>(ReleaseSpirit);
    tick(&mut server, &mut clients, Duration::from_millis(400));
    assert_eq!(seen(&mut clients[0]), Some(DeathState::Ghost));
    assert_eq!(
        seen(&mut clients[1]),
        None,
        "living observer must lose the ghost entity"
    );
    assert_eq!(
        seen(&mut clients[2]),
        Some(DeathState::Ghost),
        "owner still sees itself"
    );
    clients[2]
        .world_mut()
        .get_mut::<MessageSender<ResurrectAtCorpse>>(owner_link)
        .unwrap()
        .send::<DeathChannel>(ResurrectAtCorpse);
    tick(&mut server, &mut clients, Duration::from_millis(400));
    assert_eq!(seen(&mut clients[0]), Some(DeathState::Alive));
    assert_eq!(
        seen(&mut clients[1]),
        Some(DeathState::Alive),
        "stationary living observer regains visibility"
    );
}
