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

mod actor;
use actor::{PendingReputationRewards, ReputationReward, publish_reputation_rewards};

const BASE_MOB_KILL_REP: i32 = 10;

#[derive(Resource, Clone)]
struct ReputationRegistry(FactionRegistry);

pub struct ReputationPlugin;

impl Plugin for ReputationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ReputationRegistry(default_faction_registry()));
        app.add_systems(Update, send_initial_reputation_snapshot);
        app.init_resource::<PendingReputationRewards>();
        app.add_systems(
            PostUpdate,
            (process_npc_kill_reputation, publish_reputation_rewards).chain(),
        );
    }
}

type NewInWorldLink<'w, 's> =
    Query<'w, 's, (Entity, &'static PlayerLink), (With<Authenticated>, Added<InWorld>)>;

fn send_initial_reputation_snapshot(
    connections: NewInWorldLink,
    reputations: Query<&CharacterReputation>,
    registry: Res<ReputationRegistry>,
    mut senders: Query<&mut MessageSender<ReputationStateUpdate>>,
) {
    for (conn_entity, link) in &connections {
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
            reputation,
            &registry.0,
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
    players: Query<(Entity, &ClientId, &Level, &CharacterReputation)>,
    connections: Query<(Entity, &PlayerLink, &InWorld), With<Authenticated>>,
    world_data: Res<WorldData>,
    registry: Res<ReputationRegistry>,
    db: Option<Res<GameDatabase>>,
    mut pending: ResMut<PendingReputationRewards>,
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
        let Some((player, _, level, reputation)) =
            players.iter().find(|(_, id, ..)| id.0 == character)
        else {
            continue;
        };
        let Some((faction, amount)) =
            npc_kill_reward(level.0, template_id, &world_data, &registry.0)
        else {
            continue;
        };
        let Some((_, character_id)) = connection_for_player(&connections, player) else {
            continue;
        };
        let reward = ReputationReward {
            initial: reputation.clone(),
            faction,
            amount,
            registry: registry.0.clone(),
        };
        pending.enqueue(&db, player, character_id, reward);
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

fn npc_kill_reward(
    player_level: u8,
    template_id: u32,
    world_data: &WorldData,
    registry: &FactionRegistry,
) -> Option<(u32, i32)> {
    let template = world_data.templates.get(&template_id)?;
    let faction_id = u32::from(template.faction);
    registry.get(faction_id)?;
    let mob_level = template.max_level.max(template.min_level);
    let amount = diminished_mob_rep(BASE_MOB_KILL_REP, player_level, mob_level);
    if amount <= 0 {
        return None;
    }
    Some((faction_id, amount))
}

#[cfg(test)]
fn apply_npc_kill_reputation(
    reputation: &mut CharacterReputation,
    player_level: u8,
    template_id: u32,
    world_data: &WorldData,
    registry: &FactionRegistry,
) -> Option<String> {
    let (faction_id, amount) = npc_kill_reward(player_level, template_id, world_data, registry)?;
    let changes =
        reputation.gain_rep_with_spillover(faction_id, amount, RepGainSource::MobKill, registry);
    if changes.is_empty() {
        return None;
    }
    Some(format_change_message(&changes, registry))
}

fn format_change_message(changes: &[RepChangeResult], registry: &FactionRegistry) -> String {
    let mut message = primary_change_message(changes[0], registry);
    append_spillover_message(&mut message, spillover_changes(changes, registry));
    message
}

fn primary_change_message(change: RepChangeResult, registry: &FactionRegistry) -> String {
    let direction = rep_direction(change.actual_change());
    let faction_name = faction_name(registry, change.faction_id);
    format!(
        "{} {} reputation with {}",
        direction,
        change.actual_change().abs(),
        faction_name
    )
}

fn append_spillover_message(message: &mut String, spillovers: Vec<String>) {
    if spillovers.is_empty() {
        return;
    }
    message.push_str(" (spillover ");
    message.push_str(&spillovers.join(", "));
    message.push(')');
}

fn spillover_changes(changes: &[RepChangeResult], registry: &FactionRegistry) -> Vec<String> {
    changes
        .iter()
        .skip(1)
        .filter(|change| change.actual_change() != 0)
        .map(|change| {
            format!(
                "{:+} {}",
                change.actual_change(),
                faction_name(registry, change.faction_id)
            )
        })
        .collect()
}

fn faction_name(registry: &FactionRegistry, faction_id: u32) -> &str {
    registry
        .get(faction_id)
        .map(|faction| faction.name.as_str())
        .unwrap_or("Unknown")
}

fn rep_direction(change: i32) -> &'static str {
    if change >= 0 { "gained" } else { "lost" }
}

fn send_reputation_update(
    conn_entity: Entity,
    reputation: &CharacterReputation,
    registry: &FactionRegistry,
    message: Option<String>,
    error: Option<String>,
    senders: &mut Query<&mut MessageSender<ReputationStateUpdate>>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<ReputationChannel>(ReputationStateUpdate {
        snapshot: Some(build_reputation_snapshot(reputation, registry)),
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

pub(crate) fn build_reputation_snapshot(
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
    ReputationSnapshot { entries }
}

#[cfg(test)]
#[path = "reputation_tests.rs"]
mod tests;
