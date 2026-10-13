use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use shared::battleground::{BgInstance, BgQueue, template_by_id};
use shared::components::Level;
use shared::currency::{CONQUEST_CURRENCY_ID, CurrencyWallet, HONOR_CURRENCY_ID};
use shared::protocol::{
    DequeueFromPvp, PvpBracketSnapshot, PvpStateUpdate, QueryPvpStatus, QueueForBattleground,
    QueueForRatedPvp,
};
use std::collections::{BTreeMap, BTreeSet};

use self::pvp_support::{
    bracket_label, connection_for_character, current_seconds, default_brackets,
    format_bg_queue_error, queue_label, send_pvp_error, send_pvp_update,
};
use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;

#[path = "pvp_support.rs"]
mod pvp_support;
#[path = "pvp_talents.rs"]
pub(crate) mod pvp_talents;
#[path = "pvp_war_mode.rs"]
mod pvp_war_mode;

#[path = "pvp_alterac.rs"]
mod pvp_alterac;
pub(crate) use pvp_alterac::is_banner as is_alterac_banner;
pub(crate) use pvp_alterac::{
    check_banner_target, complete_opening, record_creature_death, record_player_death,
};
#[path = "pvp_deephaul.rs"]
mod pvp_deephaul;
#[path = "pvp_entry.rs"]
mod pvp_entry;
#[path = "pvp_invites.rs"]
mod pvp_invites;
#[path = "pvp_objects.rs"]
mod pvp_objects;
#[path = "pvp_seething.rs"]
mod pvp_seething;
#[path = "pvp_silvershard.rs"]
mod pvp_silvershard;
use pvp_invites::process_battleground_invitation_responses;
#[cfg(test)]
pub(crate) use pvp_objects::BattlegroundObjectTemplates;
pub(crate) use pvp_objects::load_battleground_objects;
pub(crate) use pvp_seething::{capture_azerite, check_azerite_target};
pub(crate) use pvp_silvershard::{change_track, insert_track_switch};

#[path = "pvp_leave.rs"]
mod pvp_leave;
use pvp_leave::process_battleground_leave_requests;
#[path = "pvp_objective.rs"]
mod pvp_objective;
pub(crate) use pvp_objective::use_battleground_object;

const HONOR_CAP: u32 = 15_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerPvpBracketState {
    pub bracket: PvpBracketSnapshot,
    pub rating: Option<u32>,
    pub season_wins: u32,
    pub season_losses: u32,
    pub weekly_wins: u32,
    pub weekly_losses: u32,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerPvpProgress {
    pub honor: u32,
    pub honor_max: u32,
    pub conquest: u32,
    pub conquest_max: Option<u32>,
    pub brackets: Vec<PlayerPvpBracketState>,
    #[serde(default)]
    pub war_mode_desired: bool,
    #[serde(default)]
    pub pvp_talents: BTreeMap<u32, BTreeMap<u8, u32>>,
}

impl Default for PlayerPvpProgress {
    fn default() -> Self {
        Self {
            honor: 0,
            honor_max: HONOR_CAP,
            conquest: 0,
            conquest_max: None,
            brackets: default_brackets(),
            war_mode_desired: false,
            pvp_talents: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerPvpQueue {
    Battleground { battleground_id: u32, name: String },
    RatedBracket { bracket: PvpBracketSnapshot },
}

#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerPvpState {
    pub progress: PlayerPvpProgress,
    pub queue: Option<PlayerPvpQueue>,
    pub battleground: Option<shared::protocol::BattlegroundSnapshot>,
    pub battleground_original_faction: Option<u32>,
    pub war_mode: Option<shared::protocol::WarModeSnapshot>,
    pub talents: Option<shared::protocol::PvpTalentsSnapshot>,
}

impl PlayerPvpState {
    pub fn from_progress(progress: PlayerPvpProgress) -> Self {
        Self {
            progress,
            queue: None,
            battleground: None,
            battleground_original_faction: None,
            war_mode: None,
            talents: None,
        }
    }
}

pub(crate) fn sync_progress_currencies_from_wallet(
    progress: &mut PlayerPvpProgress,
    wallet: &CurrencyWallet,
) -> bool {
    let honor = wallet.get(HONOR_CURRENCY_ID);
    let conquest = wallet.get(CONQUEST_CURRENCY_ID);
    let changed = progress.honor != honor || progress.conquest != conquest;
    progress.honor = honor;
    progress.conquest = conquest;
    changed
}

#[derive(Debug)]
struct InvitedBattleground {
    instance: BgInstance,
    accepted: BTreeSet<u64>,
    map: Option<crate::maps::InMap>,
    origins: BTreeMap<u64, crate::maps::WorldLocation>,
    flags: Option<shared::battleground::FlagBattle>,
    nodes: Option<shared::battleground::NodeBattle>,
    eye: Option<shared::battleground::EyeBattle>,
    kotmogu: Option<shared::battleground::KotmoguBattle>,
    silvershard: Option<pvp_silvershard::CartRuntime>,
    seething: Option<pvp_seething::SeethingRuntime>,
    deephaul: Option<pvp_deephaul::DeephaulRuntime>,
    alterac: Option<pvp_alterac::AlteracRuntime>,
    scoreboard: Scoreboard,
    /// Last carried position of each team's flag; Eye of the Storm's neutral flag uses [0].
    flag_positions: [Option<Vec3>; 2],
}

/// Displayed rows plus TC Battleground::UpdatePvpStat receipts awaiting criteria credit.
#[derive(Debug, Default)]
struct Scoreboard {
    rows: Vec<shared::protocol::BattlegroundPlayerScoreSnapshot>,
    /// (character, PvpStat) once per objective credit, drained to achievements.
    stat_credits: Vec<(u64, u32)>,
}

impl InvitedBattleground {
    fn displayed_scores(&self) -> (u32, u32) {
        if self.instance.bg_id == 1 {
            (
                self.instance.score_a.reinforcements,
                self.instance.score_b.reinforcements,
            )
        } else {
            (self.instance.score_a.score, self.instance.score_b.score)
        }
    }

    fn flag_objective(&self) -> shared::battleground::FlagObjective {
        self.flags
            .as_ref()
            .map(|battle| battle.flags.clone())
            .unwrap_or_default()
    }

    fn world_states(&self) -> Vec<(u32, i32)> {
        if let Some(alterac) = &self.alterac {
            return alterac.battle.world_states(&self.instance);
        }
        if let Some(nodes) = &self.nodes {
            return nodes.world_states(&self.instance);
        }
        if let Some(eye) = &self.eye {
            return eye.world_states(&self.instance);
        }
        if let Some(carts) = &self.silvershard {
            return carts.battle.world_states(&self.instance);
        }
        if let Some(deephaul) = &self.deephaul {
            return deephaul.battle.world_states(&self.instance);
        }
        if let Some(azerite) = &self.seething {
            return azerite.battle.world_states(&self.instance);
        }
        if let Some(orbs) = &self.kotmogu {
            return orbs.world_states(&self.instance);
        }
        self.flags
            .as_ref()
            .map(|battle| battle.world_states(&self.instance))
            .unwrap_or_default()
    }
}

#[derive(Resource, Default)]
struct PvpServerState {
    battleground_queue: BgQueue,
    rated_queues: Vec<u64>,
    battleground_matches: BTreeMap<u64, InvitedBattleground>,
    next_battleground_match: u64,
}

fn form_battleground_invitation(state: &mut PvpServerState, battleground_id: u32) -> Option<u64> {
    let template = template_by_id(battleground_id)?;
    let matched = state.battleground_queue.try_match(battleground_id)?;
    state.next_battleground_match += 1;
    let match_id = state.next_battleground_match;
    state.battleground_matches.insert(
        match_id,
        InvitedBattleground {
            instance: BgInstance::from_match(&matched, &template),
            accepted: BTreeSet::new(),
            map: None,
            origins: BTreeMap::new(),
            flags: shared::battleground::flag_rules(template.map_id)
                .map(shared::battleground::FlagBattle::new),
            nodes: shared::battleground::node_rules(template.map_id)
                .map(shared::battleground::NodeBattle::new),
            eye: shared::battleground::eye_rules(template.map_id)
                .map(shared::battleground::EyeBattle::new),
            kotmogu: (template.map_id == 998).then(shared::battleground::KotmoguBattle::new),
            silvershard: None,
            seething: None,
            deephaul: None,
            alterac: None,
            scoreboard: Scoreboard::default(),
            flag_positions: [None, None],
        },
    );
    Some(match_id)
}

fn respond_to_battleground_invitation(
    state: &mut PvpServerState,
    match_id: u64,
    character_id: u64,
    accepted: bool,
) -> Result<(), String> {
    let invitation = state
        .battleground_matches
        .get_mut(&match_id)
        .ok_or("battleground invitation is no longer available")?;
    let instance = &mut invitation.instance;
    let is_member = instance.team_a_players.contains(&character_id)
        || instance.team_b_players.contains(&character_id);
    if !is_member || instance.phase != shared::battleground::BgPhase::Waiting {
        return Err("player has no invitation to this battleground".into());
    }
    if invitation.accepted.contains(&character_id) {
        return Err("battleground invitation already accepted".into());
    }
    if accepted {
        invitation.accepted.insert(character_id);
    } else {
        instance.team_a_players.retain(|id| *id != character_id);
        instance.team_b_players.retain(|id| *id != character_id);
    }
    if instance.team_a_players.is_empty() && instance.team_b_players.is_empty() {
        state.battleground_matches.remove(&match_id);
    }
    Ok(())
}

#[derive(bevy::ecs::system::SystemParam)]
struct RatedQueueContext<'w, 's> {
    state: ResMut<'w, PvpServerState>,
    senders: Query<'w, 's, &'static mut MessageSender<PvpStateUpdate>>,
}

pub struct PvpPlugin;

impl Plugin for PvpPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PvpServerState>();
        app.add_systems(FixedUpdate, pvp_objective::update_battleground_objectives);
        app.add_systems(Update, send_initial_pvp_snapshot);
        app.add_systems(Update, process_pvp_queries);
        app.add_systems(Update, process_battleground_queue_requests);
        // Accept/decline and Leave may retire a map. Update's corpse-loot
        // commands must finish before either can destroy that map's creatures.
        app.add_systems(PostUpdate, process_battleground_invitation_responses);
        app.add_systems(
            PostUpdate,
            process_battleground_leave_requests.after(process_battleground_invitation_responses),
        );
        app.add_systems(Update, process_rated_queue_requests);
        app.add_systems(Update, process_pvp_dequeue_requests);
        app.add_systems(Update, pvp_talents::receive_requests);
        app.add_systems(PostUpdate, pvp_talents::sync_players);
        app.add_systems(
            Update,
            (pvp_war_mode::receive_requests, pvp_war_mode::sync_state).chain(),
        );
    }
}

type NewInWorldLink<'w, 's> =
    Query<'w, 's, (Entity, &'static PlayerLink), (With<Authenticated>, Added<InWorld>)>;

fn send_initial_pvp_snapshot(
    connections: NewInWorldLink,
    players: Query<&PlayerPvpState>,
    mut senders: Query<&mut MessageSender<PvpStateUpdate>>,
) {
    for (conn_entity, link) in &connections {
        let Ok(state) = players.get(link.0) else {
            send_pvp_error(conn_entity, &mut senders, "pvp unavailable for player");
            continue;
        };
        send_pvp_update(
            conn_entity,
            state,
            Some("pvp state loaded".into()),
            None,
            &mut senders,
        );
    }
}

fn process_pvp_queries(
    mut receivers: Query<
        (Entity, &PlayerLink, &mut MessageReceiver<QueryPvpStatus>),
        With<Authenticated>,
    >,
    players: Query<&PlayerPvpState>,
    mut senders: Query<&mut MessageSender<PvpStateUpdate>>,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        let requests: Vec<_> = receiver.receive().collect();
        if requests.is_empty() {
            continue;
        }
        let Ok(state) = players.get(link.0) else {
            send_pvp_error(conn_entity, &mut senders, "pvp unavailable for player");
            continue;
        };
        send_pvp_update(
            conn_entity,
            state,
            Some("pvp status refreshed".into()),
            None,
            &mut senders,
        );
    }
}

fn process_battleground_queue_requests(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &InWorld,
            &mut MessageReceiver<QueueForBattleground>,
        ),
        With<Authenticated>,
    >,
    mut players: Query<(&Level, &shared::components::Player, &mut PlayerPvpState)>,
    connections: Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    state: ResMut<PvpServerState>,
    time: Res<Time>,
    mut senders: Query<&mut MessageSender<PvpStateUpdate>>,
) {
    let state = state.into_inner();
    let now = current_seconds(&time);
    for (conn_entity, link, in_world, mut receiver) in &mut receivers {
        for request in receiver.receive() {
            let result = {
                let Ok((level, _player, mut player_state)) = players.get_mut(link.0) else {
                    send_pvp_error(conn_entity, &mut senders, "pvp unavailable for player");
                    continue;
                };
                queue_for_battleground(
                    &mut player_state,
                    state,
                    in_world.character_id,
                    level.0,
                    request.battleground_id,
                    now,
                )
            };

            match result {
                Ok(message) => {
                    if let Ok((_level, _player, player_state)) = players.get(link.0) {
                        send_pvp_update(
                            conn_entity,
                            player_state,
                            Some(message),
                            None,
                            &mut senders,
                        );
                    }
                    resolve_battleground_matches(
                        request.battleground_id,
                        state,
                        &mut players,
                        &connections,
                        &mut senders,
                    );
                }
                Err(error) => {
                    if let Ok((_level, _player, player_state)) = players.get(link.0) {
                        send_pvp_update(conn_entity, player_state, None, Some(error), &mut senders);
                    } else {
                        send_pvp_error(conn_entity, &mut senders, "pvp unavailable for player");
                    }
                }
            }
        }
    }
}

fn process_rated_queue_requests(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &InWorld,
            &mut MessageReceiver<QueueForRatedPvp>,
        ),
        With<Authenticated>,
    >,
    mut players: Query<(
        &shared::components::Player,
        &mut PlayerPvpState,
        &mut crate::currency::PlayerCurrencies,
    )>,
    mut context: RatedQueueContext,
) {
    let state = context.state.into_inner();
    for (conn_entity, link, in_world, mut receiver) in &mut receivers {
        for request in receiver.receive() {
            let result = {
                let Ok((_player, mut player_state, _currencies)) = players.get_mut(link.0) else {
                    send_pvp_error(
                        conn_entity,
                        &mut context.senders,
                        "pvp unavailable for player",
                    );
                    continue;
                };
                queue_for_rated_bracket(
                    &mut player_state,
                    state,
                    in_world.character_id,
                    request.bracket.clone(),
                )
            };

            match result {
                Ok(message) => {
                    if let Ok((_player, player_state, _currencies)) = players.get(link.0) {
                        send_pvp_update(
                            conn_entity,
                            player_state,
                            Some(message),
                            None,
                            &mut context.senders,
                        );
                    }
                }
                Err(error) => {
                    if let Ok((_player, player_state, _currencies)) = players.get(link.0) {
                        send_pvp_update(
                            conn_entity,
                            player_state,
                            None,
                            Some(error),
                            &mut context.senders,
                        );
                    } else {
                        send_pvp_error(
                            conn_entity,
                            &mut context.senders,
                            "pvp unavailable for player",
                        );
                    }
                }
            }
        }
    }
}

fn process_pvp_dequeue_requests(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &InWorld,
            &mut MessageReceiver<DequeueFromPvp>,
        ),
        With<Authenticated>,
    >,
    mut players: Query<&mut PlayerPvpState>,
    state: ResMut<PvpServerState>,
    mut senders: Query<&mut MessageSender<PvpStateUpdate>>,
) {
    let state = state.into_inner();
    for (conn_entity, link, in_world, mut receiver) in &mut receivers {
        for _request in receiver.receive() {
            let Ok(mut player_state) = players.get_mut(link.0) else {
                send_pvp_error(conn_entity, &mut senders, "pvp unavailable for player");
                continue;
            };
            match dequeue_player(&mut player_state, state, in_world.character_id) {
                Ok(message) => send_pvp_update(
                    conn_entity,
                    &player_state,
                    Some(message),
                    None,
                    &mut senders,
                ),
                Err(error) => {
                    send_pvp_update(conn_entity, &player_state, None, Some(error), &mut senders)
                }
            }
        }
    }
}

fn queue_for_battleground(
    player_state: &mut PlayerPvpState,
    state: &mut PvpServerState,
    character_id: u64,
    level: u8,
    battleground_id: u32,
    now: u64,
) -> Result<String, String> {
    if player_state.battleground.is_some() {
        return Err("respond to the current battleground invitation first".into());
    }
    if let Some(queue) = &player_state.queue {
        return Err(format!("already queued for {}", queue_label(queue)));
    }
    let Some(template) = template_by_id(battleground_id) else {
        return Err(format!("unknown battleground id {}", battleground_id));
    };
    state
        .battleground_queue
        .join_solo(character_id, level, battleground_id, now)
        .map_err(format_bg_queue_error)?;
    player_state.queue = Some(PlayerPvpQueue::Battleground {
        battleground_id,
        name: template.name.clone(),
    });
    Ok(format!("queued for {}", template.name))
}

fn queue_for_rated_bracket(
    player_state: &mut PlayerPvpState,
    state: &mut PvpServerState,
    character_id: u64,
    bracket: PvpBracketSnapshot,
) -> Result<String, String> {
    if player_state.battleground.is_some() {
        return Err("respond to the current battleground invitation first".into());
    }
    if let Some(queue) = &player_state.queue {
        return Err(format!("already queued for {}", queue_label(queue)));
    }
    if state
        .rated_queues
        .iter()
        .any(|entry| *entry == character_id)
    {
        return Err("already queued for rated pvp".into());
    }
    player_state.queue = Some(PlayerPvpQueue::RatedBracket {
        bracket: bracket.clone(),
    });
    state.rated_queues.push(character_id);
    Ok(format!(
        "queued for {}; match lifecycle and authored reward/rating policy unavailable",
        bracket_label(&bracket)
    ))
}

fn dequeue_player(
    player_state: &mut PlayerPvpState,
    state: &mut PvpServerState,
    character_id: u64,
) -> Result<String, String> {
    let Some(queue) = player_state.queue.take() else {
        return Err("not queued for pvp".into());
    };
    match &queue {
        PlayerPvpQueue::Battleground { .. } => {
            state.battleground_queue.leave(character_id);
        }
        PlayerPvpQueue::RatedBracket { .. } => {
            state.rated_queues.retain(|entry| *entry != character_id);
        }
    }
    Ok(format!("left {}", queue_label(&queue)))
}

fn resolve_battleground_matches(
    battleground_id: u32,
    state: &mut PvpServerState,
    players: &mut Query<(&Level, &shared::components::Player, &mut PlayerPvpState)>,
    connections: &Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    senders: &mut Query<&mut MessageSender<PvpStateUpdate>>,
) {
    while let Some(match_id) = form_battleground_invitation(state, battleground_id) {
        let invitation = &state.battleground_matches[&match_id];
        let template = template_by_id(invitation.instance.bg_id).expect("matched template exists");
        let teams = invitation
            .instance
            .team_a_players
            .iter()
            .map(|id| (*id, shared::battleground::BgTeam::A))
            .chain(
                invitation
                    .instance
                    .team_b_players
                    .iter()
                    .map(|id| (*id, shared::battleground::BgTeam::B)),
            );
        for (character_id, team) in teams {
            let Some((conn_entity, player_entity)) =
                connection_for_character(connections, character_id)
            else {
                continue;
            };
            let Ok((_level, _player, mut player_state)) = players.get_mut(player_entity) else {
                continue;
            };
            player_state.queue = None;
            player_state.battleground = Some(shared::protocol::BattlegroundSnapshot {
                match_id,
                battleground_id: template.id,
                name: template.name.clone(),
                map_id: template.map_id,
                team,
                phase: invitation.instance.phase,
                accepted: false,
                invitation_time_left_secs: None,
                score_a: invitation.displayed_scores().0,
                score_b: invitation.displayed_scores().1,
                flags: invitation.flag_objective(),
                world_states: invitation.world_states(),
                outcome: invitation.instance.outcome,
                players: invitation.scoreboard.rows.clone(),
                honor_reward: None,
            });
            send_pvp_update(
                conn_entity,
                &player_state,
                Some(format!("battleground ready: {}", template.name)),
                None,
                senders,
            );
        }
    }
}

#[cfg(test)]
#[path = "pvp_tests.rs"]
mod tests;
