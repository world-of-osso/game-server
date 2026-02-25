use bevy::prelude::*;
use redb::{Database, TableDefinition};
use std::path::Path;

/// Character data table: PlayerId (u64) → serialized character data (bytes).
const CHARACTERS: TableDefinition<u64, &[u8]> = TableDefinition::new("characters");

/// Inventory table: PlayerId (u64) → serialized inventory (bytes).
const INVENTORIES: TableDefinition<u64, &[u8]> = TableDefinition::new("inventories");

/// Persistence plugin — manages redb database lifecycle.
pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, open_database);
        // TODO: add save_dirty_characters to FixedUpdate on a timer
    }
}

#[derive(Resource)]
pub struct GameDatabase {
    pub db: Database,
}

fn open_database(mut commands: Commands) {
    let path = Path::new("data/game.redb");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create data directory");
    }
    let db = Database::create(path).expect("Failed to open database");

    // Ensure tables exist by starting a write transaction
    {
        let txn = db.begin_write().expect("Failed to begin write txn");
        let _ = txn.open_table(CHARACTERS);
        let _ = txn.open_table(INVENTORIES);
        txn.commit().expect("Failed to commit initial tables");
    }

    info!("Database opened at {}", path.display());
    commands.insert_resource(GameDatabase { db });
}
