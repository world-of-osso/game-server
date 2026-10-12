//! Imported TDB numeric rewards and pinned DB2 XP; no synthetic quest-XP formula.
use crate::quest_data::{QuestDb, QuestTemplate};
use shared::protocol::WorldQuestNumericReward;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub(crate) struct Rewards {
    pub xp_multiplier: f32,
    pub currencies: Vec<WorldQuestNumericReward>,
    pub reputation: Vec<WorldQuestNumericReward>,
}

pub(crate) fn load(conn: &rusqlite::Connection, quests: &mut QuestDb) -> rusqlite::Result<()> {
    if quests.world_quests.is_empty() {
        return Ok(());
    }
    let mut statement =
        conn.prepare("SELECT quest_id,source_template_json FROM content_world_quest")?;
    for row in statement.query_map([], |r| Ok((r.get::<_, u32>(0)?, r.get::<_, String>(1)?)))? {
        let (id, raw) = row?;
        let source: serde_json::Value =
            serde_json::from_str(&raw).map_err(|_| rusqlite::Error::InvalidQuery)?;
        let multiplier = source["RewardXPMultiplier"]
            .as_f64()
            .ok_or(rusqlite::Error::InvalidQuery)? as f32;
        quests.world_quest_rewards.insert(
            id,
            Rewards {
                xp_multiplier: multiplier,
                ..Default::default()
            },
        );
    }
    load_numeric_rewards(conn, &mut quests.world_quest_rewards)?;
    let mut statement =
        conn.prepare("SELECT level,difficulty,amount FROM content_world_quest_xp")?;
    for row in statement.query_map([], |r| Ok(((r.get(0)?, r.get(1)?), r.get(2)?)))? {
        let (key, value) = row?;
        quests.world_quest_xp.insert(key, value);
    }
    Ok(())
}

fn load_numeric_rewards(
    conn: &rusqlite::Connection,
    rewards: &mut HashMap<u32, Rewards>,
) -> rusqlite::Result<()> {
    let mut statement =
        conn.prepare("SELECT quest_id,kind,reward_id,amount FROM content_world_quest_reward")?;
    for row in statement.query_map([], |r| {
        Ok((
            r.get::<_, u32>(0)?,
            r.get::<_, String>(1)?,
            WorldQuestNumericReward {
                id: r.get(2)?,
                amount: r.get(3)?,
            },
        ))
    })? {
        let (id, kind, reward) = row?;
        let data = rewards.get_mut(&id).ok_or(rusqlite::Error::InvalidQuery)?;
        match kind.as_str() {
            "currency" => data.currencies.push(reward),
            "reputation" => data.reputation.push(reward),
            _ => return Err(rusqlite::Error::InvalidQuery),
        }
    }
    Ok(())
}

pub(crate) struct Settlement {
    wallet: shared::currency::CurrencyWallet,
    reputation: shared::reputation::CharacterReputation,
}

pub(crate) fn load_settlement(
    db: &redb::Database,
    character: u64,
    race: u8,
    rewards: Option<&Rewards>,
) -> Result<Option<Settlement>, String> {
    let Some(rewards) = rewards else {
        return Ok(None);
    };
    let mut wallet = crate::persistence::load_currencies(db, character);
    for reward in &rewards.currencies {
        let amount = u32::try_from(reward.amount).map_err(|_| "negative quest currency")?;
        crate::currency::earn_currency(&mut wallet, reward.id, amount)?;
    }
    let registry = shared::reputation::default_faction_registry();
    let mut reputation = crate::persistence::load_reputation(db, character, || {
        shared::reputation::CharacterReputation::new_for_race(&registry, race)
    });
    for reward in &rewards.reputation {
        reputation.modify_rep(
            reward.id,
            reward.amount,
            shared::reputation::RepGainSource::QuestTurnIn,
        );
    }
    Ok(Some(Settlement { wallet, reputation }))
}

impl Settlement {
    pub(crate) fn persist_in(&self, txn: &redb::WriteTransaction, character: u64) {
        txn.open_table(crate::persistence::CURRENCIES)
            .expect("open quest currencies")
            .insert(
                character,
                serde_json::to_vec(&self.wallet)
                    .expect("encode quest currencies")
                    .as_slice(),
            )
            .expect("write quest currencies");
        txn.open_table(crate::persistence::REPUTATIONS)
            .expect("open quest reputation")
            .insert(character, bitcode::encode(&self.reputation).as_slice())
            .expect("write quest reputation");
    }

    pub(crate) fn publish(
        self,
        world: &mut bevy::prelude::World,
        connection: bevy::prelude::Entity,
    ) {
        use lightyear::prelude::MessageSender;
        use shared::protocol::{
            CurrencyChannel, CurrencyStateUpdate, ReputationChannel, ReputationStateUpdate,
        };
        let Some(player) = world
            .get::<crate::networking::PlayerLink>(connection)
            .map(|link| link.0)
        else {
            return;
        };
        let currency = crate::currency::build_currency_snapshot(&self.wallet);
        let reputation = crate::reputation::build_reputation_snapshot(
            &self.reputation,
            &shared::reputation::default_faction_registry(),
        );
        world.entity_mut(player).insert((
            crate::currency::PlayerCurrencies(self.wallet),
            self.reputation,
        ));
        if let Some(mut sender) = world.get_mut::<MessageSender<CurrencyStateUpdate>>(connection) {
            sender.send::<CurrencyChannel>(CurrencyStateUpdate {
                snapshot: Some(currency),
                message: None,
                error: None,
            });
        }
        if let Some(mut sender) = world.get_mut::<MessageSender<ReputationStateUpdate>>(connection)
        {
            sender.send::<ReputationChannel>(ReputationStateUpdate {
                snapshot: Some(reputation),
                message: None,
                error: None,
            });
        }
    }
}

pub(crate) fn xp(quests: &QuestDb, quest: &QuestTemplate, player_level: u8) -> u32 {
    if player_level >= crate::experience::MAX_LEVEL {
        return 0;
    }
    let Some(data) = quests.world_quest_rewards.get(&quest.id) else {
        return 0;
    };
    let level = quest.money_scaling.map_or(player_level, |scaling| {
        player_level.clamp(scaling.min_level, scaling.max_level.max(scaling.min_level))
    });
    let base = quests
        .world_quest_xp
        .get(&(level, quest.reward_xp_difficulty))
        .copied()
        .unwrap_or(0);
    (base as f32 * data.xp_multiplier) as u32
}
