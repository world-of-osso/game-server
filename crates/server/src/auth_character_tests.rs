use bevy::prelude::*;
use shared::components::{CharacterAppearance, Gold, Player};
use shared::currency::currency_by_id;

use crate::auth_character::{load_selected_character_state, sync_player_gold_from_inventory};
use crate::persistence::{self, ClientId, GameDatabase, InventoryData};

fn test_db() -> GameDatabase {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("auth_character.redb");
    let db = redb::Database::create(&path).expect("db");
    persistence::create_tables(&db);
    std::mem::forget(dir);
    GameDatabase { db }
}

#[test]
fn sync_player_gold_from_inventory_updates_replicated_gold_component() {
    let db = test_db();
    persistence::save_inventory(
        &db.db,
        42,
        &InventoryData {
            gold: 12_345,
            items: Vec::new(),
        },
    );

    let mut app = App::new();
    app.insert_resource(db);
    app.add_systems(Update, sync_player_gold_from_inventory);
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
            Gold(0),
        ))
        .id();

    app.update();

    let gold = app
        .world()
        .entity(player)
        .get::<Gold>()
        .expect("gold component");
    assert_eq!(gold.0, 12_345);
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
