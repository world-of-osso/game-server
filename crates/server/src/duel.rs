use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Health, Player, Position};
use shared::protocol::{
    AcceptDuel, DeclineDuel, DuelBoundarySnapshot, DuelChannel, DuelPhaseSnapshot,
    DuelResultSnapshot, DuelSnapshot, DuelStateUpdate, InitiateDuel,
};

use crate::auth::Authenticated;
use crate::networking::PlayerLink;

const DUEL_BOUNDARY_RADIUS: f32 = 30.0;

/// A duelist's opponent while their duel is in progress (AzerothCore
/// `duel->Opponent` in `DUEL_STATE_IN_PROGRESS`): each is hostile to and a
/// valid attack target of the other (`Unit::GetReactionTo` "duel - always
/// hostile to opponent", `Unit::_IsValidAttackTarget` "check duel").
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuelOpponent(pub Entity);

/// The duelist the opponent's damage took to 1 health: the duel is lost.
#[derive(Component, Debug)]
pub(crate) struct DuelDefeated;

/// `a_opponent` is the `DuelOpponent` of a unit dueling `b`.
pub(crate) fn dueling(a_opponent: Option<&DuelOpponent>, b: Entity) -> bool {
    a_opponent.is_some_and(|opponent| opponent.0 == b)
}

/// Unit::DealDamage "prevent kill only if killed in duel and killed by
/// opponent": the opponent's hit of `damage` that would leave the victim at 1
/// health or less leaves exactly 1 and defeats them. Returns the health loss
/// and whether the hit ends the duel.
pub(crate) fn duel_health_loss(
    victim_opponent: Option<&DuelOpponent>,
    attacker: Entity,
    health: f32,
    damage: f32,
) -> (f32, bool) {
    if dueling(victim_opponent, attacker) && damage > 0.0 && damage >= health - 1.0 {
        ((health - 1.0).max(0.0), true)
    } else {
        (damage, false)
    }
}

#[derive(Resource, Default)]
struct DuelServerState {
    pending: Vec<PendingDuel>,
    active: Vec<ActiveDuel>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PendingDuel {
    challenger: Entity,
    target: Entity,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DuelBoundary {
    center_x: f32,
    center_z: f32,
    radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ActiveDuel {
    challenger: Entity,
    target: Entity,
    boundary: DuelBoundary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DuelResolution {
    /// `defeated`: the loser was beaten (TC `DUEL_WON`), not out of bounds (`DUEL_FLED`).
    WinLoss {
        winner: Entity,
        loser: Entity,
        defeated: bool,
    },
    Cancelled,
}

pub struct DuelPlugin;

impl Plugin for DuelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DuelServerState>();
        app.add_systems(Update, process_duel_challenges);
        app.add_systems(Update, process_duel_accepts);
        app.add_systems(Update, process_duel_declines);
        app.add_systems(FixedUpdate, enforce_duel_outcomes);
    }
}

type PlayerQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static Player, &'static Position, &'static Health)>;
type ConnectionQuery<'w, 's> = Query<'w, 's, (Entity, &'static PlayerLink), With<Authenticated>>;

fn process_duel_challenges(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<InitiateDuel>),
        With<Authenticated>,
    >,
    players: PlayerQuery,
    connections: ConnectionQuery,
    mut state: ResMut<DuelServerState>,
    mut senders: Query<&mut MessageSender<DuelStateUpdate>>,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        let Ok((challenger_entity, challenger, _, _)) = players.get(link.0) else {
            send_duel_error(
                conn_entity,
                &mut senders,
                "duel is unavailable: local player missing",
            );
            continue;
        };
        for request in receiver.receive() {
            handle_duel_challenge_request(
                request,
                DuelChallengeInfo {
                    conn_entity,
                    challenger_entity,
                    challenger,
                },
                &players,
                &connections,
                &mut state,
                &mut senders,
            );
        }
    }
}

struct DuelChallengeInfo<'a> {
    conn_entity: Entity,
    challenger_entity: Entity,
    challenger: &'a Player,
}

fn handle_duel_challenge_request(
    request: InitiateDuel,
    info: DuelChallengeInfo<'_>,
    players: &PlayerQuery,
    connections: &ConnectionQuery,
    state: &mut DuelServerState,
    senders: &mut Query<&mut MessageSender<DuelStateUpdate>>,
) {
    let Some(target_entity) = request.target_entity.map(Entity::from_bits) else {
        send_duel_error(info.conn_entity, senders, "no duel target selected");
        return;
    };
    let Ok((_, target, _, _)) = players.get(target_entity) else {
        send_duel_error(info.conn_entity, senders, "duel target is not a player");
        return;
    };
    if let Err(error) = state.start_pending(info.challenger_entity, target_entity) {
        send_duel_error(info.conn_entity, senders, error);
        return;
    }
    send_pending_challenge_snapshots(
        info.challenger_entity,
        info.challenger,
        target_entity,
        target,
        connections,
        senders,
    );
}

fn send_pending_challenge_snapshots(
    challenger_entity: Entity,
    challenger: &Player,
    target_entity: Entity,
    target: &Player,
    connections: &ConnectionQuery,
    senders: &mut Query<&mut MessageSender<DuelStateUpdate>>,
) {
    send_duel_snapshot_to_player(
        challenger_entity,
        connections,
        senders,
        DuelSnapshot {
            phase: DuelPhaseSnapshot::PendingOutgoing,
            opponent_name: target.name.clone(),
            boundary: None,
            result: None,
        },
        Some(format!("duel request sent to {}", target.name)),
    );
    send_duel_snapshot_to_player(
        target_entity,
        connections,
        senders,
        DuelSnapshot {
            phase: DuelPhaseSnapshot::PendingIncoming,
            opponent_name: challenger.name.clone(),
            boundary: None,
            result: None,
        },
        Some(format!("{} has challenged you to a duel", challenger.name)),
    );
}

fn process_duel_accepts(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<AcceptDuel>),
        With<Authenticated>,
    >,
    players: PlayerQuery,
    connections: ConnectionQuery,
    mut state: ResMut<DuelServerState>,
    mut senders: Query<&mut MessageSender<DuelStateUpdate>>,
    mut commands: Commands,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let Ok(active) = state.accept_pending(link.0, &players) else {
                send_duel_error(conn_entity, &mut senders, "no incoming duel request");
                continue;
            };
            let Ok((_, challenger, _, _)) = players.get(active.challenger) else {
                send_duel_error(conn_entity, &mut senders, "duel challenger is missing");
                continue;
            };
            let Ok((_, target, _, _)) = players.get(active.target) else {
                send_duel_error(conn_entity, &mut senders, "duel target is missing");
                continue;
            };
            commands
                .entity(active.challenger)
                .insert(DuelOpponent(active.target));
            commands
                .entity(active.target)
                .insert(DuelOpponent(active.challenger));
            broadcast_active_duel(active, challenger, target, &connections, &mut senders);
        }
    }
}

fn process_duel_declines(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<DeclineDuel>),
        With<Authenticated>,
    >,
    players: Query<&Player>,
    connections: ConnectionQuery,
    mut state: ResMut<DuelServerState>,
    mut senders: Query<&mut MessageSender<DuelStateUpdate>>,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let Ok((other, was_target)) = state.decline_pending(link.0) else {
                send_duel_error(conn_entity, &mut senders, "no duel request to decline");
                continue;
            };
            let Ok(player) = players.get(link.0) else {
                send_duel_error(conn_entity, &mut senders, "duel player is missing");
                continue;
            };
            let Ok(other_player) = players.get(other) else {
                send_duel_error(conn_entity, &mut senders, "duel opponent is missing");
                continue;
            };
            let (self_message, other_message) = decline_messages(was_target, &player.name);
            send_decline_snapshots(
                DuelDeclineInfo {
                    entity: link.0,
                    opponent_name: &other_player.name,
                    message: self_message.to_owned(),
                },
                DuelDeclineInfo {
                    entity: other,
                    opponent_name: &player.name,
                    message: other_message,
                },
                &connections,
                &mut senders,
            );
        }
    }
}

fn decline_messages(was_target: bool, player_name: &str) -> (&'static str, String) {
    if was_target {
        (
            "duel declined",
            format!("{player_name} declined your duel request"),
        )
    } else {
        (
            "duel request cancelled",
            format!("{player_name} cancelled the duel request"),
        )
    }
}

struct DuelDeclineInfo<'a> {
    entity: Entity,
    opponent_name: &'a str,
    message: String,
}

fn send_decline_snapshots(
    self_info: DuelDeclineInfo<'_>,
    other_info: DuelDeclineInfo<'_>,
    connections: &ConnectionQuery,
    senders: &mut Query<&mut MessageSender<DuelStateUpdate>>,
) {
    send_duel_snapshot_to_player(
        self_info.entity,
        connections,
        senders,
        DuelSnapshot {
            phase: DuelPhaseSnapshot::Completed,
            opponent_name: self_info.opponent_name.into(),
            boundary: None,
            result: Some(DuelResultSnapshot::Declined),
        },
        Some(self_info.message),
    );
    send_duel_snapshot_to_player(
        other_info.entity,
        connections,
        senders,
        DuelSnapshot {
            phase: DuelPhaseSnapshot::Completed,
            opponent_name: other_info.opponent_name.into(),
            boundary: None,
            result: Some(DuelResultSnapshot::Declined),
        },
        Some(other_info.message),
    );
}

fn enforce_duel_outcomes(
    players: PlayerQuery,
    defeated: Query<(), With<DuelDefeated>>,
    connections: ConnectionQuery,
    mut state: ResMut<DuelServerState>,
    mut senders: Query<&mut MessageSender<DuelStateUpdate>>,
    mut commands: Commands,
) {
    let mut resolutions = Vec::new();
    for duel in state.active.iter().copied() {
        if let Some(resolution) = resolve_active_duel(duel, &players, &defeated) {
            resolutions.push((duel, resolution));
        }
    }
    for (duel, resolution) in resolutions {
        state.remove_active(duel);
        commands.queue(move |world: &mut World| end_duel(world, duel));
        if let DuelResolution::WinLoss {
            winner,
            loser,
            defeated: true,
        } = resolution
        {
            // TC Player::DuelComplete(DUEL_WON), Player.cpp:7763-7765.
            commands.queue(move |world: &mut World| {
                use crate::achievement::{CriteriaEvent, record_player_event};
                record_player_event(world, loser, CriteriaEvent::LoseDuel);
                record_player_event(world, winner, CriteriaEvent::WinDuel);
            });
        }
        send_duel_resolution(duel, resolution, &players, &connections, &mut senders);
    }
}

/// Player::DuelComplete: the duelists are no longer hostile. Both stop
/// attacking and leave combat (Unit::DealDamage `CombatStopWithPets` on the
/// duel-winning hit; `AttackStop` both when a same-team duelist flees).
fn end_duel(world: &mut World, duel: ActiveDuel) {
    for duelist in [duel.challenger, duel.target] {
        if let Ok(mut entity) = world.get_entity_mut(duelist) {
            entity.remove::<(DuelOpponent, DuelDefeated)>();
            crate::combat::combat_stop(world, duelist);
        }
    }
}

fn broadcast_active_duel(
    duel: ActiveDuel,
    challenger: &Player,
    target: &Player,
    connections: &ConnectionQuery,
    senders: &mut Query<&mut MessageSender<DuelStateUpdate>>,
) {
    let boundary = Some(map_boundary(duel.boundary));
    send_duel_snapshot_to_player(
        duel.challenger,
        connections,
        senders,
        DuelSnapshot {
            phase: DuelPhaseSnapshot::Active,
            opponent_name: target.name.clone(),
            boundary: boundary.clone(),
            result: None,
        },
        Some("duel started".into()),
    );
    send_duel_snapshot_to_player(
        duel.target,
        connections,
        senders,
        DuelSnapshot {
            phase: DuelPhaseSnapshot::Active,
            opponent_name: challenger.name.clone(),
            boundary,
            result: None,
        },
        Some("duel started".into()),
    );
}

fn send_duel_resolution(
    duel: ActiveDuel,
    resolution: DuelResolution,
    players: &PlayerQuery,
    connections: &ConnectionQuery,
    senders: &mut Query<&mut MessageSender<DuelStateUpdate>>,
) {
    let Ok((_, challenger, _, _)) = players.get(duel.challenger) else {
        return;
    };
    let Ok((_, target, _, _)) = players.get(duel.target) else {
        return;
    };
    match resolution {
        DuelResolution::WinLoss { winner, loser, .. } => send_win_loss_resolution(
            DuelResolutionInfo {
                duel,
                winner,
                loser,
                challenger,
                target,
            },
            players,
            connections,
            senders,
        ),
        DuelResolution::Cancelled => {
            send_cancelled_resolution(duel, challenger, target, connections, senders)
        }
    }
}

struct DuelResolutionInfo<'a> {
    duel: ActiveDuel,
    winner: Entity,
    loser: Entity,
    challenger: &'a Player,
    target: &'a Player,
}

fn send_win_loss_resolution(
    info: DuelResolutionInfo<'_>,
    players: &PlayerQuery,
    connections: &ConnectionQuery,
    senders: &mut Query<&mut MessageSender<DuelStateUpdate>>,
) {
    let winner_name = player_name(players, info.winner);
    let loser_name = player_name(players, info.loser);
    let result_message = format!("{winner_name} won the duel against {loser_name}");
    send_duel_result_to_player(
        info.duel.challenger,
        duel_result_for_player(info.duel.challenger, info.winner),
        info.target.name.clone(),
        result_message.clone(),
        connections,
        senders,
    );
    send_duel_result_to_player(
        info.duel.target,
        duel_result_for_player(info.duel.target, info.winner),
        info.challenger.name.clone(),
        result_message,
        connections,
        senders,
    );
}

fn send_cancelled_resolution(
    duel: ActiveDuel,
    challenger: &Player,
    target: &Player,
    connections: &ConnectionQuery,
    senders: &mut Query<&mut MessageSender<DuelStateUpdate>>,
) {
    send_duel_result_to_player(
        duel.challenger,
        DuelResultSnapshot::Cancelled,
        target.name.clone(),
        "duel cancelled".into(),
        connections,
        senders,
    );
    send_duel_result_to_player(
        duel.target,
        DuelResultSnapshot::Cancelled,
        challenger.name.clone(),
        "duel cancelled".into(),
        connections,
        senders,
    );
}

fn player_name(players: &PlayerQuery, entity: Entity) -> String {
    players
        .get(entity)
        .map(|(_, player, _, _)| player.name.clone())
        .unwrap_or_else(|_| "Unknown".into())
}

fn duel_result_for_player(player_entity: Entity, winner: Entity) -> DuelResultSnapshot {
    if player_entity == winner {
        DuelResultSnapshot::Won
    } else {
        DuelResultSnapshot::Lost
    }
}

fn send_duel_result_to_player(
    player_entity: Entity,
    result: DuelResultSnapshot,
    opponent_name: String,
    message: String,
    connections: &ConnectionQuery,
    senders: &mut Query<&mut MessageSender<DuelStateUpdate>>,
) {
    send_duel_snapshot_to_player(
        player_entity,
        connections,
        senders,
        DuelSnapshot {
            phase: DuelPhaseSnapshot::Completed,
            opponent_name,
            boundary: None,
            result: Some(result),
        },
        Some(message),
    );
}

fn send_duel_snapshot_to_player(
    player_entity: Entity,
    connections: &ConnectionQuery,
    senders: &mut Query<&mut MessageSender<DuelStateUpdate>>,
    snapshot: DuelSnapshot,
    message: Option<String>,
) {
    let Some(conn_entity) = connection_for_player(connections, player_entity) else {
        return;
    };
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<DuelChannel>(DuelStateUpdate {
        snapshot: Some(snapshot),
        message,
        error: None,
    });
}

fn send_duel_error(
    conn_entity: Entity,
    senders: &mut Query<&mut MessageSender<DuelStateUpdate>>,
    error: impl Into<String>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<DuelChannel>(DuelStateUpdate {
        snapshot: None,
        message: None,
        error: Some(error.into()),
    });
}

fn connection_for_player(connections: &ConnectionQuery, player_entity: Entity) -> Option<Entity> {
    connections
        .iter()
        .find(|(_, link)| link.0 == player_entity)
        .map(|(conn_entity, _)| conn_entity)
}

impl DuelServerState {
    fn start_pending(&mut self, challenger: Entity, target: Entity) -> Result<(), &'static str> {
        if challenger == target {
            return Err("cannot duel yourself");
        }
        if self.is_busy(challenger) || self.is_busy(target) {
            return Err("one of the players is already in a duel");
        }
        self.pending.push(PendingDuel { challenger, target });
        Ok(())
    }

    fn accept_pending(
        &mut self,
        target: Entity,
        players: &PlayerQuery,
    ) -> Result<ActiveDuel, &'static str> {
        let Some(index) = self
            .pending
            .iter()
            .position(|pending| pending.target == target)
        else {
            return Err("no incoming duel request");
        };
        let pending = self.pending.remove(index);
        let Ok((_, _, challenger_pos, _)) = players.get(pending.challenger) else {
            return Err("duel challenger is missing");
        };
        let Ok((_, _, target_pos, _)) = players.get(pending.target) else {
            return Err("duel target is missing");
        };
        let active = ActiveDuel {
            challenger: pending.challenger,
            target: pending.target,
            boundary: build_duel_boundary(challenger_pos, target_pos),
        };
        self.active.push(active);
        Ok(active)
    }

    fn decline_pending(&mut self, player: Entity) -> Result<(Entity, bool), &'static str> {
        if let Some(index) = self
            .pending
            .iter()
            .position(|pending| pending.target == player)
        {
            let pending = self.pending.remove(index);
            return Ok((pending.challenger, true));
        }
        if let Some(index) = self
            .pending
            .iter()
            .position(|pending| pending.challenger == player)
        {
            let pending = self.pending.remove(index);
            return Ok((pending.target, false));
        }
        Err("no duel request to decline")
    }

    fn remove_active(&mut self, duel: ActiveDuel) {
        self.active.retain(|entry| *entry != duel);
    }

    fn is_busy(&self, player: Entity) -> bool {
        self.pending
            .iter()
            .any(|pending| pending.challenger == player || pending.target == player)
            || self
                .active
                .iter()
                .any(|active| active.challenger == player || active.target == player)
    }
}

fn build_duel_boundary(challenger: &Position, target: &Position) -> DuelBoundary {
    DuelBoundary {
        center_x: (challenger.x + target.x) * 0.5,
        center_z: (challenger.z + target.z) * 0.5,
        radius: DUEL_BOUNDARY_RADIUS,
    }
}

fn resolve_active_duel(
    duel: ActiveDuel,
    players: &PlayerQuery,
    defeated: &Query<(), With<DuelDefeated>>,
) -> Option<DuelResolution> {
    let Ok((_, _, challenger_pos, challenger_health)) = players.get(duel.challenger) else {
        return Some(DuelResolution::Cancelled);
    };
    let Ok((_, _, target_pos, target_health)) = players.get(duel.target) else {
        return Some(DuelResolution::Cancelled);
    };
    if defeated.contains(duel.challenger) {
        return Some(DuelResolution::WinLoss {
            winner: duel.target,
            loser: duel.challenger,
            defeated: true,
        });
    }
    if defeated.contains(duel.target) {
        return Some(DuelResolution::WinLoss {
            winner: duel.challenger,
            loser: duel.target,
            defeated: true,
        });
    }
    resolve_duel_state(
        challenger_pos,
        target_pos,
        challenger_health.current,
        target_health.current,
        duel.boundary,
        duel.challenger,
        duel.target,
    )
}

fn resolve_duel_state(
    challenger_pos: &Position,
    target_pos: &Position,
    challenger_health: f32,
    target_health: f32,
    boundary: DuelBoundary,
    challenger: Entity,
    target: Entity,
) -> Option<DuelResolution> {
    if challenger_health <= 0.0 && target_health <= 0.0 {
        return Some(DuelResolution::Cancelled);
    }
    if challenger_health <= 0.0 {
        return Some(DuelResolution::WinLoss {
            winner: target,
            loser: challenger,
            defeated: true,
        });
    }
    if target_health <= 0.0 {
        return Some(DuelResolution::WinLoss {
            winner: challenger,
            loser: target,
            defeated: true,
        });
    }

    let challenger_out = is_out_of_bounds(challenger_pos, boundary);
    let target_out = is_out_of_bounds(target_pos, boundary);
    if challenger_out && target_out {
        return Some(DuelResolution::Cancelled);
    }
    if challenger_out {
        return Some(DuelResolution::WinLoss {
            winner: target,
            loser: challenger,
            defeated: false,
        });
    }
    if target_out {
        return Some(DuelResolution::WinLoss {
            winner: challenger,
            loser: target,
            defeated: false,
        });
    }
    None
}

fn is_out_of_bounds(position: &Position, boundary: DuelBoundary) -> bool {
    let dx = position.x - boundary.center_x;
    let dz = position.z - boundary.center_z;
    let distance_sq = dx * dx + dz * dz;
    distance_sq > boundary.radius * boundary.radius
}

fn map_boundary(boundary: DuelBoundary) -> DuelBoundarySnapshot {
    DuelBoundarySnapshot {
        center_x: boundary.center_x,
        center_z: boundary.center_z,
        radius: boundary.radius,
    }
}

#[cfg(test)]
#[path = "duel_tests.rs"]
mod tests;
