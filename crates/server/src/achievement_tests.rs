use super::*;
use rusqlite::Connection;

#[path = "achievement_normal_lock_tests.rs"]
mod normal_locks;

#[test]
fn achievements_faction_specific_quests_grant_only_the_matching_player() {
    use shared::components::{CharacterAppearance, Player};
    for (race, expected) in [(1, true), (2, false), (24, false)] {
        let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
        let mut world = World::new();
        world.insert_resource(load_achievement_catalog(&conn).unwrap());
        world.init_resource::<crate::maps::ServerClock>();
        let player = world
            .spawn((
                Player {
                    name: "FactionQuestor".into(),
                    race,
                    class: 1,
                    appearance: CharacterAppearance::default(),
                },
                PlayerAchievementState::default(),
            ))
            .id();
        world.spawn((
            Authenticated { account_id: 1 },
            PlayerLink(player),
            InWorld { character_id: 42 },
        ));
        let mut query = conn.prepare("WITH RECURSIVE tree(ID) AS (SELECT CriteriaTree FROM achievement WHERE ID=4899 UNION SELECT t.ID FROM criteria_tree t JOIN tree ON t.Parent=tree.ID) SELECT DISTINCT c.Asset FROM tree JOIN criteria_tree t ON t.ID=tree.ID JOIN criteria c ON c.ID=t.CriteriaID WHERE c.Type=27").unwrap();
        let quests: Vec<u32> = query
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert!(!quests.is_empty());
        for quest in quests {
            record_quest_reward(&mut world, 42, quest);
        }
        let state = world.get::<PlayerAchievementState>(player).unwrap();
        assert_eq!(
            state.earned_ids.contains(&39958),
            expected,
            "Loch Modan character credit for race {race}"
        );
        assert!(
            !state.earned_ids.contains(&4899),
            "account achievement cannot grant in character scope"
        );
    }
}

#[test]
fn dungeonach_retail_level_ids_replace_placeholders() {
    let db = Connection::open(crate::test_data::world_db_path()).unwrap();
    let catalog = load_achievement_catalog(&db).unwrap();
    let mut state = PlayerAchievementState::default();
    let toasts = catalog.update(&mut state, CriteriaEvent::ReachLevel(20), None, 1);
    assert!(
        state.earned_ids.contains(&6),
        "Retail Level 10 is achievement 6"
    );
    assert!(
        state.earned_ids.contains(&7),
        "Retail Level 20 is achievement 7"
    );
    assert_eq!(
        toasts
            .iter()
            .find(|toast| toast.achievement_id == 6)
            .unwrap()
            .points,
        10
    );
}

#[test]
fn dungeonach_scheduled_instance_lock_round_trip_restores_bosses_and_encounters() {
    use crate::maps::instances::{InstanceEntrant, Now};
    use crate::maps::locks::InstanceLocks;
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let maps = crate::maps::store::load_map_store(&conn).unwrap();
    let difficulty = maps.map_difficulty(670, 2).unwrap();
    assert!(difficulty.has_reset_schedule());
    let entrant = InstanceEntrant {
        character_id: 42,
        group: None,
        leader: 42,
        difficulty_id: 2,
    };
    let now = Now {
        elapsed: 0.0,
        unix: 1_790_000_000,
    };
    let mut instances = crate::maps::InstanceManager::default();
    let copy = instances.create_map(&maps, 670, entrant, now).unwrap().map;
    let bit = maps.encounters_for_map(670, 2)[0].bit;
    instances.update_instance_lock(copy, &[(Entity::from_bits(1), 42)], 1, Some(bit), now.unix);
    let file = tempfile::NamedTempFile::new().unwrap();
    {
        let db = redb::Database::create(file.path()).unwrap();
        crate::persistence::create_tables(&db);
        crate::maps::persistence::save_instance_locks(&db, &instances.locks.saved(), &[]);
    }
    let db = redb::Database::open(file.path()).unwrap();
    let saved = crate::maps::persistence::load_instance_locks(&db);
    let mut restored = crate::maps::InstanceManager::with_locks(InstanceLocks::from_saved(saved));
    let resumed = restored
        .create_map(
            &maps,
            670,
            entrant,
            Now {
                elapsed: 1.0,
                unix: now.unix + 1,
            },
        )
        .unwrap()
        .map;
    assert_eq!(resumed, copy);
    assert_eq!(restored.initial_done_bosses(resumed), 1);
    assert_eq!(restored.completed_encounters(resumed), 1 << bit);
}
