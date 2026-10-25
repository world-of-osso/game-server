use super::*;
use crate::persistence;

#[test]
fn earn_currency_adds_to_wallet() {
    let mut wallet = CurrencyWallet::default();

    let message = earn_currency(&mut wallet, 1792, 125).expect("earn currency");

    assert_eq!(wallet.get(1792), 125);
    assert_eq!(message, "earned 125 Honor");
}

#[test]
fn spend_currency_removes_from_wallet() {
    let mut wallet = CurrencyWallet::default();
    wallet
        .add(1792, 200, shared::currency::currency_by_id(1792).unwrap())
        .unwrap();

    let message = spend_currency(&mut wallet, 1792, 50).expect("spend currency");

    assert_eq!(wallet.get(1792), 150);
    assert_eq!(message, "spent 50 Honor");
}

#[test]
fn spend_currency_rejects_insufficient_balance() {
    let mut wallet = CurrencyWallet::default();

    let error = spend_currency(&mut wallet, 1792, 50).expect_err("insufficient");

    assert_eq!(error, "insufficient Honor");
}

#[test]
fn build_currency_snapshot_uses_currency_names() {
    let mut wallet = CurrencyWallet::default();
    wallet
        .add(1602, 75, shared::currency::currency_by_id(1602).unwrap())
        .unwrap();

    let snapshot = build_currency_snapshot(&wallet);

    assert_eq!(snapshot.entries.len(), 1);
    assert_eq!(snapshot.entries[0].id, 1602);
    assert_eq!(snapshot.entries[0].name, "Conquest");
    assert_eq!(snapshot.entries[0].amount, 75);
}

#[test]
fn currency_wallet_round_trips_through_persistence() {
    let db = persistence::temp_database();
    let mut wallet = CurrencyWallet::default();
    wallet
        .add(1792, 300, shared::currency::currency_by_id(1792).unwrap())
        .unwrap();
    wallet
        .add(1602, 500, shared::currency::currency_by_id(1602).unwrap())
        .unwrap();

    persistence::save_currencies(&db, 42, &wallet);
    let loaded = persistence::load_currencies(&db, 42);

    assert_eq!(loaded, wallet);
    for currency in [1792, 1602] {
        assert_eq!(loaded.week_earned(currency), 0, "no authored weekly cap");
    }
}

#[test]
fn legacy_pvp_currency_balances_migrate_durably_without_duplicates() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("players.redb");
    {
        let db = redb::Database::create(&path).unwrap();
        persistence::create_tables(&db);
        let wallet = CurrencyWallet {
            entries: vec![
                shared::currency::CurrencyEntry {
                    id: 1,
                    amount: 300,
                    week_earned: 300,
                },
                shared::currency::CurrencyEntry {
                    id: 2,
                    amount: 80,
                    week_earned: 80,
                },
            ],
        };
        persistence::save_currencies(&db, 42, &wallet);
        let migrated = persistence::load_currencies(&db, 42);
        assert_eq!(migrated.get(1792), 300);
        assert_eq!(migrated.get(1602), 80);
        assert_eq!(migrated.get(1), 0);
        assert_eq!(migrated.entries.len(), 2);
    }
    let db = redb::Database::open(&path).unwrap();
    let wallet = persistence::load_currencies(&db, 42);
    assert_eq!(wallet.get(1792), 300);
    assert_eq!(wallet.get(1602), 80);
    assert_eq!(wallet.week_earned(1602), 80);
}

#[test]
fn pvp_currency_client_request_cannot_mint_progression() {
    let mut wallet = CurrencyWallet::default();
    for currency_id in [1792, 1602] {
        let request = EarnCurrency {
            currency_id,
            amount: 100,
        };
        let result = <EarnCurrency as CurrencyRequest>::apply(&mut wallet, &request);
        assert!(
            result.is_err(),
            "PvP currency must be awarded by a played match, not a client amount"
        );
    }
    assert!(wallet.entries.is_empty());
}
