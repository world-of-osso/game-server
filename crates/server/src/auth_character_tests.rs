use bevy::prelude::*;
use shared::components::{CharacterAppearance, Gold, Player};

use crate::auth_character::sync_player_gold_from_inventory;
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
