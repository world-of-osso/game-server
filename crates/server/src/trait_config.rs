//! Trait (talent) configuration rules for a class `TraitTree`: granted entries,
//! point budgets and validation. Ported from TrinityCore `TraitMgr`
//! (`ValidateConfig`, `MeetsTraitCondition`, `GetGrantedTraitEntriesForConfig`).
//! Rules and data evidence: `docs/wiki/systems/talents.md`.

use std::collections::HashMap;

mod professions;
pub(crate) use professions::{
    ProfessionTraitProgress, build_profession_config, check_profession_purchases,
    profession_unspent, spent_currency_totals,
};
use professions::{achievement_is_earned, profession_currency_amount, quest_is_rewarded};

use crate::spell_info::{
    COND_FLAG_GATE, COND_FLAG_SUFFICIENT, TraitCond, TraitCondType, TraitCost, TraitEdgeType,
    TraitNode, TraitNodeEntry, TraitNodeType, TraitTree,
};

/// Whose config is checked: the config's specialization and the player's level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraitContext {
    pub spec_id: u32,
    pub level: u8,
}

/// One node entry of a config. `ranks` are bought with currency; `granted`
/// ranks come from `Granted` conditions and cost nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigEntry {
    pub node_id: u32,
    pub entry_id: u32,
    pub ranks: u32,
    pub granted: u32,
}

impl ConfigEntry {
    pub fn total(&self) -> u32 {
        self.ranks + self.granted
    }
}

/// Bought ranks of one node entry, as stored.
#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Purchase {
    pub node_id: u32,
    pub entry_id: u32,
    pub ranks: u32,
}

/// Currency spent by a config, bucketed by the gate requirement of the node
/// it was spent on (TrinityCore `SpentCurrency::ByGate`).
#[derive(Debug, Default)]
struct Spent {
    total: u32,
    by_gate: Vec<(u32, u32)>,
}

type SpentMap = HashMap<u32, Spent>;

struct Checker<'a> {
    tree: &'a TraitTree,
    ctx: TraitContext,
    entries: &'a [ConfigEntry],
    spent: SpentMap,
    progress: Option<&'a ProfessionTraitProgress>,
}

/// Ranks granted for free at this spec and level, capped at each entry's max ranks.
/// Granted conditions on class trees only test spec set and level, so they are
/// evaluated against an empty config.
pub fn granted_entries(tree: &TraitTree, ctx: TraitContext) -> Vec<ConfigEntry> {
    let checker = Checker::new(tree, ctx, &[]);
    granted_from_checker(tree, &checker)
}

fn granted_from_checker(tree: &TraitTree, checker: &Checker<'_>) -> Vec<ConfigEntry> {
    let mut granted: Vec<ConfigEntry> = Vec::new();
    let mut grant = |node: &TraitNode, entry_id: u32, ranks: u32| {
        let max = node.entry(entry_id).map_or(0, |entry| entry.max_ranks);
        match granted
            .iter_mut()
            .find(|e| e.node_id == node.id && e.entry_id == entry_id)
        {
            Some(existing) => existing.granted = (existing.granted + ranks).min(max),
            None => granted.push(ConfigEntry {
                node_id: node.id,
                entry_id,
                ranks: 0,
                granted: ranks.min(max),
            }),
        }
    };
    for node in &tree.nodes {
        let available = |entry_id| checker.node_meets_conditions(node, entry_id).is_ok();
        for entry in &node.entries {
            if available(entry.id) {
                for cond in checker.met_grants(&entry.conds) {
                    grant(node, entry.id, cond.granted_ranks);
                }
            }
        }
        let node_grants = checker.met_grants(&node.conds);
        let group_grants = node
            .groups
            .iter()
            .filter_map(|&id| tree.group(id))
            .flat_map(|group| checker.met_grants(&group.conds));
        for cond in node_grants.chain(group_grants) {
            for entry in node.entries.iter().filter(|entry| available(entry.id)) {
                grant(node, entry.id, cond.granted_ranks);
            }
        }
    }
    granted
}

/// Merge `purchases` with the granted entries and validate the result.
/// Returns the config's entries (granted-only entries included).
pub fn build_config(
    tree: &TraitTree,
    ctx: TraitContext,
    purchases: &[Purchase],
) -> Result<Vec<ConfigEntry>, String> {
    let entries = merge_purchases(tree, granted_entries(tree, ctx), purchases)?;
    validate(tree, ctx, &entries)?;
    Ok(entries)
}

/// TraitMgr::InitializeStarterBuildTraitConfig: visit authored OrderIndex rows,
/// use the first node entry for ID zero, buy eligible cumulative ranks only.
/// Normal eligibility is needed as authored apex rows can be affordable before level81.
pub fn starter_purchases(tree: &TraitTree, ctx: TraitContext) -> Result<Vec<Purchase>, String> {
    let build = tree
        .starter_builds
        .iter()
        .find(|build| build.spec_id == ctx.spec_id)
        .ok_or("Starter Build is not available for this specialization")?;
    let mut purchases: Vec<Purchase> = Vec::new();
    for row in &build.purchases {
        let node = tree
            .node(row.node_id)
            .ok_or_else(|| format!("Starter node {} is missing", row.node_id))?;
        let entry = if row.entry_id == 0 {
            node.entries.first()
        } else {
            node.entry(row.entry_id)
        }
        .ok_or_else(|| {
            format!(
                "Starter entry {} is missing on node {}",
                row.entry_id, row.node_id
            )
        })?;
        let mut candidate = purchases.clone();
        if let Some(purchase) = candidate
            .iter_mut()
            .find(|p| p.node_id == node.id && p.entry_id == entry.id)
        {
            purchase.ranks = row.ranks;
        } else {
            candidate.push(Purchase {
                node_id: node.id,
                entry_id: entry.id,
                ranks: row.ranks,
            });
        }
        // Affordable authored future-level rows are not yet legal purchases.
        // This is the same eligibility check used for ordinary manual purchases.
        if build_config(tree, ctx, &candidate).is_ok() {
            purchases = candidate;
        }
    }
    build_config(tree, ctx, &purchases)?;
    Ok(purchases)
}

fn merge_purchases(
    tree: &TraitTree,
    mut entries: Vec<ConfigEntry>,
    purchases: &[Purchase],
) -> Result<Vec<ConfigEntry>, String> {
    for purchase in purchases.iter().filter(|purchase| purchase.ranks > 0) {
        let node = tree.node(purchase.node_id).ok_or_else(|| {
            format!(
                "node {} is not in trait tree {}",
                purchase.node_id, tree.tree_id
            )
        })?;
        if node.entry(purchase.entry_id).is_none() {
            return Err(format!(
                "entry {} does not belong to node {}",
                purchase.entry_id, node.id
            ));
        }
        let existing = entries
            .iter_mut()
            .find(|e| e.node_id == purchase.node_id && e.entry_id == purchase.entry_id);
        match existing {
            Some(entry) if entry.ranks > 0 => {
                return Err(format!(
                    "entry {} of node {} is listed twice",
                    purchase.entry_id, purchase.node_id
                ));
            }
            Some(entry) => entry.ranks = purchase.ranks,
            None => entries.push(ConfigEntry {
                node_id: purchase.node_id,
                entry_id: purchase.entry_id,
                ranks: purchase.ranks,
                granted: 0,
            }),
        }
    }
    Ok(entries)
}

/// Currency owned at `ctx.level` minus currency spent by `entries`, per tree
/// currency in `TraitTreeXTraitCurrency` order.
pub fn unspent(tree: &TraitTree, ctx: TraitContext, entries: &[ConfigEntry]) -> Vec<(u32, i32)> {
    let spent = spent_currencies(tree, entries);
    let owned = owned_currencies(tree, ctx, entries, None);
    tree.currencies
        .iter()
        .map(|currency| {
            let owned = owned.get(&currency.id).copied().unwrap_or(0);
            let spent = spent.get(&currency.id).map_or(0, |spent| spent.total);
            (currency.id, owned - spent as i32)
        })
        .collect()
}

/// Hero sub-tree chosen by a `SubTreeSelection` node, if any.
pub fn selected_sub_tree(tree: &TraitTree, entries: &[ConfigEntry]) -> Option<u32> {
    selected_sub_trees(tree, entries).first().copied()
}

/// Entries the config applies, with a definition spell: entries with ranks
/// whose node is outside hero sub-trees or in the selected one.
pub fn talent_spell_entries<'a>(
    tree: &'a TraitTree,
    entries: &'a [ConfigEntry],
) -> impl Iterator<Item = &'a TraitNodeEntry> + 'a {
    ranked_talent_spell_entries(tree, entries).map(|(entry, _)| entry)
}

/// Applied definition spells with purchased plus granted ranks (TC ApplyTraitEntry).
pub fn ranked_talent_spell_entries<'a>(
    tree: &'a TraitTree,
    entries: &'a [ConfigEntry],
) -> impl Iterator<Item = (&'a TraitNodeEntry, u32)> + 'a {
    let sub_tree = selected_sub_tree(tree, entries);
    entries.iter().filter_map(move |config_entry| {
        let node = tree.node(config_entry.node_id)?;
        let applies = node.sub_tree_id == 0 || Some(node.sub_tree_id) == sub_tree;
        let entry = node.entry(config_entry.entry_id)?;
        (applies && config_entry.total() > 0 && entry.spell_id != 0)
            .then_some((entry, config_entry.total()))
    })
}

fn validate(tree: &TraitTree, ctx: TraitContext, entries: &[ConfigEntry]) -> Result<(), String> {
    let checker = Checker::new(tree, ctx, entries);
    for entry in entries {
        checker.check_entry(entry)?;
    }
    check_sub_trees(tree, entries)?;
    check_budget(tree, ctx, entries, &checker.spent)
}

fn selected_sub_trees(tree: &TraitTree, entries: &[ConfigEntry]) -> Vec<u32> {
    entries
        .iter()
        .filter_map(|config_entry| {
            let node = tree.node(config_entry.node_id)?;
            if node.kind != TraitNodeType::SubTreeSelection || config_entry.total() == 0 {
                return None;
            }
            node.entry(config_entry.entry_id)
                .map(|entry| entry.sub_tree_id)
        })
        .collect()
}

/// At most one hero sub-tree, and bought ranks only in the selected one.
fn check_sub_trees(tree: &TraitTree, entries: &[ConfigEntry]) -> Result<(), String> {
    let selected = selected_sub_trees(tree, entries);
    if selected.len() > 1 {
        return Err(format!(
            "only one hero talent tree may be selected, got {selected:?}"
        ));
    }
    for entry in entries.iter().filter(|entry| entry.ranks > 0) {
        let Some(node) = tree.node(entry.node_id) else {
            continue;
        };
        if node.sub_tree_id != 0 && selected.first() != Some(&node.sub_tree_id) {
            return Err(format!(
                "node {} belongs to hero talent tree {}, which is not selected",
                node.id, node.sub_tree_id
            ));
        }
    }
    Ok(())
}

fn check_budget(
    tree: &TraitTree,
    ctx: TraitContext,
    entries: &[ConfigEntry],
    spent: &SpentMap,
) -> Result<(), String> {
    let owned = owned_currencies(tree, ctx, entries, None);
    let mut spent: Vec<_> = spent.iter().filter(|(_, spent)| spent.total > 0).collect();
    spent.sort_by_key(|(currency, _)| **currency);
    for (currency, spent) in spent {
        let owned = owned.get(currency).copied().unwrap_or(0);
        if owned < spent.total as i32 {
            return Err(format!(
                "config spends {} points of currency {currency}, level {} has {owned}",
                spent.total, ctx.level
            ));
        }
    }
    Ok(())
}

/// `TraitCurrencySource` amounts unlocked at `ctx.level`. Quest and
/// achievement sources are never met (no such systems yet).
fn owned_currencies(
    tree: &TraitTree,
    ctx: TraitContext,
    entries: &[ConfigEntry],
    progress: Option<&ProfessionTraitProgress>,
) -> HashMap<u32, i32> {
    let has_entry = |entry_id| {
        entries
            .iter()
            .any(|entry| entry.entry_id == entry_id && entry.total() > 0)
    };
    tree.currencies
        .iter()
        .map(|currency| {
            let amount = currency
                .sources
                .iter()
                .filter(|source| {
                    quest_is_rewarded(progress, source.quest_id)
                        && achievement_is_earned(progress, source.achievement_id)
                        && source.player_level <= u32::from(ctx.level)
                        && (source.node_entry_id == 0 || has_entry(source.node_entry_id))
                })
                .map(|source| source.amount)
                .sum();
            let amount = profession_currency_amount(progress, currency.id, amount);
            (currency.id, amount)
        })
        .collect()
}

/// Per currency: the highest IsGate requirement among the node's and its
/// groups' conditions.
fn gate_requirements(tree: &TraitTree, node: &TraitNode) -> HashMap<u32, u32> {
    let group_conds = node
        .groups
        .iter()
        .filter_map(|&id| tree.group(id))
        .flat_map(|group| &group.conds);
    let mut gates: HashMap<u32, u32> = HashMap::new();
    for cond in node.conds.iter().chain(group_conds) {
        if cond.flags & COND_FLAG_GATE != 0 {
            let gate = gates.entry(cond.currency_id).or_default();
            *gate = (*gate).max(cond.spent_amount_required);
        }
    }
    gates
}

/// Group, entry and node costs times bought ranks, bucketed by the node's gate.
fn spent_currencies(tree: &TraitTree, entries: &[ConfigEntry]) -> SpentMap {
    let mut spent = SpentMap::new();
    for config_entry in entries.iter().filter(|entry| entry.ranks > 0) {
        let Some(node) = tree.node(config_entry.node_id) else {
            continue;
        };
        let gates = gate_requirements(tree, node);
        let group_costs = node
            .groups
            .iter()
            .filter_map(|&id| tree.group(id))
            .flat_map(|group| &group.costs);
        let entry_costs = node
            .entry(config_entry.entry_id)
            .into_iter()
            .flat_map(|entry| &entry.costs);
        let costs: Vec<&TraitCost> = group_costs.chain(entry_costs).chain(&node.costs).collect();
        for cost in costs {
            let amount = cost.amount * config_entry.ranks;
            let currency = spent.entry(cost.currency_id).or_default();
            currency.total += amount;
            let gate = gates.get(&cost.currency_id).copied().unwrap_or(0);
            match currency.by_gate.iter_mut().find(|(g, _)| *g == gate) {
                Some((_, sum)) => *sum += amount,
                None => currency.by_gate.push((gate, amount)),
            }
        }
    }
    spent
}

/// Outcome of one condition list (TrinityCore `ConditionCheckResult`).
#[derive(Default)]
struct CondListResult {
    sufficient: bool,
    failure: Option<String>,
}

impl<'a> Checker<'a> {
    fn new(tree: &'a TraitTree, ctx: TraitContext, entries: &'a [ConfigEntry]) -> Self {
        Self {
            tree,
            ctx,
            entries,
            spent: spent_currencies(tree, entries),
            progress: None,
        }
    }

    fn check_entry(&self, config_entry: &ConfigEntry) -> Result<(), String> {
        let node = self
            .tree
            .node(config_entry.node_id)
            .ok_or_else(|| format!("node {} is not in the tree", config_entry.node_id))?;
        let entry = node.entry(config_entry.entry_id).ok_or_else(|| {
            format!(
                "entry {} does not belong to node {}",
                config_entry.entry_id, node.id
            )
        })?;
        if config_entry.total() > entry.max_ranks {
            return Err(format!(
                "entry {} of node {} has {} ranks, max {}",
                entry.id,
                node.id,
                config_entry.total(),
                entry.max_ranks
            ));
        }
        if matches!(
            node.kind,
            TraitNodeType::Selection | TraitNodeType::SubTreeSelection
        ) {
            let chosen = self
                .entries
                .iter()
                .filter(|other| other.node_id == node.id)
                .count();
            if chosen != 1 {
                return Err(format!(
                    "choice node {} must have exactly one entry, got {chosen}",
                    node.id
                ));
            }
        }
        self.node_meets_conditions(node, entry.id)
            .map_err(|reason| format!("node {} is not available: {reason}", node.id))?;
        self.check_parents(node)
    }

    /// Selection nodes are filled by any one maxed entry; other nodes need
    /// every entry maxed.
    fn is_filled(&self, node: &TraitNode) -> bool {
        let maxed = |entry: &TraitNodeEntry| {
            self.entries.iter().any(|config_entry| {
                config_entry.node_id == node.id
                    && config_entry.entry_id == entry.id
                    && config_entry.total() == entry.max_ranks
            })
        };
        match node.kind {
            TraitNodeType::Selection => node.entries.iter().any(maxed),
            _ => node.entries.iter().all(maxed),
        }
    }

    fn check_parents(&self, node: &TraitNode) -> Result<(), String> {
        let visual_only = node
            .parents
            .iter()
            .all(|(_, kind)| *kind == TraitEdgeType::VisualOnly);
        if visual_only {
            return Ok(());
        }
        let mut any_filled = false;
        for &(parent_id, kind) in &node.parents {
            if kind == TraitEdgeType::VisualOnly {
                continue;
            }
            let filled = self
                .tree
                .node(parent_id)
                .is_some_and(|parent| self.is_filled(parent));
            if filled {
                any_filled = true;
            } else if kind == TraitEdgeType::RequiredForAvailability {
                return Err(format!(
                    "node {} requires node {parent_id} fully ranked",
                    node.id
                ));
            }
        }
        if any_filled {
            return Ok(());
        }
        let parents: Vec<u32> = node.parents.iter().map(|(id, _)| *id).collect();
        Err(format!(
            "node {} requires one of nodes {parents:?} fully ranked",
            node.id
        ))
    }

    /// Visible, Available and RanksAllowed conditions of the node, its groups
    /// and the entry (TrinityCore `NodeMeetsTraitConditions`).
    fn node_meets_conditions(&self, node: &TraitNode, entry_id: u32) -> Result<(), String> {
        [
            TraitCondType::Visible,
            TraitCondType::Available,
            TraitCondType::RanksAllowed,
        ]
        .into_iter()
        .try_for_each(|kind| self.meets_conditions_of_type(node, entry_id, kind))
    }

    fn meets_conditions_of_type(
        &self,
        node: &TraitNode,
        entry_id: u32,
        kind: TraitCondType,
    ) -> Result<(), String> {
        let node_ranks = self.ranks_where(|entry| entry.node_id == node.id);
        let entry_ranks =
            self.ranks_where(|entry| entry.node_id == node.id && entry.entry_id == entry_id);
        let mut lists = vec![self.check_cond_list(&node.conds, kind, node_ranks)];
        for group in node.groups.iter().filter_map(|&id| self.tree.group(id)) {
            let group_ranks = self.ranks_where(|entry| {
                self.tree
                    .node(entry.node_id)
                    .is_some_and(|other| other.groups.contains(&group.id))
            });
            lists.push(self.check_cond_list(&group.conds, kind, group_ranks));
        }
        if let Some(entry) = node.entry(entry_id) {
            lists.push(self.check_cond_list(&entry.conds, kind, entry_ranks));
        }
        if lists.iter().any(|list| list.sufficient) {
            return Ok(());
        }
        match lists.into_iter().find_map(|list| list.failure) {
            Some(reason) => Err(reason),
            None => Ok(()),
        }
    }

    /// Bought ranks of the config entries matching `filter`.
    fn ranks_where(&self, filter: impl Fn(&ConfigEntry) -> bool) -> u32 {
        self.entries
            .iter()
            .filter(|entry| filter(entry))
            .map(|entry| entry.ranks)
            .sum()
    }

    fn check_cond_list(
        &self,
        conds: &[TraitCond],
        kind: TraitCondType,
        ranks: u32,
    ) -> CondListResult {
        let mut result = CondListResult::default();
        for cond in conds.iter().filter(|cond| cond.kind == kind) {
            if kind == TraitCondType::RanksAllowed && ranks < cond.granted_ranks {
                continue;
            }
            match self.meets_cond(cond) {
                Err(reason) => {
                    result.failure.get_or_insert(reason);
                }
                Ok(()) if cond.flags & COND_FLAG_SUFFICIENT != 0 => {
                    result.sufficient = true;
                    break;
                }
                Ok(()) => {}
            }
        }
        result
    }

    fn met_grants(&self, conds: &'a [TraitCond]) -> impl Iterator<Item = &'a TraitCond> + '_ {
        conds
            .iter()
            .filter(|cond| cond.kind == TraitCondType::Granted && self.meets_cond(cond).is_ok())
    }

    /// TrinityCore `MeetsTraitCondition`.
    fn meets_cond(&self, cond: &TraitCond) -> Result<(), String> {
        if !quest_is_rewarded(self.progress, cond.quest_id) {
            return Err(format!(
                "condition {} requires quest {}",
                cond.id, cond.quest_id
            ));
        }
        if !achievement_is_earned(self.progress, cond.achievement_id) {
            return Err(format!(
                "condition {} requires achievement {}",
                cond.id, cond.achievement_id
            ));
        }
        if cond.spec_set_id != 0 && !cond.spec_set.contains(&self.ctx.spec_id) {
            return Err(format!(
                "condition {} requires a spec in spec set {} {:?}",
                cond.id, cond.spec_set_id, cond.spec_set
            ));
        }
        let targets_node = cond.node_group_id != 0 || cond.node_id != 0 || cond.node_entry_id != 0;
        if cond.currency_id != 0 {
            if targets_node {
                self.meets_spent_requirement(cond)?;
            }
        } else if targets_node {
            self.meets_rank_requirement(cond)?;
        }
        if cond.required_level != 0 && u32::from(self.ctx.level) < cond.required_level {
            return Err(format!(
                "condition {} requires level {}",
                cond.id, cond.required_level
            ));
        }
        Ok(())
    }

    /// Gates: currency spent on nodes behind lower gates must reach the requirement.
    fn meets_spent_requirement(&self, cond: &TraitCond) -> Result<(), String> {
        let spent_before: u32 = self.spent.get(&cond.currency_id).map_or(0, |spent| {
            spent
                .by_gate
                .iter()
                .filter(|(gate, _)| *gate < cond.spent_amount_required)
                .map(|(_, amount)| amount)
                .sum()
        });
        if spent_before < cond.spent_amount_required {
            return Err(format!(
                "condition {} requires {} points of currency {} spent in earlier rows, {spent_before} spent",
                cond.id, cond.spent_amount_required, cond.currency_id
            ));
        }
        Ok(())
    }

    /// Ranks bought in the referenced group, node or entry.
    fn meets_rank_requirement(&self, cond: &TraitCond) -> Result<(), String> {
        let group = self.ranks_where(|entry| {
            cond.node_group_id != 0
                && self
                    .tree
                    .node(entry.node_id)
                    .is_some_and(|node| node.groups.contains(&cond.node_group_id))
        });
        let node = self.ranks_where(|entry| entry.node_id == cond.node_id);
        let entry = self.ranks_where(|entry| entry.entry_id == cond.node_entry_id);
        let required = cond.spent_amount_required;
        if required != 0 && group < required && node < required && entry < required {
            return Err(format!(
                "condition {} requires {required} ranks in group {}, node {} or entry {}",
                cond.id, cond.node_group_id, cond.node_id, cond.node_entry_id
            ));
        }
        if required == 0 && group != 0 && node != 0 && entry != 0 {
            return Err(format!(
                "condition {} requires no ranks in group {}, node {} or entry {}",
                cond.id, cond.node_group_id, cond.node_id, cond.node_entry_id
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "trait_config_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "profession_trait_tests.rs"]
mod profession_tests;
