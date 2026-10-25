use super::*;
use crate::spell_info::load_trait_tree_by_id;
use rusqlite::{Connection, OpenFlags};

fn leatherworking_tree() -> TraitTree {
    let conn = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    load_trait_tree_by_id(&conn, 350).unwrap()
}

fn context() -> TraitContext {
    TraitContext {
        spec_id: 0,
        level: 80,
    }
}

fn unlock() -> Purchase {
    Purchase {
        node_id: 28546,
        entry_id: 36338,
        ranks: 1,
    }
}
fn points(rank: u32) -> Purchase {
    Purchase {
        node_id: 28546,
        entry_id: 36327,
        ranks: rank,
    }
}

#[test]
fn profession_tab_requires_authored_achievement_not_character_level() {
    let tree = leatherworking_tree();
    let mut progress = ProfessionTraitProgress::default();
    assert!(build_profession_config(&tree, context(), &progress, &[unlock()]).is_err());
    progress.earned_achievements.insert(15876);
    let entries = build_profession_config(&tree, context(), &progress, &[unlock()]).unwrap();
    assert_eq!(
        entries.iter().find(|e| e.entry_id == 36338).unwrap().ranks,
        1
    );
    progress.spent_elsewhere.insert(19, 1);
    assert!(build_profession_config(&tree, context(), &progress, &[unlock()]).is_err());
}

#[test]
fn profession_knowledge_is_wallet_backed_and_cannot_skip_path_unlock() {
    let tree = leatherworking_tree();
    let mut progress = ProfessionTraitProgress::default();
    progress.earned_achievements.insert(15876);
    assert!(build_profession_config(&tree, context(), &progress, &[unlock(), points(1)]).is_err());
    progress.currency_balances.insert(9, 5);
    assert!(build_profession_config(&tree, context(), &progress, &[points(5)]).is_err());
    let entries =
        build_profession_config(&tree, context(), &progress, &[unlock(), points(5)]).unwrap();
    assert_eq!(spent_currency_totals(&tree, &entries).get(&9), Some(&5));
    assert_eq!(
        profession_unspent(&tree, context(), &progress, &entries)
            .iter()
            .find(|(id, _)| *id == 9),
        Some(&(9, 0))
    );
    assert!(build_profession_config(&tree, context(), &progress, &[unlock(), points(6)]).is_err());
}

#[test]
fn profession_perks_grant_only_from_points_in_the_authored_path() {
    let tree = leatherworking_tree();
    let mut progress = ProfessionTraitProgress::default();
    progress.earned_achievements.insert(15876);
    progress.currency_balances.insert(9, 5);
    let entries =
        build_profession_config(&tree, context(), &progress, &[unlock(), points(5)]).unwrap();
    assert_eq!(
        entries.iter().find(|e| e.node_id == 28534).unwrap().granted,
        1
    );
    assert_eq!(
        entries.iter().find(|e| e.node_id == 28535).unwrap().granted,
        1
    );
    assert!(entries.iter().all(|e| e.node_id != 28536 || e.granted == 0));
    assert!(entries.iter().all(|e| e.node_id != 28532));
}

#[test]
fn committed_profession_points_cannot_be_refunded_or_omitted() {
    let previous = vec![unlock(), points(5)];
    assert!(check_profession_purchases(&previous, &[unlock(), points(4)]).is_err());
    assert!(check_profession_purchases(&previous, &[unlock()]).is_err());
    assert!(check_profession_purchases(&previous, &previous).is_ok());
    assert!(check_profession_purchases(&previous, &[unlock(), points(6)]).is_ok());
}
