//! Class trait (talent) trees from the Retail `trait_*` DB2 tables. Values are
//! raw DB2 values; rules that read them live in `crate::trait_config`
//! (`docs/wiki/systems/talents.md`).

use std::collections::HashMap;

use rusqlite::{Connection, Row};
use serde::Serialize;

use super::class_data::ATTR0_PASSIVE;
use super::load::{int, uint};

/// `TraitNode.Type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TraitNodeType {
    Single,
    /// Entries are ranked one after another (apex talents).
    Tiered,
    /// Exactly one entry may be chosen.
    Selection,
    /// Exactly one entry may be chosen; each entry names a hero `TraitSubTree`.
    SubTreeSelection,
}

/// `TraitEdge.Type`. Only these two gate availability; the other DB2 values
/// (VisualOnly, deprecated, MutuallyExclusive) do not occur on class trees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TraitEdgeType {
    /// Drawn by the client; does not constrain purchases.
    VisualOnly,
    /// Any one filled parent with this edge type makes the node available.
    SufficientForAvailability,
    /// This parent must be filled.
    RequiredForAvailability,
}

/// `TraitCond.CondType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TraitCondType {
    Available,
    Visible,
    Granted,
    Increased,
    /// UI error text only.
    DisplayError,
    /// Holding at least `granted_ranks` ranks requires the condition.
    RanksAllowed,
}

/// `TraitCond.Flags` IsGate.
pub const COND_FLAG_GATE: u32 = 0x1;
/// `TraitCond.Flags` IsSufficient: meeting this condition passes its whole list.
pub const COND_FLAG_SUFFICIENT: u32 = 0x4;

#[derive(Debug, Clone, Serialize)]
pub struct TraitCond {
    pub id: u32,
    pub kind: TraitCondType,
    pub flags: u32,
    pub granted_ranks: u32,
    pub quest_id: u32,
    pub achievement_id: u32,
    /// `SpecSetMember.ChrSpecializationID` of `SpecSetID`; empty when `SpecSetID` is 0.
    pub spec_set: Vec<u32>,
    pub spec_set_id: u32,
    pub node_group_id: u32,
    pub node_id: u32,
    pub node_entry_id: u32,
    pub currency_id: u32,
    pub spent_amount_required: u32,
    pub required_level: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TraitCost {
    pub currency_id: u32,
    pub amount: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraitNodeEntry {
    pub id: u32,
    pub definition_id: u32,
    /// `TraitDefinition.SpellID`; 0 when the entry has no definition spell.
    pub spell_id: u32,
    pub overrides_spell_id: u32,
    /// SPELL_ATTR0_PASSIVE of `spell_id`.
    pub passive: bool,
    pub max_ranks: u32,
    pub entry_type: u32,
    /// Hero sub-tree chosen by a `SubTreeSelection` entry; 0 otherwise.
    pub sub_tree_id: u32,
    pub conds: Vec<TraitCond>,
    pub costs: Vec<TraitCost>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraitNode {
    pub id: u32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub kind: TraitNodeType,
    pub flags: u32,
    /// Hero sub-tree the node belongs to; 0 for class and spec nodes.
    pub sub_tree_id: u32,
    /// `TraitNodeXTraitNodeEntry` order.
    pub entries: Vec<TraitNodeEntry>,
    /// `TraitNodeGroup` IDs, ascending.
    pub groups: Vec<u32>,
    /// `TraitEdge.LeftTraitNodeID` with the edge type.
    pub parents: Vec<(u32, TraitEdgeType)>,
    pub conds: Vec<TraitCond>,
    pub costs: Vec<TraitCost>,
}

impl TraitNode {
    pub fn entry(&self, entry_id: u32) -> Option<&TraitNodeEntry> {
        self.entries.iter().find(|entry| entry.id == entry_id)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TraitNodeGroup {
    pub id: u32,
    pub conds: Vec<TraitCond>,
    pub costs: Vec<TraitCost>,
}

/// One `TraitCurrencySource` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CurrencySource {
    pub amount: i32,
    pub quest_id: u32,
    pub achievement_id: u32,
    pub player_level: u32,
    pub node_entry_id: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraitCurrency {
    pub id: u32,
    /// `TraitCurrency.Flags`: 0x4 class-icon (class points), 0x8 spec-icon (spec points).
    pub flags: u32,
    pub sources: Vec<CurrencySource>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraitSubTree {
    pub id: u32,
    pub name: String,
}

/// Authored cumulative purchased rank at one `TraitTreeLoadoutEntry.OrderIndex`.
#[derive(Debug, Clone, Serialize)]
pub struct StarterPurchase {
    pub node_id: u32,
    pub entry_id: u32,
    pub ranks: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct StarterBuild {
    pub spec_id: u32,
    pub purchases: Vec<StarterPurchase>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraitTree {
    pub tree_id: u32,
    /// `TraitTreeXTraitCurrency` order: class, spec, then hero currencies.
    pub currencies: Vec<TraitCurrency>,
    /// Sorted by ID.
    pub nodes: Vec<TraitNode>,
    /// Sorted by ID.
    pub groups: Vec<TraitNodeGroup>,
    pub sub_trees: Vec<TraitSubTree>,
    pub starter_builds: Vec<StarterBuild>,
}

impl TraitTree {
    pub fn node(&self, node_id: u32) -> Option<&TraitNode> {
        let index = self
            .nodes
            .binary_search_by_key(&node_id, |node| node.id)
            .ok()?;
        Some(&self.nodes[index])
    }

    pub fn group(&self, group_id: u32) -> Option<&TraitNodeGroup> {
        let index = self
            .groups
            .binary_search_by_key(&group_id, |group| group.id)
            .ok()?;
        Some(&self.groups[index])
    }
}

fn invalid(what: &str, value: i64) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Integer,
        format!("{what} {value}").into(),
    )
}

fn node_type(value: i64) -> rusqlite::Result<TraitNodeType> {
    Ok(match value {
        0 => TraitNodeType::Single,
        1 => TraitNodeType::Tiered,
        2 => TraitNodeType::Selection,
        3 => TraitNodeType::SubTreeSelection,
        _ => return Err(invalid("unknown TraitNode.Type", value)),
    })
}

fn edge_type(value: i64) -> rusqlite::Result<TraitEdgeType> {
    Ok(match value {
        0 => TraitEdgeType::VisualOnly,
        2 => TraitEdgeType::SufficientForAvailability,
        3 => TraitEdgeType::RequiredForAvailability,
        _ => return Err(invalid("unsupported TraitEdge.Type", value)),
    })
}

fn cond_type(value: i64) -> rusqlite::Result<TraitCondType> {
    Ok(match value {
        0 => TraitCondType::Available,
        1 => TraitCondType::Visible,
        2 => TraitCondType::Granted,
        3 => TraitCondType::Increased,
        4 => TraitCondType::DisplayError,
        5 => TraitCondType::RanksAllowed,
        _ => return Err(invalid("unknown TraitCond.CondType", value)),
    })
}

/// Tree of the class skill line (`skill_line_x_trait_tree`).
pub(crate) fn load_trait_tree(conn: &Connection, skill_line: u32) -> rusqlite::Result<TraitTree> {
    let tree_id: u32 = conn.query_row(
        "SELECT TraitTreeID FROM skill_line_x_trait_tree WHERE SkillLineID = ?1",
        [skill_line],
        |r| uint(r, 0),
    )?;
    load_trait_tree_by_id(conn, tree_id)
}

/// Load one explicitly selected class or profession tree, without choosing a variant.
pub(crate) fn load_trait_tree_by_id(
    conn: &Connection,
    tree_id: u32,
) -> rusqlite::Result<TraitTree> {
    let spec_sets = load_spec_sets(conn)?;
    let conds = |sql: &str| load_linked_conds(conn, sql, tree_id, &spec_sets);
    let costs = |sql: &str| load_linked_costs(conn, sql, tree_id);
    let mut node_conds = conds(&format!(
        "SELECT x.TraitNodeID, {COND_COLUMNS} FROM trait_node_x_trait_cond x
         JOIN trait_cond c ON c.ID = x.TraitCondID
         JOIN trait_node n ON n.ID = x.TraitNodeID WHERE n.TraitTreeID = ?1 ORDER BY x.ID"
    ))?;
    let mut node_costs = costs(
        "SELECT x.TraitNodeID, c.TraitCurrencyID, c.Amount FROM trait_node_x_trait_cost x
         JOIN trait_cost c ON c.ID = x.TraitCostID
         JOIN trait_node n ON n.ID = x.TraitNodeID WHERE n.TraitTreeID = ?1 ORDER BY x.ID",
    )?;
    let entry_conds = conds(&format!(
        "SELECT x.TraitNodeEntryID, {COND_COLUMNS} FROM trait_node_entry_x_trait_cond x
         JOIN trait_cond c ON c.ID = x.TraitCondID
         WHERE x.TraitNodeEntryID IN ({TREE_ENTRIES}) ORDER BY x.ID"
    ))?;
    let entry_costs = costs(&format!(
        "SELECT x.TraitNodeEntryID, c.TraitCurrencyID, c.Amount FROM trait_node_entry_x_trait_cost x
         JOIN trait_cost c ON c.ID = x.TraitCostID
         WHERE x.TraitNodeEntryID IN ({TREE_ENTRIES}) ORDER BY x.ID"
    ))?;
    let mut group_conds = conds(&format!(
        "SELECT x.TraitNodeGroupID, {COND_COLUMNS} FROM trait_node_group_x_trait_cond x
         JOIN trait_cond c ON c.ID = x.TraitCondID
         JOIN trait_node_group g ON g.ID = x.TraitNodeGroupID WHERE g.TraitTreeID = ?1 ORDER BY x.ID"
    ))?;
    let mut group_costs = costs(
        "SELECT x.TraitNodeGroupID, c.TraitCurrencyID, c.Amount FROM trait_node_group_x_trait_cost x
         JOIN trait_cost c ON c.ID = x.TraitCostID
         JOIN trait_node_group g ON g.ID = x.TraitNodeGroupID WHERE g.TraitTreeID = ?1 ORDER BY x.ID",
    )?;

    let mut entries = load_entries(conn, tree_id)?;
    for list in entries.values_mut() {
        for entry in list {
            // An entry may sit on several nodes.
            entry.conds = entry_conds.get(&entry.id).cloned().unwrap_or_default();
            entry.costs = entry_costs.get(&entry.id).cloned().unwrap_or_default();
        }
    }
    let mut groups_of = load_node_groups(conn, tree_id)?;
    let mut parents = load_parents(conn, tree_id)?;
    let mut nodes = load_nodes(conn, tree_id)?;
    for node in &mut nodes {
        node.entries = entries.remove(&node.id).unwrap_or_default();
        node.groups = groups_of.remove(&node.id).unwrap_or_default();
        node.parents = parents.remove(&node.id).unwrap_or_default();
        node.conds = node_conds.remove(&node.id).unwrap_or_default();
        node.costs = node_costs.remove(&node.id).unwrap_or_default();
    }
    let groups = load_group_ids(conn, tree_id)?
        .into_iter()
        .map(|id| TraitNodeGroup {
            id,
            conds: group_conds.remove(&id).unwrap_or_default(),
            costs: group_costs.remove(&id).unwrap_or_default(),
        })
        .collect();
    Ok(TraitTree {
        tree_id,
        currencies: load_currencies(conn, tree_id)?,
        nodes,
        groups,
        sub_trees: load_sub_trees(conn, tree_id)?,
        starter_builds: load_starter_builds(conn, tree_id)?,
    })
}

fn load_spec_sets(conn: &Connection) -> rusqlite::Result<HashMap<u32, Vec<u32>>> {
    let mut stmt =
        conn.prepare("SELECT SpecSet, ChrSpecializationID FROM spec_set_member ORDER BY ID")?;
    let mut sets: HashMap<u32, Vec<u32>> = HashMap::new();
    for row in stmt.query_map([], |r| Ok((uint(r, 0)?, uint(r, 1)?)))? {
        let (set, spec) = row?;
        sets.entry(set).or_default().push(spec);
    }
    Ok(sets)
}

/// `trait_cond` columns read by `read_cond`, after the owner ID at index 0.
const COND_COLUMNS: &str = "c.ID, c.CondType, c.GrantedRanks, c.QuestID, c.AchievementID, \
    c.SpecSetID, c.TraitNodeGroupID, c.TraitNodeID, c.TraitNodeEntryID, c.TraitCurrencyID, \
    c.SpentAmountRequired, c.Flags, c.RequiredLevel";
/// Node entry IDs of tree `?1`.
const TREE_ENTRIES: &str = "SELECT ne.TraitNodeEntryID FROM trait_node_x_trait_node_entry ne \
    JOIN trait_node n ON n.ID = ne.TraitNodeID WHERE n.TraitTreeID = ?1";

fn read_cond(r: &Row, spec_sets: &HashMap<u32, Vec<u32>>) -> rusqlite::Result<TraitCond> {
    let spec_set_id = uint(r, 6)?;
    Ok(TraitCond {
        id: uint(r, 1)?,
        kind: cond_type(int(r, 2)?)?,
        granted_ranks: uint(r, 3)?,
        quest_id: uint(r, 4)?,
        achievement_id: uint(r, 5)?,
        spec_set: spec_sets.get(&spec_set_id).cloned().unwrap_or_default(),
        spec_set_id,
        node_group_id: uint(r, 7)?,
        node_id: uint(r, 8)?,
        node_entry_id: uint(r, 9)?,
        currency_id: uint(r, 10)?,
        spent_amount_required: uint(r, 11)?,
        flags: uint(r, 12)?,
        required_level: uint(r, 13)?,
    })
}

/// Rows of (owner ID, `trait_cond.*`) grouped by owner.
fn load_linked_conds(
    conn: &Connection,
    sql: &str,
    tree_id: u32,
    spec_sets: &HashMap<u32, Vec<u32>>,
) -> rusqlite::Result<HashMap<u32, Vec<TraitCond>>> {
    let mut stmt = conn.prepare(sql)?;
    let mut linked: HashMap<u32, Vec<TraitCond>> = HashMap::new();
    for row in stmt.query_map([tree_id], |r| Ok((uint(r, 0)?, read_cond(r, spec_sets)?)))? {
        let (owner, cond) = row?;
        linked.entry(owner).or_default().push(cond);
    }
    Ok(linked)
}

/// Rows of (owner ID, currency, amount) grouped by owner.
fn load_linked_costs(
    conn: &Connection,
    sql: &str,
    tree_id: u32,
) -> rusqlite::Result<HashMap<u32, Vec<TraitCost>>> {
    let mut stmt = conn.prepare(sql)?;
    let mut linked: HashMap<u32, Vec<TraitCost>> = HashMap::new();
    let rows = stmt.query_map([tree_id], |r| {
        Ok((
            uint(r, 0)?,
            TraitCost {
                currency_id: uint(r, 1)?,
                amount: uint(r, 2)?,
            },
        ))
    })?;
    for row in rows {
        let (owner, cost) = row?;
        linked.entry(owner).or_default().push(cost);
    }
    Ok(linked)
}

fn load_entries(
    conn: &Connection,
    tree_id: u32,
) -> rusqlite::Result<HashMap<u32, Vec<TraitNodeEntry>>> {
    let mut stmt = conn.prepare(
        "SELECT x.TraitNodeID, e.ID, e.TraitDefinitionID, COALESCE(d.SpellID, 0),
                COALESCE(d.OverridesSpellID, 0), e.MaxRanks, e.NodeEntryType, e.TraitSubTreeID,
                COALESCE(m.Attributes_0, 0)
         FROM trait_node_x_trait_node_entry x
         JOIN trait_node n ON n.ID = x.TraitNodeID
         JOIN trait_node_entry e ON e.ID = x.TraitNodeEntryID
         LEFT JOIN trait_definition d ON d.ID = e.TraitDefinitionID
         LEFT JOIN spell_misc m ON m.SpellID = d.SpellID AND m.DifficultyID = 0
         WHERE n.TraitTreeID = ?1 ORDER BY x.TraitNodeID, x._Index, x.ID",
    )?;
    let rows = stmt.query_map([tree_id], |r| {
        Ok((
            uint(r, 0)?,
            TraitNodeEntry {
                id: uint(r, 1)?,
                definition_id: uint(r, 2)?,
                spell_id: uint(r, 3)?,
                overrides_spell_id: uint(r, 4)?,
                passive: uint(r, 8)? & ATTR0_PASSIVE != 0,
                max_ranks: uint(r, 5)?,
                entry_type: uint(r, 6)?,
                sub_tree_id: uint(r, 7)?,
                conds: Vec::new(),
                costs: Vec::new(),
            },
        ))
    })?;
    let mut entries: HashMap<u32, Vec<TraitNodeEntry>> = HashMap::new();
    for row in rows {
        let (node_id, entry) = row?;
        entries.entry(node_id).or_default().push(entry);
    }
    Ok(entries)
}

fn load_node_groups(conn: &Connection, tree_id: u32) -> rusqlite::Result<HashMap<u32, Vec<u32>>> {
    let mut stmt = conn.prepare(
        "SELECT x.TraitNodeID, x.TraitNodeGroupID FROM trait_node_group_x_trait_node x
         JOIN trait_node n ON n.ID = x.TraitNodeID WHERE n.TraitTreeID = ?1
         ORDER BY x.TraitNodeID, x.TraitNodeGroupID",
    )?;
    let mut groups: HashMap<u32, Vec<u32>> = HashMap::new();
    for row in stmt.query_map([tree_id], |r| Ok((uint(r, 0)?, uint(r, 1)?)))? {
        let (node_id, group_id) = row?;
        groups.entry(node_id).or_default().push(group_id);
    }
    Ok(groups)
}

fn load_parents(
    conn: &Connection,
    tree_id: u32,
) -> rusqlite::Result<HashMap<u32, Vec<(u32, TraitEdgeType)>>> {
    let mut stmt = conn.prepare(
        "SELECT e.RightTraitNodeID, e.LeftTraitNodeID, e.Type FROM trait_edge e
         JOIN trait_node r ON r.ID = e.RightTraitNodeID
         JOIN trait_node l ON l.ID = e.LeftTraitNodeID
         WHERE r.TraitTreeID = ?1 ORDER BY e.ID",
    )?;
    let mut parents: HashMap<u32, Vec<(u32, TraitEdgeType)>> = HashMap::new();
    for row in stmt.query_map([tree_id], |r| Ok((uint(r, 0)?, uint(r, 1)?, int(r, 2)?)))? {
        let (child, parent, kind) = row?;
        parents
            .entry(child)
            .or_default()
            .push((parent, edge_type(kind)?));
    }
    Ok(parents)
}

fn load_nodes(conn: &Connection, tree_id: u32) -> rusqlite::Result<Vec<TraitNode>> {
    let mut stmt = conn.prepare(
        "SELECT ID, PosX, PosY, Type, Flags, TraitSubTreeID FROM trait_node
         WHERE TraitTreeID = ?1 ORDER BY ID",
    )?;
    let rows = stmt.query_map([tree_id], |r| {
        Ok(TraitNode {
            id: uint(r, 0)?,
            pos_x: int(r, 1)? as i32,
            pos_y: int(r, 2)? as i32,
            kind: node_type(int(r, 3)?)?,
            flags: uint(r, 4)?,
            sub_tree_id: uint(r, 5)?,
            entries: Vec::new(),
            groups: Vec::new(),
            parents: Vec::new(),
            conds: Vec::new(),
            costs: Vec::new(),
        })
    })?;
    rows.collect()
}

fn load_group_ids(conn: &Connection, tree_id: u32) -> rusqlite::Result<Vec<u32>> {
    let mut stmt =
        conn.prepare("SELECT ID FROM trait_node_group WHERE TraitTreeID = ?1 ORDER BY ID")?;
    let rows = stmt.query_map([tree_id], |r| uint(r, 0))?;
    rows.collect()
}

fn load_currencies(conn: &Connection, tree_id: u32) -> rusqlite::Result<Vec<TraitCurrency>> {
    let mut stmt = conn.prepare(
        "SELECT c.ID, c.Flags FROM trait_tree_x_trait_currency t
         JOIN trait_currency c ON c.ID = t.TraitCurrencyID
         WHERE t.TraitTreeID = ?1 ORDER BY t._Index, t.ID",
    )?;
    let mut currencies = stmt
        .query_map([tree_id], |r| {
            Ok(TraitCurrency {
                id: uint(r, 0)?,
                flags: uint(r, 1)?,
                sources: Vec::new(),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut sources = conn.prepare_cached(
        "SELECT Amount, QuestID, AchievementID, PlayerLevel, TraitNodeEntryID
         FROM trait_currency_source WHERE TraitCurrencyID = ?1 ORDER BY OrderIndex, ID",
    )?;
    for currency in &mut currencies {
        currency.sources = sources
            .query_map([currency.id], |r| {
                Ok(CurrencySource {
                    amount: int(r, 0)? as i32,
                    quest_id: uint(r, 1)?,
                    achievement_id: uint(r, 2)?,
                    player_level: uint(r, 3)?,
                    node_entry_id: uint(r, 4)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
    }
    Ok(currencies)
}

fn load_starter_builds(conn: &Connection, tree_id: u32) -> rusqlite::Result<Vec<StarterBuild>> {
    let mut statement = conn.prepare(
        "SELECT ID, ChrSpecializationID FROM trait_tree_loadout WHERE TraitTreeID = ?1 ORDER BY ID",
    )?;
    let loadouts = statement.query_map([tree_id], |row| Ok((uint(row, 0)?, uint(row, 1)?)))?;
    let mut builds: Vec<StarterBuild> = Vec::new();
    let mut entries = conn.prepare(
        "SELECT SelectedTraitNodeID, SelectedTraitNodeEntryID, NumPoints
         FROM trait_tree_loadout_entry WHERE TraitTreeLoadoutID = ?1 ORDER BY OrderIndex, ID",
    )?;
    for loadout in loadouts {
        let (id, spec_id): (u32, u32) = loadout?;
        let purchases = entries
            .query_map([id], |row| {
                Ok(StarterPurchase {
                    node_id: uint(row, 0)?,
                    entry_id: uint(row, 1)?,
                    ranks: uint(row, 2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if purchases.is_empty() {
            continue;
        }
        // TraitMgr::Load keeps the last populated authored loadout per specialization.
        builds.retain(|build| build.spec_id != spec_id);
        builds.push(StarterBuild { spec_id, purchases });
    }
    Ok(builds)
}

fn load_sub_trees(conn: &Connection, tree_id: u32) -> rusqlite::Result<Vec<TraitSubTree>> {
    let mut stmt = conn.prepare(
        "SELECT ID, COALESCE(Name_lang, '') FROM trait_sub_tree WHERE TraitTreeID = ?1 ORDER BY ID",
    )?;
    let rows = stmt.query_map([tree_id], |r| {
        Ok(TraitSubTree {
            id: uint(r, 0)?,
            name: r.get(1)?,
        })
    })?;
    rows.collect()
}
