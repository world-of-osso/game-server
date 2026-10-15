//! Native login74 against persisted profession knowledge and real initialization systems.
use super::*;
use crate::class_progression::{ClassProgressionPlugin, KnownSpells};
use crate::mount_journal::{AccountMounts, MountJournalPlugin};
use crate::profession::PlayerProfessions;
use rusqlite::Connection;

const REPAIRS: [(u32, u32, u32); 3] = [
    (13166, 43205, 264211),
    (13167, 43206, 264616),
    (13169, 43211, 264628),
];

fn spawn_login(app: &mut App, account: u64, character: u64, name: &str) -> Entity {
    let professions =
        persistence::load_professions(&app.world().resource::<GameDatabase>().db, character);
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: name.into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            Level(10),
            InMap::continent(0),
            crate::persistence::ClientId(character),
            PlayerAchievementState::default(),
            professions,
        ))
        .id();
    app.world_mut().spawn((
        Authenticated {
            account_id: account,
        },
        PlayerLink(player),
        InWorld {
            character_id: character,
        },
    ));
    player
}

#[test]
fn achievements_criteria_unknown_spell_native_login_waits_for_real_spell_authority() {
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (untrained, _) = persistence::create_character(&db, 17, "Untrained", 1, 1).unwrap();
    let (trained, _) = persistence::create_character(&db, 17, "Trained", 1, 1).unwrap();
    let professions = PlayerProfessions {
        spells: REPAIRS.map(|(_, _, spell)| spell).to_vec(),
        ..Default::default()
    };
    persistence::save_professions(&db, trained, &professions);
    // Account mounts are a third real knowledge source, independent of profession training.
    crate::persistence::mount_store::learn_spells(&db, 17, &[18]);
    drop(db);
    let db = redb::Database::open(file.path()).unwrap();
    let mut app = App::new();
    app.init_resource::<Time>();
    // Achievement registration comes first: plugin insertion order is not an initialization dependency.
    app.add_plugins((
        AchievementPlugin,
        ClassProgressionPlugin,
        MountJournalPlugin,
    ));
    app.insert_resource(load_achievement_catalog(&conn).unwrap());
    app.insert_resource(crate::spell_info::load_class_spells(&conn).unwrap());
    app.insert_resource(GameDatabase { db });
    let unknown = spawn_login(&mut app, 17, untrained, "Untrained");
    let known = spawn_login(&mut app, 17, trained, "Trained");
    assert!(app.world().get::<KnownSpells>(unknown).is_none());
    assert!(app.world().get::<AccountMounts>(unknown).is_none());
    app.update();
    for player in [unknown, known] {
        assert!(app.world().get::<KnownSpells>(player).is_some());
        assert!(app.world().get::<AccountMounts>(player).is_some());
    }
    for (_, _, spell) in REPAIRS {
        assert!(
            app.world()
                .get::<KnownSpells>(known)
                .unwrap()
                .contains(spell)
        );
    }
    for (award, criterion, _) in REPAIRS {
        let state = app.world().get::<PlayerAchievementState>(unknown).unwrap();
        assert_eq!(
            state.criteria_progress.get(&criterion),
            Some(&1),
            "known absence must credit criterion {criterion} on the actual first login"
        );
        assert!(state.earned_ids.contains(&award));
        assert!(
            !app.world()
                .get::<PlayerAchievementState>(known)
                .unwrap()
                .criteria_progress
                .contains_key(&criterion),
            "persisted profession knowledge must prevent repair tracking"
        );
    }
    let dates = app
        .world()
        .get::<PlayerAchievementState>(unknown)
        .unwrap()
        .earned_at
        .clone();
    app.update();
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(unknown)
            .unwrap()
            .earned_at,
        dates
    );
    for (_, criterion, _) in REPAIRS {
        assert_eq!(
            app.world()
                .get::<PlayerAchievementState>(unknown)
                .unwrap()
                .criteria_progress
                .get(&criterion),
            Some(&1)
        );
    }
    drop(app);
    let db = redb::Database::open(file.path()).unwrap();
    let saved = persistence::load_achievements(&db, untrained);
    for (award, criterion, _) in REPAIRS {
        assert_eq!(saved.criteria_progress.get(&criterion), Some(&1));
        assert_eq!(saved.earned_at[&award], dates[&award]);
        assert!(
            !persistence::load_achievements(&db, trained)
                .criteria_progress
                .contains_key(&criterion)
        );
    }
}
