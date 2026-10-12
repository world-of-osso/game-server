use super::*;
use crate::trait_config::Purchase;
use rusqlite::Connection;
use shared::currency::{CurrencyEntry, CurrencyWallet};
use shared::profession::ProfessionSkillLine;
use std::collections::BTreeSet;

fn source_connection() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(include_str!("fixture.sql")).unwrap();
    conn
}

fn source_catalog() -> SpecializationData {
    load_specialization_data(&source_connection()).unwrap()
}

fn professions() -> PlayerProfessions {
    PlayerProfessions {
        lines: vec![ProfessionSkillLine {
            skill_line: 2830,
            step: 1,
            rank: 25,
            max_rank: 100,
        }],
        spells: vec![],
    }
}

fn wallet(amount: u32) -> CurrencyWallet {
    CurrencyWallet {
        entries: vec![CurrencyEntry {
            id: 2025,
            amount,
            week_earned: amount,
        }],
    }
}

fn actor<'a>(
    profs: &'a PlayerProfessions,
    currencies: &'a CurrencyWallet,
    achievements: &'a BTreeSet<u32>,
) -> SpecializationActor<'a> {
    SpecializationActor {
        professions: profs,
        currencies,
        achievements,
        completed_quests: BTreeSet::new(),
        level: 80,
        spec_id: 70,
    }
}

fn root_purchases(ranks: u32) -> Vec<Purchase> {
    vec![
        Purchase {
            node_id: 28546,
            entry_id: 36338,
            ranks: 1,
        },
        Purchase {
            node_id: 28546,
            entry_id: 36327,
            ranks,
        },
    ]
}

#[test]
fn specialization_commit_spends_only_new_knowledge_and_rejects_refunds() {
    let catalog = source_catalog();
    let profs = professions();
    let currencies = wallet(25);
    let achievements = BTreeSet::from([15876]);
    let prior = SpecializationState::default();
    let selection = CommitSelection {
        skill_line: 2830,
        tree_id: 350,
        purchases: root_purchases(5),
    };
    let committed = calculate_commit(
        &catalog,
        &prior,
        &actor(&profs, &currencies, &achievements),
        &selection,
    )
    .unwrap();
    assert_eq!(committed.currencies.get(2025), 20);
    let again = calculate_commit(
        &catalog,
        &committed.state,
        &actor(&committed.professions, &committed.currencies, &achievements),
        &selection,
    )
    .unwrap();
    assert_eq!(again.currencies.get(2025), 20);
    let refund = CommitSelection {
        purchases: root_purchases(4),
        ..selection
    };
    assert!(
        calculate_commit(
            &catalog,
            &committed.state,
            &actor(&committed.professions, &committed.currencies, &achievements),
            &refund
        )
        .is_err()
    );
}

#[test]
fn authored_specialization_chain_learns_recipe_without_casting_it() {
    let catalog = source_catalog();
    let profs = professions();
    let currencies = wallet(25);
    let achievements = BTreeSet::from([15876]);
    let mut purchases = root_purchases(10);
    purchases.extend([
        Purchase {
            node_id: 28540,
            entry_id: 36332,
            ranks: 1,
        },
        Purchase {
            node_id: 28540,
            entry_id: 36287,
            ranks: 10,
        },
        Purchase {
            node_id: 28536,
            entry_id: 36328,
            ranks: 1,
        },
    ]);
    let selection = CommitSelection {
        skill_line: 2830,
        tree_id: 350,
        purchases,
    };
    let committed = calculate_commit(
        &catalog,
        &SpecializationState::default(),
        &actor(&profs, &currencies, &achievements),
        &selection,
    )
    .unwrap();
    assert_eq!(committed.currencies.get(2025), 5);
    assert!(committed.professions.knows(375110));
    assert_eq!(committed.professions.rank(2830), 25);
    assert!(!profs.knows(375110));
}

#[test]
fn source_snapshot_preserves_visual_edges_wallet_mapping_and_lock_reasons() {
    let catalog = source_catalog();
    let profs = professions();
    let currencies = wallet(1);
    let achievements = BTreeSet::new();
    let view = build_snapshot(
        &catalog,
        &SpecializationState::default(),
        &actor(&profs, &currencies, &achievements),
        2830,
    )
    .unwrap();
    assert_eq!(view.knowledge_currency_id, 2025);
    assert_eq!(view.knowledge_available, 1);
    assert!(
        view.currencies
            .iter()
            .any(|c| c.trait_currency_id == 9 && c.currency_id == 2025 && c.available == 1)
    );
    let tree = view.trees.iter().find(|tree| tree.tree_id == 350).unwrap();
    assert_eq!(tree.root_node_id, 28546);
    assert!(!tree.unlocked);
    assert_eq!(tree.edges.len(), 10);
    assert!(tree.edges.iter().all(|edge| edge.kind == 0));
    let root = tree
        .nodes
        .iter()
        .find(|node| node.entry_id == 36338)
        .unwrap();
    assert!(!root.available);
    assert!(root.unavailable_reason.is_some());
    assert_eq!(root.rank, 0);
    assert_eq!(root.granted_rank, 0);
    let root_perk = tree
        .nodes
        .iter()
        .find(|node| node.node_id == 28534)
        .unwrap();
    assert_eq!(root_perk.perk.as_ref().unwrap().path_node_id, 28546);
    let knowledge = tree.nodes.iter().find(|n| n.entry_id == 36327).unwrap();
    assert_eq!(
        knowledge.description,
        "Improve at making leather armor, gaining +1 Skill per point in this Specialization."
    );
    let major = tree
        .nodes
        .iter()
        .find(|n| n.node_id == 28535)
        .unwrap()
        .perk
        .as_ref()
        .unwrap();
    assert!(major.is_major_bonus);
    assert!(!major.unlocks_subpath);
    let unlock = tree
        .nodes
        .iter()
        .find(|n| n.node_id == 28533)
        .unwrap()
        .perk
        .as_ref()
        .unwrap();
    assert!(!unlock.is_major_bonus);
    assert!(unlock.unlocks_subpath);
}

#[test]
fn sourced_description_variables_do_not_invent_missing_bindings() {
    let sources = super::descriptions::load_description_sources(&source_connection()).unwrap();
    assert_eq!(sources.resolve(36464, "+$EV1 $EN1").unwrap(), "+1 Skill");
    assert!(sources.resolve(129480, "$ev1").is_err());
    assert!(sources.resolve(36464, "$ev2").is_err());
    assert!(sources.resolve(0, "$ev1").is_err());
    assert!(sources.resolve(36464, "$ev").is_err());
    assert!(sources.resolve(36464, "$unknown1").is_err());
    let catalog = source_catalog();
    assert!(
        catalog
            .nodes
            .values()
            .all(|node| !node.description.contains('$'))
    );
}

#[test]
fn source_knowledge_reward_is_one_point_not_a_level_grant() {
    let catalog = source_catalog();
    let profs = professions();
    let currencies = CurrencyWallet::default();
    let awarded = award_knowledge(&catalog, &profs, &currencies, 70191).unwrap();
    assert_eq!(awarded.get(2025), 1);
    assert_eq!(currencies.get(2025), 0);
    assert!(award_knowledge(&catalog, &PlayerProfessions::default(), &currencies, 70191).is_err());
    assert_eq!(
        award_knowledge(&catalog, &profs, &currencies, 1).unwrap(),
        currencies
    );
}

#[test]
fn specialization_state_wallet_and_recipes_survive_atomic_reopen() {
    let catalog = source_catalog();
    let profs = professions();
    let currencies = wallet(25);
    let achievements = BTreeSet::from([15876]);
    let selection = CommitSelection {
        skill_line: 2830,
        tree_id: 350,
        purchases: root_purchases(5),
    };
    let committed = calculate_commit(
        &catalog,
        &SpecializationState::default(),
        &actor(&profs, &currencies, &achievements),
        &selection,
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("specializations.redb");
    {
        let database = redb::Database::create(&path).unwrap();
        store::save_commit(&database, 42, &committed);
    }
    let database = redb::Database::open(&path).unwrap();
    assert_eq!(store::load_state(&database, 42), committed.state);
    assert_eq!(
        crate::persistence::load_currencies(&database, 42),
        committed.currencies
    );
    assert_eq!(
        crate::persistence::load_professions(&database, 42),
        committed.professions
    );
    let selection = CommitSelection {
        skill_line: 2830,
        tree_id: 350,
        purchases: root_purchases(6),
    };
    let next = calculate_commit(
        &catalog,
        &committed.state,
        &actor(&committed.professions, &committed.currencies, &achievements),
        &selection,
    )
    .unwrap();
    let transaction = database.begin_write().unwrap();
    store::write_commit_in(&transaction, 42, &next);
    transaction.abort().unwrap();
    assert_eq!(store::load_state(&database, 42), committed.state);
    assert_eq!(
        crate::persistence::load_currencies(&database, 42),
        committed.currencies
    );
    assert_eq!(
        crate::persistence::load_professions(&database, 42),
        committed.professions
    );
}
