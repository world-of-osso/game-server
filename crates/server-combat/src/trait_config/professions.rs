//! Profession configurations use earned knowledge and source-earned unlocks,
//! never the level-derived budgets used by class talent trees.
use super::*;
use std::collections::BTreeSet;

#[derive(Debug, Default, Clone)]
pub(crate) struct ProfessionTraitProgress {
    /// Total budget for this tree: remaining wallet quantity plus its existing spend.
    pub currency_balances: HashMap<u32, i32>,
    pub completed_quests: BTreeSet<u32>,
    pub earned_achievements: BTreeSet<u32>,
    /// Derived tokens already used by another tree of the same profession.
    pub spent_elsewhere: HashMap<u32, u32>,
}

pub(super) fn quest_is_rewarded(progress: Option<&ProfessionTraitProgress>, quest: u32) -> bool {
    quest == 0 || progress.is_some_and(|progress| progress.completed_quests.contains(&quest))
}

pub(super) fn achievement_is_earned(
    progress: Option<&ProfessionTraitProgress>,
    achievement: u32,
) -> bool {
    achievement == 0
        || progress.is_some_and(|progress| progress.earned_achievements.contains(&achievement))
}

pub(super) fn profession_currency_amount(
    progress: Option<&ProfessionTraitProgress>,
    currency: u32,
    sourced: i32,
) -> i32 {
    let Some(progress) = progress else {
        return sourced;
    };
    if let Some(balance) = progress.currency_balances.get(&currency) {
        return *balance;
    }
    sourced
        - progress
            .spent_elsewhere
            .get(&currency)
            .copied()
            .unwrap_or(0) as i32
}

pub(crate) fn check_profession_purchases(
    previous: &[Purchase],
    purchases: &[Purchase],
) -> Result<(), String> {
    for prior in previous {
        let rank = purchases
            .iter()
            .find(|purchase| {
                purchase.node_id == prior.node_id && purchase.entry_id == prior.entry_id
            })
            .map_or(0, |purchase| purchase.ranks);
        if rank < prior.ranks {
            return Err("Committed profession knowledge cannot be refunded".into());
        }
    }
    Ok(())
}

pub(crate) fn spent_currency_totals(
    tree: &TraitTree,
    entries: &[ConfigEntry],
) -> HashMap<u32, u32> {
    spent_currencies(tree, entries)
        .into_iter()
        .map(|(currency, spent)| (currency, spent.total))
        .collect()
}

fn checker_with_progress<'a>(
    tree: &'a TraitTree,
    ctx: TraitContext,
    entries: &'a [ConfigEntry],
    progress: &'a ProfessionTraitProgress,
) -> Checker<'a> {
    let mut checker = Checker::new(tree, ctx, entries);
    checker.progress = Some(progress);
    checker
}

fn merge_granted(entries: &[ConfigEntry], grants: &[ConfigEntry]) -> Vec<ConfigEntry> {
    let mut merged = entries.to_vec();
    for grant in grants {
        match merged
            .iter_mut()
            .find(|entry| entry.node_id == grant.node_id && entry.entry_id == grant.entry_id)
        {
            Some(entry) => entry.granted = entry.granted.max(grant.granted),
            None => merged.push(*grant),
        }
    }
    merged
}

/// Purchased entries first, then source-granted perks and their unlock currencies.
pub(crate) fn build_profession_config(
    tree: &TraitTree,
    ctx: TraitContext,
    progress: &ProfessionTraitProgress,
    purchases: &[Purchase],
) -> Result<Vec<ConfigEntry>, String> {
    let mut entries = merge_purchases(tree, Vec::new(), purchases)?;
    loop {
        let checker = checker_with_progress(tree, ctx, &entries, progress);
        let grants = granted_from_checker(tree, &checker);
        let merged = merge_granted(&entries, &grants);
        if merged == entries {
            break;
        }
        entries = merged;
    }
    validate_profession_entries(tree, ctx, progress, &entries)?;
    Ok(entries)
}

fn validate_profession_entries(
    tree: &TraitTree,
    ctx: TraitContext,
    progress: &ProfessionTraitProgress,
    entries: &[ConfigEntry],
) -> Result<(), String> {
    let checker = checker_with_progress(tree, ctx, entries, progress);
    for entry in entries {
        checker.check_entry(entry)?;
        check_profession_path_unlock(tree, entries, entry)?;
    }
    check_sub_trees(tree, entries)?;
    for (currency, available) in profession_unspent(tree, ctx, progress, entries) {
        if available < 0 {
            return Err(format!(
                "Not enough profession currency {currency}: short by {}",
                -available
            ));
        }
    }
    Ok(())
}

fn check_profession_path_unlock(
    tree: &TraitTree,
    entries: &[ConfigEntry],
    entry: &ConfigEntry,
) -> Result<(), String> {
    let node = tree
        .node(entry.node_id)
        .expect("entry checked against tree");
    if node.kind != TraitNodeType::Tiered || entry.ranks == 0 {
        return Ok(());
    }
    let position = node
        .entries
        .iter()
        .position(|candidate| candidate.id == entry.entry_id)
        .expect("entry checked against node");
    for earlier in &node.entries[..position] {
        let unlocked = entries.iter().any(|candidate| {
            candidate.node_id == node.id
                && candidate.entry_id == earlier.id
                && candidate.total() == earlier.max_ranks
        });
        if !unlocked {
            return Err(format!(
                "Profession path {} must be unlocked before spending knowledge",
                node.id
            ));
        }
    }
    Ok(())
}

pub(crate) fn profession_unspent(
    tree: &TraitTree,
    ctx: TraitContext,
    progress: &ProfessionTraitProgress,
    entries: &[ConfigEntry],
) -> Vec<(u32, i32)> {
    let spent = spent_currency_totals(tree, entries);
    let owned = owned_currencies(tree, ctx, entries, Some(progress));
    tree.currencies
        .iter()
        .map(|currency| {
            let owned = owned.get(&currency.id).copied().unwrap_or(0);
            let spent = spent.get(&currency.id).copied().unwrap_or(0);
            (currency.id, owned - spent as i32)
        })
        .collect()
}
