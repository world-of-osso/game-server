use super::data::TreeMetadata;
use super::state;
use super::{SpecializationActor, SpecializationData, SpecializationState};
use crate::spell_info::{TraitNode, TraitNodeEntry, TraitNodeType};
use crate::trait_config::ConfigEntry;
use shared::protocol::*;
use std::collections::BTreeMap;

pub(super) fn build_snapshot(
    catalog: &SpecializationData,
    state: &SpecializationState,
    actor: &SpecializationActor<'_>,
    skill: u32,
) -> Result<ProfessionSpecializationsSnapshot, String> {
    if actor.professions.rank(skill) == 0 {
        return Err("Profession skill line is not learned".into());
    }
    let sources: Vec<_> = catalog
        .trees
        .values()
        .filter(|tree| tree.skill_line == skill)
        .collect();
    if sources.is_empty() {
        return Err("No specialization source data for this profession tier".into());
    }
    let mut currencies = BTreeMap::new();
    let mut trees = Vec::new();
    for tree in sources {
        for (currency, available) in state::unspent(catalog, state, actor, tree)? {
            currencies.insert(
                currency,
                ProfessionSpecCurrencySnapshot {
                    trait_currency_id: currency,
                    currency_id: tree.currencies[&currency],
                    available: u32::try_from(available)
                        .map_err(|_| "Stored profession config overspends its currency")?,
                },
            );
        }
        trees.push(tree_snapshot(catalog, state, actor, tree)?);
    }
    let knowledge = currencies
        .values()
        .find(|currency| currency.currency_id != 0)
        .ok_or("Missing profession knowledge currency source")?;
    Ok(ProfessionSpecializationsSnapshot {
        skill_line: skill,
        knowledge_currency_id: knowledge.currency_id,
        knowledge_available: actor.currencies.get(knowledge.currency_id),
        currencies: currencies.into_values().collect(),
        trees,
    })
}

fn tree_snapshot(
    catalog: &SpecializationData,
    state: &SpecializationState,
    actor: &SpecializationActor<'_>,
    tree: &TreeMetadata,
) -> Result<ProfessionSpecTreeSnapshot, String> {
    let entries = state::current_entries(catalog, state, actor, tree)?;
    let root = tree
        .tree
        .node(tree.root_node)
        .ok_or("Missing profession root node source")?;
    let first = root
        .entries
        .first()
        .ok_or("Missing profession root entry source")?;
    let name = catalog
        .nodes
        .get(&first.id)
        .ok_or("Missing profession root name source")?
        .name
        .clone();
    let unlocked = entries
        .iter()
        .any(|entry| entry.node_id == root.id && entry.entry_id == first.id && entry.total() > 0);
    let mut nodes = Vec::new();
    for node in &tree.tree.nodes {
        for entry in &node.entries {
            nodes.push(node_snapshot(
                catalog, state, actor, tree, node, entry, &entries,
            )?);
        }
    }
    Ok(ProfessionSpecTreeSnapshot {
        tree_id: tree.tree.tree_id,
        name,
        root_node_id: root.id,
        unlocked,
        nodes,
        edges: tree.edges.clone(),
    })
}

fn node_snapshot(
    catalog: &SpecializationData,
    state: &SpecializationState,
    actor: &SpecializationActor<'_>,
    tree: &TreeMetadata,
    node: &TraitNode,
    entry: &TraitNodeEntry,
    entries: &[ConfigEntry],
) -> Result<ProfessionSpecNodeSnapshot, String> {
    let metadata = catalog
        .nodes
        .get(&entry.id)
        .ok_or("Missing profession node definition source")?;
    let rank = entries
        .iter()
        .find(|rank| rank.node_id == node.id && rank.entry_id == entry.id);
    let unavailable_reason = state::node_purchase_error(catalog, state, actor, tree, node, entry);
    let kind = match node.kind {
        TraitNodeType::Single => 0,
        TraitNodeType::Tiered => 1,
        TraitNodeType::Selection => 2,
        TraitNodeType::SubTreeSelection => 3,
    };
    Ok(ProfessionSpecNodeSnapshot {
        node_id: node.id,
        entry_id: entry.id,
        name: metadata.name.clone(),
        description: metadata.description.clone(),
        icon: metadata.icon,
        spell_id: entry.spell_id,
        rank: rank.map_or(0, |rank| rank.ranks),
        granted_rank: rank.map_or(0, |rank| rank.granted),
        max_rank: entry.max_ranks,
        pos_x: node.pos_x,
        pos_y: node.pos_y,
        kind,
        available: unavailable_reason.is_none(),
        unavailable_reason,
        costs: node_costs(tree, node, entry),
        path: catalog.paths.get(&node.id).cloned(),
        perk: catalog.perks.get(&node.id).cloned(),
    })
}

fn node_costs(
    tree: &TreeMetadata,
    node: &TraitNode,
    entry: &TraitNodeEntry,
) -> Vec<ProfessionSpecCost> {
    let mut costs = BTreeMap::new();
    let group_costs = node
        .groups
        .iter()
        .filter_map(|&id| tree.tree.group(id))
        .flat_map(|group| &group.costs);
    for cost in group_costs.chain(&node.costs).chain(&entry.costs) {
        *costs.entry(cost.currency_id).or_default() += cost.amount;
    }
    costs
        .into_iter()
        .map(|(trait_currency_id, amount)| ProfessionSpecCost {
            trait_currency_id,
            amount,
        })
        .collect()
}
