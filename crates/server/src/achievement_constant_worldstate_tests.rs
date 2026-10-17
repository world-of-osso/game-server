//! Constant WorldStateExpression67 still requires every authored native cast.
use super::*;
use crate::spell_cast::test_harness::{Harness, harness_with_plugins};
use rusqlite::Connection;

const HORDE: [(u32, u32); 5] = [
    (243551, 41120),
    (281594, 41121),
    (256097, 41122),
    (256063, 41123),
    (314318, 43403),
];
const ALLIANCE: [(u32, u32); 5] = [
    (281653, 41124),
    (281658, 41125),
    (281662, 41126),
    (281666, 41127),
    (314319, 42138),
];

fn spawn_leader_cast_harness(
    db: redb::Database,
    character: u64,
    race: u8,
    casts: &[(u32, u32)],
) -> Harness {
    let saved = persistence::load_achievements(&db, character);
    let spells: Vec<_> = casts.iter().map(|(spell, _)| *spell).collect();
    let mut h = harness_with_plugins(vec![], &spells, 1, false, |app| {
        app.add_plugins(AchievementPlugin);
    })
    .unwrap();
    let source = Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(load_achievement_catalog(&source).unwrap());
    h.app.insert_resource(GameDatabase { db });
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .remove::<InWorld>();
    h.app.world_mut().entity_mut(h.connection).insert(InWorld {
        character_id: character,
    });
    h.app
        .world_mut()
        .get_mut::<Authenticated>(h.connection)
        .unwrap()
        .account_id = 17;
    h.app.world_mut().get_mut::<Player>(h.caster).unwrap().race = race;
    h.app.world_mut().entity_mut(h.caster).insert((
        persistence::ClientId(character),
        shared::components::UnitFactionTemplate(crate::faction::faction_template_for_race(race)),
        saved,
    ));
    h.app.update();
    h
}

#[test]
fn achievements_criteria_constant67_native_full_leaders_keep_companions_and_dates() {
    for (race, award, casts) in [(2, 12878, HORDE), (1, 12884, ALLIANCE)] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = redb::Database::create(file.path()).unwrap();
        persistence::create_tables(&db);
        let (character, _) =
            persistence::create_character(&db, 17, "Leaderproof", race, 1).unwrap();
        let (foreign, _) = persistence::create_character(&db, 18, "Nonleader", race, 1).unwrap();
        let mut h = spawn_leader_cast_harness(db, character, race, &casts);
        assert!(!crate::spell_cast::try_cast_triggered(
            h.app.world_mut(),
            h.caster,
            casts[4].0,
            Some(h.caster),
            u8::MAX,
        ));
        for (spell, criterion) in &casts[..4] {
            assert!(crate::spell_cast::try_cast_triggered(
                h.app.world_mut(),
                h.caster,
                *spell,
                Some(h.caster),
                0,
            ));
            assert_eq!(
                h.app
                    .world()
                    .get::<PlayerAchievementState>(h.caster)
                    .unwrap()
                    .criteria_progress
                    .get(criterion),
                Some(&1),
                "unconditional companion must prove genuine accepted native cast {spell}",
            );
        }
        assert!(
            !h.app
                .world()
                .get::<PlayerAchievementState>(h.caster)
                .unwrap()
                .account
                .as_ref()
                .unwrap()
                .earned_ids
                .contains(&award)
        );
        assert!(crate::spell_cast::try_cast_triggered(
            h.app.world_mut(),
            h.caster,
            casts[4].0,
            Some(h.caster),
            0,
        ));
        let state = h
            .app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap();
        assert_eq!(
            state.criteria_progress.get(&casts[4].1),
            Some(&1),
            "literal5497 is1==1; real fifth cast must satisfy67 without fabricated world state",
        );
        let account = state.account.as_ref().unwrap();
        assert!(
            account.earned_ids.contains(&award),
            "all five authored casts must grant {award}"
        );
        let date = account.earned_at[&award];
        h.advance(20);
        assert_eq!(
            h.app
                .world()
                .get::<PlayerAchievementState>(h.caster)
                .unwrap()
                .account
                .as_ref()
                .unwrap()
                .earned_at[&award],
            date
        );
        drop(h);
        let db = redb::Database::open(file.path()).unwrap();
        let saved = persistence::load_achievements(&db, character);
        for (_, criterion) in casts {
            assert_eq!(saved.criteria_progress.get(&criterion), Some(&1));
        }
        assert!(
            persistence::load_achievements(&db, foreign)
                .criteria_progress
                .is_empty()
        );
        let h = spawn_leader_cast_harness(db, character, race, &casts);
        assert_eq!(
            h.app
                .world()
                .get::<PlayerAchievementState>(h.caster)
                .unwrap()
                .account
                .as_ref()
                .unwrap()
                .earned_at[&award],
            date
        );
    }
}
