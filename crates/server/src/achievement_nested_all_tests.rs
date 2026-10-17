use super::*;
use rusqlite::Connection;

/// Exercise the native defeat handler, not Kil'jaeden combat or artifact acquisition.
/// Source prerequisite11611 is a declared persisted baseline, never an earned result.
#[test]
fn achievements_criteria_nested_all_native_encounter_respects_prerequisite_and_difficulty() {
    for (difficulty, prerequisite, expected) in [
        (15, true, Some(1)),
        (16, true, Some(1)),
        (14, true, None),
        (15, false, None),
    ] {
        let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = redb::Database::create(file.path()).unwrap();
        persistence::create_tables(&db);
        let (character, _) = persistence::create_character(&db, 1, "Nestedguard", 1, 1).unwrap();
        let (foreign, _) = persistence::create_character(&db, 2, "Otherguard", 1, 1).unwrap();
        let baseline = PlayerAchievementState {
            earned_ids: if prerequisite {
                [11611].into_iter().collect()
            } else {
                BTreeSet::new()
            },
            earned_at: if prerequisite {
                [(11611, 1)].into_iter().collect()
            } else {
                BTreeMap::new()
            },
            ..Default::default()
        };
        persistence::save_achievements(&db, character, &baseline);
        let mut h =
            crate::spell_cast::test_harness::harness_with_plugins(vec![], &[], 1, false, |app| {
                app.add_plugins(AchievementPlugin);
            })
            .unwrap();
        h.app
            .insert_resource(load_achievement_catalog(&conn).unwrap());
        let store = crate::maps::store::load_map_store(&conn).unwrap();
        let mut manager = crate::maps::InstanceManager::default();
        let map = manager
            .create_map(
                &store,
                1676,
                crate::maps::instances::InstanceEntrant {
                    character_id: character,
                    leader: character,
                    difficulty_id: difficulty,
                    raid_difficulty: crate::maps::RaidDifficulty {
                        primary: difficulty,
                        legacy: 3,
                    },
                    ..Default::default()
                },
                crate::maps::instances::Now {
                    elapsed: 0.0,
                    unix: 1791763200,
                },
            )
            .unwrap()
            .map;
        h.app.insert_resource(store);
        h.app.insert_resource(manager);
        h.app.world_mut().entity_mut(h.caster).insert((
            map,
            persistence::ClientId(character),
            persistence::load_achievements(&db, character),
        ));
        h.app
            .world_mut()
            .get_mut::<InWorld>(h.connection)
            .unwrap()
            .character_id = character;
        h.app.insert_resource(GameDatabase::new(db));
        h.advance(1);
        assert!(
            !h.app
                .world()
                .get::<PlayerAchievementState>(h.caster)
                .unwrap()
                .criteria_progress
                .contains_key(&36050),
            "entry alone is not an encounter defeat"
        );
        record_encounter_defeat(h.app.world_mut(), map, u32::MAX, difficulty);
        assert!(
            !h.app
                .world()
                .get::<PlayerAchievementState>(h.caster)
                .unwrap()
                .criteria_progress
                .contains_key(&36050),
            "unknown encounter is refused"
        );
        record_encounter_defeat(h.app.world_mut(), map, 2051, difficulty);
        let state = h
            .app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap();
        assert_eq!(
            state.criteria_progress.get(&36050).copied(),
            expected,
            "source2051 difficulty{difficulty}, persisted prerequisite{prerequisite}"
        );
        assert_eq!(state.earned_ids.contains(&11660), expected.is_some());
        if prerequisite {
            assert_eq!(state.earned_at[&11611], 1);
        }
        let saved = state.clone();
        h.advance(3);
        let state = h
            .app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap();
        assert_eq!(
            state.criteria_progress, saved.criteria_progress,
            "idle updates cannot invent another encounter defeat"
        );
        assert_eq!(
            state.earned_at, saved.earned_at,
            "original dates survive idle updates"
        );
        drop(h);
        let reopened = redb::Database::open(file.path()).unwrap();
        let persisted = persistence::load_achievements(&reopened, character);
        assert_eq!(persisted.criteria_progress.get(&36050).copied(), expected);
        assert_eq!(persisted.earned_at, saved.earned_at);
        assert!(
            persistence::load_achievements(&reopened, foreign)
                .criteria_progress
                .is_empty()
        );
    }
}
