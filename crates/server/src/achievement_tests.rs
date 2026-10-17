use super::*;
use rusqlite::Connection;

#[test]
fn guild_news_real_flagged_player_achievement_commits_once_and_survives_restart() {
    use redb::ReadableDatabase;
    use shared::protocol::GuildNewsKind;
    let connection = Connection::open(crate::test_data::world_db_path()).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("achievement-news.redb");
    let db = redb::Database::create(&path).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 17, "Newsreader", 1, 1).unwrap();
    crate::persistence_character::set_character_guild_by_id(&db, character, 7, "Real News")
        .unwrap();
    assert!(crate::guild_news::query(&db, character).entries.is_empty());
    let mut world = World::new();
    world.insert_resource(load_achievement_catalog(&connection).unwrap());
    world.insert_resource(GameDatabase::new(db));
    world.insert_resource(crate::maps::ServerClock {
        offset_secs: 0,
        fixed_unix: Some(1_792_118_000),
    });
    let player = world
        .spawn((
            Player {
                name: "Newsreader".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            Level(50),
            InMap::continent(0),
            PlayerAchievementState::default(),
        ))
        .id();
    world.spawn((
        Authenticated { account_id: 17 },
        InWorld {
            character_id: character,
        },
        PlayerLink(player),
    ));
    let held = world.resource::<GameDatabase>().db.begin_write().unwrap();
    let (release, blocked) = std::sync::mpsc::channel();
    let holder = std::thread::spawn(move || {
        let signal = blocked.recv_timeout(std::time::Duration::from_secs(5));
        held.abort().unwrap();
        signal.expect("fixture explicitly releases the held writer");
    });
    let (_, toasts) =
        update_player_events(&mut world, player, &[CriteriaEvent::ReachLevel(50)], None).unwrap();
    assert!(
        !holder.is_finished(),
        "producer must return before the held writer is released"
    );
    assert!(
        crate::guild_news::query(&world.resource::<GameDatabase>().db, character)
            .entries
            .is_empty()
    );
    {
        let read = world.resource::<GameDatabase>().db.begin_read().unwrap();
        let rows = read.open_table(persistence::ACHIEVEMENTS).unwrap();
        let before = rows
            .get(character)
            .unwrap()
            .map(|row| decode_saved_achievement_state(row.value()).0)
            .unwrap_or_default();
        assert!(
            !before.earned_ids.contains(&14782),
            "achievement/news pair remains uncommitted"
        );
    }
    release.send(()).unwrap();
    holder.join().unwrap();
    crate::test_data::drain_persistence(world.resource::<GameDatabase>());
    assert!(
        toasts.iter().any(|toast| toast.achievement_id == 14782),
        "Earn actual installed Level50"
    );
    let reply = crate::guild_news::query(&world.resource::<GameDatabase>().db, character);
    assert_eq!(
        reply.entries.len(),
        1,
        "Only actual SHOW_IN_GUILD_NEWS achievements enter guild news"
    );
    assert_eq!(reply.entries[0].timestamp, 1_792_118_000);
    let GuildNewsKind::PlayerAchievement {
        member,
        achievement_id,
        name,
        description,
    } = &reply.entries[0].kind
    else {
        panic!("Real achievement news expected")
    };
    assert_eq!(
        (member.as_str(), *achievement_id, name.as_str()),
        ("Newsreader", 14782, "Level 50")
    );
    assert!(!description.is_empty(), "Actual installed tooltip metadata");
    update_player_events(&mut world, player, &[CriteriaEvent::ReachLevel(50)], None).unwrap();
    crate::test_data::drain_persistence(world.resource::<GameDatabase>());
    assert_eq!(
        crate::guild_news::query(&world.resource::<GameDatabase>().db, character).entries,
        reply.entries,
        "Repeated credit cannot duplicate news"
    );
    let saved = persistence::load_achievements(&world.resource::<GameDatabase>().db, character);
    assert!(
        saved.earned_ids.contains(&14782),
        "Achievement and news commit together"
    );
    drop(world);
    let db = redb::Database::create(&path).unwrap();
    persistence::create_tables(&db);
    assert_eq!(
        crate::guild_news::query(&db, character).entries,
        reply.entries
    );
}

#[test]
fn achievements_criteria_encounter_loot_eligibility_reads_active_native_locks_and_authority() {
    use crate::maps::instances::{InstanceEntrant, Now};
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let store = crate::maps::store::load_map_store(&conn).unwrap();
    let now = Now {
        elapsed: 0.0,
        unix: 1791763200,
    };
    let mut manager = crate::maps::InstanceManager::default();
    let map = manager
        .create_map(
            &store,
            1530,
            InstanceEntrant {
                character_id: 1,
                leader: 1,
                difficulty_id: 14,
                ..Default::default()
            },
            now,
        )
        .unwrap()
        .map;
    let mut world = World::new();
    world.insert_resource(store);
    world.insert_resource(manager);
    world.init_resource::<Time>();
    world.insert_resource(crate::maps::ServerClock {
        offset_secs: 0,
        fixed_unix: Some(now.unix),
    });
    let player = world
        .spawn((
            Player {
                name: "Looteligible".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            map,
            crate::persistence::ClientId(1),
        ))
        .id();
    let connection = world
        .spawn((
            Authenticated { account_id: 1 },
            InWorld { character_id: 1 },
            PlayerLink(player),
        ))
        .id();
    assert_eq!(
        encounter_loot_eligible_players(&mut world, map, 1849, 14),
        vec![player]
    );
    assert!(encounter_loot_eligible_players(&mut world, map, 1849, 15).is_empty());
    assert!(
        encounter_loot_eligible_players(
            &mut world,
            InMap {
                instance_id: map.instance_id + 900,
                ..map
            },
            1849,
            14
        )
        .is_empty()
    );
    assert!(encounter_loot_eligible_players(&mut world, map, u32::MAX, 14).is_empty());
    world.entity_mut(connection).remove::<Authenticated>();
    assert!(encounter_loot_eligible_players(&mut world, map, 1849, 14).is_empty());
    world
        .entity_mut(connection)
        .insert(Authenticated { account_id: 1 });
    world
        .entity_mut(connection)
        .insert(InWorld { character_id: 2 });
    assert!(encounter_loot_eligible_players(&mut world, map, 1849, 14).is_empty());
    world
        .entity_mut(connection)
        .insert(InWorld { character_id: 1 });
    world
        .entity_mut(player)
        .remove::<crate::persistence::ClientId>();
    assert!(encounter_loot_eligible_players(&mut world, map, 1849, 14).is_empty());
    world
        .entity_mut(player)
        .insert(crate::persistence::ClientId(1));
    world
        .resource_mut::<crate::maps::InstanceManager>()
        .update_instance_lock(map, &[(player, 1)], 2, Some(1), now.unix);
    assert!(
        encounter_loot_eligible_players(&mut world, map, 1849, 14).is_empty(),
        "native saved encounter bit is already loot locked"
    );
}

#[test]
fn achievements_criteria_top_level_area_uses_authentic_dragon_isles_terrain_and_persists() {
    use shared::components::{CharacterAppearance, Position};
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 17, "Zoneproof", 1, 1).unwrap();
    let maps = crate::maps::store::load_map_store(&conn).unwrap();
    let data = std::env::var_os("GAME_SERVER_GROUND_DIR").expect("real ground asset directory");
    let mut ground = crate::ground::GroundMap::new(
        std::path::Path::new(&data),
        maps.maps
            .iter()
            .map(|(&id, map)| (id, map.wdt_file_data_id))
            .collect(),
    );
    // Authored MCNK centres from WDT3694921: roots3727661/4019752/4020532/4018737/4019782.
    let points = [
        (13644, 56789, 13644, (4883.333, 149.99935, 17.0797)),
        (13645, 56811, 13766, (883.3333, 2150.0003, 256.679)),
        (13646, 56812, 13646, (-1583.3326, -1616.6666, 1130.9233)),
        (13647, 57004, 13794, (3816.6663, -3050.0007, 13.76233)),
        (13862, 56813, 13862, (549.99935, -783.3327, 710.2002)),
    ];
    for &(_, _, area, (x, y, z)) in &points {
        let (x, y, z) = crate::terrain_height::wow_to_bevy(x, y, z);
        assert_eq!(ground.area_at(2444, Vec3::new(x, y, z)), Some(area));
    }
    let (x, y, z) = crate::terrain_height::wow_to_bevy(4883.333, 149.99935, 17.0797);
    let mut app = App::new();
    app.add_plugins(AchievementPlugin);
    app.insert_resource(load_achievement_catalog(&conn).unwrap());
    app.insert_resource(crate::area_table::load_area_table(&conn).unwrap());
    app.insert_resource(ground);
    app.insert_resource(GameDatabase::new(db));
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Zoneproof".into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            Level(1),
            Position { x, y, z },
            InMap::continent(2444),
            PlayerAchievementState::default(),
            crate::death::PlayerDeathState::default(),
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((Authenticated { account_id: 17 }, PlayerLink(player)))
        .id();
    app.update();
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .contains_key(&56789)
    );
    app.world_mut().entity_mut(connection).insert(InWorld {
        character_id: character,
    });
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&56789),
        Some(&1)
    );
    for &(_, criterion, _, (x, y, z)) in &points {
        let (x, y, z) = crate::terrain_height::wow_to_bevy(x, y, z);
        app.world_mut()
            .entity_mut(player)
            .insert(Position { x, y, z });
        app.update();
        app.update();
        assert_eq!(
            app.world()
                .get::<PlayerAchievementState>(player)
                .unwrap()
                .criteria_progress
                .get(&criterion),
            Some(&1)
        );
    }
    let state = app.world().get::<PlayerAchievementState>(player).unwrap();
    assert!(state.earned_ids.contains(&16759));
    assert_eq!(state.criteria_progress.get(&56318), Some(&1));
    // TC LeaveArea uses the actual previous area and its negative IsInArea predicate.
    assert_eq!(state.criteria_progress.get(&47848), Some(&1));
    drop(app);
    let reopened = redb::Database::open(file.path()).unwrap();
    let saved = persistence::load_achievements(&reopened, character);
    for &(_, criterion, _, _) in &points {
        assert_eq!(saved.criteria_progress.get(&criterion), Some(&1));
    }
    assert_eq!(saved.criteria_progress.get(&56318), Some(&1));
    assert_eq!(saved.criteria_progress.get(&47848), Some(&1));
}

#[test]
fn achievements_criteria_knowledge_skill_line_counts_distinct_actual_spells_and_recomputes() {
    use crate::class_progression::KnownSpells;
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let mut app = App::new();
    app.add_plugins(AchievementPlugin);
    app.insert_resource(load_achievement_catalog(&conn).unwrap());
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Skillproof".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            PlayerAchievementState::default(),
            KnownSpells::fixture(vec![22717, 22717, 60116, 2538, 2538, 2540]),
        ))
        .id();
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&7221),
        Some(&2)
    );
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&4787),
        Some(&2)
    );
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&4787),
        Some(&2)
    );
    app.world_mut()
        .entity_mut(player)
        .insert(KnownSpells::fixture(vec![60116, 2540]));
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&7221),
        Some(&1)
    );
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&4787),
        Some(&1)
    );
}

#[test]
fn achievements_criteria_knowledge_pet_uses_the_owned_account_journal_not_cosmetic_defaults() {
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 42, "Petproof", 1, 1).unwrap();
    let mut app = App::new();
    app.add_plugins(AchievementPlugin);
    app.add_systems(Update, crate::battle_pets::load_account_journals);
    app.insert_resource(load_achievement_catalog(&conn).unwrap());
    app.insert_resource(crate::battle_pets::load_test_catalog());
    app.insert_resource(GameDatabase::new(db));
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Petproof".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            PlayerAchievementState::default(),
            Level(1),
            crate::collection::default_collection_state(),
        ))
        .id();
    app.world_mut()
        .get_mut::<crate::collection::PlayerCollectionState>(player)
        .unwrap()
        .pets
        .push(crate::collection::CollectionPet {
            pet_id: 224,
            name: "Calico Cat".into(),
            known: true,
            active: false,
        });
    app.world_mut().spawn((
        Authenticated { account_id: 42 },
        PlayerLink(player),
        InWorld {
            character_id: character,
        },
    ));
    app.update();
    app.update();
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .contains_key(&23579)
    );
    assert!(
        crate::battle_pets::learn_companion_from_item(app.world_mut(), player, 65358, 1).unwrap()
    );
    app.update();
    let state = app.world().get::<PlayerAchievementState>(player).unwrap();
    assert_eq!(state.criteria_progress.get(&19598), Some(&1));
    assert!(state.account.as_ref().unwrap().earned_ids.contains(&1017));
    assert_eq!(state.criteria_progress.get(&23579), Some(&1));
    assert!(
        crate::battle_pets::learn_companion_from_item(app.world_mut(), player, 65358, 1).unwrap()
    );
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&23579),
        Some(&1)
    );
    assert!(
        crate::battle_pets::learn_companion_from_item(app.world_mut(), player, 4055, 1).unwrap()
    );
    app.update();
    let (sibling_character, _) = persistence::create_character(
        &app.world().resource::<GameDatabase>().db,
        42,
        "Petsibling",
        1,
        1,
    )
    .unwrap();
    let sibling = app
        .world_mut()
        .spawn((
            Player {
                name: "Petsibling".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            PlayerAchievementState::default(),
            Level(1),
            crate::collection::default_collection_state(),
        ))
        .id();
    let sibling_connection = app
        .world_mut()
        .spawn((
            Authenticated { account_id: 42 },
            PlayerLink(sibling),
            InWorld {
                character_id: sibling_character,
            },
        ))
        .id();
    app.update();
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(sibling)
            .unwrap()
            .criteria_progress
            .get(&19598),
        Some(&2)
    );
    let mut journal =
        crate::battle_pets::load_journal(&app.world().resource::<GameDatabase>().db, 42);
    let squirrel = journal.by_species(39)[0].id;
    // The shared release API changes the authoritative journal; no native release-UI claim.
    journal.remove(squirrel).unwrap();
    crate::battle_pets::save_journal(&app.world().resource::<GameDatabase>().db, 42, &journal);
    app.world_mut()
        .get_mut::<crate::battle_pets::AccountPetJournal>(player)
        .unwrap()
        .journal = journal;
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .account
            .as_ref()
            .unwrap()
            .criteria_progress
            .get(&19598),
        Some(&1)
    );
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(sibling)
            .unwrap()
            .criteria_progress
            .get(&19598),
        Some(&2),
        "stale character copy must not override the owning account journal"
    );
    app.world_mut()
        .entity_mut(sibling_connection)
        .remove::<InWorld>()
        .insert(InWorld {
            character_id: sibling_character,
        });
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(sibling)
            .unwrap()
            .account
            .as_ref()
            .unwrap()
            .criteria_progress
            .get(&19598),
        Some(&1)
    );
    drop(app);
    let db = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        crate::battle_pets::load_journal(&db, 42).count_species(224),
        2
    );
    assert_eq!(crate::battle_pets::load_journal(&db, 43).count(), 0);
    assert_eq!(
        persistence::load_achievements(&db, character)
            .criteria_progress
            .get(&23579),
        Some(&1)
    );
}

#[test]
fn achievements_comparison_reads_target_account_and_character_without_mutating_viewer() {
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let mut world = World::new();
    world.insert_resource(load_achievement_catalog(&conn).unwrap());
    let viewer = world.spawn(PlayerAchievementState::default()).id();
    let mut state = PlayerAchievementState::default();
    state.earned_ids.insert(633);
    state.earned_at.insert(633, 1_791_244_800);
    let mut account = PlayerAchievementState::default();
    account.earned_ids.insert(4899);
    state.account = Some(Box::new(account));
    let target = world
        .spawn((
            Player {
                name: "Otherplayer".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            state,
        ))
        .id();
    let connection = world
        .spawn((
            Authenticated { account_id: 41 },
            PlayerLink(target),
            InWorld { character_id: 51 },
        ))
        .id();
    let page = comparison_page(
        &mut world,
        viewer,
        target.to_bits(),
        QueryAchievementCatalog::Achievement {
            achievement_id: 633,
        },
    );
    let AchievementCatalogPage::Comparison {
        target_name,
        page: Some(page),
        error: None,
        ..
    } = page
    else {
        panic!("comparison reply");
    };
    assert_eq!(target_name, "Otherplayer");
    let AchievementCatalogPage::Achievement {
        entry: Some(row), ..
    } = (*page).into()
    else {
        panic!("comparison details");
    };
    assert!(row.earned);
    assert_eq!(row.earned_at, Some(1_791_244_800));
    let page = comparison_page(
        &mut world,
        viewer,
        target.to_bits(),
        QueryAchievementCatalog::Summary {
            scope: shared::protocol::AchievementScope::All,
        },
    );
    let AchievementCatalogPage::Comparison {
        page: Some(page), ..
    } = page
    else {
        panic!("comparison summary");
    };
    let AchievementCatalogPage::Summary(summary) = (*page).into() else {
        panic!("summary");
    };
    assert_eq!(summary.completed, 2);
    assert_eq!(summary.points, 20);
    assert!(
        world
            .get::<PlayerAchievementState>(viewer)
            .unwrap()
            .earned_ids
            .is_empty()
    );
    for target_bits in [viewer.to_bits(), u64::MAX] {
        let reply = comparison_page(
            &mut world,
            viewer,
            target_bits,
            QueryAchievementCatalog::Summary {
                scope: shared::protocol::AchievementScope::All,
            },
        );
        assert!(matches!(
            reply,
            AchievementCatalogPage::Comparison {
                page: None,
                error: Some(_),
                ..
            }
        ));
    }
    let reply = comparison_page(
        &mut world,
        viewer,
        target.to_bits(),
        QueryAchievementCatalog::Comparison {
            target_entity: viewer.to_bits(),
            query: Box::new(shared::protocol::AchievementComparisonQuery::Summary {
                scope: shared::protocol::AchievementScope::All,
            }),
        },
    );
    assert!(matches!(
        reply,
        AchievementCatalogPage::Comparison {
            page: None,
            error: Some(_),
            ..
        }
    ));
    world.entity_mut(connection).remove::<InWorld>();
    let reply = comparison_page(
        &mut world,
        viewer,
        target.to_bits(),
        QueryAchievementCatalog::Achievement {
            achievement_id: 633,
        },
    );
    assert!(matches!(
        reply,
        AchievementCatalogPage::Comparison {
            page: None,
            error: Some(_),
            ..
        }
    ));
}

#[test]
fn achievements_criteria_exploration_uses_real_terrain_and_survives_reopen() {
    use shared::components::{CharacterAppearance, Position};
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 17, "Exploreproof", 1, 1).unwrap();
    let maps = crate::maps::store::load_map_store(&conn).unwrap();
    let ground = crate::ground::GroundMap::new(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data"),
        maps.maps
            .iter()
            .map(|(&id, map)| (id, map.wdt_file_data_id))
            .collect(),
    );
    let mut app = App::new();
    app.add_plugins(AchievementPlugin);
    app.insert_resource(load_achievement_catalog(&conn).unwrap());
    app.insert_resource(ground);
    app.insert_resource(GameDatabase::new(db));
    // Goldshire's real MCNK AreaID87 maps to WorldMapOverlay122/criterion1147.
    let (x, y, z) = crate::terrain_height::wow_to_bevy(-9460.0, 65.0, 60.0);
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Exploreproof".into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            Level(1),
            Position { x, y, z },
            InMap::continent(0),
            PlayerAchievementState::default(),
            crate::death::PlayerDeathState::default(),
        ))
        .id();
    assert_eq!(
        app.world_mut()
            .resource_mut::<crate::ground::GroundMap>()
            .area_at(0, Vec3::new(x, y, z)),
        Some(87)
    );
    let connection = app
        .world_mut()
        .spawn((Authenticated { account_id: 17 }, PlayerLink(player)))
        .id();
    app.update();
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .contains_key(&1147)
    );
    app.world_mut().entity_mut(connection).insert(InWorld {
        character_id: character,
    });
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&1147),
        Some(&1)
    );
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&1147),
        Some(&1)
    );
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .contains_key(&502)
    );
    drop(app);
    let db = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        persistence::load_achievements(&db, character)
            .criteria_progress
            .get(&1147),
        Some(&1)
    );
}

#[test]
fn achievements_criteria_login_credits_authenticated_world_entry_once_and_survives_reopen() {
    use shared::components::CharacterAppearance;
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 17, "Loginproof", 1, 1).unwrap();
    let mut app = App::new();
    app.add_plugins(AchievementPlugin);
    app.insert_resource(load_achievement_catalog(&conn).unwrap());
    app.insert_resource(GameDatabase::new(db));
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Loginproof".into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            Level(1),
            PlayerAchievementState::default(),
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((Authenticated { account_id: 17 }, PlayerLink(player)))
        .id();
    app.update();
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .contains_key(&23250)
    );
    app.world_mut().entity_mut(connection).insert(InWorld {
        character_id: character,
    });
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&23250),
        Some(&1)
    );
    app.update();
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&23250),
        Some(&1)
    );
    app.world_mut().entity_mut(connection).remove::<InWorld>();
    app.update();
    app.world_mut().entity_mut(connection).insert(InWorld {
        character_id: character,
    });
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&23250),
        Some(&2)
    );
    drop(app);
    let db = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        persistence::load_achievements(&db, character)
            .criteria_progress
            .get(&23250),
        Some(&2)
    );
}

#[test]
fn achievements_criteria_native_location_some_credits_real_selfie_cast_in_area_alternative() {
    const SPELL: u32 = 181842;
    const CRITERION: u32 = 27968;
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 1, "Photoproof", 1, 1).unwrap();
    let mut h =
        crate::spell_cast::test_harness::harness_with_plugins(vec![], &[SPELL], 1, false, |app| {
            app.add_plugins(AchievementPlugin);
        })
        .unwrap();
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(load_achievement_catalog(&conn).unwrap());
    h.app.insert_resource(GameDatabase::new(db));
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .remove::<InWorld>();
    h.app.world_mut().entity_mut(h.connection).insert((
        InWorld {
            character_id: character,
        },
        Authenticated { account_id: 1 },
    ));
    h.app.world_mut().entity_mut(h.caster).insert((
        persistence::ClientId(character),
        PlayerAchievementState::default(),
    ));
    h.app.update();
    // Controlled cached authoritative area, not a terrain-travel/acquisition claim.
    h.app
        .world_mut()
        .get_mut::<AchievementTracker>(h.caster)
        .unwrap()
        .last_processed_area = Some(1519);
    assert!(crate::spell_cast::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        SPELL,
        Some(h.caster),
        0
    ));
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&CRITERION)
    );
    h.app
        .world_mut()
        .get_mut::<AchievementTracker>(h.caster)
        .unwrap()
        .last_processed_area = Some(254);
    assert!(!crate::spell_cast::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        SPELL,
        Some(h.caster),
        u8::MAX
    ));
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&CRITERION)
    );
    assert!(crate::spell_cast::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        SPELL,
        Some(h.caster),
        0
    ));
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&CRITERION),
        Some(&1),
        "actual selfie cast must accept the area17 alternative even though Zone41(25) is false in area254"
    );
    h.advance(10);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&CRITERION),
        Some(&1)
    );
    drop(h);
    let db = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        persistence::load_achievements(&db, character)
            .criteria_progress
            .get(&CRITERION),
        Some(&1)
    );
}

fn spawn_saved_gear_cast_fixture(
    db: redb::Database,
    character: u64,
    account_id: u64,
) -> crate::spell_cast::test_harness::Harness {
    let saved = persistence::load_achievements(&db, character);
    let mut h = crate::spell_cast::test_harness::harness_with_plugins(
        vec![],
        &[1217637],
        1,
        false,
        |app| {
            app.add_plugins(AchievementPlugin);
        },
    )
    .unwrap();
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(load_achievement_catalog(&conn).unwrap());
    h.app.insert_resource(GameDatabase::new(db));
    // Authentication inserts InWorld; changing its ID alone cannot replay Added<InWorld>.
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .remove::<InWorld>();
    h.app.world_mut().entity_mut(h.connection).insert(InWorld {
        character_id: character,
    });
    h.app
        .world_mut()
        .get_mut::<crate::auth::Authenticated>(h.connection)
        .unwrap()
        .account_id = account_id;
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((persistence::ClientId(character), saved));
    h.app.update();
    h
}

#[test]
fn achievements_criteria_native_unearned_self_guard_stops_after_real_twenty_cast_grant() {
    const SPELL: u32 = 1217637;
    const CRITERION: u32 = 103451;
    const AWARD: u32 = 41707;
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 1, "Gearproof", 1, 1).unwrap();
    let (sibling, _) = persistence::create_character(&db, 1, "Gearsibling", 1, 1).unwrap();
    let (foreign, _) = persistence::create_character(&db, 2, "Gearforeign", 1, 1).unwrap();
    let mut h = spawn_saved_gear_cast_fixture(db, character, 1);
    let started = h.app.world().resource::<crate::maps::ServerClock>().unix();
    assert!(!crate::spell_cast::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        SPELL,
        Some(h.caster),
        u8::MAX,
    ));
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&CRITERION)
    );
    for casts in 1..=20 {
        assert!(crate::spell_cast::try_cast_triggered(
            h.app.world_mut(),
            h.caster,
            SPELL,
            Some(h.caster),
            0,
        ));
        let state = h
            .app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap();
        assert_eq!(
            state.criteria_progress.get(&CRITERION),
            Some(&casts),
            "genuine accepted source spell must credit before its self-guard award"
        );
        assert_eq!(
            state
                .account
                .as_ref()
                .is_some_and(|account| account.earned_ids.contains(&AWARD)),
            casts == 20,
            "all twenty authored gear casts are required"
        );
    }
    let earned_at = h
        .app
        .world()
        .get::<PlayerAchievementState>(h.caster)
        .unwrap()
        .account
        .as_ref()
        .unwrap()
        .earned_at[&AWARD];
    assert!(
        (started..=h.app.world().resource::<crate::maps::ServerClock>().unix())
            .contains(&earned_at)
    );
    assert!(crate::spell_cast::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        SPELL,
        Some(h.caster),
        0,
    ));
    h.advance(20);
    let state = h
        .app
        .world()
        .get::<PlayerAchievementState>(h.caster)
        .unwrap();
    assert_eq!(
        state.criteria_progress.get(&CRITERION),
        Some(&20),
        "earned self-guard and idle updates must stop further progress"
    );
    assert_eq!(state.account.as_ref().unwrap().earned_at[&AWARD], earned_at);
    drop(h);
    let db = redb::Database::open(file.path()).unwrap();
    let saved = persistence::load_achievements(&db, character);
    assert_eq!(saved.criteria_progress.get(&CRITERION), Some(&20));
    let mut h = spawn_saved_gear_cast_fixture(db, character, 1);
    let state = h
        .app
        .world()
        .get::<PlayerAchievementState>(h.caster)
        .unwrap();
    assert!(state.account.as_ref().unwrap().earned_ids.contains(&AWARD));
    assert_eq!(state.account.as_ref().unwrap().earned_at[&AWARD], earned_at);
    assert!(crate::spell_cast::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        SPELL,
        Some(h.caster),
        0,
    ));
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&CRITERION),
        Some(&20)
    );
    drop(h);
    let db = redb::Database::open(file.path()).unwrap();
    let mut h = spawn_saved_gear_cast_fixture(db, sibling, 1);
    assert!(crate::spell_cast::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        SPELL,
        Some(h.caster),
        0,
    ));
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&CRITERION),
        "account-wide earned metadata must stop the sibling's self-guard too"
    );
    drop(h);
    let db = redb::Database::open(file.path()).unwrap();
    let mut h = spawn_saved_gear_cast_fixture(db, foreign, 2);
    assert!(crate::spell_cast::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        SPELL,
        Some(h.caster),
        0,
    ));
    let state = h
        .app
        .world()
        .get::<PlayerAchievementState>(h.caster)
        .unwrap();
    assert_eq!(state.criteria_progress.get(&CRITERION), Some(&1));
    assert!(!state.account.as_ref().unwrap().earned_ids.contains(&AWARD));
}

#[test]
fn achievements_criteria_native_target_aura_reads_food_hit_not_caster() {
    let mut h = crate::spell_cast::test_harness::harness_with_plugins(
        vec![],
        &[61925, 65403],
        1,
        false,
        |app| {
            app.add_plugins(AchievementPlugin);
        },
    )
    .unwrap();
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(load_achievement_catalog(&conn).unwrap());
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 1, "Foodhitproof", 1, 1).unwrap();
    h.app.insert_resource(GameDatabase::new(db));
    h.app
        .world_mut()
        .get_mut::<InWorld>(h.connection)
        .unwrap()
        .character_id = character;
    h.app.world_mut().entity_mut(h.caster).insert((
        persistence::ClientId(character),
        PlayerAchievementState::default(),
    ));
    h.app.update();
    let target = crate::spell_cast::test_harness::spawn_player(&mut h.app, 1, 3.0);
    assert!(crate::spell_cast::cast_triggered_with_item_level(
        h.app.world_mut(),
        h.caster,
        65403,
        Some(h.caster),
        0,
        None,
    ));
    h.advance(2);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 65403));
    assert!(!crate::auras::has_aura(h.app.world(), target, 65403));
    assert!(crate::spell_cast::cast_triggered_with_item_level(
        h.app.world_mut(),
        h.caster,
        61925,
        Some(target),
        0,
        None,
    ));
    h.advance(30);
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&11168),
        "caster aura cannot replace target aura"
    );
    assert!(crate::spell_cast::cast_triggered_with_item_level(
        h.app.world_mut(),
        h.caster,
        65403,
        Some(target),
        0,
        None,
    ));
    h.advance(2);
    assert!(crate::auras::has_aura(h.app.world(), target, 65403));
    assert!(!crate::spell_cast::cast_triggered_with_item_level(
        h.app.world_mut(),
        h.caster,
        61925,
        Some(target),
        u8::MAX,
        None,
    ));
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&11168),
        "rejected depth is not a hit"
    );
    assert!(crate::spell_cast::cast_triggered_with_item_level(
        h.app.world_mut(),
        h.caster,
        61925,
        Some(target),
        0,
        None,
    ));
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&11168),
        "launch is not landing"
    );
    h.advance(30);
    let state = h
        .app
        .world()
        .get::<PlayerAchievementState>(h.caster)
        .unwrap();
    assert_eq!(
        state.criteria_progress.get(&11168),
        Some(&1),
        "actual food spell hit on aura65403 target must credit"
    );
    assert!(state.earned_ids.contains(&3579));
    h.advance(20);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&11168),
        Some(&1),
        "idle cannot replay hit credit"
    );
    let missing = h.app.world_mut().spawn_empty().id();
    record_targeted_spell_hit(h.app.world_mut(), h.caster, missing, 61925);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&11168),
        Some(&1),
        "missing target cannot inherit caster aura"
    );
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.npc, 65403, Some(h.npc), 0);
    h.advance(2);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, 65403));
    record_targeted_spell_hit(h.app.world_mut(), h.caster, h.npc, 61925);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&11168),
        Some(&2),
        "actual unit aura is not Player-only"
    );
    drop(h);
    let db = redb::Database::open(file.path()).unwrap();
    let saved = persistence::load_achievements(&db, character);
    assert_eq!(saved.criteria_progress.get(&11168), Some(&2));
    assert!(saved.earned_ids.contains(&3579));
}

#[test]
fn achievements_criteria_native_target_health_reads_bandage_hit_before_healing() {
    use shared::components::Health;
    let mut h =
        crate::spell_cast::test_harness::harness_with_plugins(vec![], &[45544], 1, false, |app| {
            app.add_plugins(AchievementPlugin);
        })
        .unwrap();
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(load_achievement_catalog(&conn).unwrap());
    // Authored Heavy Frostweave Bandage supplies the item-level scaling context.
    let item_level: u16 = conn
        .query_row(
            "SELECT i.ItemLevel FROM content_item i JOIN item_x_item_effect x ON x.ItemID=i.ID \
         JOIN item_effect e ON e.ID=x.ItemEffectID WHERE i.ID=34722 AND e.SpellID=45544",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 1, "Triagehit", 1, 1).unwrap();
    h.app.insert_resource(GameDatabase::new(db));
    h.app
        .world_mut()
        .get_mut::<InWorld>(h.connection)
        .unwrap()
        .character_id = character;
    h.app.world_mut().entity_mut(h.caster).insert((
        persistence::ClientId(character),
        PlayerAchievementState::default(),
    ));
    h.app.update();
    let target = crate::spell_cast::test_harness::spawn_player(&mut h.app, 1, 3.0);
    h.app.world_mut().get_mut::<Health>(target).unwrap().current = 5.01;
    h.app.world_mut().get_mut::<Health>(target).unwrap().max = 100.0;
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 1.0;
    crate::spell_cast::cast_triggered_with_item_level(
        h.app.world_mut(),
        h.caster,
        45544,
        Some(target),
        u8::MAX,
        Some(item_level),
    );
    assert!(!crate::auras::has_aura(h.app.world(), target, 45544));
    crate::spell_cast::cast_triggered_with_item_level(
        h.app.world_mut(),
        h.caster,
        45544,
        Some(target),
        0,
        Some(item_level),
    );
    h.advance(2);
    assert!(crate::auras::has_aura(h.app.world(), target, 45544));
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&6799),
        "low caster health cannot replace target above5percent"
    );
    let missing = h.app.world_mut().spawn_empty().id();
    record_targeted_spell_hit(h.app.world_mut(), h.caster, missing, 45544);
    for health in [
        Health {
            current: 5.0,
            max: 0.0,
        },
        Health {
            current: f32::NAN,
            max: 100.0,
        },
        Health {
            current: 5.0,
            max: f32::INFINITY,
        },
        Health {
            current: f32::INFINITY,
            max: 100.0,
        },
    ] {
        h.app.world_mut().entity_mut(target).insert(health);
        record_targeted_spell_hit(h.app.world_mut(), h.caster, target, 45544);
        assert!(
            !h.app
                .world()
                .get::<PlayerAchievementState>(h.caster)
                .unwrap()
                .criteria_progress
                .contains_key(&6799),
            "invalid target health {health:?} cannot qualify"
        );
    }
    h.app.world_mut().entity_mut(target).insert(Health {
        current: 5.0,
        max: 100.0,
    });
    crate::spell_cast::cast_triggered_with_item_level(
        h.app.world_mut(),
        h.caster,
        45544,
        Some(target),
        0,
        Some(item_level),
    );
    h.advance(2);
    assert!(
        crate::auras::has_aura(h.app.world(), target, 45544),
        "actual bandage hit landed"
    );
    assert_eq!(
        h.app.world().get::<Health>(target).unwrap().current,
        5.0,
        "authored periodic healing has not ticked at100ms"
    );
    let state = h
        .app
        .world()
        .get::<PlayerAchievementState>(h.caster)
        .unwrap();
    assert_eq!(
        state.criteria_progress.get(&6799),
        Some(&1),
        "native target at exactly5percent must credit before healing"
    );
    assert!(state.earned_ids.contains(&141));
    h.advance(24);
    assert!(
        h.app.world().get::<Health>(target).unwrap().current > 5.0,
        "real periodic bandage heals after the qualifying hit"
    );
    h.advance(20);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&6799),
        Some(&1),
        "idle/heal ticks cannot replay hit credit"
    );
    {
        let mut npc_health = h.app.world_mut().get_mut::<Health>(h.npc).unwrap();
        npc_health.current = 1.0;
        npc_health.max = 100.0;
    }
    record_targeted_spell_hit(h.app.world_mut(), h.caster, h.npc, 45544);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&6799),
        Some(&2),
        "hit-unit snapshot is not Player-only"
    );
    h.advance(20);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&6799),
        Some(&2)
    );
    drop(h);
    let db = redb::Database::open(file.path()).unwrap();
    let saved = persistence::load_achievements(&db, character);
    assert_eq!(saved.criteria_progress.get(&6799), Some(&2));
    assert!(saved.earned_ids.contains(&141));
}

#[test]
fn achievements_criteria_native_target_class_uses_actual_player_hit_and_race() {
    let mut h = crate::spell_cast::test_harness::harness_with_plugins(
        vec![],
        &[27571, 61781],
        1,
        false,
        |app| {
            app.add_plugins(AchievementPlugin);
        },
    )
    .unwrap();
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(load_achievement_catalog(&conn).unwrap());
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 1, "Classhit", 1, 1).unwrap();
    h.app.insert_resource(GameDatabase::new(db));
    h.app
        .world_mut()
        .get_mut::<InWorld>(h.connection)
        .unwrap()
        .character_id = character;
    h.app.world_mut().entity_mut(h.caster).insert((
        persistence::ClientId(character),
        PlayerAchievementState::default(),
    ));
    h.app.update();
    h.app.world_mut().get_mut::<Player>(h.caster).unwrap().race = 2;
    let target = crate::spell_cast::test_harness::spawn_player(&mut h.app, 6, 3.0);
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 27571, Some(target), u8::MAX);
    assert!(!crate::auras::has_aura(h.app.world(), target, 27571));
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&6313)
    );
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 27571, Some(target), 0);
    h.advance(9);
    assert!(
        !crate::auras::has_aura(h.app.world(), target, 27571),
        "authored missile has not landed"
    );
    h.advance(3);
    assert!(
        crate::auras::has_aura(h.app.world(), target, 27571),
        "actual sourced rose petals landed"
    );
    let progress = &h
        .app
        .world()
        .get::<PlayerAchievementState>(h.caster)
        .unwrap()
        .criteria_progress;
    assert_eq!(
        progress.get(&6313),
        Some(&1),
        "actual Human Death Knight target qualifies"
    );
    assert!(
        !progress.contains_key(&6302),
        "Orc caster race cannot substitute for Human target"
    );
    h.app.world_mut().get_mut::<Player>(target).unwrap().race = 2;
    h.app.world_mut().get_mut::<Player>(target).unwrap().class = 1;
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 27571, Some(target), 0);
    h.advance(12);
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&6302),
        "correct race cannot replace required target class"
    );
    h.app.world_mut().get_mut::<Player>(target).unwrap().class = 6;
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 27571, Some(target), 0);
    h.advance(12);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&6302),
        Some(&1)
    );
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 27571, Some(h.npc), 0);
    h.advance(12);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, 27571));
    let missing = h.app.world_mut().spawn_empty().id();
    record_targeted_spell_hit(h.app.world_mut(), h.caster, missing, 27571);
    h.app.world_mut().get_mut::<Player>(target).unwrap().race = 1;
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 61781, Some(target), 0);
    h.advance(10);
    assert!(crate::auras::has_aura(h.app.world(), target, 61781));
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&11158),
        "turkey hit on Death Knight cannot credit Rogue"
    );
    h.app.world_mut().get_mut::<Player>(target).unwrap().class = 4;
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 61781, Some(target), 0);
    h.advance(10);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&11158),
        Some(&1),
        "positive race-only authority anchors actual target class"
    );
    h.advance(20);
    let progress = &h
        .app
        .world()
        .get::<PlayerAchievementState>(h.caster)
        .unwrap()
        .criteria_progress;
    assert_eq!(progress.get(&6313), Some(&1));
    assert_eq!(progress.get(&6302), Some(&1));
    assert_eq!(progress.get(&11158), Some(&1));
    drop(h);
    let db = redb::Database::open(file.path()).unwrap();
    let saved = persistence::load_achievements(&db, character);
    assert_eq!(saved.criteria_progress.get(&6313), Some(&1));
    assert_eq!(saved.criteria_progress.get(&6302), Some(&1));
    assert_eq!(saved.criteria_progress.get(&11158), Some(&1));
}

#[test]
fn achievements_criteria_native_target_race_uses_each_actual_spell_hit_unit() {
    let mut h =
        crate::spell_cast::test_harness::harness_with_plugins(vec![], &[44212], 1, false, |app| {
            app.add_plugins(AchievementPlugin);
        })
        .unwrap();
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(load_achievement_catalog(&conn).unwrap());
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 1, "Racehitproof", 1, 1).unwrap();
    h.app.insert_resource(GameDatabase::new(db));
    h.app
        .world_mut()
        .get_mut::<InWorld>(h.connection)
        .unwrap()
        .character_id = character;
    h.app.world_mut().entity_mut(h.caster).insert((
        persistence::ClientId(character),
        PlayerAchievementState::default(),
    ));
    h.app.update();
    h.app.world_mut().get_mut::<Player>(h.caster).unwrap().race = 2;
    let target = crate::spell_cast::test_harness::spawn_player(&mut h.app, 1, 3.0);
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 44212, Some(target), u8::MAX);
    assert!(
        !crate::auras::has_aura(h.app.world(), target, 44212),
        "refused depth installs no aura"
    );
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&5777)
    );
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 44212, Some(target), 0);
    // Authored Speed10 and TC minimum5yd flight take500ms; the fixture advances50ms/frame.
    h.advance(9);
    assert!(
        !crate::auras::has_aura(h.app.world(), target, 44212),
        "launch is not a landing"
    );
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&5777)
    );
    h.advance(3);
    assert!(
        crate::auras::has_aura(h.app.world(), target, 44212),
        "actual sourced pumpkin aura landed"
    );
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&5777),
        Some(&1),
        "native Human target hit must credit its authored race leaf"
    );
    let progress = &h
        .app
        .world()
        .get::<PlayerAchievementState>(h.caster)
        .unwrap()
        .criteria_progress;
    assert!(
        !progress.contains_key(&5779),
        "Orc caster cannot supply target race"
    );
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 44212, Some(h.npc), 0);
    h.advance(12);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, 44212));
    let progress = &h
        .app
        .world()
        .get::<PlayerAchievementState>(h.caster)
        .unwrap()
        .criteria_progress;
    assert_eq!(progress.get(&5777), Some(&1));
    assert!(
        !progress.contains_key(&5779),
        "NPC has no playable target race"
    );
    let missing = h.app.world_mut().spawn_empty().id();
    record_targeted_spell_hit(h.app.world_mut(), h.caster, missing, 44212);
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&5779)
    );
    h.app.world_mut().get_mut::<Player>(target).unwrap().race = 2;
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 44212, Some(target), 0);
    h.advance(12);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&5779),
        Some(&1)
    );
    h.advance(20);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&5777),
        Some(&1)
    );
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&5779),
        Some(&1)
    );
    drop(h);
    let db = redb::Database::open(file.path()).unwrap();
    let saved = persistence::load_achievements(&db, character);
    assert_eq!(saved.criteria_progress.get(&5777), Some(&1));
    assert_eq!(saved.criteria_progress.get(&5779), Some(&1));
}

#[test]
fn achievements_criteria_native_player_aura_some_requires_actual_jaguar_aura_at_loa_landing() {
    for buff in [136775, 136777, 136778] {
        let mut h = crate::spell_cast::test_harness::harness_with_plugins(
            vec![],
            &[136749, 136775, 136777, 136778],
            1,
            false,
            |app| {
                app.add_plugins(AchievementPlugin);
            },
        )
        .unwrap();
        let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
        h.app
            .insert_resource(load_achievement_catalog(&conn).unwrap());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jaguar-aura.redb");
        let db = redb::Database::create(&path).unwrap();
        persistence::create_tables(&db);
        let (character, _) = persistence::create_character(&db, 1, "Jaguarproof", 1, 1).unwrap();
        h.app.insert_resource(GameDatabase::new(db));
        h.app
            .world_mut()
            .get_mut::<InWorld>(h.connection)
            .unwrap()
            .character_id = character;
        h.app.world_mut().entity_mut(h.caster).insert((
            persistence::ClientId(character),
            PlayerAchievementState::default(),
        ));
        h.app.update();
        let count = |h: &crate::spell_cast::test_harness::Harness| {
            h.app
                .world()
                .get::<PlayerAchievementState>(h.caster)
                .unwrap()
                .criteria_progress
                .get(&22840)
                .copied()
        };
        crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 136749, None, 0);
        h.advance(1);
        assert_eq!(count(&h), None, "no Jaguar aura");
        // Mighty Loa itself is a real installed aura, but not a Jaguar leaf.
        assert!(
            crate::auras::has_aura(h.app.world(), h.caster, 136749),
            "real unrelated native Mighty Loa aura"
        );
        crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 136749, None, 0);
        h.advance(1);
        assert_eq!(
            count(&h),
            None,
            "unrelated aura is not a Jaguar alternative"
        );
        crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, buff, None, u8::MAX);
        assert!(
            !crate::auras::has_aura(h.app.world(), h.caster, buff),
            "refused-depth buff cast"
        );
        crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, buff, None, 0);
        h.advance(1);
        assert!(
            crate::auras::has_aura(h.app.world(), h.caster, buff),
            "real sourced Jaguar buff {buff}"
        );
        assert_eq!(count(&h), None, "buff alone is not Mighty Loa landing");
        crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 136749, None, u8::MAX);
        assert_eq!(count(&h), None, "refused-depth Mighty Loa cast");
        crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 136749, None, 0);
        h.advance(1);
        assert_eq!(
            count(&h),
            Some(1),
            "real Mighty Loa landing under independent Jaguar {buff}"
        );
        crate::auras::remove_auras(
            h.app.world_mut(),
            h.caster,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == buff,
        );
        assert!(!crate::auras::has_aura(h.app.world(), h.caster, buff));
        crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 136749, None, 0);
        h.advance(3);
        assert_eq!(
            count(&h),
            Some(1),
            "removed aura and idle cannot borrow previous state"
        );
        drop(h);
        let reopened = redb::Database::open(&path).unwrap();
        assert_eq!(
            persistence::load_achievements(&reopened, character)
                .criteria_progress
                .get(&22840),
            Some(&1),
            "reopened actual landing {buff}"
        );
    }
}

#[test]
fn achievements_criteria_native_death_player_area_some_uses_cached_authority_and_persists() {
    for (area, qualifies) in [
        (Some(153), true),
        (Some(1497), true),
        (Some(4617), true),
        (Some(8299), true),
        (Some(13979), true),
        (Some(1519), false),
        (None, false),
    ] {
        let mut h =
            crate::spell_cast::test_harness::harness_with_plugins(vec![], &[], 1, false, |app| {
                app.add_plugins((AchievementPlugin, crate::death::DeathPlugin));
            })
            .unwrap();
        let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
        h.app
            .insert_resource(load_achievement_catalog(&conn).unwrap());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("area-death.redb");
        let db = redb::Database::create(&path).unwrap();
        persistence::create_tables(&db);
        let (character, _) = persistence::create_character(&db, 1, "Areadeath", 1, 1).unwrap();
        h.app.insert_resource(GameDatabase::new(db));
        h.app
            .world_mut()
            .get_mut::<InWorld>(h.connection)
            .unwrap()
            .character_id = character;
        h.app.world_mut().entity_mut(h.caster).insert((
            persistence::ClientId(character),
            PlayerAchievementState::default(),
            InMap::continent(0),
        ));
        h.app.update();
        // Controlled cached authoritative area, not a rendered terrain-travel claim.
        // Native Unit::Kill and queued player-death processing remain real.
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(AchievementTracker {
                last_processed_level: 1,
                last_processed_zone: None,
                last_processed_area: area,
            });
        assert!(
            !h.app
                .world()
                .get::<PlayerAchievementState>(h.caster)
                .unwrap()
                .criteria_progress
                .contains_key(&109755),
            "area alone is not a death"
        );
        crate::combat::kill_unit(h.app.world_mut(), h.npc, h.caster);
        h.advance(3);
        assert_eq!(h.health(h.caster), 0.0);
        let state = h
            .app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap();
        assert_eq!(
            state.criteria_progress.get(&109755),
            qualifies.then_some(&1),
            "native death area {area:?}"
        );
        assert_eq!(
            state.earned_ids.contains(&61456),
            qualifies,
            "native character grant area {area:?}"
        );
        crate::combat::kill_unit(h.app.world_mut(), h.npc, h.caster);
        h.advance(3);
        drop(h);
        let reopened = redb::Database::open(&path).unwrap();
        let saved = persistence::load_achievements(&reopened, character);
        assert_eq!(
            saved.criteria_progress.get(&109755),
            qualifies.then_some(&1),
            "reopened death area {area:?}"
        );
        assert_eq!(saved.earned_ids.contains(&61456), qualifies);
    }
}

#[test]
fn achievements_criteria_native_death_exact_level_rejects_lower_and_higher_levels() {
    for (level, expected) in [(1, None), (2, Some(16188)), (3, Some(16402))] {
        let mut h =
            crate::spell_cast::test_harness::harness_with_plugins(vec![], &[], 1, false, |app| {
                app.add_plugins((AchievementPlugin, crate::death::DeathPlugin));
            })
            .unwrap();
        let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
        h.app
            .insert_resource(load_achievement_catalog(&conn).unwrap());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("level-death.redb");
        let db = redb::Database::create(&path).unwrap();
        persistence::create_tables(&db);
        let (character, _) = persistence::create_character(&db, 1, "Leveldeath", 1, 1).unwrap();
        h.app.insert_resource(GameDatabase::new(db));
        h.app
            .world_mut()
            .get_mut::<InWorld>(h.connection)
            .unwrap()
            .character_id = character;
        h.app.world_mut().entity_mut(h.caster).insert((
            persistence::ClientId(character),
            PlayerAchievementState::default(),
            Level(level),
        ));
        h.app.update();
        crate::combat::kill_unit(h.app.world_mut(), h.npc, h.caster);
        h.advance(3);
        assert_eq!(h.health(h.caster), 0.0);
        let state = h
            .app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap();
        for criterion in [16188, 16402] {
            assert_eq!(
                state.criteria_progress.get(&criterion).copied(),
                (expected == Some(criterion)).then_some(1),
                "level {level}, criterion {criterion}"
            );
        }
        crate::combat::kill_unit(h.app.world_mut(), h.npc, h.caster);
        h.advance(3);
        drop(h);
        let reopened = redb::Database::open(&path).unwrap();
        let saved = persistence::load_achievements(&reopened, character);
        for criterion in [16188, 16402] {
            assert_eq!(
                saved.criteria_progress.get(&criterion).copied(),
                (expected == Some(criterion)).then_some(1),
                "reopened level {level}"
            );
        }
    }
}

#[test]
fn achievements_criteria_native_instance_death_uses_allocated_capacity_and_persists() {
    for (map_id, difficulty, allocated, expected) in [
        (33, 1, true, Some(6847)),
        (533, 3, true, Some(6845)),
        (533, 4, true, Some(6846)),
        (0, 1, true, None),
        (533, 3, false, None),
    ] {
        let mut h =
            crate::spell_cast::test_harness::harness_with_plugins(vec![], &[], 1, false, |app| {
                app.add_plugins((AchievementPlugin, crate::death::DeathPlugin));
            })
            .unwrap();
        let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
        h.app
            .insert_resource(load_achievement_catalog(&conn).unwrap());
        let store = crate::maps::store::load_map_store(&conn).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("instance-death.redb");
        let db = redb::Database::create(&path).unwrap();
        persistence::create_tables(&db);
        let (character, _) = persistence::create_character(&db, 1, "Copydeath", 1, 1).unwrap();
        h.app.insert_resource(GameDatabase::new(db));
        h.app
            .world_mut()
            .get_mut::<InWorld>(h.connection)
            .unwrap()
            .character_id = character;
        let mut copies = crate::maps::InstanceManager::default();
        let map = if allocated {
            copies
                .create_map(
                    &store,
                    map_id,
                    crate::maps::instances::InstanceEntrant {
                        character_id: character,
                        leader: character,
                        difficulty_id: difficulty,
                        raid_difficulty: crate::maps::RaidDifficulty {
                            primary: 14,
                            legacy: difficulty,
                        },
                        ..Default::default()
                    },
                    crate::maps::instances::Now {
                        elapsed: 0.0,
                        unix: 1780000000,
                    },
                )
                .unwrap()
                .map
        } else {
            InMap {
                map_id,
                instance_id: 999999,
            }
        };
        h.app.insert_resource(store);
        h.app.insert_resource(copies);
        h.app.world_mut().entity_mut(h.caster).insert((
            persistence::ClientId(character),
            PlayerAchievementState::default(),
            map,
        ));
        h.app.world_mut().entity_mut(h.npc).insert(map);
        h.app.update();
        // A real Unit::Kill commits health/death; no synthetic criteria credit.
        crate::combat::kill_unit(h.app.world_mut(), h.npc, h.caster);
        h.advance(3);
        assert_eq!(h.health(h.caster), 0.0);
        let state = h
            .app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap();
        for criterion in [
            6847, 7849, 6845, 6846, 8838, 8918, 8919, 50010, 50011, 69351,
        ] {
            assert_eq!(
                state.criteria_progress.get(&criterion).copied(),
                (expected == Some(criterion)).then_some(1),
                "map {map:?}, difficulty {difficulty}, criterion {criterion}"
            );
        }
        crate::combat::kill_unit(h.app.world_mut(), h.npc, h.caster);
        h.app
            .world_mut()
            .write_message(crate::death::PlayerDeathTransition { player: h.caster });
        h.advance(3);
        drop(h);
        let reopened = redb::Database::open(&path).unwrap();
        let saved = persistence::load_achievements(&reopened, character);
        for criterion in [
            6847, 7849, 6845, 6846, 8838, 8918, 8919, 50010, 50011, 69351,
        ] {
            assert_eq!(
                saved.criteria_progress.get(&criterion).copied(),
                (expected == Some(criterion)).then_some(1),
                "idle/reopened {map:?}"
            );
        }
    }
}

#[test]
fn achievements_criteria_native_player_death_records_once_with_actual_killer_and_map() {
    prove_death_criteria(false);
    prove_death_criteria(true);
}

fn prove_death_criteria(player_killer: bool) {
    use shared::components::Npc;
    let mut h =
        crate::spell_cast::test_harness::harness_with_plugins(vec![], &[], 1, false, |app| {
            app.add_plugins((AchievementPlugin, crate::death::DeathPlugin));
        })
        .unwrap();
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(load_achievement_catalog(&conn).unwrap());
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 1, "Deathproof", 1, 1).unwrap();
    h.app.insert_resource(GameDatabase::new(db));
    h.app
        .world_mut()
        .get_mut::<InWorld>(h.connection)
        .unwrap()
        .character_id = character;
    let map = InMap {
        map_id: 533,
        instance_id: 17,
    };
    h.app.world_mut().entity_mut(h.caster).insert((
        persistence::ClientId(character),
        PlayerAchievementState::default(),
        map,
    ));
    h.app.world_mut().entity_mut(h.npc).insert((
        Npc {
            template_id: 448,
            name: "Hogger".into(),
        },
        map,
    ));
    h.app.update();
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&111)
    );
    let killer = if player_killer {
        let killer = crate::spell_cast::test_harness::spawn_player(&mut h.app, 1, 3.0);
        h.app.world_mut().entity_mut(killer).insert(map);
        killer
    } else {
        h.npc
    };
    let killer_criterion = if player_killer { 5594 } else { 455 };
    let other_killer_criterion = if player_killer { 455 } else { 5594 };
    // Native Unit::Kill reaches the accepted death transition, not a seeded counter.
    crate::combat::kill_unit(h.app.world_mut(), killer, h.caster);
    h.advance(3);
    assert_eq!(h.health(h.caster), 0.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::death::PlayerDeathState>(h.caster)
            .unwrap()
            .state,
        shared::death::DeathState::Dead
    );
    for criterion in [111, 943, killer_criterion] {
        assert_eq!(
            h.app
                .world()
                .get::<PlayerAchievementState>(h.caster)
                .unwrap()
                .criteria_progress
                .get(&criterion),
            Some(&1),
            "criterion{criterion}"
        );
    }
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&other_killer_criterion)
    );
    h.app
        .world_mut()
        .write_message(crate::death::PlayerDeathTransition { player: h.caster });
    crate::combat::kill_unit(h.app.world_mut(), killer, h.caster);
    h.advance(3);
    for criterion in [111, 943, killer_criterion] {
        assert_eq!(
            h.app
                .world()
                .get::<PlayerAchievementState>(h.caster)
                .unwrap()
                .criteria_progress
                .get(&criterion),
            Some(&1)
        );
    }
    drop(h);
    let db = redb::Database::open(file.path()).unwrap();
    let saved = persistence::load_achievements(&db, character);
    for criterion in [111, 943, killer_criterion] {
        assert_eq!(saved.criteria_progress.get(&criterion), Some(&1));
    }
}

#[test]
fn achievements_criteria_native_spy_marker_requires_known_solo_group_authority() {
    use crate::group::{GroupRegistry, OnlineMember, OnlineView};
    let mut h =
        crate::spell_cast::test_harness::harness_with_plugins(vec![], &[280339], 1, false, |app| {
            app.add_plugins(AchievementPlugin);
        })
        .unwrap();
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(load_achievement_catalog(&conn).unwrap());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("solo-spy.redb");
    let db = redb::Database::create(&path).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 1, "Spyproof", 1, 1).unwrap();
    h.app.insert_resource(GameDatabase::new(db));
    h.app
        .world_mut()
        .get_mut::<InWorld>(h.connection)
        .unwrap()
        .character_id = character;
    h.app.world_mut().entity_mut(h.caster).insert((
        PlayerAchievementState::default(),
        persistence::ClientId(character),
    ));
    h.app.update();
    // A native cast with no authoritative group registry cannot establish solo status.
    h.app.world_mut().remove_resource::<GroupRegistry>();
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 280339, None, 0);
    h.app.update();
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&41658)
    );
    h.app.init_resource::<GroupRegistry>();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .remove::<persistence::ClientId>();
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 280339, None, 0);
    h.app.update();
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&41658)
    );
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(persistence::ClientId(character));
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 280339, None, u8::MAX);
    h.app.update();
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&41658)
    );
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 280339, None, 0);
    h.app.update();
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&41658),
        Some(&1)
    );
    assert!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .earned_ids
            .contains(&13026)
    );
    let online: OnlineView = [(character, "Spyproof"), (character + 1, "Offlinefriend")]
        .into_iter()
        .map(|(id, name)| {
            (
                id,
                OnlineMember {
                    entity: id,
                    name: name.into(),
                    class: 1,
                    level: 1,
                    portrait: Default::default(),
                    dungeon_difficulty: 1,
                    raid_difficulty: Default::default(),
                },
            )
        })
        .collect();
    let mut groups = h.app.world_mut().resource_mut::<GroupRegistry>();
    groups.invite(character, "Offlinefriend", &online);
    groups.respond_invite(character + 1, true);
    assert_eq!(groups.group_members(character).len(), 2);
    // The roster includes an offline member; physical co-location cannot imply solo.
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 280339, None, 0);
    h.app.update();
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&41658),
        Some(&1)
    );
    h.app.update();
    drop(h);
    let reopened = redb::Database::open(&path).unwrap();
    assert_eq!(
        persistence::load_achievements(&reopened, character)
            .criteria_progress
            .get(&41658),
        Some(&1)
    );
}

#[test]
fn achievements_criteria_targeted_spell_credits_caster_after_native_hit_only() {
    let mut h =
        crate::spell_cast::test_harness::harness_with_plugins(vec![], &[61781], 1, false, |app| {
            app.add_plugins(AchievementPlugin);
        })
        .unwrap();
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(load_achievement_catalog(&conn).unwrap());
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 1, "Turkeyproof", 1, 1).unwrap();
    h.app.insert_resource(GameDatabase::new(db));
    h.app
        .world_mut()
        .get_mut::<InWorld>(h.connection)
        .unwrap()
        .character_id = character;
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(PlayerAchievementState::default());
    let target = crate::spell_cast::test_harness::spawn_player(&mut h.app, 1, 2.0);
    h.app
        .world_mut()
        .entity_mut(target)
        .insert(PlayerAchievementState::default());
    h.app.update();
    // Authored spell61781/effect55694 selects an allied unit and applies turkey feathers.
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 61781, Some(target), u8::MAX);
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&61577)
    );
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 61781, Some(target), 0);
    // Authored speed15 delays the hit; launching alone cannot credit type110.
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&61577)
    );
    h.advance(20);
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(target)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == 61781)
    );
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&61577),
        Some(&1)
    );
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(target)
            .unwrap()
            .criteria_progress
            .contains_key(&61577)
    );
    h.app.update();
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&61577),
        Some(&1)
    );
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 61781, Some(target), 0);
    h.advance(20);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&61577),
        Some(&2)
    );
    drop(h);
    let db = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        persistence::load_achievements(&db, character)
            .criteria_progress
            .get(&61577),
        Some(&2)
    );
}

#[test]
fn achievements_criteria_gain_aura_credits_new_application_not_refresh_or_dead_target() {
    let mut h =
        crate::spell_cast::test_harness::harness_with_plugins(vec![], &[76145], 1, false, |app| {
            app.add_plugins(AchievementPlugin);
        })
        .unwrap();
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(load_achievement_catalog(&conn).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(PlayerAchievementState::default());
    h.app.update();
    let application = crate::auras::AuraApplication {
        caster: h.npc,
        target: h.caster,
        spell_id: 76145,
        effect_mask: 1,
        source: crate::auras::AuraSource::Cast,
    };
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(h.caster)
        .unwrap()
        .current = 0.0;
    crate::auras::apply_aura(h.app.world_mut(), &application);
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&13673)
    );
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(h.caster)
        .unwrap()
        .current = 1000.0;
    crate::auras::apply_aura(h.app.world_mut(), &application);
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == 76145)
    );
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&13673),
        Some(&1)
    );
    crate::auras::apply_aura(h.app.world_mut(), &application);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&13673),
        Some(&1)
    );
    h.app.update();
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&13673),
        Some(&1)
    );
}

#[test]
fn achievements_criteria_game_event_credits_authentic_send_event_caster_only() {
    let mut h = crate::spell_cast::test_harness::harness_with_plugins(
        vec![],
        &[1217323],
        1,
        false,
        |app| {
            app.add_plugins(AchievementPlugin);
        },
    )
    .unwrap();
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(load_achievement_catalog(&conn).unwrap());
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 1, "Eventproof", 1, 1).unwrap();
    h.app.insert_resource(GameDatabase::new(db));
    h.app
        .world_mut()
        .get_mut::<InWorld>(h.connection)
        .unwrap()
        .character_id = character;
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(PlayerAchievementState::default());
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(PlayerAchievementState::default());
    h.app.update();
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 1217323, None, u8::MAX);
    h.app.update();
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&102782)
    );
    // The Hataclysm's real spell1217323 sends event95324 with no unit target.
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 1217323, None, 0);
    h.app.update();
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&102782),
        Some(&1)
    );
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.npc)
            .unwrap()
            .criteria_progress
            .contains_key(&102782)
    );
    h.app.update();
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&102782),
        Some(&1)
    );
    drop(h);
    let db = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        persistence::load_achievements(&db, character)
            .criteria_progress
            .get(&102782),
        Some(&1)
    );
}

#[test]
fn achievements_criteria_known_spell_credits_real_learning_once_and_rejected_cast_never() {
    use crate::class_progression::{ClassProgressionPlugin, KnownSpells};
    use crate::profession::PlayerProfessions;
    let mut h = crate::spell_cast::test_harness::harness_with_plugins(
        vec![],
        &[42426, 32298],
        1,
        false,
        |app| {
            app.add_plugins((AchievementPlugin, ClassProgressionPlugin));
        },
    )
    .unwrap();
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(load_achievement_catalog(&conn).unwrap());
    h.app
        .insert_resource(crate::profession::load_profession_data(&conn).unwrap());
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 1, "Learnproof", 1, 1).unwrap();
    h.app.insert_resource(GameDatabase::new(db));
    h.app
        .world_mut()
        .get_mut::<InWorld>(h.connection)
        .unwrap()
        .character_id = character;
    h.app.world_mut().entity_mut(h.caster).insert((
        crate::persistence::ClientId(character),
        PlayerAchievementState::default(),
        PlayerProfessions::default(),
        KnownSpells::fixture(vec![42426]),
    ));
    h.app.update();
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 42426, Some(h.caster), u8::MAX);
    for _ in 0..3 {
        h.app.update();
    }
    assert!(
        !h.app
            .world()
            .get::<KnownSpells>(h.caster)
            .unwrap()
            .contains(32298)
    );
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&6155)
    );
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 42426, Some(h.caster), 0);
    for _ in 0..3 {
        h.app.update();
    }
    assert!(
        h.app
            .world()
            .get::<KnownSpells>(h.caster)
            .unwrap()
            .contains(32298)
    );
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&6155),
        Some(&1)
    );
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 42426, Some(h.caster), 0);
    for _ in 0..3 {
        h.app.update();
    }
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&6155),
        Some(&1)
    );
    assert_eq!(
        persistence::load_achievements(&h.app.world().resource::<GameDatabase>().db, character)
            .criteria_progress
            .get(&6155),
        Some(&1)
    );
}

#[test]
fn achievements_criteria_rank_totals_follow_committed_authentic_quest_reputation() {
    use crate::world_quest_rewards::{Rewards, load_settlement};
    use shared::reputation::CharacterReputation;
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 17, "Rankproof", 1, 1).unwrap();
    let reputation: CharacterReputation = serde_json::from_value(serde_json::json!({
        "values": {"1090":41999,"1859":8999,"1273":42000,"2511":42000,"2089":42000,"999999":42000},
        "tracked": [], "watched": null
    }))
    .unwrap();
    persistence::save_reputation(&db, character, &reputation);
    let mut app = App::new();
    app.add_plugins(AchievementPlugin);
    app.insert_resource(load_achievement_catalog(&conn).unwrap());
    app.insert_resource(GameDatabase::new(db));
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Rankproof".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            PlayerAchievementState::default(),
            reputation,
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            Authenticated { account_id: 17 },
            PlayerLink(player),
            InWorld {
                character_id: character,
            },
        ))
        .id();
    app.update();
    let state = app.world().get::<PlayerAchievementState>(player).unwrap();
    assert_eq!(state.criteria_progress.get(&5299), Some(&1));
    assert_eq!(state.criteria_progress.get(&5300), Some(&1));
    assert!(!state.criteria_progress.contains_key(&982));
    let mut rewards = Rewards::default();
    let mut query = conn.prepare("SELECT reward_id,amount,cap FROM content_world_quest_reward WHERE quest_id=45068 AND kind='reputation'").unwrap();
    for row in query
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, u8>(2)?,
            ))
        })
        .unwrap()
    {
        let (id, amount, cap) = row.unwrap();
        rewards
            .reputation
            .push(shared::protocol::WorldQuestNumericReward { id, amount });
        rewards.reputation_caps.insert(id, cap);
    }
    assert_eq!(rewards.reputation.len(), 2);
    let db = &app.world().resource::<GameDatabase>().db;
    let settlement = load_settlement(db, character, 1, Some(&rewards))
        .unwrap()
        .unwrap();
    let txn = db.begin_write().unwrap();
    settlement.persist_in(&txn, character);
    txn.commit().unwrap();
    settlement.publish(app.world_mut(), connection);
    app.update();
    let state = app.world().get::<PlayerAchievementState>(player).unwrap();
    assert_eq!(state.criteria_progress.get(&982), Some(&1));
    assert_eq!(state.criteria_progress.get(&5299), Some(&1));
    assert_eq!(state.criteria_progress.get(&5300), Some(&2));
    // Native traditional reputation loss recomputes current totals, not historical maxima.
    app.world_mut()
        .get_mut::<CharacterReputation>(player)
        .unwrap()
        .lose_rep(1090, 33001, shared::reputation::RepGainSource::QuestTurnIn);
    app.update();
    app.update();
    let state = app.world().get::<PlayerAchievementState>(player).unwrap();
    assert_eq!(state.criteria_progress.get(&982), Some(&0));
    assert_eq!(state.criteria_progress.get(&5299), Some(&0));
    assert_eq!(state.criteria_progress.get(&5300), Some(&1));
    drop(app);
    let reopened = redb::Database::open(file.path()).unwrap();
    let saved = persistence::load_achievements(&reopened, character);
    assert_eq!(saved.criteria_progress.get(&982), Some(&0));
    assert_eq!(saved.criteria_progress.get(&5300), Some(&1));
}

#[test]
fn achievements_criteria_reputation_tracks_signed_changes_and_grants_real_timbers() {
    use shared::components::CharacterAppearance;
    use shared::reputation::CharacterReputation;
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 17, "Timberproof", 1, 1).unwrap();
    let mut app = App::new();
    app.add_plugins(AchievementPlugin);
    app.insert_resource(load_achievement_catalog(&conn).unwrap());
    app.insert_resource(GameDatabase::new(db));
    let reputation: CharacterReputation = serde_json::from_value(serde_json::json!({
        "values": {"576": 41999}, "tracked": [], "watched": null
    }))
    .unwrap();
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Timberproof".into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            PlayerAchievementState::default(),
            reputation,
        ))
        .id();
    app.world_mut().spawn((
        Authenticated { account_id: 17 },
        PlayerLink(player),
        InWorld {
            character_id: character,
        },
    ));
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&2011),
        Some(&41999)
    );
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .account
            .as_deref()
            .unwrap()
            .earned_ids
            .contains(&944)
    );
    app.world_mut()
        .get_mut::<CharacterReputation>(player)
        .unwrap()
        .set_rep(576, 42000);
    app.update();
    let state = app.world().get::<PlayerAchievementState>(player).unwrap();
    assert!(state.account.as_deref().unwrap().earned_ids.contains(&944));
    assert_eq!(
        state
            .account
            .as_deref()
            .unwrap()
            .criteria_progress
            .get(&2011),
        Some(&42000)
    );
    app.world_mut()
        .get_mut::<CharacterReputation>(player)
        .unwrap()
        .set_rep(576, 21000);
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&2011),
        Some(&21000)
    );
    app.world_mut()
        .get_mut::<CharacterReputation>(player)
        .unwrap()
        .set_rep(576, -2900);
    app.update();
    // TC ignores nonpositive standing; it does not invent an unsigned42000 offset.
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&2011),
        Some(&21000)
    );
    assert!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .account
            .as_deref()
            .unwrap()
            .earned_ids
            .contains(&944)
    );
}

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
    world.insert_resource(GameDatabase::new(db));
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
    world.insert_resource(GameDatabase::new(db));
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

#[test]
fn achievements2_spell_targets_credit_recipient_not_caster_and_count_once_per_hit() {
    let mut h =
        crate::spell_cast::test_harness::harness_with(vec![], &[24733, 24732], 1, false).unwrap();
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
    let recipient = crate::spell_cast::test_harness::spawn_player(&mut h.app, 1, 2.0);
    h.app
        .world_mut()
        .entity_mut(recipient)
        .insert(PlayerAchievementState::default());
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 24733, Some(recipient), u8::MAX);
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(recipient)
            .unwrap()
            .criteria_progress
            .contains_key(&3209)
    );
    for expected in [1, 2] {
        crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 24733, Some(recipient), 0);
        assert_eq!(
            h.app
                .world()
                .get::<PlayerAchievementState>(recipient)
                .unwrap()
                .criteria_progress
                .get(&3209),
            Some(&expected)
        );
    }
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&3209)
    );
}

#[test]
fn achievements2_account_migration_keeps_dates_and_distinct_counter_semantics_after_deletion() {
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let catalog = load_achievement_catalog(&conn).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (first, _) = persistence::create_character(&db, 17, "Counterone", 1, 1).unwrap();
    let (second, _) = persistence::create_character(&db, 17, "Countertwo", 1, 1).unwrap();
    let mut a = PlayerAchievementState::default();
    a.criteria_progress
        .extend([(1832, 2), (5565, 100), (38400, 1)]);
    a.earned_ids.insert(4899);
    a.earned_at.insert(4899, 123);
    let mut b = PlayerAchievementState::default();
    b.criteria_progress
        .extend([(1832, 3), (5565, 200), (38400, 1)]);
    b.earned_ids.insert(4899);
    b.earned_at.insert(4899, 100);
    persistence::save_achievements(&db, second, &b);
    let db = GameDatabase::new(db);
    let (aggregate, toasts) = account_store::save_account_progress(
        &db,
        &a,
        &catalog,
        account_store::AccountUpdate {
            account: 17,
            character: first,
            completed_map: None,
            race: 1,
            now: 999,
        },
    );
    assert_eq!(aggregate.criteria_progress[&1832], 5);
    assert_eq!(aggregate.criteria_progress[&5565], 200);
    assert_eq!(aggregate.criteria_progress[&38400], 1);
    assert_eq!(aggregate.earned_at[&4899], 100);
    assert!(!toasts.iter().any(|toast| toast.achievement_id == 4899));
    let db = db.into_database();
    assert_eq!(persistence::load_achievements(&db, first), a);
    assert_eq!(persistence::load_achievements(&db, second), b);
    persistence::delete_character(&db, 17, second).unwrap();
    drop(db);
    let db = GameDatabase::new(redb::Database::open(file.path()).unwrap());
    let (aggregate, _) = account_store::save_account_progress(
        &db,
        &a,
        &catalog,
        account_store::AccountUpdate {
            account: 17,
            character: first,
            completed_map: None,
            race: 1,
            now: 1000,
        },
    );
    assert_eq!(aggregate.criteria_progress[&1832], 5);
    assert_eq!(aggregate.earned_at[&4899], 100);
}

#[test]
fn achievements_criteria_exploration_does_not_complete_an_entire_zone_from_one_area() {
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let catalog = load_achievement_catalog(&conn).unwrap();
    let mut state = PlayerAchievementState::default();
    // Real overlay117 requires Area800; another area is not equivalent.
    catalog.update(
        &mut state,
        CriteriaEvent::ExploreAreaBit(catalog.area_bit(87).unwrap()),
        None,
        1,
    );
    assert!(!state.criteria_progress.contains_key(&502));
    catalog.update(
        &mut state,
        CriteriaEvent::ExploreAreaBit(catalog.area_bit(800).unwrap()),
        None,
        1,
    );
    assert_eq!(state.criteria_progress.get(&502), Some(&1));
    assert!(!state.earned_ids.contains(&627));
}

#[test]
fn achievements_criteria_reputation_rejects_unmigrated_unsigned_counter() {
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let catalog = load_achievement_catalog(&conn).unwrap();
    let mut state = PlayerAchievementState::default();
    state.criteria_progress.insert(2011, 63_000);
    catalog.update(&mut state, CriteriaEvent::ReachLevel(1), None, 1);
    assert!(!state.criteria_progress.contains_key(&2011));
    assert!(state.signed_reputation);
    assert!(
        !catalog
            .aggregate(std::iter::once(PlayerAchievementState {
                criteria_progress: BTreeMap::from([(2011, 63_000)]),
                ..Default::default()
            }))
            .criteria_progress
            .contains_key(&2011)
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
            record_quest_reward(&mut world, 42, quest, false, 0);
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

/// Predator10334: criterion28899 KillCreature96235 behind TC PlayerHasAura8 Nether Trap190820
/// (CriteriaHandler1798): a native kill credits only while the killer holds the aura.
#[test]
fn achievements_criteria_player_has_aura_gates_native_kill_credit() {
    use shared::components::Npc;
    let mut h =
        crate::spell_cast::test_harness::harness_with_plugins(vec![], &[190820], 1, false, |app| {
            app.add_plugins(AchievementPlugin);
        })
        .unwrap();
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(load_achievement_catalog(&conn).unwrap());
    let map = InMap::continent(1);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((PlayerAchievementState::default(), map));
    let progress = |h: &crate::spell_cast::test_harness::Harness| {
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&28899)
            .copied()
    };
    let kill_trapped_demon = |h: &mut crate::spell_cast::test_harness::Harness| {
        let demon = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 3.0);
        h.app.world_mut().entity_mut(demon).insert((
            Npc {
                template_id: 96235,
                name: "Trapped demon".into(),
            },
            map,
        ));
        crate::combat::kill_unit(h.app.world_mut(), h.caster, demon);
        h.advance(1);
    };
    kill_trapped_demon(&mut h);
    assert_eq!(progress(&h), None, "no Nether Trap aura");
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 190820,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 190820));
    kill_trapped_demon(&mut h);
    assert_eq!(progress(&h), Some(1), "kill under Nether Trap");
    assert!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .earned_ids
            .contains(&10334)
    );
}

/// Twin Peaks Killing Blows5797: criterion17416 DeliveredKillingBlow56 behind TC PlayerMap32
/// 726 (CriteriaHandler1922): only a native killing blow on that map counts.
#[test]
fn achievements_criteria_native_zero_map_killing_blow_requires_known_world_map_and_persists() {
    for (map, qualifies) in [(Some(0), true), (Some(1), false), (None, false)] {
        let mut h =
            crate::spell_cast::test_harness::harness_with_plugins(vec![], &[], 1, false, |app| {
                app.add_plugins(AchievementPlugin);
            })
            .unwrap();
        let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
        h.app
            .insert_resource(load_achievement_catalog(&conn).unwrap());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("zero-map-blow.redb");
        let db = redb::Database::create(&path).unwrap();
        persistence::create_tables(&db);
        let (character, _) = persistence::create_character(&db, 1, "Worldblow", 1, 1).unwrap();
        h.app.insert_resource(GameDatabase::new(db));
        h.app
            .world_mut()
            .get_mut::<InWorld>(h.connection)
            .unwrap()
            .character_id = character;
        h.app.world_mut().entity_mut(h.caster).insert((
            persistence::ClientId(character),
            PlayerAchievementState::default(),
        ));
        if let Some(map) = map {
            h.app
                .world_mut()
                .entity_mut(h.caster)
                .insert(InMap::continent(map));
            h.app
                .world_mut()
                .entity_mut(h.npc)
                .insert(InMap::continent(map));
        } else {
            h.app.world_mut().entity_mut(h.caster).remove::<InMap>();
        }
        h.app.update();
        assert!(
            !h.app
                .world()
                .get::<PlayerAchievementState>(h.caster)
                .unwrap()
                .criteria_progress
                .contains_key(&5512),
            "known map alone cannot credit"
        );
        crate::combat::kill_unit(h.app.world_mut(), h.caster, h.npc);
        h.advance(1);
        assert_eq!(h.health(h.npc), 0.0);
        let state = h
            .app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap();
        assert_eq!(
            state.criteria_progress.get(&5512),
            qualifies.then_some(&1),
            "native killing blow map {map:?}"
        );
        let shared::protocol::AchievementCatalogPage::Criteria { criteria, .. } =
            h.app.world().resource::<AchievementCatalog>().page(
                state,
                shared::protocol::QueryAchievementCatalog::Criteria {
                    achievement_id: 1488,
                    after_id: 0,
                },
                1,
            )
        else {
            panic!("World Killing Blows criteria reply")
        };
        let line = criteria.iter().find(|row| row.criteria_id == 5512).unwrap();
        assert!(
            line.progress_supported,
            "existing statistic1488 exposes the zero-map counter"
        );
        assert_eq!(
            line.current,
            u64::from(qualifies),
            "authoritative statistic criterion map {map:?}"
        );
        assert!(
            !state.earned_ids.contains(&1488),
            "statistics are not ordinary grants"
        );
        assert!(
            !state.criteria_progress.contains_key(&5492),
            "no inferred world-honor credit"
        );
        crate::combat::kill_unit(h.app.world_mut(), h.caster, h.npc);
        h.advance(3);
        drop(h);
        let reopened = redb::Database::open(&path).unwrap();
        let saved = persistence::load_achievements(&reopened, character);
        assert_eq!(
            saved.criteria_progress.get(&5512),
            qualifies.then_some(&1),
            "dead/idle/reopen map {map:?}"
        );
        assert!(!saved.criteria_progress.contains_key(&5492));
    }
}

#[test]
fn achievements_criteria_player_map_gates_native_killing_blow() {
    for (map_id, expected) in [(1, None), (726, Some(1))] {
        let mut h =
            crate::spell_cast::test_harness::harness_with_plugins(vec![], &[], 1, false, |app| {
                app.add_plugins(AchievementPlugin);
            })
            .unwrap();
        let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
        h.app
            .insert_resource(load_achievement_catalog(&conn).unwrap());
        let map = InMap {
            map_id,
            instance_id: 1,
        };
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert((PlayerAchievementState::default(), map));
        h.app.world_mut().entity_mut(h.npc).insert(map);
        crate::combat::kill_unit(h.app.world_mut(), h.caster, h.npc);
        h.advance(1);
        assert_eq!(
            h.app
                .world()
                .get::<PlayerAchievementState>(h.caster)
                .unwrap()
                .criteria_progress
                .get(&17416)
                .copied(),
            expected,
            "map {map_id}"
        );
    }
}

/// Dynamic Flight Power 1 40172 (account): criterion67179 ReachLevel5 behind TC
/// PlayerLevelEqualOrGreaterThan69 10 (CriteriaHandler2094) at the level-up update.
#[test]
fn achievements_criteria_player_level_gates_reach_level_credit() {
    use shared::components::CharacterAppearance;
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let mut world = World::new();
    world.insert_resource(load_achievement_catalog(&conn).unwrap());
    world.init_resource::<crate::maps::ServerClock>();
    let player = world
        .spawn((
            Player {
                name: "Levelproof".into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            Level(9),
            InMap::continent(0),
            PlayerAchievementState::default(),
        ))
        .id();
    let progress = |world: &World| {
        world
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&67179)
            .copied()
    };
    update_player_events(&mut world, player, &[CriteriaEvent::ReachLevel(9)], None).unwrap();
    assert_eq!(progress(&world), None, "level 9");
    world.entity_mut(player).insert(Level(10));
    update_player_events(&mut world, player, &[CriteriaEvent::ReachLevel(10)], None).unwrap();
    assert_eq!(progress(&world), Some(10), "level 10");
}

/// What the Eck? 1864: criterion7136 KillCreature29306 behind All4 of legacy Heroic
/// (Difficulty2/4/39) and PlayerHasAura Eck Residue55817; both must hold.
#[test]
fn achievements_criteria_mixed_difficulty_and_aura_tree_needs_both() {
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let catalog = load_achievement_catalog(&conn).unwrap();
    for (difficulty, aura, expected) in [(2, true, Some(1)), (2, false, None), (1, true, None)] {
        let conditions = PlayerConditions {
            difficulty,
            player: Some(ReferencePlayer {
                auras: aura.then_some(55817).into_iter().collect(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut state = PlayerAchievementState::default();
        catalog.update_at(
            &mut state,
            CriteriaEvent::KillCreature(29306),
            None,
            1,
            &conditions,
        );
        assert_eq!(
            state.criteria_progress.get(&7136).copied(),
            expected,
            "difficulty {difficulty}, aura {aura}"
        );
    }
}

/// Glory of the War Within Delver40438: criterion112574 KillCreature221427 behind
/// HasTimeEventPassed289 600, which TC defaults to now (CriteriaHandler3578-3611), so
/// it holds even for map-wide credit without a reference player.
#[test]
fn achievements_criteria_untimestamped_time_event_has_always_passed() {
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let catalog = load_achievement_catalog(&conn).unwrap();
    let mut state = PlayerAchievementState::default();
    catalog.update_at(
        &mut state,
        CriteriaEvent::KillCreature(221427),
        None,
        1,
        &PlayerConditions::default(),
    );
    assert_eq!(state.criteria_progress.get(&112574).copied(), Some(1));
}

/// Scenario and Adventure Guide credit use the same reference-player conditions:
/// criterion16784 (legacy Heroic Erudax) under Erudax kills5733 needs a Heroic copy.
#[test]
fn achievements_criteria_tree_event_credit_honors_difficulty_conditions() {
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let catalog = load_achievement_catalog(&conn).unwrap();
    for (difficulty, credited) in [(1, false), (2, true)] {
        let mut state = PlayerAchievementState::default();
        let conditions = PlayerConditions {
            difficulty,
            ..Default::default()
        };
        assert_eq!(
            catalog.update_tree_event(
                20570,
                &mut state,
                CriteriaEvent::KillCreature(40484),
                &conditions
            ),
            credited,
            "difficulty {difficulty}"
        );
        assert_eq!(
            state.criteria_progress.get(&16784).copied(),
            credited.then_some(1)
        );
    }
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
        ..Default::default()
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
