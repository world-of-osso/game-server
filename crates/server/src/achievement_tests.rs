use super::*;
use rusqlite::Connection;

#[test]
fn achievements2_account_quests_migrate_across_characters_without_changing_character_credit() {
    use shared::components::CharacterAppearance;
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let catalog = load_achievement_catalog(&conn).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (first, _) = persistence::create_character(&db, 17, "Firstalt", 1, 1).unwrap();
    let (second, _) = persistence::create_character(&db, 17, "Secondalt", 1, 1).unwrap();
    let (other, _) = persistence::create_character(&db, 18, "Otheralt", 1, 1).unwrap();
    let mut query = conn.prepare("WITH RECURSIVE tree(ID) AS (SELECT CriteriaTree FROM achievement WHERE ID=4899 UNION SELECT t.ID FROM criteria_tree t JOIN tree ON t.Parent=tree.ID) SELECT DISTINCT c.Asset FROM tree JOIN criteria_tree t ON t.ID=tree.ID JOIN criteria c ON c.ID=t.CriteriaID WHERE c.Type=27 ORDER BY c.Asset").unwrap();
    let quests: Vec<u32> = query
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(quests.len() > 2);
    let mut contributions = [
        PlayerAchievementState::default(),
        PlayerAchievementState::default(),
    ];
    for (index, quest) in quests.into_iter().enumerate() {
        catalog.update(
            &mut contributions[index % 2],
            CriteriaEvent::CompleteQuest(quest),
            None,
            1,
        );
    }
    persistence::save_achievements(&db, first, &contributions[0]);
    persistence::save_achievements(&db, second, &contributions[1]);
    let mut world = World::new();
    world.insert_resource(catalog);
    world.insert_resource(GameDatabase { db });
    world.init_resource::<crate::maps::ServerClock>();
    let player = world
        .spawn((
            Player {
                name: "Firstalt".into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            contributions[0].clone(),
        ))
        .id();
    let connection = world
        .spawn((
            Authenticated { account_id: 17 },
            PlayerLink(player),
            InWorld {
                character_id: first,
            },
        ))
        .id();
    let (snapshot, toasts) =
        update_player_events(&mut world, player, &[CriteriaEvent::ReachLevel(1)], None).unwrap();
    assert!(
        snapshot.earned_ids.contains(&4899),
        "account quests must combine both characters"
    );
    assert_eq!(
        toasts
            .iter()
            .filter(|toast| toast.achievement_id == 4899)
            .count(),
        1
    );
    assert!(
        !world
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .earned_ids
            .contains(&4899)
    );
    let saved = persistence::load_achievements(&world.resource::<GameDatabase>().db, second);
    assert_eq!(saved, contributions[1]);
    assert!(
        update_player_events(&mut world, player, &[], None)
            .unwrap()
            .1
            .is_empty()
    );
    world.entity_mut(connection).insert((
        Authenticated { account_id: 18 },
        InWorld {
            character_id: other,
        },
    ));
    world
        .entity_mut(player)
        .insert(PlayerAchievementState::default());
    assert!(
        !update_player_events(&mut world, player, &[], None)
            .unwrap()
            .0
            .earned_ids
            .contains(&4899)
    );
    drop(world);
    let db = redb::Database::open(file.path()).unwrap();
    let mut world = World::new();
    world.insert_resource(load_achievement_catalog(&conn).unwrap());
    world.insert_resource(GameDatabase { db });
    world.init_resource::<crate::maps::ServerClock>();
    let player = world
        .spawn((
            Player {
                name: "Secondalt".into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            contributions[1].clone(),
        ))
        .id();
    world.spawn((
        Authenticated { account_id: 17 },
        PlayerLink(player),
        InWorld {
            character_id: second,
        },
    ));
    let (snapshot, toasts) = update_player_events(&mut world, player, &[], None).unwrap();
    assert!(snapshot.earned_ids.contains(&4899));
    assert!(toasts.is_empty());
}

#[test]
fn achievements2_successful_spell_cast_counts_once_and_rejected_trigger_does_not() {
    let mut h = crate::spell_cast::test_harness::harness_with(vec![], &[43779], 1, false).unwrap();
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(load_achievement_catalog(&conn).unwrap());
    h.app
        .world_mut()
        .init_resource::<crate::maps::ServerClock>();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(PlayerAchievementState::default());
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 43779, None, u8::MAX);
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&1832)
    );
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 43779, None, 0);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&1832),
        Some(&1)
    );
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 43779, None, 0);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&1832),
        Some(&2)
    );
}

#[test]
fn achievements2_skill_rank_uses_current_base_rank_not_maximum_or_increment() {
    use shared::components::CharacterAppearance;
    use shared::profession::ProfessionSkillLine;
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let mut app = App::new();
    app.add_plugins(AchievementPlugin);
    app.insert_resource(load_achievement_catalog(&conn).unwrap());
    let line = ProfessionSkillLine {
        skill_line: 171,
        rank: 299,
        max_rank: 450,
        step: 1,
    };
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Skilled".into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            PlayerAchievementState::default(),
            crate::profession::PlayerProfessions {
                lines: vec![line],
                spells: vec![],
            },
        ))
        .id();
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&5565),
        Some(&299)
    );
    app.world_mut()
        .get_mut::<crate::profession::PlayerProfessions>(player)
        .unwrap()
        .lines[0]
        .rank = 250;
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&5565),
        Some(&250)
    );
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&5565),
        Some(&250)
    );
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .contains_key(&5566)
    );
}

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
