//! Realm-backed Journeys renown. TrinityCore a352b1fa ReputationMgr semantics: renown
//! level is the faction's renown currency, capped at its MaxQty; reputation past each
//! Faction.ReputationMax threshold rolls into the next level. No in-world source grants
//! renown yet; the admin socket is the only writer.
#[cfg(test)]
#[path = "renown_tests.rs"]
mod tests;
use crate::{
    auth::{Authenticated, InWorld},
    networking::PlayerLink,
    persistence::GameDatabase,
};
use bevy::prelude::*;
use lightyear::prelude::*;
use redb::{Database, ReadableDatabase, TableDefinition};
use serde::{Deserialize, Serialize};
use shared::protocol::{
    InstanceChannel, MajorFactionSnapshot, RenownRequest, RenownReward, RenownSnapshot,
    RenownUnlock,
};
use std::collections::BTreeMap;
use std::sync::OnceLock;

const PROGRESS: TableDefinition<u64, &[u8]> = TableDefinition::new("renown_progress_v1");

#[derive(Deserialize)]
pub(crate) struct MajorFaction {
    pub id: u32,
    faction_id: u32,
    name: String,
    description: String,
    expansion: u8,
    max_level: u32,
    level_threshold: u32,
    player_condition_id: u32,
    unlock_description: Option<String>,
    rewards: Vec<RenownReward>,
}

#[derive(Deserialize)]
struct Catalog {
    major_factions: Vec<MajorFaction>,
}

/// Pinned Retail 12.1.0.69933 export (game-engine `scripts/export_renown_catalog.py`).
pub(crate) fn catalog() -> &'static [MajorFaction] {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    &CATALOG
        .get_or_init(|| {
            serde_json::from_str(include_str!(
                "../../../scripts/reference/retail-12.1.0.69933/renown.json"
            ))
            .expect("decode pinned Retail renown catalog")
        })
        .major_factions
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Progress {
    pub level: u32,
    pub standing: u32,
}

/// `ReputationMgr::SetOneFactionReputation` renown branch for a positive gain.
pub(crate) fn gain(progress: Progress, faction: &MajorFaction, amount: u32) -> Progress {
    if progress.level >= faction.max_level {
        return progress;
    }
    let threshold = u64::from(faction.level_threshold);
    let cap = u64::from(faction.max_level) * threshold;
    let total =
        (u64::from(progress.level) * threshold + u64::from(progress.standing) + u64::from(amount))
            .min(cap);
    Progress {
        level: (total / threshold) as u32,
        standing: (total % threshold) as u32,
    }
}

fn read_progress(db: &Database, character: u64) -> BTreeMap<u32, Progress> {
    let txn = db.begin_read().expect("begin renown read");
    let Ok(table) = txn.open_table(PROGRESS) else {
        return BTreeMap::new();
    };
    table
        .get(character)
        .expect("read renown progress")
        .map(|bytes| serde_json::from_slice(bytes.value()).expect("decode renown progress"))
        .unwrap_or_default()
}

fn write_progress(db: &Database, character: u64, progress: &BTreeMap<u32, Progress>) {
    let txn = db.begin_write().expect("begin renown write");
    {
        let mut table = txn.open_table(PROGRESS).expect("open renown progress");
        let bytes = serde_json::to_vec(progress).expect("encode renown progress");
        table
            .insert(character, bytes.as_slice())
            .expect("write renown progress");
    }
    txn.commit().expect("commit renown progress");
}

/// Admin-socket test setup: add `amount` renown reputation for `character_name`.
pub(crate) fn admin_grant(
    db: &Database,
    character_name: &str,
    major_faction: u32,
    amount: u32,
) -> Result<Progress, String> {
    let character = crate::persistence_character::find_character_id_by_name(db, character_name)
        .ok_or_else(|| format!("Character {character_name} not found"))?;
    let faction = catalog()
        .iter()
        .find(|faction| faction.id == major_faction)
        .ok_or_else(|| format!("Major faction {major_faction} is not a sourced renown faction"))?;
    let mut progress = read_progress(db, character);
    let updated = gain(
        progress.get(&major_faction).copied().unwrap_or_default(),
        faction,
        amount,
    );
    progress.insert(major_faction, updated);
    write_progress(db, character, &progress);
    Ok(updated)
}

fn unlock(faction: &MajorFaction) -> RenownUnlock {
    match faction.player_condition_id {
        0 => RenownUnlock::Ungated,
        condition_id => RenownUnlock::Unsupported {
            condition_id,
            description: faction.unlock_description.clone().unwrap_or_default(),
        },
    }
}

pub(crate) fn snapshot(db: &Database, character: u64) -> RenownSnapshot {
    let progress = read_progress(db, character);
    let factions = catalog()
        .iter()
        .map(|faction| {
            let current = progress.get(&faction.id).copied().unwrap_or_default();
            MajorFactionSnapshot {
                id: faction.id,
                faction_id: faction.faction_id,
                name: faction.name.clone(),
                description: faction.description.clone(),
                expansion: faction.expansion,
                max_level: faction.max_level,
                level_threshold: faction.level_threshold,
                level: current.level,
                standing: current.standing,
                unlock: unlock(faction),
                rewards: faction.rewards.clone(),
            }
        })
        .collect();
    RenownSnapshot {
        factions,
        source_error: None,
        watched_faction: None,
        watch_error: None,
    }
}

/// ReputationBar.lua's major-faction branch, independent of reward-catalog delivery.
pub(crate) fn read_watched_renown(
    db: &Database,
    character: u64,
    faction_id: u32,
) -> Option<shared::protocol_snapshots::WatchedReputationSnapshot> {
    let faction = catalog()
        .iter()
        .find(|faction| faction.faction_id == faction_id)?;
    let current = read_progress(db, character)
        .get(&faction.id)
        .copied()
        .unwrap_or_default();
    Some(shared::protocol_snapshots::WatchedReputationSnapshot {
        faction_id,
        name: faction.name.clone(),
        value: current.standing,
        maximum: faction.level_threshold,
        capped: current.level >= faction.max_level,
        is_major_faction: true,
        reaction: shared::reputation::Standing::Neutral,
    })
}

pub(crate) struct RenownPlugin;
impl Plugin for RenownPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, receive_requests);
    }
}

/// Retail watch setter. Unknown unlock status never establishes eligibility.
fn set_watched_faction(
    reputation: &mut shared::reputation::CharacterReputation,
    faction_id: u32,
    eligibility: Option<&RenownUnlock>,
) -> Result<(), String> {
    if faction_id == 0 {
        reputation.unwatch();
        return Ok(());
    }
    match eligibility {
        Some(RenownUnlock::Ungated) => reputation.watch(faction_id),
        Some(RenownUnlock::Unsupported { .. }) => {
            return Err("Faction unlock condition is not evaluated by this realm.".into());
        }
        None => return Err(format!("Unknown major faction reputation {faction_id}")),
    }
    Ok(())
}

fn reply_to_request(
    db: &Database,
    character: u64,
    reputation: Option<&mut shared::reputation::CharacterReputation>,
    request: RenownRequest,
) -> RenownSnapshot {
    let mut reply = snapshot(db, character);
    let Some(reputation) = reputation else {
        reply.watch_error = Some("Watched faction reputation is unavailable.".into());
        return reply;
    };
    if let RenownRequest::SetWatchedFaction { faction_id } = request {
        let eligibility = catalog()
            .iter()
            .find(|f| f.faction_id == faction_id)
            .map(unlock);
        match set_watched_faction(reputation, faction_id, eligibility.as_ref()) {
            Ok(()) => crate::persistence::save_reputation(db, character, reputation),
            Err(error) => reply.watch_error = Some(error),
        }
    }
    reply.watched_faction = reputation.watched_faction();
    reply
}

fn send_watched_bar_update(
    db: &Database,
    character: u64,
    reputation: &shared::reputation::CharacterReputation,
    sender: &mut MessageSender<shared::protocol::ReputationStateUpdate>,
) {
    let snapshot = crate::reputation::read_reputation_snapshot(
        db,
        character,
        reputation,
        &shared::reputation::default_faction_registry(),
    );
    sender.send::<shared::protocol::ReputationChannel>(shared::protocol::ReputationStateUpdate {
        snapshot: Some(snapshot),
        message: None,
        error: None,
    });
}

fn receive_requests(
    db: Res<GameDatabase>,
    mut reputations: Query<&mut shared::reputation::CharacterReputation>,
    mut connections: Query<
        (
            &InWorld,
            &PlayerLink,
            &mut MessageReceiver<RenownRequest>,
            &mut MessageSender<RenownSnapshot>,
            &mut MessageSender<shared::protocol::ReputationStateUpdate>,
        ),
        With<Authenticated>,
    >,
) {
    for (owner, link, mut receiver, mut sender, mut reputation_sender) in &mut connections {
        for request in receiver.receive() {
            let updates_watch = matches!(request, RenownRequest::SetWatchedFaction { .. });
            let mut reputation = reputations.get_mut(link.0).ok();
            let reply = reply_to_request(
                &db.db,
                owner.character_id,
                reputation.as_deref_mut(),
                request,
            );
            if updates_watch && let Some(reputation) = reputation.as_deref() {
                send_watched_bar_update(
                    &db.db,
                    owner.character_id,
                    reputation,
                    &mut reputation_sender,
                );
            }
            sender.send::<InstanceChannel>(reply);
        }
    }
}
