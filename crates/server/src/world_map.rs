mod actor;
use actor::{PendingDiscoveries, publish_discoveries};
use std::collections::BTreeSet;

use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use shared::components::Zone;
use shared::protocol::{WorldMapChannel, WorldMapSnapshot, WorldMapStateUpdate};

use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::{self, GameDatabase};

#[derive(Component, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PlayerWorldMapState {
    pub discovered_zone_ids: BTreeSet<u32>,
}

pub struct WorldMapPlugin;

impl Plugin for WorldMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingDiscoveries>().add_systems(
            Update,
            (
                send_initial_world_map_snapshot,
                process_zone_discovery_updates,
                publish_discoveries,
            )
                .chain(),
        );
    }
}

type NewInWorldConnection<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static PlayerLink, &'static InWorld),
    (With<Authenticated>, Added<InWorld>),
>;

fn send_initial_world_map_snapshot(
    connections: NewInWorldConnection,
    mut world_maps: Query<(&Zone, &mut PlayerWorldMapState)>,
    db: Option<Res<GameDatabase>>,
    mut pending: ResMut<PendingDiscoveries>,
    mut senders: Query<&mut MessageSender<WorldMapStateUpdate>>,
) {
    for (conn_entity, link, in_world) in &connections {
        let Ok((zone, mut state)) = world_maps.get_mut(link.0) else {
            send_world_map_error(
                conn_entity,
                &mut senders,
                "world map unavailable for player",
            );
            continue;
        };
        discover_zone(&mut state, zone.id);
        if let Some(db) = db.as_ref() {
            pending.enqueue(
                db,
                conn_entity,
                link.0,
                in_world.character_id,
                state.clone(),
                "world map discovery loaded".into(),
            );
            continue;
        }
        send_world_map_update(
            conn_entity,
            build_world_map_snapshot(&state),
            Some("world map discovery loaded".into()),
            None,
            &mut senders,
        );
    }
}

fn process_zone_discovery_updates(
    connections: Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    mut world_maps: Query<(Entity, &Zone, &mut PlayerWorldMapState), Changed<Zone>>,
    db: Option<Res<GameDatabase>>,
    mut pending: ResMut<PendingDiscoveries>,
) {
    let Some(db) = db else { return };
    for (player_entity, zone, mut state) in &mut world_maps {
        if !discover_zone(&mut state, zone.id) {
            continue;
        }
        let Some((conn_entity, character_id)) = connection_for_player(&connections, player_entity)
        else {
            continue;
        };
        pending.enqueue(
            &db,
            conn_entity,
            player_entity,
            character_id,
            state.clone(),
            format!("discovered zone {}", zone.id),
        );
    }
}

fn connection_for_player(
    connections: &Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    player_entity: Entity,
) -> Option<(Entity, u64)> {
    connections
        .iter()
        .find_map(|(conn_entity, link, in_world)| {
            (link.0 == player_entity).then_some((conn_entity, in_world.character_id))
        })
}

fn discover_zone(state: &mut PlayerWorldMapState, zone_id: u32) -> bool {
    zone_id != 0 && state.discovered_zone_ids.insert(zone_id)
}

fn build_world_map_snapshot(state: &PlayerWorldMapState) -> WorldMapSnapshot {
    WorldMapSnapshot {
        discovered_zone_ids: state.discovered_zone_ids.iter().copied().collect(),
    }
}

fn send_world_map_update(
    conn_entity: Entity,
    snapshot: WorldMapSnapshot,
    message: Option<String>,
    error: Option<String>,
    senders: &mut Query<&mut MessageSender<WorldMapStateUpdate>>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<WorldMapChannel>(WorldMapStateUpdate {
        snapshot: Some(snapshot),
        message,
        error,
    });
}

fn send_world_map_error(
    conn_entity: Entity,
    senders: &mut Query<&mut MessageSender<WorldMapStateUpdate>>,
    error: &str,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<WorldMapChannel>(WorldMapStateUpdate {
        snapshot: None,
        message: None,
        error: Some(error.into()),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_map_discovery_waits_for_fifo_without_blocking_updates() {
        use lightyear::connection::host::HostClient;
        use std::time::{Duration, Instant};
        let db = persistence::temp_database();
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.add_plugins(lightyear::prelude::server::ServerPlugins {
            tick_duration: Duration::from_millis(50),
        });
        app.register_message::<WorldMapStateUpdate>();
        app.insert_resource(GameDatabase::new(db))
            .add_plugins(WorldMapPlugin);
        app.finish();
        app.cleanup();
        let player = app
            .world_mut()
            .spawn((Zone { id: 12 }, PlayerWorldMapState::default()))
            .id();
        let connection = app
            .world_mut()
            .spawn((
                Connected,
                RemoteId(PeerId::Local(1)),
                HostClient { buffer: Vec::new() },
                Authenticated { account_id: 1 },
                InWorld { character_id: 42 },
                PlayerLink(player),
                MessageSender::<WorldMapStateUpdate>::default(),
                MessageReceiver::<WorldMapStateUpdate>::default(),
            ))
            .id();
        let held = app
            .world()
            .resource::<GameDatabase>()
            .db
            .begin_write()
            .unwrap();
        let release = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(250));
            held.abort().unwrap();
        });
        let started = Instant::now();
        app.update();
        app.world_mut().get_mut::<Zone>(player).unwrap().id = 1519;
        app.update();
        let elapsed = started.elapsed();
        let premature = app
            .world_mut()
            .get_mut::<MessageReceiver<WorldMapStateUpdate>>(connection)
            .unwrap()
            .receive()
            .next();
        release.join().unwrap();
        assert!(
            elapsed < Duration::from_millis(50),
            "world map tick waited: {elapsed:?}"
        );
        assert!(premature.is_none(), "discovery snapshot precedes commit");
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            app.update();
            let responses: Vec<_> = app
                .world_mut()
                .get_mut::<MessageReceiver<WorldMapStateUpdate>>(connection)
                .unwrap()
                .receive()
                .collect();
            if responses.iter().any(|response| {
                response
                    .snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.discovered_zone_ids == vec![12, 1519])
            }) {
                break;
            }
            assert!(Instant::now() < deadline, "no committed discovery snapshot");
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            persistence::load_world_map(&app.world().resource::<GameDatabase>().db, 42)
                .discovered_zone_ids,
            BTreeSet::from([12, 1519])
        );
    }

    #[test]
    fn absent_world_map_reader_does_not_wait_for_a_writer() {
        use std::time::{Duration, Instant};
        let db = persistence::temp_database();
        let held = db.begin_write().unwrap();
        let release = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(250));
            held.abort().unwrap();
        });
        let started = Instant::now();
        let state = persistence::load_world_map(&db, 42);
        let elapsed = started.elapsed();
        release.join().unwrap();
        assert!(
            elapsed < Duration::from_millis(50),
            "world map reader waited: {elapsed:?}"
        );
        assert_eq!(state, PlayerWorldMapState::default());
    }

    #[test]
    fn discover_zone_ignores_zero_and_duplicates() {
        let mut state = PlayerWorldMapState::default();

        assert!(!discover_zone(&mut state, 0));
        assert!(discover_zone(&mut state, 12));
        assert!(!discover_zone(&mut state, 12));
        assert!(discover_zone(&mut state, 1519));
        assert_eq!(
            build_world_map_snapshot(&state).discovered_zone_ids,
            vec![12, 1519]
        );
    }

    #[test]
    fn world_map_state_round_trips_through_persistence() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = redb::Database::create(file.path()).unwrap();
        crate::persistence::create_tables(&db);

        let state = PlayerWorldMapState {
            discovered_zone_ids: BTreeSet::from([12, 1519, 1537]),
        };
        crate::persistence::save_world_map(&db, 42, &state);

        let loaded = crate::persistence::load_world_map(&db, 42);
        assert_eq!(loaded, state);
    }
}
