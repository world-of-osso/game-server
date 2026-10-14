use super::*;
use crate::persistence;

#[test]
fn currency_ack_waits_for_fifo_without_blocking_updates() {
    use lightyear::connection::host::HostClient;
    use std::time::{Duration, Instant};
    let db = persistence::temp_database();
    let (character, _) = persistence::create_character(&db, 1, "Fifocoin", 1, 1).unwrap();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.register_message::<EarnCurrency>();
    app.register_message::<SpendCurrency>();
    app.register_message::<CurrencyStateUpdate>();
    app.insert_resource(GameDatabase::new(db))
        .add_plugins(CurrencyPlugin);
    app.finish();
    app.cleanup();
    let player = app.world_mut().spawn(PlayerCurrencies::default()).id();
    let connection = app
        .world_mut()
        .spawn((
            Connected,
            RemoteId(PeerId::Local(1)),
            HostClient { buffer: Vec::new() },
            Authenticated { account_id: 1 },
            InWorld {
                character_id: character,
            },
            PlayerLink(player),
            MessageSender::<EarnCurrency>::default(),
            MessageReceiver::<EarnCurrency>::default(),
            MessageSender::<SpendCurrency>::default(),
            MessageReceiver::<SpendCurrency>::default(),
            MessageSender::<CurrencyStateUpdate>::default(),
            MessageReceiver::<CurrencyStateUpdate>::default(),
        ))
        .id();
    app.update();
    app.world_mut()
        .get_mut::<MessageReceiver<CurrencyStateUpdate>>(connection)
        .unwrap()
        .receive()
        .for_each(drop);
    let held = app
        .world()
        .resource::<GameDatabase>()
        .db
        .begin_write()
        .unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        held.abort().unwrap();
    });
    app.world_mut()
        .get_mut::<MessageSender<EarnCurrency>>(connection)
        .unwrap()
        .send::<CurrencyChannel>(EarnCurrency {
            currency_id: 3,
            amount: 17,
        });
    let started = Instant::now();
    app.update();
    app.update();
    let elapsed = started.elapsed();
    let premature = app
        .world_mut()
        .get_mut::<MessageReceiver<CurrencyStateUpdate>>(connection)
        .unwrap()
        .receive()
        .next();
    release.join().unwrap();
    assert!(
        elapsed < Duration::from_millis(50),
        "currency tick waited: {elapsed:?}"
    );
    assert!(premature.is_none(), "currency reply precedes commit");
    let deadline = Instant::now() + Duration::from_secs(3);
    let response = loop {
        app.update();
        if let Some(response) = app
            .world_mut()
            .get_mut::<MessageReceiver<CurrencyStateUpdate>>(connection)
            .unwrap()
            .receive()
            .next()
        {
            break response;
        }
        assert!(Instant::now() < deadline, "no committed currency response");
        std::thread::sleep(Duration::from_millis(1));
    };
    assert!(response.error.is_none());
    assert_eq!(
        persistence::load_currencies(&app.world().resource::<GameDatabase>().db, character).get(3),
        17
    );
    assert_eq!(
        app.world()
            .get::<PlayerCurrencies>(player)
            .unwrap()
            .0
            .get(3),
        17
    );
}

#[test]
fn legacy_currency_reader_normalizes_without_waiting_for_the_writer() {
    use std::time::{Duration, Instant};
    let db = persistence::temp_database();
    let wallet = CurrencyWallet {
        entries: vec![shared::currency::CurrencyEntry {
            id: 1,
            amount: 37,
            week_earned: 37,
        }],
    };
    persistence::save_currencies(&db, 42, &wallet);
    let held = db.begin_write().unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        held.abort().unwrap();
    });
    let started = Instant::now();
    let wallet = persistence::load_currencies(&db, 42);
    let elapsed = started.elapsed();
    release.join().unwrap();
    assert!(
        elapsed < Duration::from_millis(50),
        "legacy currency read waited: {elapsed:?}"
    );
    assert_eq!(wallet.get(1792), 37);
    assert_eq!(wallet.get(1), 0);
}

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

    persistence::save_currencies(&db, 42, &wallet);
    let loaded = persistence::load_currencies(&db, 42);

    assert_eq!(loaded, wallet);
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
        persistence::save_currencies(&db, 42, &migrated);
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
