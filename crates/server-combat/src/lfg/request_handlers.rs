use super::*;

pub(super) fn process_player_lfg_requests<M, F>(
    receivers: &mut Query<(Entity, &PlayerLink, &mut MessageReceiver<M>), With<Authenticated>>,
    player_states: &mut Query<(&shared::components::Player, &mut PlayerLfgState)>,
    state: &mut LfgServerState,
    connections: &Query<(Entity, &PlayerLink), With<Authenticated>>,
    senders: &mut Query<&mut MessageSender<LfgStateUpdate>>,
    now: u64,
    mut handle_request: F,
) where
    M: lightyear::prelude::Message,
    F: FnMut(Entity, &mut PlayerLfgState, &mut LfgServerState, M, u64) -> SnapshotUpdate,
{
    for (conn_entity, link, mut receiver) in &mut *receivers {
        let requests: Vec<_> = receiver.receive().collect();
        if requests.is_empty() {
            continue;
        }
        let Ok((_player, mut player_state)) = player_states.get_mut(link.0) else {
            send_error(conn_entity, senders, "lfg unavailable for player");
            continue;
        };
        for request in requests {
            let update = handle_request(link.0, &mut player_state, state, request, now);
            send_snapshot_update(
                link.0,
                &player_state,
                state,
                connections,
                senders,
                update,
                now,
            );
        }
    }
}

pub(super) fn handle_queue_request(
    player_entity: Entity,
    player_state: &mut PlayerLfgState,
    state: &mut LfgServerState,
    request: QueueForLfg,
    now: u64,
) -> SnapshotUpdate {
    if player_state.role_check.is_some() {
        return SnapshotUpdate::error("role check already in progress");
    }

    player_state.match_found = None;
    let role = map_snapshot_role(request.role);
    let dungeon_ids = request.dungeon_ids;
    match state
        .queue
        .queue_solo(player_entity.to_bits(), role, dungeon_ids.clone(), now)
    {
        Ok(()) => {
            player_state.selected_role = Some(role);
            player_state.dungeon_ids = dungeon_ids;
            SnapshotUpdate::message("queued for dungeon finder")
        }
        Err(error) => SnapshotUpdate::error(queue_error_message(error)),
    }
}

pub(super) fn handle_dequeue_request(
    player_entity: Entity,
    player_state: &mut PlayerLfgState,
    state: &mut LfgServerState,
) -> SnapshotUpdate {
    if player_state.role_check.is_some() {
        return SnapshotUpdate::error("cannot dequeue during a role check");
    }
    if !state.queue.dequeue(player_entity.to_bits()) {
        return SnapshotUpdate::error("not queued for dungeon finder");
    }

    clear_queue_state(player_state);
    SnapshotUpdate::message("removed from dungeon finder queue")
}
