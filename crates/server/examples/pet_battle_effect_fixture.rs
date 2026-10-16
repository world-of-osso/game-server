//! Seed only the owned private petbattle-effects journal, never a realm database.
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use rusqlite::{Connection, OpenFlags};
use server::pet_battle::{Catalog, calculate_stats};
use shared::pet_battle::{PetJournal, PetQuality};
use std::{collections::BTreeMap, path::Path};

fn fixture_abilities(catalog: &Catalog, species: u32, level: u8) -> [u32; 3] {
    let mut abilities = std::array::from_fn(|slot| {
        catalog
            .species_abilities
            .iter()
            .filter(|row| row.species == species && row.slot == slot as i32 && row.level <= level)
            .min_by_key(|row| (row.level, row.ability))
            .map_or(0, |row| row.ability)
    });
    if species == 1348 {
        let fade = catalog
            .species_abilities
            .iter()
            .find(|row| row.species == species && row.ability == 1084 && row.level <= level)
            .expect("Authored Fade option must be unlocked");
        abilities[fade.slot as usize] = fade.ability;
    }
    abilities
}

fn seed_fixture_pets(catalog: &Catalog) -> (PetJournal, BTreeMap<u64, serde_json::Value>) {
    let mut journal = PetJournal::default();
    let mut states = BTreeMap::new();
    for (guid, species) in [(41, 1348), (42, 39), (43, 39)] {
        let level = 25;
        journal
            .add_with_guid(species, level, PetQuality::Rare, 3, guid)
            .unwrap();
        let abilities = fixture_abilities(catalog, species, level);
        let stats = calculate_stats(catalog, species, 3, 3, level).unwrap();
        states.insert(
            guid,
            serde_json::json!({"health": stats.health, "selected_abilities": abilities}),
        );
    }
    (journal, states)
}

fn save_fixture_pets(journal: &PetJournal, states: &BTreeMap<u64, serde_json::Value>) {
    let path = Path::new("data/diagnostics/petbattle-effects/game.redb");
    assert!(path.exists(), "Create the private account before seeding");
    let db = Database::open(path).expect("Open stopped private5961 database");
    let journals: TableDefinition<u64, &[u8]> = TableDefinition::new("account_pet_journals");
    let combat: TableDefinition<u64, &[u8]> = TableDefinition::new("account_pet_battle_states");
    let journal_bytes = serde_json::to_vec(journal).unwrap();
    let state_bytes = serde_json::to_vec(states).unwrap();
    let tx = db.begin_write().unwrap();
    tx.open_table(journals)
        .unwrap()
        .insert(1, journal_bytes.as_slice())
        .unwrap();
    tx.open_table(combat)
        .unwrap()
        .insert(1, state_bytes.as_slice())
        .unwrap();
    tx.commit().unwrap();
    let tx = db.begin_read().unwrap();
    let stored = tx.open_table(journals).unwrap();
    let decoded: PetJournal =
        serde_json::from_slice(stored.get(1).unwrap().unwrap().value()).unwrap();
    assert_eq!(&decoded, journal);
}

fn main() {
    let world = Connection::open_with_flags("data/world.db", OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("Open private current-build world copy read-only");
    let catalog = Catalog::load_world_db(&world).expect("Load current pet tables");
    let (journal, states) = seed_fixture_pets(&catalog);
    save_fixture_pets(&journal, &states);
    println!(
        "Seeded private account1: Jadefire Spirit1348 Fade1084 plus two Chicken39 reserves; not acquisition proof"
    );
}
