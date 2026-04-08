use super::*;
use crate::persistence;

#[test]
fn earn_currency_adds_to_wallet() {
    let mut wallet = CurrencyWallet::default();

    let message = earn_currency(&mut wallet, 1, 125).expect("earn currency");

    assert_eq!(wallet.get(1), 125);
    assert_eq!(message, "earned 125 Honor");
}

#[test]
fn spend_currency_removes_from_wallet() {
    let mut wallet = CurrencyWallet::default();
    wallet
        .add(1, 200, shared::currency::currency_by_id(1).unwrap())
        .unwrap();

    let message = spend_currency(&mut wallet, 1, 50).expect("spend currency");

    assert_eq!(wallet.get(1), 150);
    assert_eq!(message, "spent 50 Honor");
}

#[test]
fn spend_currency_rejects_insufficient_balance() {
    let mut wallet = CurrencyWallet::default();

    let error = spend_currency(&mut wallet, 1, 50).expect_err("insufficient");

    assert_eq!(error, "insufficient Honor");
}

#[test]
fn build_currency_snapshot_uses_currency_names() {
    let mut wallet = CurrencyWallet::default();
    wallet
        .add(2, 75, shared::currency::currency_by_id(2).unwrap())
        .unwrap();

    let snapshot = build_currency_snapshot(&wallet);

    assert_eq!(snapshot.entries.len(), 1);
    assert_eq!(snapshot.entries[0].id, 2);
    assert_eq!(snapshot.entries[0].name, "Conquest");
    assert_eq!(snapshot.entries[0].amount, 75);
}

#[test]
fn currency_wallet_round_trips_through_persistence() {
    let db = persistence::temp_database();
    let mut wallet = CurrencyWallet::default();
    wallet
        .add(1, 300, shared::currency::currency_by_id(1).unwrap())
        .unwrap();

    persistence::save_currencies(&db, 42, &wallet);
    let loaded = persistence::load_currencies(&db, 42);

    assert_eq!(loaded, wallet);
}
