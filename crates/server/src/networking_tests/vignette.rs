//! Creature vignettes over real UDP against the imported world.db
//! (`cargo test -p server vignette -- --ignored`): Doomwalker (creature_template
//! 167749, VignetteID 6520, content_creature guid 30800000 on map 1, a game event 89
//! spawn) shows its vignette to a player within the interest radius, not to one beyond
//! it, and loses it on death until it respawns (TrinityCore Vignette.cpp,
//! `Unit::setDeathState`).

use super::regen::{advance, network_client};
use super::*;
use crate::interest::{InterestDirtyQueue, mark_interest_position_changed};
use crate::world_data::{WorldData, load_spawn, load_world_data};
use bevy::ecs::system::RunSystemOnce;
use shared::components::{Npc, UnitVignette};
use std::net::UdpSocket;
use std::path::PathBuf;
use std::time::Duration;

const DOOMWALKER_GUID: u32 = 30_800_000;
const DOOMWALKER: u32 = 167_749;

fn network_server(address: std::net::SocketAddr) -> App {
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
    server.init_resource::<crate::combat::CombatLog>();
    server.add_systems(FixedUpdate, crate::combat::respawn_system);
    server.insert_resource(load_world_data(&world_db()));
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

fn world_db() -> PathBuf {
    crate::test_data::world_db_path()
}

/// Doomwalker's content_creature row, spawned as its game event would.
fn spawn_doomwalker(server: &mut App) -> Entity {
    let conn = rusqlite::Connection::open(world_db()).unwrap();
    let spawn = load_spawn(&conn, DOOMWALKER_GUID).expect("Doomwalker in content_creature");
    server
        .world_mut()
        .run_system_once(move |mut commands: Commands, data: Res<WorldData>| {
            let template = data.template_for(spawn.template_entry, 0).unwrap();
            crate::zones::spawn_creature(
                &mut commands,
                &data,
                template,
                &mut crate::zones::CreatureSpawnContext {
                    map: crate::maps::InMap::continent(1),
                    difficulty_id: 0,
                    ground: Some(&mut crate::test_data::ground_map()),
                    temporary: false,
                },
                true,
                &spawn,
                &mut rand::thread_rng(),
            )
            .expect("source Doomwalker level context")
        })
        .unwrap()
}

/// What the client sees of Doomwalker: `None` when it is not replicated, else its
/// vignette.
fn seen(client: &mut App) -> Option<Option<UnitVignette>> {
    client
        .world_mut()
        .query::<(&Npc, Option<&UnitVignette>)>()
        .iter(client.world())
        .find(|(npc, _)| npc.template_id == DOOMWALKER)
        .map(|(_, vignette)| vignette.copied())
}

fn advance_until(
    server: &mut App,
    client: &mut App,
    what: &str,
    expected: Option<Option<UnitVignette>>,
) {
    for _ in 0..200 {
        advance(server, client, 1);
        if seen(client) == expected {
            return;
        }
    }
    panic!("{what}: client sees {:?}", seen(client));
}

fn move_player(server: &mut App, player: Entity, position: Position) {
    *server.world_mut().get_mut::<Position>(player).unwrap() = position;
    mark_interest_position_changed(server.world().get_resource::<InterestDirtyQueue>(), player);
}

#[test]
#[ignore]
fn doomwalkers_vignette_follows_range_death_and_respawn() {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let address = socket.local_addr().unwrap();
    drop(socket);
    let mut server = network_server(address);
    let (mut client, client_entity) = network_client(address, 931);
    for app in [&mut server, &mut client] {
        app.finish();
        app.cleanup();
    }
    advance(&mut server, &mut client, 250);
    assert!(client.world().get::<Connected>(client_entity).is_some());

    let doomwalker = spawn_doomwalker(&mut server);
    let at_doomwalker = *server.world().get::<Position>(doomwalker).unwrap();
    let player = server
        .world_mut()
        .spawn((
            at_doomwalker,
            crate::maps::InMap::continent(1),
            Player {
                name: "Vignette".into(),
                race: 1,
                class: 1,
                appearance: default(),
            },
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

    let vignette = Some(Some(UnitVignette(6_520)));
    advance_until(&mut server, &mut client, "in range", vignette);

    let far = Position {
        x: at_doomwalker.x + 150.0,
        ..at_doomwalker
    };
    move_player(&mut server, player, far);
    advance_until(&mut server, &mut client, "out of range", None);
    move_player(&mut server, player, at_doomwalker);
    advance_until(&mut server, &mut client, "back in range", vignette);

    // The corpse stays replicated without its vignette.
    server
        .world_mut()
        .get_mut::<Health>(doomwalker)
        .unwrap()
        .current = 0.0;
    crate::combat::mark_npc_dead_world(server.world_mut(), doomwalker);
    advance_until(&mut server, &mut client, "dead", Some(None));

    server
        .world_mut()
        .get_mut::<crate::combat::DeathState>(doomwalker)
        .unwrap()
        .respawn_timer = 0.0;
    advance_until(&mut server, &mut client, "respawned", vignette);
}
