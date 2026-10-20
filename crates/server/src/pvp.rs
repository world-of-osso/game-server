use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use shared::battleground::{BgInstance, BgPhase, BgQueue, template_by_id};
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
use crate::auras::ActiveAuras;
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

#[path = "pvp_bg_backfill.rs"]
mod bg_backfill;
#[path = "pvp_leave.rs"]
mod pvp_leave;

#[path = "pvp_random_bg.rs"]
pub(crate) mod random_bg;

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
    /// LIFETIME_HK_TRACKING_STARTS_AT_ZERO: no historical backfill or XP/type35 alias.
    #[serde(default)]
    pub lifetime_honorable_kills: u64,
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
            lifetime_honorable_kills: 0,
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

/// TC Battleground.cpp1038 AddPlayer runs only after successful WorldPortAck.
pub(crate) fn dismount_battleground_arrival(world: &mut World, player: Entity) {
    let ordinary_battleground =
        read_player_battleground(world, player).is_some_and(|matched| matched.arena.is_none());
    if !ordinary_battleground {
        return;
    }
    remove_battleground_mount_auras(world, player);
}

/// TC Battleground.cpp834/1038: normal aura removal also clears mount display/speed.
fn remove_battleground_mount_auras(world: &mut World, player: Entity) {
    crate::auras::remove_auras(
        world,
        player,
        crate::auras::RemoveReason::Cancelled,
        |aura| {
            aura.effects
                .iter()
                .any(|effect| effect.aura_type == crate::auras::handlers::MOUNTED)
        },
    );
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

/// Controlled pre-terminal baseline; subsequent channel fixtures must produce the actual end.
#[cfg(test)]
pub(crate) fn seed_terminal_reinforcements(
    world: &mut World,
    match_id: u64,
    team: shared::battleground::BgTeam,
    amount: u32,
) {
    let mut state = world.resource_mut::<PvpServerState>();
    let matched = state.battleground_matches.get_mut(&match_id).unwrap();
    match team {
        shared::battleground::BgTeam::A => matched.instance.score_a.reinforcements = amount,
        shared::battleground::BgTeam::B => matched.instance.score_b.reinforcements = amount,
    }
}

#[derive(Debug)]
struct InvitedBattleground {
    instance: BgInstance,
    accepted: BTreeSet<u64>,
    map: Option<crate::maps::InMap>,
    origins: BTreeMap<u64, crate::maps::WorldLocation>,
    /// Original requested queue per player; never overwritten by the selected concrete map.
    original_queues: BTreeMap<u64, u32>,
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
    battleground_refill_pending: bool,
}

fn form_battleground_invitation(
    state: &mut PvpServerState,
    battleground_id: u32,
    random_catalog: &random_bg::Catalog,
) -> Option<u64> {
    let matched = if battleground_id == shared::battleground::RANDOM_BATTLEGROUND_ID {
        random_bg::draw_match(&mut state.battleground_queue, random_catalog)?
    } else {
        state.battleground_queue.try_match(battleground_id)?
    };
    let template = template_by_id(matched.bg_id)?;
    let original_queues = matched
        .team_a
        .iter()
        .chain(&matched.team_b)
        .map(|&character| (character, battleground_id))
        .collect();
    state.next_battleground_match += 1;
    let match_id = state.next_battleground_match;
    state.battleground_matches.insert(
        match_id,
        InvitedBattleground {
            instance: BgInstance::from_match(&matched, &template),
            accepted: BTreeSet::new(),
            map: None,
            origins: BTreeMap::new(),
            original_queues,
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
    let accepting = match instance.phase {
        BgPhase::Waiting => true,
        BgPhase::InProgress => invitation.arena.is_none(),
        BgPhase::Ended => false,
    };
    if !is_member || !accepting {
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
        invitation.original_queues.remove(&character_id);
        state.battleground_refill_pending |= invitation.arena.is_none();
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
        app.init_resource::<crate::maps::InstanceManager>();
        app.init_resource::<random_bg::Catalog>();
        app.init_resource::<crate::group::GroupRegistry>();
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
            PostUpdate,
            refill_battleground_queues.after(reconnect::expire_offline_players),
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
    mut players: Query<(
        &Level,
        &shared::components::Player,
        &mut PlayerPvpState,
        Option<&ActiveAuras>,
    )>,
    connections: Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    state: ResMut<PvpServerState>,
    groups: Res<crate::group::GroupRegistry>,
    random_catalog: Res<random_bg::Catalog>,
    lfg: Option<Res<crate::lfg::LfgServerState>>,
    time: Res<Time>,
    mut senders: Query<&mut MessageSender<PvpStateUpdate>>,
) {
    let state = state.into_inner();
    let lfg = lfg.as_deref();
    let now = current_seconds(&time);
    for (conn_entity, link, in_world, mut receiver) in &mut receivers {
        for request in receiver.receive() {
            let result = if groups.leader_of(in_world.character_id).is_some() {
                queue_battleground_party(
                    &groups,
                    &mut players,
                    &connections,
                    state,
                    in_world.character_id,
                    request.battleground_id,
                    now,
                    &random_catalog,
                    lfg,
                )
            } else {
                let Ok((level, _player, mut player_state, auras)) = players.get_mut(link.0) else {
                    send_pvp_error(conn_entity, &mut senders, "pvp unavailable for player");
                    continue;
                };
                let eligibility = validate_random_queue_levels(
                    &random_catalog,
                    request.battleground_id,
                    &[level.0],
                );
                eligibility
                    .and_then(|()| validate_no_active_lfg(lfg, link.0))
                    .and_then(|()| validate_no_deserter(auras))
                    .and_then(|()| {
                        queue_for_battleground(
                            &mut player_state,
                            state,
                            in_world.character_id,
                            level.0,
                            request.battleground_id,
                            now,
                        )
                    })
                    .map(|message| (vec![(conn_entity, link.0)], message))
            };

            match result {
                Ok((members, message)) => {
                    for (connection, player) in members {
                        if let Ok((_level, _player, player_state, _)) = players.get(player) {
                            send_pvp_update(
                                connection,
                                player_state,
                                Some(message.clone()),
                                None,
                                &mut senders,
                            );
                        }
                    }
                    state.battleground_refill_pending = true;
                }
                Err(error) => {
                    if let Ok((_level, _player, player_state, _)) = players.get(link.0) {
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

/// TC Handler130–136 / Group1250–1253; matched-dungeon/session state is deferred.
fn validate_no_active_lfg(
    lfg: Option<&crate::lfg::LfgServerState>,
    player: Entity,
) -> Result<(), String> {
    if lfg.is_some_and(|state| state.is_queued_or_checking_roles(player)) {
        return Err(
            "cannot join battleground while dungeon finder queue or role check is active".into(),
        );
    }
    Ok(())
}

/// TC a352b1fa Player.h2594; Handler147–153 and Group1243–1245.
fn validate_no_deserter(auras: Option<&ActiveAuras>) -> Result<(), String> {
    const DESERTER_SPELL_ID: u32 = 26013;
    if auras.is_some_and(|auras| auras.has_spell(DESERTER_SPELL_ID)) {
        return Err(format_bg_queue_error(
            shared::battleground::BgQueueError::Deserter,
        ));
    }
    Ok(())
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
    let name = shared::battleground::queue_name(battleground_id)
        .ok_or_else(|| format!("unknown battleground id {battleground_id}"))?;
    state
        .battleground_queue
        .join_solo(character_id, level, battleground_id, now)
        .map_err(format_bg_queue_error)?;
    player_state.queue = Some(PlayerPvpQueue::Battleground {
        battleground_id,
        name: name.clone(),
    });
    Ok(format!("queued for {name}"))
}

/// TC BattleGroundHandler.cpp189–226: only the leader queues the whole group.
fn queue_battleground_party(
    groups: &crate::group::GroupRegistry,
    players: &mut Query<(
        &Level,
        &shared::components::Player,
        &mut PlayerPvpState,
        Option<&ActiveAuras>,
    )>,
    connections: &Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    state: &mut PvpServerState,
    leader: u64,
    battleground_id: u32,
    now: u64,
    random_catalog: &random_bg::Catalog,
    lfg: Option<&crate::lfg::LfgServerState>,
) -> Result<(Vec<(Entity, Entity)>, String), String> {
    if groups.leader_of(leader) != Some(leader) {
        return Err("only the group leader can queue the party".into());
    }
    let name = shared::battleground::queue_name(battleground_id)
        .ok_or_else(|| format!("unknown battleground id {battleground_id}"))?;
    let max_group = shared::battleground::queue_group_limit(battleground_id)
        .expect("validated battleground queue");
    let members = groups.group_members(leader);
    if members.len() > usize::from(max_group) {
        return Err("group exceeds the battleground team size".into());
    }
    let mut online = Vec::with_capacity(members.len());
    let mut levels = Vec::with_capacity(members.len());
    for &character in &members {
        let (connection, player) = connection_for_character(connections, character)
            .ok_or_else(|| format!("group member {character} is not in world"))?;
        let (level, _, player_state, auras) = players
            .get(player)
            .map_err(|_| format!("pvp unavailable for group member {character}"))?;
        if player_state.battleground.is_some() || player_state.queue.is_some() {
            return Err(format!(
                "group member {character} is already queued or invited to PvP"
            ));
        }
        validate_no_deserter(auras)?;
        validate_no_active_lfg(lfg, player)?;
        levels.push(level.0);
        online.push((connection, player));
    }
    validate_random_queue_levels(random_catalog, battleground_id, &levels)?;
    state
        .battleground_queue
        .join_group(leader, members, levels, battleground_id, now)
        .map_err(format_bg_queue_error)?;
    for &(_, player) in &online {
        let (_, _, mut player_state, _) = players
            .get_mut(player)
            .expect("validated online party member");
        player_state.queue = Some(PlayerPvpQueue::Battleground {
            battleground_id,
            name: name.clone(),
        });
    }
    Ok((online, format!("queued for {name}")))
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

fn validate_random_queue_levels(
    catalog: &random_bg::Catalog,
    queue_id: u32,
    levels: &[u8],
) -> Result<(), String> {
    if queue_id == shared::battleground::RANDOM_BATTLEGROUND_ID {
        catalog.validate_levels(levels)?;
    }
    Ok(())
}

fn refill_battleground_queues(
    mut state: ResMut<PvpServerState>,
    catalog: Res<random_bg::Catalog>,
    mut instances: ResMut<crate::maps::InstanceManager>,
    mut players: Query<(&Level, &shared::components::Player, &mut PlayerPvpState)>,
    connections: Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    mut senders: Query<&mut MessageSender<PvpStateUpdate>>,
) {
    if !std::mem::take(&mut state.battleground_refill_pending) {
        return;
    }
    for queue_id in state.battleground_queue.queued_ids() {
        resolve_battleground_matches(
            queue_id,
            &mut state,
            &catalog,
            &mut instances,
            &mut players,
            &connections,
            &mut senders,
        );
    }
}

fn resolve_battleground_matches(
    queue_id: u32,
    state: &mut PvpServerState,
    catalog: &random_bg::Catalog,
    instances: &mut crate::maps::InstanceManager,
    players: &mut Query<(&Level, &shared::components::Player, &mut PlayerPvpState)>,
    connections: &Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    senders: &mut Query<&mut MessageSender<PvpStateUpdate>>,
) {
    loop {
        match bg_backfill::fill_existing_copy(state, instances, catalog, queue_id) {
            Ok(Some(batch)) => {
                send_battleground_invitations(state, batch, players, connections, senders);
                continue;
            }
            Ok(None) => {}
            Err(message) => {
                error!("{message}");
                return;
            }
        }
        let Some(batch) = form_battleground_invitation_batch(state, queue_id, catalog) else {
            return;
        };
        send_battleground_invitations(state, batch, players, connections, senders);
    }
}

fn form_battleground_invitation_batch(
    state: &mut PvpServerState,
    queue_id: u32,
    catalog: &random_bg::Catalog,
) -> Option<bg_backfill::Invitations> {
    let match_id = form_battleground_invitation(state, queue_id, catalog)?;
    let instance = &state.battleground_matches[&match_id].instance;
    let characters = instance
        .team_a_players
        .iter()
        .chain(&instance.team_b_players)
        .copied()
        .collect();
    Some(bg_backfill::Invitations {
        match_id,
        characters,
    })
}

fn send_battleground_invitations(
    state: &PvpServerState,
    batch: bg_backfill::Invitations,
    players: &mut Query<(&Level, &shared::components::Player, &mut PlayerPvpState)>,
    connections: &Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    senders: &mut Query<&mut MessageSender<PvpStateUpdate>>,
) {
    let invitation = &state.battleground_matches[&batch.match_id];
    for character in batch.characters {
        let Some((connection, player)) = connection_for_character(connections, character) else {
            continue;
        };
        let Ok((_, _, mut player_state)) = players.get_mut(player) else {
            continue;
        };
        let snapshot =
            reconnect::build_match_snapshot(invitation, batch.match_id, character, false);
        let message = format!("battleground ready: {}", snapshot.name);
        player_state.queue = None;
        player_state.battleground = Some(snapshot);
        send_pvp_update(connection, &player_state, Some(message), None, senders);
    }
}

#[cfg(test)]
#[path = "pvp_tests.rs"]
mod tests;
