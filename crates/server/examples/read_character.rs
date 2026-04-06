use redb::{ReadableDatabase, TableDefinition};

#[allow(dead_code)]
mod terrain_height {
    pub fn bevy_to_wow(x: f32, y: f32, z: f32) -> (f32, f32, f32) {
        (x, y, z)
    }
}

#[path = "../src/persistence_types.rs"]
#[allow(dead_code)]
mod persistence_types;

const CHARACTERS: TableDefinition<u64, &[u8]> = TableDefinition::new("characters");
const CHARACTER_NAMES: TableDefinition<&str, u64> = TableDefinition::new("character_names");

fn main() {
    let name = std::env::args()
        .nth(1)
        .expect("usage: read_character <name>");
    let db_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "data/game.redb".to_string());
    let db = redb::Database::open(db_path).expect("open db");

    let read = db.begin_read().expect("begin read");
    let names = read.open_table(CHARACTER_NAMES).expect("open names");
    let Some(id_guard) = names.get(name.as_str()).expect("lookup name") else {
        eprintln!("character not found: {name}");
        std::process::exit(1);
    };
    let character_id = id_guard.value();

    let characters = read.open_table(CHARACTERS).expect("open characters");
    let Some(raw) = characters.get(character_id).expect("read character") else {
        eprintln!("character row missing: {name}");
        std::process::exit(1);
    };

    let data = persistence_types::CharacterData::decode_compatible(raw.value()).expect("decode");
    println!("character_id={character_id}");
    println!("name={}", data.name);
    println!("appearance={:?}", data.appearance);
    println!("equipment_appearance={:?}", data.equipment_appearance);
}
