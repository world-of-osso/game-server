//! Private native recipe-cooldown proof. The seed learns Prismatic Sphere28027 through the
//! real trainer path in a test harness and copies those professions to the private
//! characters with its reagents and tool; the native run performs the craft.
use super::spell_cooldown_persistence_tests::{install_spells, learn_prismatic_sphere};
use super::*;
use crate::persistence_accounts;
use std::path::{Path, PathBuf};

const SPHERE: u32 = 28027;
const OWNERS: [(&str, &str); 2] = [
    ("fb_moderncraft_cooldown_m", "Cdmodern"),
    ("fb_moderncraft_cooldown_f", "Cdforever"),
];

fn private_paths() -> (PathBuf, PathBuf) {
    let directory = Path::new(
        "/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/recipe-cooldown-2026-10-20/native/server/data",
    );
    (
        directory.join("recipe-cooldown.redb"),
        directory.join("world.db"),
    )
}

#[test]
#[ignore = "creates only the fixed private recipe cooldown acceptance database"]
fn seed_private_recipe_cooldown_fixture() {
    let (path, world) = private_paths();
    assert!(!path.exists(), "never replace an acceptance database");
    let mut h = harness(10).unwrap();
    install_spells(&mut h.app, &h.conn);
    learn_prismatic_sphere(&mut h);
    let professions = h.professions();
    let database = redb::Database::create(path).unwrap();
    persistence::create_tables(&database);
    let conn = Connection::open_with_flags(&world, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let templates = crate::item_templates::load_runtime_item_templates(&world).unwrap();
    let goods = load_vendor_catalog(&conn).unwrap();
    for (account, name) in OWNERS {
        let (account_id, _) =
            persistence_accounts::register_account(&database, account, "fbtest").unwrap();
        let (id, mut character) =
            persistence::create_character(&database, account_id, name, 1, 1).unwrap();
        character.level = 10;
        persistence::save_character(&database, id, &character);
        persistence::save_professions(&database, id, &professions);
        // Three spheres' worth of shards (4 each) and the category-41 Runed Fel Iron Rod.
        for item in [(22449, 12), (22461, 1)] {
            crate::inventory::grant_item(&database, &templates, &goods, id, item).unwrap();
        }
        println!(
            "PRIVATE RECIPE COOLDOWN account={account} character={name} id={id} lines={:?}",
            professions.lines
        );
    }
}

#[test]
#[ignore = "requires the native crafts and a stopped server generation"]
fn assert_private_recipe_cooldown_persisted() {
    let (path, _) = private_paths();
    let db = redb::Database::open(path).unwrap();
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    for (_, name) in OWNERS {
        let id = persistence::find_character_id_by_name(&db, name).unwrap();
        let (spells, categories) = crate::spell_cast::history_store::stored_deadlines(&db, id);
        println!("PRIVATE RECIPE COOLDOWN ROWS {name} spells={spells:?} categories={categories:?}");
        let spell = spells.iter().find(|(spell, _)| *spell == SPHERE).copied();
        let category = categories
            .iter()
            .find(|(category, _, _)| *category == 1174)
            .copied();
        let (_, ready) = spell.unwrap_or_else(|| panic!("{name}: no 28027 deadline"));
        assert!(ready > now_ms && ready <= now_ms + 172_800_000, "{name}");
        assert_eq!(
            category.map(|(_, spell, at)| (spell, at)),
            Some((SPHERE, ready))
        );
        let count = |item_id| {
            persistence::load_inventory(&db, id)
                .items
                .iter()
                .filter(|item| item.item_id == item_id)
                .map(|item| item.stack_count)
                .sum::<u32>()
        };
        assert_eq!(
            (count(22460), count(22449)),
            (1, 8),
            "{name}: exactly one sphere crafted"
        );
        println!(
            "PRIVATE RECIPE COOLDOWN PERSISTED {name} remaining_ms={}",
            ready - now_ms
        );
    }
}
