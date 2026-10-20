//! Private native crafting-provenance proof. Seeds prerequisites only: no profession,
//! recipe, crafted output or origin row is granted; the native run acquires and crafts.
use crate::persistence::inventory_store;
use crate::{persistence, persistence_accounts};
use shared::protocol::CraftingOrigin;
use std::path::{Path, PathBuf};

const PANTS: u32 = 2303;
const OWNERS: [(&str, &str); 2] = [
    ("fb_moderncraft_provenance_m", "Provmodern"),
    ("fb_moderncraft_provenance_f", "Provforever"),
];

fn private_paths() -> (PathBuf, PathBuf) {
    let directory = Path::new(
        "/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/crafting-provenance-2026-10-19/native/server/data",
    );
    (
        directory.join("crafting-provenance.redb"),
        directory.join("world.db"),
    )
}

#[test]
#[ignore = "creates only the fixed private crafting provenance acceptance database"]
fn seed_private_crafting_provenance_fixture() {
    let (path, world) = private_paths();
    assert!(!path.exists(), "never replace an acceptance database");
    let database = redb::Database::create(path).unwrap();
    persistence::create_tables(&database);
    let conn =
        rusqlite::Connection::open_with_flags(&world, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let templates = crate::item_templates::load_runtime_item_templates(&world).unwrap();
    let goods = crate::merchant::load_vendor_catalog(&conn).unwrap();
    for (account, name) in OWNERS {
        let (account_id, _) =
            persistence_accounts::register_account(&database, account, "fbtest").unwrap();
        let (id, mut character) =
            persistence::create_character(&database, account_id, name, 1, 2).unwrap();
        character.level = 10;
        // In front of Simon Tanner's authored spawn (creature guid 79727).
        character.x = -8724.3;
        character.y = 475.3;
        character.z = 98.7;
        persistence::save_character(&database, id, &character);
        crate::maps::persistence::save_character_map(&database, id, 0);
        // Classic acquisition reagents plus one generic, uncrafted output control.
        for item in [(2934, 42), (2320, 1), (PANTS, 1)] {
            crate::inventory::grant_item(&database, &templates, &goods, id, item).unwrap();
        }
        let mut inventory = persistence::load_inventory(&database, id);
        inventory.gold = 1000;
        persistence::save_inventory(&database, id, &inventory);
        let record = inventory_store::load_inventory_record(&database, id).unwrap();
        assert!(
            record.crafting_origins.is_empty(),
            "generic grants carry no origin"
        );
        println!("PRIVATE CRAFTING PROVENANCE account={account} character={name} id={id}");
    }
}

#[test]
#[ignore = "requires both native crafts and the reopened server generation to have stopped"]
fn assert_private_crafting_provenance_after_reopen() {
    let (path, _) = private_paths();
    let db = redb::Database::open(path).unwrap();
    let crafted = CraftingOrigin {
        skill_line_ability_id: 1355,
        recipe_spell_id: 2153,
        crafting_quality_id: None,
    };
    for (_, name) in OWNERS {
        let id = persistence::find_character_id_by_name(&db, name).unwrap();
        let record = inventory_store::load_inventory_record(&db, id).unwrap();
        let pants: Vec<u64> = record
            .inventory
            .items
            .iter()
            .filter(|item| item.item_id == PANTS)
            .map(|item| item.item_guid)
            .collect();
        assert_eq!(pants.len(), 2, "{name}: one control and one crafted output");
        let origins: Vec<_> = pants
            .iter()
            .filter_map(|guid| record.crafting_origins.get(guid).map(|o| (*guid, *o)))
            .collect();
        assert_eq!(
            origins.len(),
            1,
            "{name}: only the crafted GUID has an origin"
        );
        assert_eq!(origins[0].1, crafted);
        assert_eq!(
            record.crafting_origins.len(),
            1,
            "{name}: no other origin rows"
        );
        let control = pants.iter().find(|guid| **guid != origins[0].0).unwrap();
        assert_eq!(
            record.inventory.gold, 940,
            "{name}: 1000 - trainer 10 - recipe 50"
        );
        let count = |item_id| {
            record
                .inventory
                .items
                .iter()
                .filter(|item| item.item_id == item_id)
                .map(|item| item.stack_count)
                .sum::<u32>()
        };
        assert_eq!((count(2934), count(2318), count(2320)), (0, 10, 0));
        let professions = persistence::load_professions(&db, id);
        assert!(professions.spells.contains(&2153));
        println!(
            "PRIVATE CRAFTING PROVENANCE REOPENED {name} crafted={} origin={:?} control={control} gold={} lines={:?}",
            origins[0].0, origins[0].1, record.inventory.gold, professions.lines
        );
    }
}
