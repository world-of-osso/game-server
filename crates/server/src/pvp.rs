use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use shared::battleground::{BgInstance, BgQueue, template_by_id};
use shared::components::Level;
use shared::currency::{CONQUEST_CURRENCY_ID, CurrencyWallet, HONOR_CURRENCY_ID};
use shared::protocol::{
    DequeueFromPvp, PvpBracketSnapshot, PvpStateUpdate, QueryPvpStatus, QueueForBattleground,
};
use std::collections::{BTreeMap, BTreeSet};

use self::pvp_support::{
    connection_for_character, current_seconds, default_brackets, format_bg_queue_error,
    queue_label, send_pvp_error, send_pvp_update,
};
use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;

#[path = "pvp_support.rs"]
mod pvp_support;
#[path = "pvp_talents.rs"]
pub(crate) mod pvp_talents;
#[path = "pvp_war_mode.rs"]
mod pvp_war_mode;

#[path = "pvp_arena.rs"]
pub(crate) mod arena;
#[path = "pvp_arena_cage.rs"]
pub(crate) mod arena_cage;
#[path = "pvp_arena_combat.rs"]
pub(crate) mod arena_combat;
#[path = "pvp_arena_dalaran.rs"]
pub(crate) mod arena_dalaran;
#[path = "pvp_arena_lifecycle.rs"]
mod arena_lifecycle;
#[path = "pvp_arena_objects.rs"]
pub(crate) mod arena_objects;
#[path = "pvp_arena_preparation.rs"]
mod arena_preparation;
#[path = "pvp_arena_rated_queue.rs"]
mod arena_rated_queue;
#[path = "pvp_arena_rated_results.rs"]
mod arena_rated_results;
#[path = "pvp_arena_scores.rs"]
mod arena_scores;
#[path = "pvp_arena_shuffle.rs"]
mod arena_shuffle;
pub(crate) use arena_scores::record as record_arena_event;
#[path = "pvp_arena_rating.rs"]
mod arena_rating;
#[path = "pvp_arena_sight.rs"]
mod arena_sight;

#[path = "pvp_kill_criteria.rs"]
mod kill_criteria;
pub(crate) use kill_criteria::record_battleground_player_kill;
#[path = "pvp_honor.rs"]
mod honor;
pub(crate) use honor::reward_world_player_kill;

#[path = "pvp_isle.rs"]
pub(crate) mod isle;
#[path = "pvp_isle_siege.rs"]
pub(crate) mod isle_siege;
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
#[path = "pvp_reconnect.rs"]
pub(crate) mod reconnect;
use pvp_leave::process_battleground_leave_requests;
#[path = "pvp_objective.rs"]
mod pvp_objective;
pub(crate) use pvp_objective::use_battleground_object;

const HONOR_CAP: u32 = 15_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerPvpBracketState {
    pub bracket: PvpBracketSnapshot,
    pub rating: Option<u32>,
    #[serde(default)]
    pub matchmaker_rating: Option<u32>,
    pub season_wins: u32,
    pub season_losses: u32,
    pub weekly_wins: u32,
    pub weekly_losses: u32,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerPvpProgress {
    /// Lifetime XP is not spendable Honor1792; HONOR_LEVEL_XP_CURVE is unresolved.
    #[serde(default)]
    pub lifetime_honor_xp: u64,
    #[serde(default)]
    pub honor_level: Option<u32>,
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
            lifetime_honor_xp: 0,
            honor_level: None,
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
    Battleground {
        battleground_id: u32,
        name: String,
    },
    RatedBracket {
        bracket: PvpBracketSnapshot,
    },
    Arena {
        mode: shared::protocol::ArenaQueueMode,
    },
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

/// TC Player::GetBattleground: current accepted copy, not queued preference.
fn read_player_battleground(world: &World, player: Entity) -> Option<&InvitedBattleground> {
    let character = world.get::<crate::persistence::ClientId>(player)?.0;
    let map = *world.get::<crate::maps::InMap>(player)?;
    let snapshot = world.get::<PlayerPvpState>(player)?.battleground.as_ref()?;
    let matched = world
        .get_resource::<PvpServerState>()?
        .battleground_matches
        .get(&snapshot.match_id)?;
    if matched.map != Some(map) || !matched.accepted.contains(&character) {
        return None;
    }
    Some(matched)
}

pub(crate) fn player_arena_conditions(world: &World, player: Entity) -> Option<(u8, bool)> {
    let arena = read_player_battleground(world, player)?.arena.as_ref()?;
    Some((arena::team_size(arena.mode)?, arena.rated.is_some()))
}

/// Source occupied-base world states, read from the live node ledger before win credit.
pub(crate) fn player_controlled_node_counts(world: &World, player: Entity) -> Option<[u8; 2]> {
    let matched = read_player_battleground(world, player)?;
    let states = matched.nodes.as_ref()?.world_states(&matched.instance);
    let count = |state| {
        let (_, value) = states.iter().find(|(id, _)| *id == state)?;
        u8::try_from(*value).ok()
    };
    Some([count(1779)?, count(1778)?])
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

#[cfg(test)]
pub(crate) fn forget_matches_for_reconnect_fixture(world: &mut World) {
    world.insert_resource(PvpServerState::default());
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
    alterac: Option<pvp_alterac::BannerRuntime>,
    scoreboard: Scoreboard,
    /// Last carried position of each team's flag; Eye of the Storm's neutral flag uses [0].
    flag_positions: [Option<Vec3>; 2],
    arena: Option<arena::MatchState>,
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
        if self.alterac.is_some() {
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
pub(crate) struct PvpServerState {
    offline_players: BTreeMap<u64, reconnect::OfflinePlayer>,
    battleground_queue: BgQueue,
    rated_team_queues: BTreeMap<shared::protocol::ArenaQueueMode, Vec<arena_rating::QueuedTeam>>,
    arena_queues: BTreeMap<shared::protocol::ArenaQueueMode, Vec<(u64, u8)>>,
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
            arena: None,
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

pub struct PvpPlugin;

impl Plugin for PvpPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PvpServerState>();
        app.init_resource::<arena::Catalog>();
        app.init_resource::<arena_objects::GateCatalog>();
        app.init_resource::<arena_objects::SightCatalog>();
        app.init_resource::<arena_objects::SightTimers>();
        app.init_resource::<arena_dalaran::Catalog>();
        app.init_resource::<arena_dalaran::Timers>();
        app.init_resource::<arena_cage::Catalog>();
        app.init_resource::<arena_cage::Timers>();
        app.add_systems(
            Update,
            arena::receive_queue_requests.before(process_battleground_invitation_responses),
        );
        app.add_systems(FixedUpdate, pvp_objective::update_battleground_objectives);
        app.add_systems(
            FixedUpdate,
            crate::transport::advance_ships
                .before(crate::game_objects::collision::sync_live_objects)
                .before(crate::networking::process_player_inputs),
        );
        app.add_systems(
            FixedUpdate,
            crate::transport::store_passenger_offsets
                .after(crate::networking::process_player_inputs)
                .after(crate::networking::apply_terrain_gravity),
        );
        app.add_systems(
            FixedUpdate,
            arena::update_matches.after(crate::death::process_player_death_transitions),
        );
        app.add_systems(Update, send_initial_pvp_snapshot);
        app.add_systems(Update, process_pvp_queries);
        app.add_systems(Update, process_battleground_queue_requests);
        // Retiring a copy must follow Update's corpse-loot commands and
        // PostUpdate's target-mirror commands, including their deferred inserts.
        app.add_systems(
            PostUpdate,
            process_battleground_invitation_responses.after(crate::unit_target::UnitTargetSync),
        );
        app.add_systems(
            PostUpdate,
            process_battleground_leave_requests.after(process_battleground_invitation_responses),
        );
        app.add_systems(
            PostUpdate,
            reconnect::expire_offline_players.after(process_battleground_leave_requests),
        );
        app.add_systems(
            Update,
            arena_rated_queue::receive_requests.before(process_battleground_invitation_responses),
        );
        app.add_systems(FixedUpdate, arena_rated_queue::match_queued_teams);
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
            for queue in state.rated_team_queues.values_mut() {
                queue.retain(|team| !team.members.contains(&character_id));
            }
        }
        PlayerPvpQueue::Arena { .. } => {
            for queue in state.arena_queues.values_mut() {
                queue.retain(|(id, _)| *id != character_id);
            }
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
                arena: None,
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
