use bevy::prelude::*;
use shared::components::{CharacterAppearance, Gold, Player};
use shared::currency::currency_by_id;

use crate::auth_character::load_selected_character_state;
use crate::gold::{GoldPlugin, GoldUpdateQueue};
use crate::persistence::{self, ClientId, GameDatabase};

fn test_db() -> GameDatabase {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("auth_character.redb");
    let db = redb::Database::create(&path).expect("db");
    persistence::create_tables(&db);
    std::mem::forget(dir);
    GameDatabase { db }
}

#[test]
fn queued_gold_updates_coalesce_and_skip_equal_or_offline_players() {
    let mut app = App::new();
    app.add_plugins(GoldPlugin);
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Thrall".into(),
                race: 2,
                class: 7,
                appearance: CharacterAppearance::default(),
            },
            ClientId(42),
            Gold(100),
        ))
        .id();

    {
        let mut updates = app.world_mut().resource_mut::<GoldUpdateQueue>();
        updates.record(42, 125);
        updates.record(42, 150);
    }
    app.update();

    assert_eq!(app.world().entity(player).get::<Gold>().unwrap().0, 150);
    let changed_after_update = app
        .world()
        .entity(player)
        .get_ref::<Gold>()
        .unwrap()
        .last_changed();

    {
        let mut updates = app.world_mut().resource_mut::<GoldUpdateQueue>();
        updates.record(42, 150);
        updates.record(99, 900);
    }
    app.update();

    assert_eq!(app.world().entity(player).get::<Gold>().unwrap().0, 150);
    assert_eq!(
        app.world()
            .entity(player)
            .get_ref::<Gold>()
            .unwrap()
            .last_changed(),
        changed_after_update
    );
    assert!(app.world().resource::<GoldUpdateQueue>().is_empty());
}

#[test]
fn load_selected_character_state_syncs_pvp_currencies_from_wallet() {
    let db = test_db();
    let character_id = 55;
    let mut wallet = shared::currency::CurrencyWallet::default();
    wallet.add(1, 420, currency_by_id(1).unwrap()).unwrap();
    wallet.add(2, 80, currency_by_id(2).unwrap()).unwrap();
    persistence::save_currencies(&db.db, character_id, &wallet);

    let pvp = crate::pvp::PlayerPvpProgress {
        honor: 12,
        conquest: 7,
        ..Default::default()
    };
    persistence::save_pvp(&db.db, character_id, &pvp);

    let state = load_selected_character_state(&db, character_id, 1);

    assert_eq!(state.currencies.get(1), 420);
    assert_eq!(state.currencies.get(2), 80);
    assert_eq!(state.pvp.progress.honor, 420);
    assert_eq!(state.pvp.progress.conquest, 80);
}
