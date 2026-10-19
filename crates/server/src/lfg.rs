use bevy::prelude::*;
use lightyear::prelude::*;
use shared::dungeon_finder::{DungeonQueue, MatchResult, in_demand_roles};
use shared::group::GroupRole;
use shared::protocol::{
    DequeueFromLfg, GroupRoleSnapshot, LfgChannel, LfgMatchFoundSnapshot, LfgMatchMemberSnapshot,
    LfgRoleCheckSnapshot, LfgSnapshot, LfgStateUpdate, QueryLfgStatus, QueueForLfg,
    RespondToLfgRoleCheck,
};

use crate::auth::Authenticated;
use crate::networking::PlayerLink;

mod follower_catalog;
mod request_handlers;
mod snapshots;

use self::request_handlers::{
    handle_dequeue_request, handle_queue_request, process_player_lfg_requests,
};
use self::snapshots::{
    SnapshotOptions, SnapshotUpdate, player_name, player_snapshot, send_error,
    send_snapshot_to_player, send_snapshot_update,
};

#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerLfgState {
    pub selected_role: Option<GroupRole>,
    pub dungeon_ids: Vec<u32>,
    pub role_check: Option<PlayerRoleCheckState>,
    pub match_found: Option<PlayerMatchFoundState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerRoleCheckState {
    pub dungeon_id: u32,
    pub dungeon_name: String,
    pub assigned_role: GroupRole,
    pub accepted_count: u8,
    pub total_count: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerMatchFoundState {
    pub dungeon_id: u32,
    pub dungeon_name: String,
    pub assigned_role: GroupRole,
    pub members: Vec<PlayerMatchMemberState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerMatchMemberState {
    pub name: String,
    pub role: GroupRole,
}

#[derive(Resource, Default)]
pub(crate) struct LfgServerState {
    queue: DungeonQueue,
    role_checks: Vec<RoleCheckState>,
}

impl LfgServerState {
    /// Current queue/role-check activity, not retained player selection history.
    pub(crate) fn is_queued_or_checking_roles(&self, player: Entity) -> bool {
        self.queue.is_queued(player.to_bits())
            || self
                .role_checks
                .iter()
                .any(|check| check.has_player(player))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RoleCheckState {
    dungeon_id: u32,
    members: Vec<RoleCheckMember>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RoleCheckMember {
    player: Entity,
    role: GroupRole,
    accepted: bool,
}

pub struct LfgPlugin;

impl Plugin for LfgPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LfgServerState>();
        app.add_systems(Update, follower_catalog::send_follower_catalog);
        app.add_systems(Update, process_lfg_status_queries);
        app.add_systems(Update, process_lfg_queue_requests);
        app.add_systems(Update, process_lfg_dequeue_requests);
        app.add_systems(Update, process_lfg_role_check_responses);
        app.add_systems(Update, advance_lfg_matchmaking);
    }
}

fn process_lfg_status_queries(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<QueryLfgStatus>),
        With<Authenticated>,
    >,
    players: Query<(&shared::components::Player, &PlayerLfgState)>,
    state: Res<LfgServerState>,
    time: Res<Time>,
    connections: Query<(Entity, &PlayerLink), With<Authenticated>>,
    mut senders: Query<&mut MessageSender<LfgStateUpdate>>,
) {
    for (_conn_entity, link, mut receiver) in &mut receivers {
        let requests: Vec<_> = receiver.receive().collect();
        if requests.is_empty() {
            continue;
        }
        let Ok((_player, player_state)) = players.get(link.0) else {
            continue;
        };
        send_snapshot_to_player(
            link.0,
            player_state,
            &state,
            &connections,
            &mut senders,
            SnapshotOptions {
                message: Some("lfg status refreshed".into()),
                error: None,
                now: current_seconds(&time),
            },
        );
    }
}

fn process_lfg_queue_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<QueueForLfg>),
        With<Authenticated>,
    >,
    mut player_states: Query<(&shared::components::Player, &mut PlayerLfgState)>,
    state: ResMut<LfgServerState>,
    time: Res<Time>,
    connections: Query<(Entity, &PlayerLink), With<Authenticated>>,
    mut senders: Query<&mut MessageSender<LfgStateUpdate>>,
    followers: Res<crate::followers::FollowerCatalog>,
    pvp_states: Query<&crate::pvp::PlayerPvpState>,
    mut commands: Commands,
) {
    process_player_lfg_requests(
        &mut receivers,
        &mut player_states,
        state.into_inner(),
        &connections,
        &mut senders,
        current_seconds(&time),
        |player, player_state, state, request, now| {
            let follower_queue = request
                .dungeon_ids
                .iter()
                .any(|id| followers.follower_ids.contains(id));
            if !follower_queue {
                // TC a352b1fa LFGMgr.cpp437–438: no dungeon join during PvP activity.
                let battlefield_active = pvp_states.get(player).is_ok_and(|pvp| {
                    pvp.battleground.is_some()
                        || matches!(
                            pvp.queue,
                            Some(crate::pvp::PlayerPvpQueue::Battleground { .. })
                                | Some(crate::pvp::PlayerPvpQueue::Arena { .. })
                                | Some(crate::pvp::PlayerPvpQueue::RatedBracket {
                                    bracket: shared::protocol::PvpBracketSnapshot::Arena2v2
                                        | shared::protocol::PvpBracketSnapshot::Arena3v3,
                                })
                        )
                });
                if battlefield_active {
                    return SnapshotUpdate::error(
                        "cannot join dungeon finder during battlefield activity",
                    );
                }
                return handle_queue_request(player, player_state, state, request, now);
            }
            if request.dungeon_ids.len() != 1 {
                return SnapshotUpdate::error("select one follower dungeon");
            }
            if state.queue.is_queued(player.to_bits()) || player_state.role_check.is_some() {
                return SnapshotUpdate::error("already queued or checking roles");
            }
            let dungeon = request.dungeon_ids[0];
            let role = map_snapshot_role(request.role);
            commands.queue(move |world: &mut World| {
                crate::followers::queue_player(world, player, role, dungeon)
            });
            SnapshotUpdate::message("follower dungeon requested")
        },
    );
}

fn process_lfg_dequeue_requests(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<DequeueFromLfg>),
        With<Authenticated>,
    >,
    mut player_states: Query<(&shared::components::Player, &mut PlayerLfgState)>,
    state: ResMut<LfgServerState>,
    time: Res<Time>,
    connections: Query<(Entity, &PlayerLink), With<Authenticated>>,
    mut senders: Query<&mut MessageSender<LfgStateUpdate>>,
) {
    process_player_lfg_requests(
        &mut receivers,
        &mut player_states,
        state.into_inner(),
        &connections,
        &mut senders,
        current_seconds(&time),
        |player_entity, player_state, state, _request, _now| {
            handle_dequeue_request(player_entity, player_state, state)
        },
    );
}

fn process_lfg_role_check_responses(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &mut MessageReceiver<RespondToLfgRoleCheck>,
        ),
        With<Authenticated>,
    >,
    mut player_states: Query<(&shared::components::Player, &mut PlayerLfgState)>,
    state: ResMut<LfgServerState>,
    time: Res<Time>,
    connections: Query<(Entity, &PlayerLink), With<Authenticated>>,
    mut senders: Query<&mut MessageSender<LfgStateUpdate>>,
) {
    let state = state.into_inner();
    let now = current_seconds(&time);
    for (conn_entity, link, mut receiver) in &mut receivers {
        let requests: Vec<_> = receiver.receive().collect();
        if requests.is_empty() {
            continue;
        }
        let Some(player_name) = player_name(&player_states, link.0) else {
            send_error(conn_entity, &mut senders, "lfg unavailable for player");
            continue;
        };
        for request in requests {
            let Some(index) = state
                .role_checks
                .iter()
                .position(|role_check| role_check.has_player(link.0))
            else {
                let Some(player_state) = player_snapshot(&player_states, link.0) else {
                    send_error(conn_entity, &mut senders, "lfg unavailable for player");
                    continue;
                };
                send_snapshot_update(
                    link.0,
                    &player_state,
                    state,
                    &connections,
                    &mut senders,
                    SnapshotUpdate::error("no role check in progress"),
                    now,
                );
                continue;
            };
            if request.accepted {
                let all_accepted =
                    accept_role_check_response(&mut state.role_checks[index], link.0);
                if all_accepted {
                    let role_check = state.role_checks.remove(index);
                    finalize_match(
                        role_check,
                        state,
                        &mut player_states,
                        &connections,
                        &mut senders,
                        now,
                    );
                } else {
                    broadcast_role_check_progress(
                        &state.role_checks[index],
                        state,
                        &mut player_states,
                        &connections,
                        &mut senders,
                        format!("{player_name} accepted the role check"),
                        now,
                    );
                }
            } else {
                let role_check = state.role_checks.remove(index);
                cancel_role_check(
                    role_check,
                    link.0,
                    state,
                    &mut player_states,
                    &connections,
                    &mut senders,
                    now,
                );
            }
        }
    }
}

fn advance_lfg_matchmaking(
    state: ResMut<LfgServerState>,
    mut player_states: Query<(&shared::components::Player, &mut PlayerLfgState)>,
    connections: Query<(Entity, &PlayerLink), With<Authenticated>>,
    mut senders: Query<&mut MessageSender<LfgStateUpdate>>,
    time: Res<Time>,
) {
    let state = state.into_inner();
    let now = current_seconds(&time);
    while let Some(matched) = state.queue.try_match() {
        let role_check = RoleCheckState::from_match(&matched);
        apply_role_check_state(&role_check, &mut player_states);
        broadcast_role_check_started(
            &role_check,
            state,
            &mut player_states,
            &connections,
            &mut senders,
            now,
        );
        state.role_checks.push(role_check);
    }
}

fn finalize_match(
    role_check: RoleCheckState,
    state: &LfgServerState,
    player_states: &mut Query<(&shared::components::Player, &mut PlayerLfgState)>,
    connections: &Query<(Entity, &PlayerLink), With<Authenticated>>,
    senders: &mut Query<&mut MessageSender<LfgStateUpdate>>,
    now: u64,
) {
    let members = role_check.members_with_names(player_states);
    let dungeon_name = dungeon_name(role_check.dungeon_id);
    for member in &role_check.members {
        let Ok((_player, mut player_state)) = player_states.get_mut(member.player) else {
            continue;
        };
        player_state.role_check = None;
        player_state.match_found = Some(PlayerMatchFoundState {
            dungeon_id: role_check.dungeon_id,
            dungeon_name: dungeon_name.clone(),
            assigned_role: member.role,
            members: members.clone(),
        });
    }
    for member in &role_check.members {
        let Ok((_player, player_state)) = player_states.get_mut(member.player) else {
            continue;
        };
        send_snapshot_to_player(
            member.player,
            &player_state,
            state,
            connections,
            senders,
            SnapshotOptions {
                message: Some(format!("match found for {dungeon_name}")),
                error: None,
                now,
            },
        );
    }
}

fn cancel_role_check(
    role_check: RoleCheckState,
    decliner: Entity,
    state: &LfgServerState,
    player_states: &mut Query<(&shared::components::Player, &mut PlayerLfgState)>,
    connections: &Query<(Entity, &PlayerLink), With<Authenticated>>,
    senders: &mut Query<&mut MessageSender<LfgStateUpdate>>,
    now: u64,
) {
    let decliner_name = player_name(player_states, decliner).unwrap_or_else(|| "A player".into());
    for member in &role_check.members {
        let Ok((_player, mut player_state)) = player_states.get_mut(member.player) else {
            continue;
        };
        player_state.role_check = None;
        player_state.match_found = None;
    }
    for member in &role_check.members {
        let Ok((_player, player_state)) = player_states.get_mut(member.player) else {
            continue;
        };
        send_snapshot_to_player(
            member.player,
            &player_state,
            state,
            connections,
            senders,
            SnapshotOptions {
                message: Some(format!("{decliner_name} declined the role check")),
                error: None,
                now,
            },
        );
    }
}

fn broadcast_role_check_started(
    role_check: &RoleCheckState,
    state: &LfgServerState,
    player_states: &mut Query<(&shared::components::Player, &mut PlayerLfgState)>,
    connections: &Query<(Entity, &PlayerLink), With<Authenticated>>,
    senders: &mut Query<&mut MessageSender<LfgStateUpdate>>,
    now: u64,
) {
    broadcast_role_check_progress(
        role_check,
        state,
        player_states,
        connections,
        senders,
        format!(
            "role check started for {}",
            dungeon_name(role_check.dungeon_id)
        ),
        now,
    );
}

fn broadcast_role_check_progress(
    role_check: &RoleCheckState,
    state: &LfgServerState,
    player_states: &mut Query<(&shared::components::Player, &mut PlayerLfgState)>,
    connections: &Query<(Entity, &PlayerLink), With<Authenticated>>,
    senders: &mut Query<&mut MessageSender<LfgStateUpdate>>,
    message: String,
    now: u64,
) {
    for member in &role_check.members {
        let Ok((_player, player_state)) = player_states.get_mut(member.player) else {
            continue;
        };
        send_snapshot_to_player(
            member.player,
            &player_state,
            state,
            connections,
            senders,
            SnapshotOptions {
                message: Some(message.clone()),
                error: None,
                now,
            },
        );
    }
}

fn apply_role_check_state(
    role_check: &RoleCheckState,
    player_states: &mut Query<(&shared::components::Player, &mut PlayerLfgState)>,
) {
    let accepted_count = role_check.accepted_count();
    let total_count = role_check.members.len() as u8;
    let dungeon_name = dungeon_name(role_check.dungeon_id);
    for member in &role_check.members {
        let Ok((_player, mut player_state)) = player_states.get_mut(member.player) else {
            continue;
        };
        player_state.role_check = Some(PlayerRoleCheckState {
            dungeon_id: role_check.dungeon_id,
            dungeon_name: dungeon_name.clone(),
            assigned_role: member.role,
            accepted_count,
            total_count,
        });
        player_state.match_found = None;
    }
}

fn clear_queue_state(player_state: &mut PlayerLfgState) {
    player_state.selected_role = None;
    player_state.dungeon_ids.clear();
    player_state.role_check = None;
    player_state.match_found = None;
}

fn accept_role_check_response(role_check: &mut RoleCheckState, player_entity: Entity) -> bool {
    role_check.accept(player_entity);
    role_check.all_accepted()
}

fn queue_error_message(error: shared::dungeon_finder::QueueError) -> String {
    match error {
        shared::dungeon_finder::QueueError::AlreadyQueued => {
            "already queued for dungeon finder".into()
        }
        shared::dungeon_finder::QueueError::NoDungeons => "no dungeons selected".into(),
        shared::dungeon_finder::QueueError::NoRoles => "no role selected".into(),
        shared::dungeon_finder::QueueError::Deserter => {
            "cannot queue while deserter is active".into()
        }
    }
}

pub(crate) fn map_group_role(role: GroupRole) -> GroupRoleSnapshot {
    match role {
        GroupRole::Tank => GroupRoleSnapshot::Tank,
        GroupRole::Healer => GroupRoleSnapshot::Healer,
        GroupRole::Dps => GroupRoleSnapshot::Damage,
    }
}

fn map_snapshot_role(role: GroupRoleSnapshot) -> GroupRole {
    match role {
        GroupRoleSnapshot::Tank => GroupRole::Tank,
        GroupRoleSnapshot::Healer => GroupRole::Healer,
        GroupRoleSnapshot::Damage | GroupRoleSnapshot::None => GroupRole::Dps,
    }
}

fn current_seconds(time: &Time) -> u64 {
    time.elapsed_secs() as u64
}

fn dungeon_name(dungeon_id: u32) -> String {
    match dungeon_id {
        100 => "Deadmines".into(),
        200 => "Shadowfang Keep".into(),
        _ => format!("Dungeon {dungeon_id}"),
    }
}

impl RoleCheckState {
    fn from_match(matched: &MatchResult) -> Self {
        let mut members = vec![
            RoleCheckMember {
                player: Entity::from_bits(matched.tank),
                role: GroupRole::Tank,
                accepted: false,
            },
            RoleCheckMember {
                player: Entity::from_bits(matched.healer),
                role: GroupRole::Healer,
                accepted: false,
            },
        ];
        members.extend(matched.dps.iter().copied().map(|player| RoleCheckMember {
            player: Entity::from_bits(player),
            role: GroupRole::Dps,
            accepted: false,
        }));
        Self {
            dungeon_id: matched.dungeon_id,
            members,
        }
    }

    fn has_player(&self, player: Entity) -> bool {
        self.members.iter().any(|member| member.player == player)
    }

    fn accept(&mut self, player: Entity) {
        if let Some(member) = self
            .members
            .iter_mut()
            .find(|member| member.player == player)
        {
            member.accepted = true;
        }
    }

    fn all_accepted(&self) -> bool {
        self.members.iter().all(|member| member.accepted)
    }

    fn accepted_count(&self) -> u8 {
        self.members.iter().filter(|member| member.accepted).count() as u8
    }

    fn members_with_names(
        &self,
        player_states: &mut Query<(&shared::components::Player, &mut PlayerLfgState)>,
    ) -> Vec<PlayerMatchMemberState> {
        self.members
            .iter()
            .filter_map(|member| {
                player_states
                    .get(member.player)
                    .ok()
                    .map(|(player, _)| PlayerMatchMemberState {
                        name: player.name.clone(),
                        role: member.role,
                    })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_check_tracks_acceptance() {
        let mut role_check = RoleCheckState {
            dungeon_id: 100,
            members: vec![
                RoleCheckMember {
                    player: Entity::from_bits(1),
                    role: GroupRole::Tank,
                    accepted: false,
                },
                RoleCheckMember {
                    player: Entity::from_bits(2),
                    role: GroupRole::Healer,
                    accepted: false,
                },
            ],
        };

        role_check.accept(Entity::from_bits(1));

        assert_eq!(role_check.accepted_count(), 1);
        assert!(!role_check.all_accepted());
    }

    #[test]
    fn queue_error_messages_are_human_readable() {
        assert_eq!(
            queue_error_message(shared::dungeon_finder::QueueError::NoDungeons),
            "no dungeons selected"
        );
        assert_eq!(
            queue_error_message(shared::dungeon_finder::QueueError::AlreadyQueued),
            "already queued for dungeon finder"
        );
    }

    #[test]
    fn dungeon_names_fallback_when_unknown() {
        assert_eq!(dungeon_name(100), "Deadmines");
        assert_eq!(dungeon_name(999), "Dungeon 999");
    }

    #[test]
    fn queue_request_updates_player_queue_state() {
        let player = Entity::from_bits(7);
        let mut player_state = PlayerLfgState {
            match_found: Some(PlayerMatchFoundState {
                dungeon_id: 100,
                dungeon_name: "Deadmines".into(),
                assigned_role: GroupRole::Tank,
                members: vec![],
            }),
            ..Default::default()
        };
        let mut state = LfgServerState::default();

        let update = handle_queue_request(
            player,
            &mut player_state,
            &mut state,
            QueueForLfg {
                role: GroupRoleSnapshot::Healer,
                dungeon_ids: vec![100, 200],
            },
            42,
        );

        assert_eq!(update, SnapshotUpdate::message("queued for dungeon finder"));
        assert_eq!(player_state.selected_role, Some(GroupRole::Healer));
        assert_eq!(player_state.dungeon_ids, vec![100, 200]);
        assert!(player_state.match_found.is_none());
        assert!(state.queue.is_queued(player.to_bits()));
    }

    #[test]
    fn dequeue_request_clears_player_queue_state() {
        let player = Entity::from_bits(9);
        let mut player_state = PlayerLfgState {
            selected_role: Some(GroupRole::Dps),
            dungeon_ids: vec![100],
            role_check: None,
            match_found: Some(PlayerMatchFoundState {
                dungeon_id: 100,
                dungeon_name: "Deadmines".into(),
                assigned_role: GroupRole::Dps,
                members: vec![],
            }),
        };
        let mut state = LfgServerState::default();
        state
            .queue
            .queue_solo(player.to_bits(), GroupRole::Dps, vec![100], 1)
            .expect("queued");

        let update = handle_dequeue_request(player, &mut player_state, &mut state);

        assert_eq!(
            update,
            SnapshotUpdate::message("removed from dungeon finder queue")
        );
        assert_eq!(player_state, PlayerLfgState::default());
        assert!(!state.queue.is_queued(player.to_bits()));
    }
}
