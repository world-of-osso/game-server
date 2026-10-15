use redb::{ReadableDatabase, TableDefinition};
#[path = "../src/character_data.rs"]
mod character_data;

const CHARACTERS: TableDefinition<u64, &[u8]> = TableDefinition::new("characters");
const CHARACTER_NAMES: TableDefinition<&str, u64> = TableDefinition::new("character_names");
const CHARACTER_MAPS: TableDefinition<u64, u32> = TableDefinition::new("character_maps");

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

    let data = character_data::CharacterData::decode_compatible(raw.value()).expect("decode");
    let stored = data.encode_stored();
    let round_trip =
        character_data::CharacterData::decode_stored(&stored).expect("decode current storage");
    assert_eq!(
        round_trip, data,
        "current storage codec changed the inspected character"
    );
    println!("character_id={character_id}");
    println!("stored_bytes={}", stored.len());
    println!("name={}", data.name);
    println!("position_wow=({}, {}, {})", data.x, data.y, data.z);
    let maps = read
        .open_table(CHARACTER_MAPS)
        .expect("open character maps");
    let map = maps.get(character_id).expect("read character map");
    println!("map_id={:?}", map.map(|value| value.value()));
    println!("appearance={:?}", data.appearance);
    println!("equipment_appearance={:?}", data.equipment_appearance);
}
