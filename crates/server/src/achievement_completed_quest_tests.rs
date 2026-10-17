//! Login74 reads this character's persisted rewarded quests, not quest acquisition.
use super::*;
use crate::class_progression::ClassProgressionPlugin;
use crate::mount_journal::MountJournalPlugin;
use lightyear::connection::host::HostClient;
use rusqlite::Connection;
use shared::protocol::{AuthChannel, EnterWorldResponse, LoadTerrain, SelectCharacter};
use std::time::Duration;

const QUEST: u32 = 72360;
const AWARD: u32 = 17495;
const LOGIN_CRITERION: u32 = 58438;
const LOGIN_PEER_OFFSET: u64 = 100;

fn create_login_app(db: redb::Database, conn: &Connection) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.register_message::<SelectCharacter>();
    app.register_message::<EnterWorldResponse>();
    app.register_message::<LoadTerrain>();
    app.add_message::<crate::death::PlayerDeathTransition>();
    insert_login_resources(&mut app, db, conn);
    app.add_plugins((AchievementPlugin, ClassProgressionPlugin));
    app.add_systems(
        Update,
        crate::auth::process_select_character.before(send_initial_achievement_snapshot),
    );
    app.finish();
    app.cleanup();
    app.update();
    // The existing native mount fixture installs the loaded catalog after Startup.
    Plugin::build(&MountJournalPlugin, &mut app);
    app
}

fn insert_login_resources(app: &mut App, db: redb::Database, conn: &Connection) {
    app.insert_resource(GameDatabase { db });
    app.insert_resource(load_achievement_catalog(conn).unwrap());
    app.insert_resource(crate::spell_info::load_class_spells(conn).unwrap());
    app.insert_resource(crate::mount_journal::load(conn).unwrap());
    app.insert_resource(crate::maps::MapStore::continents());
    app.init_resource::<crate::maps::InstanceManager>();
}

fn spawn_login_connection(app: &mut App, account: u64, character: u64) -> Entity {
    app.world_mut()
        .spawn((
            RemoteId(PeerId::Local(character + LOGIN_PEER_OFFSET)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated {
                account_id: account,
            },
            MessageSender::<SelectCharacter>::default(),
            MessageReceiver::<SelectCharacter>::default(),
            MessageSender::<EnterWorldResponse>::default(),
            MessageReceiver::<EnterWorldResponse>::default(),
            MessageSender::<LoadTerrain>::default(),
            MessageReceiver::<LoadTerrain>::default(),
        ))
        .id()
}

fn select_login_character(app: &mut App, account: u64, character: u64) -> Entity {
    let connection = spawn_login_connection(app, account, character);
    app.world_mut()
        .get_mut::<MessageSender<SelectCharacter>>(connection)
        .unwrap()
        .send::<AuthChannel>(SelectCharacter {
            character_id: character,
        });
    for _ in 0..4 {
        app.update();
    }
    app.world().get::<PlayerLink>(connection).unwrap().0
}

#[test]
fn achievements_criteria_completed_quest_real_login_preserves_character_history_and_account_award()
{
    use crate::persistence::quest_store::{ActiveQuestData, QuestLogData, save_quest_log};
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (rewarded, _) = persistence::create_character(&db, 17, "Questknown", 1, 1).unwrap();
    let (sibling, _) = persistence::create_character(&db, 17, "Questsibling", 1, 1).unwrap();
    let (active, _) = persistence::create_character(&db, 18, "Questactive", 1, 1).unwrap();
    let (foreign, _) = persistence::create_character(&db, 19, "Questother", 1, 1).unwrap();
    // Declared persisted baseline: this test does not acquire or turn in quest72360.
    save_quest_log(
        &db,
        rewarded,
        &QuestLogData {
            rewarded: vec![QUEST],
            ..Default::default()
        },
    );
    save_quest_log(
        &db,
        active,
        &QuestLogData {
            active: vec![ActiveQuestData {
                quest_id: QUEST,
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    save_quest_log(
        &db,
        foreign,
        &QuestLogData {
            rewarded: vec![28766],
            ..Default::default()
        },
    );
    drop(db);
    let db = redb::Database::open(file.path()).unwrap();
    let mut app = create_login_app(db, &conn);
    let player = select_login_character(&mut app, 17, rewarded);
    let state = app.world().get::<PlayerAchievementState>(player).unwrap();
    assert_eq!(
        state.criteria_progress.get(&58437),
        Some(&1),
        "existing type27 sees the same persisted reward"
    );
    assert_eq!(
        state.criteria_progress.get(&LOGIN_CRITERION),
        Some(&1),
        "actual first login must consume character quest history"
    );
    let date = state.account.as_ref().unwrap().earned_at[&AWARD];
    for (account, character, same_account) in [
        (17, sibling, true),
        (18, active, false),
        (19, foreign, false),
    ] {
        let other = select_login_character(&mut app, account, character);
        let state = app.world().get::<PlayerAchievementState>(other).unwrap();
        assert!(
            !state.criteria_progress.contains_key(&LOGIN_CRITERION),
            "another character's reward, an active quest and an unrelated reward cannot substitute"
        );
        assert_eq!(
            state.account.as_ref().unwrap().earned_ids.contains(&AWARD),
            same_account
        );
    }
    for _ in 0..3 {
        app.update();
    }
    let state = app.world().get::<PlayerAchievementState>(player).unwrap();
    assert_eq!(state.criteria_progress.get(&LOGIN_CRITERION), Some(&1));
    assert_eq!(state.account.as_ref().unwrap().earned_at[&AWARD], date);
    drop(app);
    let db = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        persistence::load_achievements(&db, rewarded)
            .criteria_progress
            .get(&LOGIN_CRITERION),
        Some(&1)
    );
    for character in [sibling, active, foreign] {
        assert!(
            !persistence::load_achievements(&db, character)
                .criteria_progress
                .contains_key(&LOGIN_CRITERION)
        );
    }
    let mut reopened = create_login_app(db, &conn);
    let player = select_login_character(&mut reopened, 17, rewarded);
    let state = reopened
        .world()
        .get::<PlayerAchievementState>(player)
        .unwrap();
    // A real second Login74 is another event; idle updates above remain unchanged.
    assert_eq!(state.criteria_progress.get(&LOGIN_CRITERION), Some(&2));
    assert_eq!(state.account.as_ref().unwrap().earned_at[&AWARD], date);
}

#[test]
fn achievements_criteria_completed_quest_rejects_missing_login_history_or_reference() {
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let catalog = load_achievement_catalog(&conn).unwrap();
    let mut app = App::new();
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Unknownhistory".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            Level(10),
            InMap::continent(0),
        ))
        .id();
    let events = load_login_events(app.world(), 1, 17, 10);
    for conditions in [
        PlayerConditions::default(),
        event_conditions(app.world(), player, &events),
    ] {
        let mut state = PlayerAchievementState::default();
        catalog.update_at(&mut state, CriteriaEvent::Login, None, 1, &conditions);
        assert!(!state.criteria_progress.contains_key(&LOGIN_CRITERION));
    }
}
