use crate::spell_info::{TraitTree, load_trait_tree_by_id};
use bevy::prelude::Resource;
use rusqlite::Connection;
use shared::protocol::{ProfessionSpecEdgeSnapshot, ProfessionSpecPath, ProfessionSpecPerk};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub(super) struct NodeMetadata {
    pub name: String,
    pub description: String,
    pub icon: u32,
}

#[derive(Debug, Clone)]
pub(super) struct TreeMetadata {
    pub skill_line: u32,
    pub root_node: u32,
    pub tree: TraitTree,
    pub currencies: BTreeMap<u32, u32>,
    pub edges: Vec<ProfessionSpecEdgeSnapshot>,
}

#[derive(Debug, Clone)]
pub(super) struct KnowledgeCurrency {
    pub cap: u32,
    pub weekly_cap: u32,
    pub skill_lines: BTreeSet<u32>,
}

#[derive(Debug, Clone)]
pub(super) struct KnowledgeReward {
    pub currency: u32,
    pub quantity: u32,
}

#[derive(Resource, Debug)]
pub(crate) struct SpecializationData {
    pub(super) trees: BTreeMap<u32, TreeMetadata>,
    pub(super) nodes: BTreeMap<u32, NodeMetadata>,
    pub(super) paths: BTreeMap<u32, ProfessionSpecPath>,
    pub(super) perks: BTreeMap<u32, ProfessionSpecPerk>,
    pub(super) knowledge: BTreeMap<u32, KnowledgeCurrency>,
    pub(super) rewards: BTreeMap<u32, Vec<KnowledgeReward>>,
}

pub(crate) fn load_specialization_data(conn: &Connection) -> rusqlite::Result<SpecializationData> {
    let trees = load_trees(conn)?;
    let knowledge = load_knowledge_currencies(conn, &trees)?;
    Ok(SpecializationData {
        nodes: load_node_metadata(conn)?,
        paths: load_paths(conn)?,
        perks: load_perks(conn)?,
        rewards: load_knowledge_rewards(conn, &knowledge)?,
        trees,
        knowledge,
    })
}

fn load_trees(conn: &Connection) -> rusqlite::Result<BTreeMap<u32, TreeMetadata>> {
    let mut statement = conn.prepare("SELECT x.SkillLineID,x.TraitTreeID,t.FirstTraitNodeID FROM skill_line_x_trait_tree x JOIN trait_tree t ON t.ID=x.TraitTreeID JOIN prof_trait_tree p ON p.TraitTreeID=t.ID WHERE x.Variant>0 ORDER BY x.ID")?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, u32>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut trees = BTreeMap::new();
    for (skill_line, tree_id, root_node) in rows {
        let tree = load_trait_tree_by_id(conn, tree_id)?;
        trees.insert(
            tree_id,
            TreeMetadata {
                skill_line,
                root_node,
                tree,
                currencies: load_currency_bindings(conn, tree_id)?,
                edges: load_edges(conn, tree_id)?,
            },
        );
    }
    Ok(trees)
}

fn load_currency_bindings(conn: &Connection, tree: u32) -> rusqlite::Result<BTreeMap<u32, u32>> {
    let mut statement = conn.prepare("SELECT c.ID,c.CurrencyTypesID FROM trait_tree_x_trait_currency x JOIN trait_currency c ON c.ID=x.TraitCurrencyID WHERE x.TraitTreeID=?1 ORDER BY x._Index,x.ID")?;
    statement
        .query_map([tree], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect()
}

fn load_edges(conn: &Connection, tree: u32) -> rusqlite::Result<Vec<ProfessionSpecEdgeSnapshot>> {
    let mut statement = conn.prepare("SELECT e.LeftTraitNodeID,e.RightTraitNodeID,e.Type,e.VisualStyle FROM trait_edge e JOIN trait_node n ON n.ID=e.LeftTraitNodeID WHERE n.TraitTreeID=?1 ORDER BY e.ID")?;
    statement
        .query_map([tree], |row| {
            Ok(ProfessionSpecEdgeSnapshot {
                left_node_id: row.get(0)?,
                right_node_id: row.get(1)?,
                kind: row.get(2)?,
                visual_style: row.get(3)?,
            })
        })?
        .collect()
}

fn load_node_metadata(conn: &Connection) -> rusqlite::Result<BTreeMap<u32, NodeMetadata>> {
    let sources = super::descriptions::load_description_sources(conn)?;
    let mut statement = conn.prepare("SELECT e.ID,CASE WHEN d.OverrideName_lang<>'' THEN d.OverrideName_lang ELSE COALESCE(s.Name_lang,'') END,d.OverrideDescription_lang,d.OverrideIcon,d.ID FROM trait_node_entry e JOIN trait_definition d ON d.ID=e.TraitDefinitionID LEFT JOIN spell_name s ON s.ID=d.SpellID WHERE e.ID IN (SELECT x.TraitNodeEntryID FROM trait_node_x_trait_node_entry x JOIN trait_node n ON n.ID=x.TraitNodeID JOIN skill_line_x_trait_tree t ON t.TraitTreeID=n.TraitTreeID WHERE t.Variant>0)")?;
    statement
        .query_map([], |row| {
            let definition: u32 = row.get(4)?;
            let text: String = row.get(2)?;
            let description = match sources.resolve(definition, &text) {
                Ok(description) => description,
                Err(reason) => {
                    bevy::log::warn!(
                        "Profession definition {definition} description unavailable: {reason}"
                    );
                    format!("Description unavailable (definition {definition}): {reason}")
                }
            };
            Ok((
                row.get(0)?,
                NodeMetadata {
                    name: row.get(1)?,
                    description,
                    icon: row.get(3)?,
                },
            ))
        })?
        .collect()
}

fn load_paths(conn: &Connection) -> rusqlite::Result<BTreeMap<u32, ProfessionSpecPath>> {
    let mut statement = conn.prepare("SELECT TraitNodeID,LeftTraitNodeGroupID,RightTraitNodeGroupID,TraitCurrencyID FROM prof_trait_path_node ORDER BY ID")?;
    statement
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                ProfessionSpecPath {
                    left_group: row.get(1)?,
                    right_group: row.get(2)?,
                    trait_currency_id: row.get(3)?,
                },
            ))
        })?
        .collect()
}

fn load_perks(conn: &Connection) -> rusqlite::Result<BTreeMap<u32, ProfessionSpecPerk>> {
    // Retail ProfessionSpecConstantsDocumentation.lua: ProfTraitPerkNodeFlags.
    const UNLOCKS_SUBPATH: u32 = 1;
    const IS_MAJOR_BONUS: u32 = 2;
    let mut statement = conn.prepare(
        "SELECT p.TraitNodeID,p.RequiredPointsInThisNode,p.Flags,
            CASE WHEN COUNT(DISTINCT path.TraitNodeID)=1 THEN MIN(path.TraitNodeID) ELSE 0 END
         FROM prof_trait_perk_node p
         LEFT JOIN trait_node_x_trait_cond x ON x.TraitNodeID=p.TraitNodeID
         LEFT JOIN trait_cond c ON c.ID=x.TraitCondID AND c.CondType=2
         LEFT JOIN prof_trait_path_node path ON path.TraitNodeID=c.TraitNodeID
         GROUP BY p.ID ORDER BY p.ID",
    )?;
    statement
        .query_map([], |row| {
            let flags: u32 = row.get(2)?;
            Ok((
                row.get(0)?,
                ProfessionSpecPerk {
                    path_node_id: row.get(3)?,
                    required_points: row.get(1)?,
                    unlocks_subpath: flags & UNLOCKS_SUBPATH != 0,
                    is_major_bonus: flags & IS_MAJOR_BONUS != 0,
                },
            ))
        })?
        .collect()
}

fn load_knowledge_currencies(
    conn: &Connection,
    trees: &BTreeMap<u32, TreeMetadata>,
) -> rusqlite::Result<BTreeMap<u32, KnowledgeCurrency>> {
    let mut knowledge = BTreeMap::new();
    for metadata in trees.values() {
        for &currency in metadata
            .currencies
            .values()
            .filter(|currency| **currency != 0)
        {
            if !knowledge.contains_key(&currency) {
                let (cap,weekly_cap,flags): (u32,u32,u64) = conn.query_row("SELECT MaxQty,MaxEarnablePerWeek,(Flags_1<<32)|Flags_0 FROM currency_types WHERE ID=?1",[currency],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?)))?;
                // Every source profession knowledge currency has both flag words0.
                // Flagged/account currencies need their authored ledger semantics first.
                if flags != 0 {
                    return Err(rusqlite::Error::InvalidQuery);
                }
                knowledge.insert(
                    currency,
                    KnowledgeCurrency {
                        cap,
                        weekly_cap,
                        skill_lines: BTreeSet::new(),
                    },
                );
            }
            knowledge
                .get_mut(&currency)
                .expect("currency loaded")
                .skill_lines
                .insert(metadata.skill_line);
        }
    }
    Ok(knowledge)
}

fn load_knowledge_rewards(
    conn: &Connection,
    knowledge: &BTreeMap<u32, KnowledgeCurrency>,
) -> rusqlite::Result<BTreeMap<u32, Vec<KnowledgeReward>>> {
    let currencies = knowledge.keys().copied().collect();
    load_rewards_for_currencies(conn, &currencies)
}

pub(crate) fn load_knowledge_quest_rewards(
    conn: &Connection,
) -> rusqlite::Result<BTreeMap<u32, Vec<(u32, u32)>>> {
    let mut statement = conn.prepare("SELECT DISTINCT c.CurrencyTypesID FROM trait_currency c JOIN trait_tree_x_trait_currency x ON x.TraitCurrencyID=c.ID JOIN skill_line_x_trait_tree t ON t.TraitTreeID=x.TraitTreeID WHERE c.Type=1 AND t.Variant>0")?;
    let currencies = statement
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<BTreeSet<u32>>>()?;
    let rewards = load_rewards_for_currencies(conn, &currencies)?;
    Ok(rewards
        .into_iter()
        .map(|(quest, rewards)| {
            (
                quest,
                rewards
                    .into_iter()
                    .map(|reward| (reward.currency, reward.quantity))
                    .collect(),
            )
        })
        .collect())
}

fn load_rewards_for_currencies(
    conn: &Connection,
    currencies: &BTreeSet<u32>,
) -> rusqlite::Result<BTreeMap<u32, Vec<KnowledgeReward>>> {
    let mut statement = conn.prepare("SELECT ID,RewardCurrencyID1,RewardCurrencyQty1,RewardCurrencyID2,RewardCurrencyQty2,RewardCurrencyID3,RewardCurrencyQty3,RewardCurrencyID4,RewardCurrencyQty4 FROM tdb_quest_template ORDER BY ID")?;
    let mut rewards = BTreeMap::new();
    for row in statement.query_map([], |row| {
        let mut rewards = Vec::new();
        for index in 0..4 {
            let currency: u32 = row.get(index * 2 + 1)?;
            let quantity: u32 = row.get(index * 2 + 2)?;
            if quantity > 0 && currencies.contains(&currency) {
                rewards.push(KnowledgeReward { currency, quantity });
            }
        }
        Ok((row.get::<_, u32>(0)?, rewards))
    })? {
        let (id, items) = row?;
        if !items.is_empty() {
            rewards.insert(id, items);
        }
    }
    Ok(rewards)
}
