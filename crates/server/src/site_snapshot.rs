use std::path::Path;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn run(_args: Vec<String>) -> Result<()> {
    Err("not implemented".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character_data::CharacterData;
    use redb::{Database, TableDefinition};
    use serde_json::Value;
    use shared::components::{CharacterAppearance, EquipmentAppearance};

    const CHARACTERS: TableDefinition<u64, &[u8]> = TableDefinition::new("characters");

    fn character(name: &str) -> CharacterData {
        CharacterData {
            name: name.into(),
            x: 1.0,
            y: 2.0,
            z: 3.0,
            health: 100.0,
            max_health: 100.0,
            race: 1,
            class: 1,
            level: 80,
            appearance: CharacterAppearance::default(),
            equipment_appearance: EquipmentAppearance::default(),
            durability: vec![],
        }
    }

    fn seed(root: &Path) {
        let db = Database::create(root.join("private.redb")).unwrap();
        let txn = db.begin_write().unwrap();
        {
            let mut table = txn.open_table(CHARACTERS).unwrap();
            for (id, name) in [(1, "FbAlpha"), (2, "FbHidden"), (3, "FbOtherRealm")] {
                table
                    .insert(id, character(name).encode_stored().as_slice())
                    .unwrap();
            }
        }
        txn.commit().unwrap();
        let world = rusqlite::Connection::open(root.join("world.db")).unwrap();
        world.execute_batch("CREATE TABLE chr_classes(ID INTEGER,Name_lang TEXT); INSERT INTO chr_classes VALUES(1,'Warrior'); CREATE TABLE chr_specialization(ID INTEGER,Name_lang TEXT); CREATE TABLE item_sparse(ID INTEGER,ItemLevel INTEGER); CREATE TABLE item(ID INTEGER,IconFileDataID INTEGER); CREATE TABLE achievement(ID INTEGER,Title_lang TEXT,Points INTEGER,IconFileID INTEGER); CREATE TABLE map(ID INTEGER,MapName_lang TEXT,InstanceType INTEGER); CREATE TABLE dungeon_encounter(ID INTEGER,Name_lang TEXT,MapID INTEGER,DifficultyID INTEGER,Bit INTEGER);").unwrap();
        std::fs::write(root.join("public.json"), "[\"1\",\"2\"]").unwrap();
        std::fs::write(root.join("opt-outs.json"), "[\"2\"]").unwrap();
        std::fs::create_dir(root.join("textures")).unwrap();
    }
    fn arguments(root: &Path, output: &str) -> Vec<String> {
        [
            "--redb",
            root.join("private.redb").to_str().unwrap(),
            "--world-db",
            root.join("world.db").to_str().unwrap(),
            "--public-characters",
            root.join("public.json").to_str().unwrap(),
            "--opt-outs",
            root.join("opt-outs.json").to_str().unwrap(),
            "--realm",
            "test",
            "--realm-name",
            "Private Proof",
            "--as-of",
            "100",
            "--textures",
            root.join("textures").to_str().unwrap(),
            "--output",
            root.join(output).to_str().unwrap(),
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    }
    #[test]
    fn site_snapshot_public_allowlist_opt_out_and_determinism() {
        let dir = tempfile::tempdir().unwrap();
        seed(dir.path());
        let before = std::fs::read(dir.path().join("private.redb")).unwrap();
        run(arguments(dir.path(), "a")).unwrap();
        run(arguments(dir.path(), "b")).unwrap();
        assert_eq!(
            before,
            std::fs::read(dir.path().join("private.redb")).unwrap()
        );
        let index: Value =
            serde_json::from_slice(&std::fs::read(dir.path().join("a/index.json")).unwrap())
                .unwrap();
        assert_eq!(index["characters"].as_array().unwrap().len(), 1);
        assert_eq!(index["characters"][0]["id"], "1");
        assert_eq!(
            index["characters"][0]["search_keys"],
            serde_json::json!(["fbalpha"])
        );
        assert!(!dir.path().join("a/characters/2.json").exists());
        assert!(!dir.path().join("a/characters/3.json").exists());
        let bytes = std::fs::read(dir.path().join("a/characters/1.json")).unwrap();
        assert_eq!(
            bytes,
            std::fs::read(dir.path().join("b/characters/1.json")).unwrap()
        );
        let page: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(page["class"]["name"], "Warrior");
        assert!(page["pvp"].is_null());
        assert!(page["item_level"].is_null());
        for forbidden in ["account_id", "email", "password", "x", "y", "z"] {
            assert!(page.get(forbidden).is_none());
        }
    }
    #[test]
    fn site_snapshot_rejects_missing_public_character_and_existing_output() {
        let dir = tempfile::tempdir().unwrap();
        seed(dir.path());
        std::fs::write(dir.path().join("public.json"), "[\"999\"]").unwrap();
        assert!(
            run(arguments(dir.path(), "a"))
                .unwrap_err()
                .to_string()
                .contains("999")
        );
        assert!(!dir.path().join("a").exists());
        std::fs::write(dir.path().join("public.json"), "[\"1\"]").unwrap();
        run(arguments(dir.path(), "a")).unwrap();
        assert!(
            run(arguments(dir.path(), "a"))
                .unwrap_err()
                .to_string()
                .contains("exists")
        );
    }
}
