use super::*;
use rusqlite::Connection;
use std::path::PathBuf;

fn csv_records(input: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => row.push(std::mem::take(&mut field)),
            '\n' if !quoted => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            '\r' if !quoted => (),
            _ => field.push(ch),
        }
    }
    assert!(!quoted, "unterminated CSV string");
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

fn fixture_db() -> Connection {
    let root = std::env::var_os("PET_BATTLE_DB2")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933")
        });
    let db = Connection::open_in_memory().unwrap();
    for (export, table) in [
        ("BattlePetAbility", "battle_pet_ability"),
        ("BattlePetAbilityTurn", "battle_pet_ability_turn"),
        ("BattlePetAbilityEffect", "battle_pet_ability_effect"),
        ("BattlePetEffectProperties", "battle_pet_effect_properties"),
        ("BattlePetState", "battle_pet_state"),
        ("BattlePetSpeciesState", "battle_pet_species_state"),
        ("BattlePetBreedState", "battle_pet_breed_state"),
        ("BattlePetBreedQuality", "battle_pet_breed_quality"),
        ("BattlePetSpeciesXAbility", "battle_pet_species_x_ability"),
        ("BattlePetAbilityState", "battle_pet_ability_state"),
    ] {
        let path = root.join(format!("{export}.csv"));
        let input =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let rows = csv_records(&input);
        let headers = &rows[0];
        let columns = headers
            .iter()
            .map(|h| {
                format!(
                    "\"{h}\" {} NOT NULL",
                    if h.contains("lang") || h.contains("Label") || h == "LuaName" {
                        "TEXT"
                    } else if h == "StateMultiplier" {
                        "REAL"
                    } else {
                        "INTEGER"
                    }
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        db.execute_batch(&format!("CREATE TABLE {table} ({columns});"))
            .unwrap();
        let placeholders = vec!["?"; headers.len()].join(",");
        let mut insert = db
            .prepare(&format!("INSERT INTO {table} VALUES ({placeholders})"))
            .unwrap();
        for row in &rows[1..] {
            assert_eq!(row.len(), headers.len());
            insert.execute(rusqlite::params_from_iter(row)).unwrap();
        }
    }
    db
}

#[test]
fn pet_battle_real_species39_rare_breed3_stats() {
    let catalog = Catalog::load_world_db(&fixture_db()).unwrap();
    // CASC: species39 stamina+100, power-100, speed0; breed3 each1700.
    // Rare=.65, level25: round(1800*.65*25/20)+100=1563;
    // round(1600*.65*25/100)=260; round(1700*.65*25/100)=276.
    assert_eq!(
        calculate_stats(&catalog, 39, 3, 3, 25).unwrap(),
        Stats {
            health: 1563,
            power: 260,
            speed: 276
        }
    );
}

#[test]
fn pet_battle_missing_world_tables_are_explicit() {
    let err = Catalog::load_world_db(&Connection::open_in_memory().unwrap()).unwrap_err();
    assert!(err.contains("battle_pet_ability"), "{err}");
}
