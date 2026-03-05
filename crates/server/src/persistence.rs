use bevy::prelude::*;
use redb::{Database, ReadableDatabase, TableDefinition};
use shared::components::{Health, Player, Position};
use std::path::Path;

/// Character data table: client_id (u64) -> serialized CharacterData (bytes).
const CHARACTERS: TableDefinition<u64, &[u8]> = TableDefinition::new("characters");

/// Inventory table: PlayerId (u64) -> serialized inventory (bytes).
const INVENTORIES: TableDefinition<u64, &[u8]> = TableDefinition::new("inventories");

/// Serializable character data for persistence.
#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct CharacterData {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub health: f32,
    pub max_health: f32,
}

impl CharacterData {
    pub fn from_components(player: &Player, pos: &Position, health: &Health) -> Self {
        Self {
            name: player.name.clone(),
            x: pos.x,
            y: pos.y,
            z: pos.z,
            health: health.current,
            max_health: health.max,
        }
    }
}

/// Persistence plugin — manages redb database lifecycle and periodic saves.
pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, open_database);
        app.add_systems(FixedUpdate, save_dirty_characters);
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
    create_tables(&db);
    info!("Database opened at {}", path.display());
    commands.insert_resource(GameDatabase { db });
}

fn create_tables(db: &Database) {
    let txn = db.begin_write().expect("Failed to begin write txn");
    let _ = txn.open_table(CHARACTERS);
    let _ = txn.open_table(INVENTORIES);
    txn.commit().expect("Failed to commit initial tables");
}

/// Save a single character to the database.
pub fn save_character(db: &Database, client_id: u64, data: &CharacterData) {
    let bytes = bitcode::encode(data);
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn.open_table(CHARACTERS).expect("Failed to open characters table");
        table.insert(client_id, bytes.as_slice()).expect("Failed to insert character");
    }
    txn.commit().expect("Failed to commit character save");
}

/// Load a character from the database, if one exists for this client_id.
pub fn load_character(db: &Database, client_id: u64) -> Option<CharacterData> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn.open_table(CHARACTERS).expect("Failed to open characters table");
    let guard = table.get(client_id).expect("Failed to read character");
    guard.map(|v| bitcode::decode::<CharacterData>(v.value()).expect("Failed to decode character data"))
}

/// Marker component linking a player entity to its client_id for periodic saves.
#[derive(Component, Debug)]
pub struct ClientId(pub u64);

/// Save interval in ticks (300 ticks at 20Hz = 15 seconds).
const SAVE_INTERVAL_TICKS: u64 = 300;

/// Periodically save all player data to the database.
fn save_dirty_characters(
    db: Option<Res<GameDatabase>>,
    players: Query<(&Position, &Health, &Player, &ClientId)>,
    mut tick_count: Local<u64>,
) {
    *tick_count += 1;
    if *tick_count % SAVE_INTERVAL_TICKS != 0 {
        return;
    }
    let Some(db) = db else { return };
    for (pos, health, player, cid) in players.iter() {
        let data = CharacterData::from_components(player, pos, health);
        save_character(&db.db, cid.0, &data);
    }
    info!("Periodic save: {} characters written", players.iter().count());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_database() -> Database {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.redb");
        let db = Database::create(&path).unwrap();
        create_tables(&db);
        // Leak the tempdir so it's not deleted while db is open
        std::mem::forget(dir);
        db
    }

    fn sample_data() -> CharacterData {
        CharacterData {
            name: "TestPlayer".into(),
            x: -8949.0,
            y: -132.0,
            z: 83.0,
            health: 75.0,
            max_health: 100.0,
        }
    }

    #[test]
    fn character_data_round_trip() {
        let db = temp_database();
        let data = sample_data();
        save_character(&db, 42, &data);

        let loaded = load_character(&db, 42).expect("should find saved character");
        assert_eq!(loaded, data);
    }

    #[test]
    fn load_missing_character_returns_none() {
        let db = temp_database();
        assert!(load_character(&db, 999).is_none());
    }

    #[test]
    fn save_overwrites_existing() {
        let db = temp_database();
        let mut data = sample_data();
        save_character(&db, 1, &data);

        data.x = 100.0;
        data.health = 50.0;
        save_character(&db, 1, &data);

        let loaded = load_character(&db, 1).unwrap();
        assert_eq!(loaded.x, 100.0);
        assert_eq!(loaded.health, 50.0);
    }
}
