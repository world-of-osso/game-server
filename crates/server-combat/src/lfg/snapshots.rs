use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SnapshotUpdate {
    pub(super) message: Option<String>,
    pub(super) error: Option<String>,
}

impl SnapshotUpdate {
    pub(super) fn message(message: &str) -> Self {
        Self {
            message: Some(message.into()),
            error: None,
        }
    }

    pub(super) fn error(error: impl Into<String>) -> Self {
        Self {
            message: None,
            error: Some(error.into()),
        }
    }
}

pub(super) fn player_name(
    player_states: &Query<(&shared::components::Player, &mut PlayerLfgState)>,
    player_entity: Entity,
) -> Option<String> {
    player_states
        .get(player_entity)
        .ok()
        .map(|(player, _)| player.name.clone())
}

pub(super) fn player_snapshot(
    player_states: &Query<(&shared::components::Player, &mut PlayerLfgState)>,
    player_entity: Entity,
) -> Option<PlayerLfgState> {
    player_states
        .get(player_entity)
        .ok()
        .map(|(_, player_state)| player_state.clone())
}

fn build_snapshot(
    player_entity: Entity,
    player_state: &PlayerLfgState,
    queue: &DungeonQueue,
    now: u64,
) -> LfgSnapshot {
    LfgSnapshot {
        queued: queue.is_queued(player_entity.to_bits()),
        selected_role: player_state.selected_role.map(map_group_role),
        dungeon_ids: player_state.dungeon_ids.clone(),
        queue_size: queue.len() as u16,
        average_wait_secs: queue.average_wait(now) as u32,
        in_demand_roles: in_demand_roles(queue)
            .into_iter()
            .map(map_group_role)
            .collect(),
        role_check: build_role_check_snapshot(player_state),
        match_found: build_match_found_snapshot(player_state),
    }
}

fn build_role_check_snapshot(player_state: &PlayerLfgState) -> Option<LfgRoleCheckSnapshot> {
    player_state
        .role_check
        .as_ref()
        .map(|role_check| LfgRoleCheckSnapshot {
            dungeon_id: role_check.dungeon_id,
            dungeon_name: role_check.dungeon_name.clone(),
            assigned_role: map_group_role(role_check.assigned_role),
            accepted_count: role_check.accepted_count,
            total_count: role_check.total_count,
        })
}

fn build_match_found_snapshot(player_state: &PlayerLfgState) -> Option<LfgMatchFoundSnapshot> {
    player_state
        .match_found
        .as_ref()
        .map(|match_found| LfgMatchFoundSnapshot {
            dungeon_id: match_found.dungeon_id,
            dungeon_name: match_found.dungeon_name.clone(),
            assigned_role: map_group_role(match_found.assigned_role),
            members: match_found
                .members
                .iter()
                .map(|member| LfgMatchMemberSnapshot {
                    name: member.name.clone(),
                    role: map_group_role(member.role),
                })
                .collect(),
        })
}

pub(super) struct SnapshotOptions {
    pub(super) message: Option<String>,
    pub(super) error: Option<String>,
    pub(super) now: u64,
}

pub(super) fn send_snapshot_to_player(
    player_entity: Entity,
    player_state: &PlayerLfgState,
    state: &LfgServerState,
    connections: &Query<(Entity, &PlayerLink), With<Authenticated>>,
    senders: &mut Query<&mut MessageSender<LfgStateUpdate>>,
    opts: SnapshotOptions,
) {
    let Some(conn_entity) = connection_entity_for_player(player_entity, connections) else {
        return;
    };
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<LfgChannel>(LfgStateUpdate {
        snapshot: Some(build_snapshot(
            player_entity,
            player_state,
            &state.queue,
            opts.now,
        )),
        message: opts.message,
        error: opts.error,
    });
}

pub(super) fn send_snapshot_update(
    player_entity: Entity,
    player_state: &PlayerLfgState,
    state: &LfgServerState,
    connections: &Query<(Entity, &PlayerLink), With<Authenticated>>,
    senders: &mut Query<&mut MessageSender<LfgStateUpdate>>,
    update: SnapshotUpdate,
    now: u64,
) {
    send_snapshot_to_player(
        player_entity,
        player_state,
        state,
        connections,
        senders,
        SnapshotOptions {
            message: update.message,
            error: update.error,
            now,
        },
    );
}

pub(super) fn send_error(
    conn_entity: Entity,
    senders: &mut Query<&mut MessageSender<LfgStateUpdate>>,
    error: &str,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<LfgChannel>(LfgStateUpdate {
        snapshot: None,
        message: None,
        error: Some(error.into()),
    });
}

fn connection_entity_for_player(
    player_entity: Entity,
    connections: &Query<(Entity, &PlayerLink), With<Authenticated>>,
) -> Option<Entity> {
    connections
        .iter()
        .find_map(|(conn_entity, link)| (link.0 == player_entity).then_some(conn_entity))
}
