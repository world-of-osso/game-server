//! Explicit private acceptance setup; these grants are fixtures, not acquisition proof.
use super::PlayerProfessions;
use crate::{persistence, persistence_accounts};
use shared::profession::ProfessionSkillLine;
use std::path::Path;

#[test]
#[ignore = "creates the explicitly selected private crafting proof redb"]
fn seed_private_crafting_joint_fixture() {
    let path = std::env::var("CRAFTING_PRIVATE_REDB").expect("explicit CRAFTING_PRIVATE_REDB");
    let world = std::env::var("CRAFTING_PRIVATE_WORLD").expect("explicit CRAFTING_PRIVATE_WORLD");
    let allowed = Path::new(
        "/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/crafting-2026-10-11/live/server/data",
    );
    assert_eq!(Path::new(&path), allowed.join("crafting.redb"));
    assert_eq!(Path::new(&world), allowed.join("world.db"));
    assert!(!Path::new(&path).exists(), "never replace a proof database");
    let database = redb::Database::create(&path).unwrap();
    persistence::create_tables(&database);
    let conn =
        rusqlite::Connection::open_with_flags(&world, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let templates = crate::item_templates::load_runtime_item_templates(Path::new(&world)).unwrap();
    let goods = crate::merchant::load_vendor_catalog(&conn).unwrap();
    for (username, name, worker) in [
        ("fb_crafting_m", "Craftmodern", false),
        ("fb_crafting_f", "Craftforever", false),
        ("fb_crafting_worker", "Craftworker", true),
    ] {
        let (account, _) =
            persistence_accounts::register_account(&database, username, "fbtest").unwrap();
        let (id, mut character) =
            persistence::create_character(&database, account, name, 1, 2).unwrap();
        character.level = 80;
        character.x = -8911.0;
        character.y = -133.0;
        character.z = 80.8;
        persistence::save_character(&database, id, &character);
        crate::maps::persistence::save_character_map(&database, id, 0);
        let (parent, skill, rank, max_rank, spells) = if worker {
            (197, 2540, 300, 300, vec![3908, 28208])
        } else {
            (165, 2830, 25, 100, vec![366249, 375166])
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
                        max_rank,
                    },
                    ProfessionSkillLine {
                        skill_line: skill,
                        step: 1,
                        rank,
                        max_rank,
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
                earned_ids: std::collections::BTreeSet::from([15876]),
                ..Default::default()
            };
            persistence::save_achievements(&database, id, &achievement);
            for reagent in [(22682, 10), (14048, 8), (7080, 4), (14227, 8)] {
                crate::inventory::grant_item(&database, &templates, &goods, id, reagent).unwrap();
            }
        }
        let mut inventory = persistence::load_inventory(&database, id);
        inventory.gold = 100_000;
        persistence::save_inventory(&database, id, &inventory);
        assert_eq!(
            persistence::load_professions(&database, id).rank(skill),
            rank
        );
        println!(
            "PRIVATE CRAFTING {username} {name} id={id} skill={skill} rank={rank} knowledge={} map=0 spot=-8911,-133,80.8",
            persistence::load_currencies(&database, id).get(2025)
        );
    }
}

#[test]
#[ignore = "requires the private modern crafting native run to have stopped"]
fn assert_private_modern_knowledge_commit_persisted() {
    let directory = Path::new(
        "/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/modern-crafting-2026-10-16/server/data",
    );
    let database = redb::Database::open(directory.join("modern-crafting.redb")).unwrap();
    let connection = rusqlite::Connection::open_with_flags(
        directory.join("world.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let catalog = super::load_specialization_data(&connection).unwrap();
    for name in ["Craftmodern", "Craftforever"] {
        let id = persistence::find_character_id_by_name(&database, name).unwrap();
        let professions = persistence::load_professions(&database, id);
        let currencies = persistence::load_currencies(&database, id);
        let achievements = [15876].into();
        let actor = super::SpecializationActor {
            professions: &professions,
            currencies: &currencies,
            achievements: &achievements,
            completed_quests: Default::default(),
            level: 80,
            spec_id: 70,
        };
        let state = super::store::load_state(&database, id);
        let snapshot = super::build_snapshot(&catalog, &state, &actor, 2830).unwrap();
        let tree = snapshot
            .trees
            .iter()
            .find(|tree| tree.tree_id == 350)
            .unwrap();
        let path = tree
            .nodes
            .iter()
            .find(|node| node.entry_id == 36327)
            .unwrap();
        assert!(tree.unlocked, "{name} tree350");
        assert_eq!(path.rank, 1, "{name} leather armor path");
        assert_eq!(snapshot.knowledge_available, 24, "{name} knowledge");
        assert_eq!(
            professions.rank(2830),
            25,
            "knowledge spending is not skill gain"
        );
        println!(
            "PRIVATE KNOWLEDGE PERSISTED {name} tree350=unlocked rank=1 knowledge=24 skill2830=25"
        );
    }
}
