//! Imported TDB numeric rewards and pinned DB2 XP; no synthetic quest-XP formula.
use crate::quest_data::{QuestDb, QuestTemplate};
use shared::protocol::WorldQuestNumericReward;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub(crate) struct Rewards {
    pub xp_multiplier: f32,
    pub currencies: Vec<WorldQuestNumericReward>,
    pub reputation: Vec<WorldQuestNumericReward>,
    pub reputation_caps: HashMap<u32, u8>,
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
        conn.prepare("SELECT quest_id,kind,reward_id,amount,cap FROM content_world_quest_reward")?;
    for row in statement.query_map([], |r| {
        Ok((
            r.get::<_, u32>(0)?,
            r.get::<_, String>(1)?,
            WorldQuestNumericReward {
                id: r.get(2)?,
                amount: r.get(3)?,
            },
            r.get::<_, u8>(4)?,
        ))
    })? {
        let (id, kind, reward, cap) = row?;
        if cap > 7 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let data = rewards.get_mut(&id).ok_or(rusqlite::Error::InvalidQuery)?;
        match kind.as_str() {
            "currency" => data.currencies.push(reward),
            "reputation" => {
                data.reputation_caps.insert(reward.id, cap);
                data.reputation.push(reward);
            }
            _ => return Err(rusqlite::Error::InvalidQuery),
        }
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn world_quest_authentic_reputation_cap_prevents_exalted_reward_but_allows_losses() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE content_world_quest_reward(quest_id INTEGER,kind TEXT,reward_id INTEGER,amount INTEGER,cap INTEGER); INSERT INTO content_world_quest_reward VALUES(45068,'reputation',1090,75,7)").unwrap();
    let mut rewards = HashMap::from([(45068, Rewards::default())]);
    load_numeric_rewards(&conn, &mut rewards).unwrap();
    let db = crate::persistence::temp_database();
    let registry = shared::reputation::default_faction_registry();
    let mut reputation = shared::reputation::CharacterReputation::new_for_race(&registry, 1);
    reputation.modify_rep(1090, 51000, shared::reputation::RepGainSource::QuestTurnIn);
    assert_eq!(
        reputation.get_standing(1090),
        shared::reputation::Standing::Exalted
    );
    let exalted = reputation.get_value(1090);
    crate::persistence::save_reputation(&db, 31, &reputation);
    let settlement = load_settlement(&db, 31, 1, Some(&rewards[&45068]))
        .unwrap()
        .unwrap();
    let txn = db.begin_write().unwrap();
    settlement.persist_in(&txn, 31);
    crate::persistence_commit::commit(txn).unwrap();
    let persisted =
        crate::persistence::load_reputation(&db, 31, || panic!("persisted reputation required"));
    assert_eq!(
        persisted.get_value(1090),
        exalted,
        "TDB cap7 blocks gains while already Exalted"
    );
    rewards.get_mut(&45068).unwrap().reputation[0].amount = -75;
    let settlement = load_settlement(&db, 31, 1, Some(&rewards[&45068]))
        .unwrap()
        .unwrap();
    assert_eq!(
        settlement.reputation.get_value(1090),
        exalted - 75,
        "caps never block negative rewards"
    );
}

#[cfg(test)]
#[test]
fn world_quest_publication_keeps_watched_bar_in_step_with_committed_reputation() {
    use lightyear::prelude::{MessageReceiver, MessageSender};
    use shared::protocol::{CurrencyStateUpdate, ReputationStateUpdate};
    let mut h = crate::bank_test_harness::harness().unwrap();
    let registry = shared::reputation::default_faction_registry();
    let mut reputation = shared::reputation::CharacterReputation::new_for_race(&registry, 1);
    reputation.set_rep(1090, 3800);
    reputation.watch(1090);
    crate::persistence::save_reputation(h.db(), h.character_id, &reputation);
    let rewards = Rewards {
        reputation: vec![WorldQuestNumericReward {
            id: 1090,
            amount: 75,
        }],
        ..Default::default()
    };
    let settlement = load_settlement(h.db(), h.character_id, 1, Some(&rewards))
        .unwrap()
        .unwrap();
    let txn = h.db().begin_write().unwrap();
    settlement.persist_in(&txn, h.character_id);
    txn.commit().unwrap();
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<ReputationStateUpdate>::default(),
        MessageReceiver::<ReputationStateUpdate>::default(),
        MessageSender::<CurrencyStateUpdate>::default(),
    ));
    settlement.publish(h.app.world_mut(), h.connection);
    let mut updates = Vec::new();
    for _ in 0..10 {
        h.app.update();
        updates.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<ReputationStateUpdate>>(h.connection)
                .unwrap()
                .receive(),
        );
    }
    let watched = updates
        .into_iter()
        .find_map(|update| update.snapshot.and_then(|snapshot| snapshot.watched))
        .expect("committed quest reputation must publish watched progress");
    assert_eq!(
        (watched.faction_id, watched.value, watched.maximum),
        (1090, 875, 6000)
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::reputation::CharacterReputation>(h.player)
            .unwrap()
            .get_value(1090),
        3875
    );
}

pub(crate) struct Settlement {
    pub(crate) wallet: shared::currency::CurrencyWallet,
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
        // Pinned Player.cpp6645: a positive reward stops at its authored standing
        // cap; zero means unlimited and losses never use the cap.
        let capped = rewards
            .reputation_caps
            .get(&reward.id)
            .is_some_and(|&cap| cap != 0 && reputation.get_standing(reward.id) as u8 >= cap);
        if reward.amount > 0 && capped {
            continue;
        }
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
        let character = world
            .get::<crate::auth::InWorld>(connection)
            .expect("world-quest reward owner must be in world")
            .character_id;
        let reputation = crate::reputation::read_reputation_snapshot(
            &world.resource::<crate::persistence::GameDatabase>().db,
            character,
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
    let data = quests
        .world_quest_rewards
        .get(&quest.id)
        .expect("imported world quest reward metadata");
    let level = quest.money_scaling.map_or(player_level, |scaling| {
        player_level.clamp(scaling.min_level, scaling.max_level.max(scaling.min_level))
    });
    let base = quests
        .world_quest_xp
        .get(&(level, quest.reward_xp_difficulty))
        .copied()
        .expect("pinned QuestXP level/difficulty row");
    (base as f32 * data.xp_multiplier) as u32
}
