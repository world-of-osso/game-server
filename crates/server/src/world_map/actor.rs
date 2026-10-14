//! Discovery replies follow the serial writer's durability receipt.
use super::*;
use crate::persistence_writer::CommitReceipt;
use std::collections::VecDeque;

struct PendingDiscovery {
    connection: Entity,
    player: Entity,
    character: u64,
    message: String,
    receipt: CommitReceipt<PlayerWorldMapState>,
}

#[derive(Resource, Default)]
pub(super) struct PendingDiscoveries(VecDeque<PendingDiscovery>);

impl PendingDiscoveries {
    pub(super) fn enqueue(
        &mut self,
        db: &GameDatabase,
        connection: Entity,
        player: Entity,
        character: u64,
        state: PlayerWorldMapState,
        message: String,
    ) {
        let receipt = db.writer.enqueue("world map discovery", move |db| {
            let mut durable = persistence::load_world_map(db, character);
            let previous_count = durable.discovered_zone_ids.len();
            durable
                .discovered_zone_ids
                .extend(state.discovered_zone_ids);
            if durable.discovered_zone_ids.len() != previous_count {
                persistence::save_world_map(db, character, &durable);
            }
            Ok(durable)
        });
        self.0.push_back(PendingDiscovery {
            connection,
            player,
            character,
            message,
            receipt,
        });
    }
}

pub(super) fn publish_discoveries(
    mut pending: ResMut<PendingDiscoveries>,
    sessions: Query<(&PlayerLink, &InWorld), With<Authenticated>>,
    mut world_maps: Query<&mut PlayerWorldMapState>,
    mut senders: Query<&mut MessageSender<WorldMapStateUpdate>>,
) {
    while let Some(result) = pending
        .0
        .front()
        .and_then(|pending| pending.receipt.result())
    {
        let discovery = pending.0.pop_front().expect("completed discovery receipt");
        let same_player = sessions
            .get(discovery.connection)
            .is_ok_and(|(link, session)| {
                link.0 == discovery.player && session.character_id == discovery.character
            });
        if !same_player {
            continue;
        }
        publish_discovery(discovery, result, &mut world_maps, &mut senders);
    }
}

fn publish_discovery(
    discovery: PendingDiscovery,
    result: Result<PlayerWorldMapState, String>,
    world_maps: &mut Query<&mut PlayerWorldMapState>,
    senders: &mut Query<&mut MessageSender<WorldMapStateUpdate>>,
) {
    let durable = match result {
        Ok(durable) => durable,
        Err(error) => {
            send_world_map_error(discovery.connection, senders, &error);
            return;
        }
    };
    if let Ok(mut live) = world_maps.get_mut(discovery.player) {
        live.discovered_zone_ids
            .extend(durable.discovered_zone_ids.iter().copied());
    }
    send_world_map_update(
        discovery.connection,
        build_world_map_snapshot(&durable),
        Some(discovery.message),
        None,
        senders,
    );
}
