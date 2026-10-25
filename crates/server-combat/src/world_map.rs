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
        app.add_systems(
            Update,
            (
                send_initial_world_map_snapshot,
                process_zone_discovery_updates,
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
        let discovered_new_zone = discover_zone(&mut state, zone.id);
        if discovered_new_zone && let Some(db) = db.as_ref() {
            persistence::save_world_map(&db.db, in_world.character_id, &state);
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
    mut senders: Query<&mut MessageSender<WorldMapStateUpdate>>,
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
        persistence::save_world_map(&db.db, character_id, &state);
        send_world_map_update(
            conn_entity,
            build_world_map_snapshot(&state),
            Some(format!("discovered zone {}", zone.id)),
            None,
            &mut senders,
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
