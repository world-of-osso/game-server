//! Guild reputation (faction 1168) and the guild rewards list.
//!
//! Rewards: TrinityCore `guild_rewards` / `guild_rewards_req_achievements` (TDB1210,
//! `GuildMgr::LoadGuildRewards`, `HandleRequestGuildRewardsList`). TrinityCore never gates
//! the purchase; Retail does (GuildRewards.lua `REQUIRES_GUILD_FACTION`, achievement
//! lock), so guild vendors refuse a reward below its `MinGuildRep` or without its
//! guild achievements. Guild achievements are not earned yet, so those rewards stay locked.
//!
//! Gain: no TrinityCore version awards guild reputation (TDB434.09 Guild.cpp:3559
//! "@todo: Award reputation"). warcraft.wiki.gg Guild_reputation, Patch 5.0.4 note: a
//! green or better quest gives a flat 60000 guild experience, grey quests nothing, and
//! "1 reputation per 360 experience"; the weekly cap was removed. Boss kills and rated
//! PvP wins also award guild reputation in Retail, but no source gives their amounts.
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use redb::{ReadableTable, TableDefinition};
use rusqlite::Connection;
use shared::level_scaling::LevelDifficulty;
use shared::protocol::{
    GuildChannel, GuildRewardAchievement, GuildRewardItem, GuildRewardsError, GuildRewardsRequest,
    GuildRewardsState, MerchantError, ReputationChannel, ReputationStateUpdate,
};
use shared::reputation::{CharacterReputation, RepGainSource, Standing};

use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::GameDatabase;

pub(crate) const GUILD_FACTION: u32 = 1168;
const QUEST_GUILD_XP: u64 = 60_000;
const GUILD_XP_PER_REPUTATION: u64 = 360;

/// Guild experience a character earned towards guild reputation, so the 360 XP
/// remainder carries between quests (166 or 167 reputation per quest).
const GUILD_REPUTATION_XP: TableDefinition<u64, u64> = TableDefinition::new("guild_reputation_xp");

#[derive(Debug, Clone, PartialEq, Eq)]
struct RewardAchievement {
    id: u32,
    name: String,
    /// `Achievement.Faction`: -1 any, 0 Horde, 1 Alliance.
    faction: i8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Reward {
    item_id: u32,
    min_standing: Standing,
    race_mask: u64,
    cost: u64,
    achievements: Vec<RewardAchievement>,
}

#[derive(Resource, Debug, Default, Clone)]
pub struct GuildRewardCatalog {
    rewards: Vec<Reward>,
}

/// `ReputationRank` 0 Hated .. 7 Exalted.
fn standing_for_rank(rank: i64) -> rusqlite::Result<Standing> {
    Ok(match rank {
        0 => Standing::Hated,
        1 => Standing::Hostile,
        2 => Standing::Unfriendly,
        3 => Standing::Neutral,
        4 => Standing::Friendly,
        5 => Standing::Honored,
        6 => Standing::Revered,
        7 => Standing::Exalted,
        _ => return Err(rusqlite::Error::IntegralValueOutOfRange(1, rank)),
    })
}

pub fn load_guild_reward_catalog(conn: &Connection) -> rusqlite::Result<GuildRewardCatalog> {
    let mut achievements = conn.prepare(
        "SELECT r.AchievementRequired, a.Title_lang, a.Faction FROM tdb_guild_rewards_req_achievements r \
         JOIN achievement a ON a.ID = r.AchievementRequired WHERE r.ItemID = ?1 ORDER BY r.AchievementRequired",
    )?;
    let mut rows = conn.prepare(
        "SELECT ItemID, MinGuildRep, RaceMask, Cost FROM tdb_guild_rewards ORDER BY ItemID",
    )?;
    let rewards = rows
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?
        .map(|row| {
            let (item_id, rank, race_mask, cost) = row?;
            let achievements = achievements
                .query_map([item_id], |row| {
                    Ok(RewardAchievement {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        faction: row.get(2)?,
                    })
                })?
                .collect::<rusqlite::Result<_>>()?;
            Ok(Reward {
                item_id,
                min_standing: standing_for_rank(rank)?,
                race_mask: race_mask as u64,
                cost: cost.max(0) as u64,
                achievements,
            })
        })
        .collect::<rusqlite::Result<_>>()?;
    Ok(GuildRewardCatalog { rewards })
}

impl Reward {
    fn offered_to(&self, race: u8) -> bool {
        self.race_mask == 0
            || crate::race_class::race_mask_has_race(
                [self.race_mask as u32, (self.race_mask >> 32) as u32],
                race,
            )
    }

    /// The requester's team's achievements (`Achievement.Faction`).
    fn achievements_for(&self, race: u8) -> impl Iterator<Item = &RewardAchievement> {
        let team = match crate::faction::faction_for_race(race) {
            1 => 1,
            2 => 0,
            _ => -2,
        };
        self.achievements
            .iter()
            .filter(move |achievement| achievement.faction == -1 || achievement.faction == team)
    }
}

impl GuildRewardCatalog {
    fn state(&self, race: u8) -> GuildRewardsState {
        let rewards = self
            .rewards
            .iter()
            .filter(|reward| reward.offered_to(race))
            .map(|reward| GuildRewardItem {
                item_id: reward.item_id,
                min_standing: reward.min_standing,
                cost: reward.cost,
                achievements: reward
                    .achievements_for(race)
                    .map(|achievement| GuildRewardAchievement {
                        id: achievement.id,
                        name: achievement.name.clone(),
                    })
                    .collect(),
            })
            .collect();
        GuildRewardsState {
            rewards,
            error: None,
        }
    }

    /// Whether a vendor may sell `item_id` to this buyer. Guild achievements are not
    /// earned by any guild yet, so a reward requiring one is always refused.
    pub(crate) fn check_purchase(
        &self,
        item_id: u32,
        race: u8,
        reputation: Option<&CharacterReputation>,
    ) -> Result<(), MerchantError> {
        let Some(reward) = self.rewards.iter().find(|reward| reward.item_id == item_id) else {
            return Ok(());
        };
        let standing = reputation.map_or(Standing::Neutral, |rep| rep.get_standing(GUILD_FACTION));
        if standing < reward.min_standing {
            return Err(MerchantError::ReputationRequired);
        }
        if reward.achievements_for(race).next().is_some() {
            return Err(MerchantError::MissingTurnins);
        }
        Ok(())
    }
}

type RewardRequests<'w, 's> = Query<
    'w,
    's,
    (
        &'static InWorld,
        &'static PlayerLink,
        &'static mut MessageReceiver<GuildRewardsRequest>,
        &'static mut MessageSender<GuildRewardsState>,
    ),
    With<Authenticated>,
>;

pub(crate) fn answer_reward_requests(
    db: Res<GameDatabase>,
    catalog: Res<GuildRewardCatalog>,
    players: Query<&shared::components::Player>,
    mut requests: RewardRequests,
) {
    for (owner, link, mut receiver, mut sender) in &mut requests {
        if receiver.receive().count() == 0 {
            continue;
        }
        let state =
            if crate::persistence::load_character_guild(&db.db, owner.character_id).is_none() {
                GuildRewardsState {
                    rewards: Vec::new(),
                    error: Some(GuildRewardsError::NotInGuild),
                }
            } else {
                catalog.state(players.get(link.0).map_or(0, |player| player.race))
            };
        sender.send::<GuildChannel>(state);
    }
}

/// Guild reputation earned by `accumulated` guild XP plus one quest:
/// `(new accumulated XP, reputation gained)`.
fn quest_reputation(accumulated: u64) -> (u64, i32) {
    let total = accumulated + QUEST_GUILD_XP;
    let gained = total / GUILD_XP_PER_REPUTATION - accumulated / GUILD_XP_PER_REPUTATION;
    (total, gained as i32)
}

/// Award guild reputation for a rewarded quest, once its reward has committed.
/// Grey quests and characters outside a guild earn nothing.
pub(crate) fn award_quest_reputation(
    world: &mut World,
    connection: Entity,
    character: u64,
    player_level: u8,
    quest_level: i32,
) {
    let quest_level = u8::try_from(quest_level.max(0)).unwrap_or(u8::MAX);
    if LevelDifficulty::for_levels(player_level, quest_level) == LevelDifficulty::Trivial {
        return;
    }
    let db = world.resource::<GameDatabase>().db.clone();
    if crate::persistence::load_character_guild(&db, character).is_none() {
        return;
    }
    let Some(player) = world.get::<PlayerLink>(connection).map(|link| link.0) else {
        return;
    };
    let Some(mut reputation) = world.get::<CharacterReputation>(player).cloned() else {
        return;
    };
    let txn = db.begin_write().expect("begin guild reputation award");
    let change = {
        let mut table = txn
            .open_table(GUILD_REPUTATION_XP)
            .expect("open guild reputation experience");
        let accumulated = table
            .get(character)
            .expect("read guild reputation experience")
            .map_or(0, |value| value.value());
        let (accumulated, gained) = quest_reputation(accumulated);
        table
            .insert(character, accumulated)
            .expect("write guild reputation experience");
        reputation.gain_rep(GUILD_FACTION, gained, RepGainSource::QuestTurnIn)
    };
    txn.open_table(crate::persistence::REPUTATIONS)
        .expect("open reputations")
        .insert(character, bitcode::encode(&reputation).as_slice())
        .expect("write guild reputation");
    txn.commit().expect("commit guild reputation award");
    let registry = shared::reputation::default_faction_registry();
    let snapshot =
        crate::reputation::read_reputation_snapshot(&db, character, &reputation, &registry);
    world.entity_mut(player).insert(reputation);
    if change.actual_change() == 0 {
        return;
    }
    let message = crate::reputation::format_change_message(&[change], &registry);
    if let Some(mut sender) = world.get_mut::<MessageSender<ReputationStateUpdate>>(connection) {
        sender.send::<ReputationChannel>(ReputationStateUpdate {
            snapshot: Some(snapshot),
            message: Some(message),
            error: None,
        });
    }
}

#[cfg(test)]
#[path = "guild_rewards_tests.rs"]
pub(crate) mod tests;
