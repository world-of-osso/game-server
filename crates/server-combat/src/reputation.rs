use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Level, Npc, Position};
use shared::protocol::{
    ReputationChannel, ReputationEntrySnapshot, ReputationSnapshot, ReputationStateUpdate,
};
use shared::reputation::{
    CharacterReputation, FactionRegistry, RepChangeResult, RepGainSource, default_faction_registry,
    diminished_mob_rep,
};

use crate::auth::{Authenticated, InWorld};
use crate::combat::DeathState;
use crate::creature_tap::{CreatureTap, RewardPlayers, rewarded_characters};
use crate::group::GroupRegistry;
use crate::networking::PlayerLink;
use crate::persistence::{self, ClientId, GameDatabase};
use crate::world_data::WorldData;
use shared::protocol_snapshots::WatchedReputationSnapshot;
use shared::reputation::{Standing, tier_progress};

const BASE_MOB_KILL_REP: i32 = 10;

#[derive(Resource, Clone)]
struct ReputationRegistry(FactionRegistry);

pub struct ReputationPlugin;

impl Plugin for ReputationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ReputationRegistry(default_faction_registry()));
        app.add_systems(Update, send_initial_reputation_snapshot);
        app.add_systems(PostUpdate, process_npc_kill_reputation);
    }
}

type NewInWorldLink<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static PlayerLink, &'static InWorld),
    (With<Authenticated>, Added<InWorld>),
>;

fn send_initial_reputation_snapshot(
    connections: NewInWorldLink,
    reputations: Query<&CharacterReputation>,
    registry: Res<ReputationRegistry>,
    db: Res<GameDatabase>,
    mut senders: Query<&mut MessageSender<ReputationStateUpdate>>,
) {
    for (conn_entity, link, owner) in &connections {
        let Ok(reputation) = reputations.get(link.0) else {
            send_reputation_error(
                conn_entity,
                &mut senders,
                "reputation unavailable for player",
            );
            continue;
        };
        send_reputation_update(
            conn_entity,
            read_reputation_snapshot(&db.db, owner.character_id, reputation, &registry.0),
            Some("reputation loaded".into()),
            None,
            &mut senders,
        );
    }
}

/// `KillRewarder::_RewardReputation` for every rewarded player of a dead creature
/// (`creature_tap`).
fn process_npc_kill_reputation(
    killed_npcs: Query<(&Npc, &CreatureTap, &Position, &crate::maps::InMap), Added<DeathState>>,
    reward_players: RewardPlayers,
    groups: Option<Res<GroupRegistry>>,
    mut players: Query<(Entity, &ClientId, &Level, &mut CharacterReputation)>,
    connections: Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    world_data: Res<WorldData>,
    registry: Res<ReputationRegistry>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<ReputationStateUpdate>>,
) {
    let Some(db) = db else { return };
    let rewarded = killed_npcs.iter().flat_map(|(npc, tap, position, map)| {
        rewarded_characters(
            &tap.tappers,
            (map, position),
            &reward_players,
            groups.as_deref(),
        )
        .into_iter()
        .map(|character| (npc.template_id, character))
    });
    for (template_id, character) in rewarded.collect::<Vec<_>>() {
        let Some((player, _, level, mut reputation)) =
            players.iter_mut().find(|(_, id, ..)| id.0 == character)
        else {
            continue;
        };
        let Some(snapshot_message) = apply_npc_kill_reputation(
            &mut reputation,
            level.0,
            template_id,
            &world_data,
            &registry.0,
        ) else {
            continue;
        };
        let Some((conn_entity, character_id)) = connection_for_player(&connections, player) else {
            continue;
        };
        persistence::save_reputation(&db.db, character_id, &reputation);
        send_reputation_update(
            conn_entity,
            read_reputation_snapshot(&db.db, character_id, &reputation, &registry.0),
            Some(snapshot_message),
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

fn apply_npc_kill_reputation(
    reputation: &mut CharacterReputation,
    player_level: u8,
    template_id: u32,
    world_data: &WorldData,
    registry: &FactionRegistry,
) -> Option<String> {
    let template = world_data.templates.get(&template_id)?;
    let faction_id = u32::from(template.faction);
    registry.get(faction_id)?;
    let mob_level = template.max_level.max(template.min_level);
    let amount = diminished_mob_rep(BASE_MOB_KILL_REP, player_level, mob_level);
    if amount <= 0 {
        return None;
    }
    let changes =
        reputation.gain_rep_with_spillover(faction_id, amount, RepGainSource::MobKill, registry);
    format_change_message(&changes, registry)
}

/// `CHAT_MSG_COMBAT_FACTION_CHANGE` lines, one per changed faction (FACTION_STANDING_INCREASED
/// / FACTION_STANDING_DECREASED); `None` when no value moved.
pub(crate) fn format_change_message(
    changes: &[RepChangeResult],
    registry: &FactionRegistry,
) -> Option<String> {
    let lines: Vec<String> = changes
        .iter()
        .filter(|change| change.actual_change() != 0)
        .map(|change| {
            let name = faction_name(registry, change.faction_id);
            match change.actual_change() {
                amount if amount > 0 => format!("Reputation with {name} increased by {amount}."),
                amount => format!("Reputation with {name} decreased by {}.", -amount),
            }
        })
        .collect();
    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn faction_name(registry: &FactionRegistry, faction_id: u32) -> &str {
    registry
        .get(faction_id)
        .map(|faction| faction.name.as_str())
        .unwrap_or("Unknown")
}

fn send_reputation_update(
    conn_entity: Entity,
    snapshot: ReputationSnapshot,
    message: Option<String>,
    error: Option<String>,
    senders: &mut Query<&mut MessageSender<ReputationStateUpdate>>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<ReputationChannel>(ReputationStateUpdate {
        snapshot: Some(snapshot),
        message,
        error,
    });
}

fn send_reputation_error(
    conn_entity: Entity,
    senders: &mut Query<&mut MessageSender<ReputationStateUpdate>>,
    error: impl Into<String>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<ReputationChannel>(ReputationStateUpdate {
        snapshot: None,
        message: None,
        error: Some(error.into()),
    });
}

/// Admin `set-reputation`: `faction_id` at `value`, saved, and sent to the live player.
pub(crate) fn set_reputation(world: &mut World, character: u64, faction_id: u32, value: i32) {
    let db = world.resource::<GameDatabase>().db.clone();
    let registry = default_faction_registry();
    let live = world
        .query::<(Entity, &ClientId, &CharacterReputation)>()
        .iter(world)
        .find(|(_, id, _)| id.0 == character)
        .map(|(player, _, reputation)| (player, reputation.clone()));
    let mut reputation = match &live {
        Some((_, reputation)) => reputation.clone(),
        None => {
            let Some(data) = persistence::load_character(&db, character) else {
                warn!("set-reputation: character {character} has no saved data");
                return;
            };
            persistence::load_reputation(&db, character, || {
                CharacterReputation::new_for_player(&registry, data.race, data.class)
            })
        }
    };
    reputation.set_rep(faction_id, value);
    persistence::save_reputation(&db, character, &reputation);
    let Some((player, _)) = live else {
        return;
    };
    let snapshot = read_reputation_snapshot(&db, character, &reputation, &registry);
    world.entity_mut(player).insert(reputation);
    let connection = world
        .query::<(Entity, &PlayerLink)>()
        .iter(world)
        .find(|(_, link)| link.0 == player)
        .map(|(connection, _)| connection);
    if let Some(mut sender) =
        connection.and_then(|entity| world.get_mut::<MessageSender<ReputationStateUpdate>>(entity))
    {
        sender.send::<ReputationChannel>(ReputationStateUpdate {
            snapshot: Some(snapshot),
            message: None,
            error: None,
        });
    }
}

pub(crate) fn read_reputation_snapshot(
    db: &redb::Database,
    character: u64,
    reputation: &CharacterReputation,
    registry: &FactionRegistry,
) -> ReputationSnapshot {
    let mut entries = reputation
        .all_factions()
        .into_iter()
        .filter_map(|faction_id| {
            let faction = registry.get(faction_id)?;
            Some(ReputationEntrySnapshot {
                faction_id,
                faction_name: faction.name.clone(),
                standing: format!("{:?}", reputation.get_standing(faction_id)),
                value: reputation.get_value(faction_id),
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| a.faction_name.cmp(&b.faction_name));
    ReputationSnapshot {
        entries,
        watched: read_watched_reputation(db, character, reputation, registry),
    }
}

/// Small owner-only watched state: no reward catalog or fabricated condition evaluation.
fn read_watched_reputation(
    db: &redb::Database,
    character: u64,
    reputation: &CharacterReputation,
    registry: &FactionRegistry,
) -> Option<WatchedReputationSnapshot> {
    let faction_id = reputation.watched_faction()?;
    if let Some(renown) = crate::renown::read_watched_renown(db, character, faction_id) {
        return Some(renown);
    }
    let faction = registry.get(faction_id)?;
    let reaction = reputation.get_standing(faction_id);
    let capped = reaction == Standing::Exalted;
    let (value, maximum) = if capped {
        (1, 1)
    } else {
        tier_progress(reputation.get_value(faction_id))
    };
    Some(WatchedReputationSnapshot {
        faction_id,
        name: faction.name.clone(),
        value: value as u32,
        maximum: maximum as u32,
        capped,
        is_major_faction: false,
        reaction,
    })
}

#[cfg(test)]
#[path = "reputation_tests.rs"]
mod tests;
