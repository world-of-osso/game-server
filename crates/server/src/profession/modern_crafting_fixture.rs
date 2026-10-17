//! Explicit private test grants, never profession/knowledge acquisition evidence.
use super::PlayerProfessions;
use crate::{persistence, persistence_accounts};
use shared::profession::ProfessionSkillLine;
use std::path::{Path, PathBuf};

fn private_paths() -> (PathBuf, PathBuf) {
    let directory = Path::new(
        "/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/modern-crafting-2026-10-16/server/data",
    );
    (
        directory.join("modern-crafting.redb"),
        directory.join("world.db"),
    )
}

#[test]
#[ignore = "creates only the fixed private modern crafting acceptance database"]
fn seed_private_modern_crafting_fixture() {
    let (path, world) = private_paths();
    assert!(!path.exists(), "never replace an acceptance database");
    let database = redb::Database::create(path).unwrap();
    persistence::create_tables(&database);
    let conn =
        rusqlite::Connection::open_with_flags(&world, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let templates = crate::item_templates::load_runtime_item_templates(&world).unwrap();
    let goods = crate::merchant::load_vendor_catalog(&conn).unwrap();
    for (account, name, worker) in [
        ("fb_moderncraft_m", "Craftmodern", false),
        ("fb_moderncraft_f", "Craftforever", false),
        ("fb_moderncraft_w", "Craftworker", true),
    ] {
        let (account_id, _) =
            persistence_accounts::register_account(&database, account, "fbtest").unwrap();
        let (id, mut character) =
            persistence::create_character(&database, account_id, name, 1, 2).unwrap();
        character.level = 80;
        character.x = -8911.0;
        character.y = -133.0;
        character.z = 80.8;
        persistence::save_character(&database, id, &character);
        crate::maps::persistence::save_character_map(&database, id, 0);
        let (parent, skill, rank, cap, spells) = if worker {
            (197, 2540, 300, 300, vec![3908, 28208])
        } else {
            (165, 2830, 25, 100, vec![366249, 375188])
        };
        persistence::save_professions(
            &database,
            id,
            &PlayerProfessions {
                lines: vec![
                    ProfessionSkillLine {
                        skill_line: parent,
                        step: 1,
                        rank,
                        max_rank: cap,
                    },
                    ProfessionSkillLine {
                        skill_line: skill,
                        step: 1,
                        rank,
                        max_rank: cap,
                    },
                ],
                spells,
            },
        );
        if !worker {
            persistence::save_currencies(
                &database,
                id,
                &shared::currency::CurrencyWallet {
                    entries: vec![shared::currency::CurrencyEntry {
                        id: 2025,
                        amount: 25,
                        week_earned: 0,
                    }],
                },
            );
            let achievement = crate::achievement::PlayerAchievementState {
                earned_ids: [15876].into(),
                ..Default::default()
            };
            persistence::save_achievements(&database, id, &achievement);
            for reagent in [
                (193222, 2),
                (193208, 6),
                (193210, 4),
                (22682, 10),
                (14048, 8),
                (7080, 4),
                (14227, 8),
            ] {
                crate::inventory::grant_item(&database, &templates, &goods, id, reagent).unwrap();
            }
        }
        let mut inventory = persistence::load_inventory(&database, id);
        inventory.gold = 100_000;
        persistence::save_inventory(&database, id, &inventory);
        println!(
            "PRIVATE MODERN CRAFTING account={account} character={name} id={id} fixture_skill={skill} fixture_rank={rank}"
        );
    }
}

#[test]
#[ignore = "requires the private concentration native run to have stopped"]
fn assert_private_concentration_display_did_not_initialize_wallet() {
    let (path, _) = private_paths();
    let db = redb::Database::open(path).unwrap();
    for name in ["Craftmodern", "Craftforever"] {
        let id = persistence::find_character_id_by_name(&db, name).unwrap();
        let wallet = persistence::load_currencies(&db, id);
        assert!(wallet.entries.iter().all(|entry| entry.id != 3049));
        println!("PRIVATE CONCENTRATION {name} currency3049 remains uninitialized");
    }
}

#[test]
#[ignore = "requires all six private native orders and mail claims to have finished"]
fn assert_private_order_deliveries_and_commissions_survive_reopen() {
    let (path, _) = private_paths();
    let db = redb::Database::open(path).unwrap();
    for (name, gold, outputs) in [
        ("Craftmodern", 70_000, 3),
        ("Craftforever", 70_000, 3),
        ("Craftworker", 160_000, 0),
    ] {
        let id = persistence::find_character_id_by_name(&db, name).unwrap();
        let inventory = persistence::load_inventory(&db, id);
        let count_item = |item_id| -> u32 {
            inventory
                .items
                .iter()
                .filter(|entry| entry.item_id == item_id)
                .map(|entry| entry.stack_count)
                .sum()
        };
        assert_eq!(inventory.gold, gold, "{name} settled copper");
        assert_eq!(
            count_item(22658),
            outputs,
            "{name} delivered Glacial Cloaks"
        );
        for reagent in [22682, 14048, 7080, 14227] {
            assert_eq!(
                count_item(reagent),
                0,
                "{name} consumed order reagent {reagent}"
            );
        }
        println!(
            "PRIVATE ORDERS {name} gold={gold} output22658={outputs}; no escrow reagents remain"
        );
    }
}

#[test]
#[ignore = "requires the private native run to have stopped and released its database"]
fn assert_private_quality_selection_conserved_reagents_and_skill() {
    let (path, _) = private_paths();
    let db = redb::Database::open(path).unwrap();
    for name in ["Craftmodern", "Craftforever"] {
        let id = persistence::find_character_id_by_name(&db, name).unwrap();
        let inventory = persistence::load_inventory(&db, id);
        for (item, expected) in [(193222, 2), (193208, 6), (193210, 4)] {
            let count: u32 = inventory
                .items
                .iter()
                .filter(|entry| entry.item_id == item)
                .map(|entry| entry.stack_count)
                .sum();
            assert_eq!(count, expected, "{name} item {item}");
        }
        assert_eq!(persistence::load_professions(&db, id).rank(2830), 25);
        println!("PRIVATE QUALITY CONSERVED {name} 193222=2 193208=6 193210=4 rank2830=25");
    }
}
