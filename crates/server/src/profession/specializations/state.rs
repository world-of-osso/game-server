use super::data::TreeMetadata;
use super::{PlayerProfessions, SpecializationData};
use crate::spell_info::{TraitNode, TraitNodeEntry, TraitTree};
use crate::trait_config::{self, ConfigEntry, ProfessionTraitProgress, Purchase, TraitContext};
use bevy::prelude::Component;
use shared::currency::{CurrencyDef, CurrencyWallet};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Component, Debug, Clone, Default, PartialEq, Eq, bitcode::Encode, bitcode::Decode)]
pub(crate) struct SpecializationState {
    pub trees: BTreeMap<u32, Vec<Purchase>>,
}

pub(super) struct SpecializationActor<'a> {
    pub professions: &'a PlayerProfessions,
    pub currencies: &'a CurrencyWallet,
    pub achievements: &'a BTreeSet<u32>,
    pub completed_quests: BTreeSet<u32>,
    pub level: u8,
    pub spec_id: u32,
}

pub(super) struct CommitSelection {
    pub skill_line: u32,
    pub tree_id: u32,
    pub purchases: Vec<Purchase>,
}

pub(super) struct SpecializationCommit {
    pub state: SpecializationState,
    pub currencies: CurrencyWallet,
    pub professions: PlayerProfessions,
}

fn context(actor: &SpecializationActor<'_>) -> TraitContext {
    TraitContext {
        spec_id: actor.spec_id,
        level: actor.level,
    }
}

pub(super) fn purchases_in(state: &SpecializationState, tree: u32) -> &[Purchase] {
    state.trees.get(&tree).map(Vec::as_slice).unwrap_or(&[])
}

fn bought_entries(purchases: &[Purchase]) -> Vec<ConfigEntry> {
    purchases
        .iter()
        .map(|purchase| ConfigEntry {
            node_id: purchase.node_id,
            entry_id: purchase.entry_id,
            ranks: purchase.ranks,
            granted: 0,
        })
        .collect()
}

pub(super) fn progress_for(
    catalog: &SpecializationData,
    state: &SpecializationState,
    actor: &SpecializationActor<'_>,
    tree: &TreeMetadata,
) -> Result<ProfessionTraitProgress, String> {
    let current_spent = trait_config::spent_currency_totals(
        &tree.tree,
        &bought_entries(purchases_in(state, tree.tree.tree_id)),
    );
    let mut progress = ProfessionTraitProgress {
        earned_achievements: actor.achievements.clone(),
        completed_quests: actor.completed_quests.clone(),
        ..Default::default()
    };
    for (&trait_currency, &currency) in tree
        .currencies
        .iter()
        .filter(|(_, currency)| **currency != 0)
    {
        let budget = actor
            .currencies
            .get(currency)
            .checked_add(current_spent.get(&trait_currency).copied().unwrap_or(0))
            .ok_or("Knowledge budget overflow")?;
        let budget = i32::try_from(budget)
            .map_err(|_| "Knowledge budget exceeds supported trait integer range")?;
        progress.currency_balances.insert(trait_currency, budget);
    }
    progress.spent_elsewhere = spent_in_other_trees(catalog, state, tree)?;
    Ok(progress)
}

fn spent_in_other_trees(
    catalog: &SpecializationData,
    state: &SpecializationState,
    current: &TreeMetadata,
) -> Result<HashMap<u32, u32>, String> {
    let mut spent = HashMap::new();
    for (&tree_id, purchases) in &state.trees {
        let tree = catalog
            .trees
            .get(&tree_id)
            .ok_or_else(|| format!("Stored profession tree {tree_id} has no source data"))?;
        if tree_id == current.tree.tree_id || tree.skill_line != current.skill_line {
            continue;
        }
        for (currency, total) in
            trait_config::spent_currency_totals(&tree.tree, &bought_entries(purchases))
        {
            *spent.entry(currency).or_default() += total;
        }
    }
    Ok(spent)
}

pub(super) fn current_entries(
    catalog: &SpecializationData,
    state: &SpecializationState,
    actor: &SpecializationActor<'_>,
    tree: &TreeMetadata,
) -> Result<Vec<ConfigEntry>, String> {
    let progress = progress_for(catalog, state, actor, tree)?;
    trait_config::build_profession_config(
        &tree.tree,
        context(actor),
        &progress,
        purchases_in(state, tree.tree.tree_id),
    )
}

pub(super) fn calculate_commit(
    catalog: &SpecializationData,
    state: &SpecializationState,
    actor: &SpecializationActor<'_>,
    selection: &CommitSelection,
) -> Result<SpecializationCommit, String> {
    if actor.professions.rank(selection.skill_line) == 0 {
        return Err("Profession skill line is not learned".into());
    }
    let tree = catalog
        .trees
        .get(&selection.tree_id)
        .filter(|tree| tree.skill_line == selection.skill_line)
        .ok_or("Specialization tree does not belong to this profession")?;
    let previous = purchases_in(state, selection.tree_id);
    trait_config::check_profession_purchases(previous, &selection.purchases)?;
    let progress = progress_for(catalog, state, actor, tree)?;
    let entries = trait_config::build_profession_config(
        &tree.tree,
        context(actor),
        &progress,
        &selection.purchases,
    )?;
    let currencies = spend_new_knowledge(tree, previous, &entries, actor.currencies)?;
    let professions = learn_definition_spells(&tree.tree, &entries, actor.professions);
    let mut state = state.clone();
    state
        .trees
        .insert(selection.tree_id, selection.purchases.clone());
    Ok(SpecializationCommit {
        state,
        currencies,
        professions,
    })
}

fn spend_new_knowledge(
    tree: &TreeMetadata,
    previous: &[Purchase],
    entries: &[ConfigEntry],
    wallet: &CurrencyWallet,
) -> Result<CurrencyWallet, String> {
    let prior_spent = trait_config::spent_currency_totals(&tree.tree, &bought_entries(previous));
    let next_spent = trait_config::spent_currency_totals(&tree.tree, entries);
    let mut wallet = wallet.clone();
    for (&trait_currency, &currency) in tree
        .currencies
        .iter()
        .filter(|(_, currency)| **currency != 0)
    {
        let before = prior_spent.get(&trait_currency).copied().unwrap_or(0);
        let after = next_spent.get(&trait_currency).copied().unwrap_or(0);
        let delta = after
            .checked_sub(before)
            .ok_or("Committed profession knowledge cannot be refunded")?;
        wallet
            .remove(currency, delta)
            .map_err(|_| "Not enough profession knowledge")?;
    }
    Ok(wallet)
}

fn learn_definition_spells(
    tree: &TraitTree,
    entries: &[ConfigEntry],
    previous: &PlayerProfessions,
) -> PlayerProfessions {
    let mut professions = previous.clone();
    for entry in trait_config::talent_spell_entries(tree, entries) {
        if !professions.knows(entry.spell_id) {
            professions.spells.push(entry.spell_id);
        }
    }
    professions
}

pub(super) fn node_purchase_error(
    catalog: &SpecializationData,
    state: &SpecializationState,
    actor: &SpecializationActor<'_>,
    tree: &TreeMetadata,
    node: &TraitNode,
    entry: &TraitNodeEntry,
) -> Option<String> {
    let mut purchases = purchases_in(state, tree.tree.tree_id).to_vec();
    match purchases
        .iter_mut()
        .find(|purchase| purchase.node_id == node.id && purchase.entry_id == entry.id)
    {
        Some(purchase) => purchase.ranks += 1,
        None => purchases.push(Purchase {
            node_id: node.id,
            entry_id: entry.id,
            ranks: 1,
        }),
    }
    let progress = match progress_for(catalog, state, actor, tree) {
        Ok(progress) => progress,
        Err(error) => return Some(error),
    };
    trait_config::build_profession_config(&tree.tree, context(actor), &progress, &purchases).err()
}

pub(super) fn unspent(
    catalog: &SpecializationData,
    state: &SpecializationState,
    actor: &SpecializationActor<'_>,
    tree: &TreeMetadata,
) -> Result<Vec<(u32, i32)>, String> {
    let entries = current_entries(catalog, state, actor, tree)?;
    let progress = progress_for(catalog, state, actor, tree)?;
    Ok(trait_config::profession_unspent(
        &tree.tree,
        context(actor),
        &progress,
        &entries,
    ))
}

/// Called only at the authoritative quest-reward transaction boundary.
pub(super) fn award_knowledge(
    catalog: &SpecializationData,
    professions: &PlayerProfessions,
    wallet: &CurrencyWallet,
    quest: u32,
) -> Result<CurrencyWallet, String> {
    let mut wallet = wallet.clone();
    for reward in catalog.rewards.get(&quest).into_iter().flatten() {
        let source = catalog
            .knowledge
            .get(&reward.currency)
            .expect("reward currency source loaded");
        if !source
            .skill_lines
            .iter()
            .any(|&skill| professions.rank(skill) > 0)
        {
            return Err("Knowledge reward profession is not learned".into());
        }
        // The wallet uses only the source identity/caps; its static name is not displayed.
        let definition = CurrencyDef {
            id: reward.currency,
            name: "Profession knowledge",
            cap: source.cap,
            weekly_cap: source.weekly_cap,
            account_wide: false,
        };
        wallet
            .add(reward.currency, reward.quantity, &definition)
            .map_err(|_| "Invalid profession knowledge currency")?;
    }
    Ok(wallet)
}
