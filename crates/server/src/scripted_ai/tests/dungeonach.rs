use super::*;
use crate::achievement::{AchievementCatalog, PlayerAchievementState, load_achievement_catalog};
use shared::protocol::{
    AchievementCatalogPage, AchievementDelta, AchievementStateUpdate, DungeonProgress,
    QueryAchievementCatalog,
};

#[test]
fn dungeon2_stockade_entry_kills_reconnect_and_earned_category() {
    let data = data().expect("Stockade DB2 and script fixtures");
    let mut app = stockade_app(data);
    let conn = Connection::open(world_db_path()).unwrap();
    app.insert_resource(load_achievement_catalog(&conn).unwrap());
    app.insert_resource(crate::maps::ServerClock {
        offset_secs: 0,
        fixed_unix: Some(1_791_262_800),
    });
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    crate::persistence::create_tables(&db);
    app.insert_resource(crate::persistence::GameDatabase { db });
    let (mut player, mut connection) =
        spawn_player(&mut app, COPY, [158.0, 116.0, -35.1, 0.0], 1001);
    let (other, other_connection) =
        spawn_player(&mut app, OTHER_COPY, [158.0, 116.0, -35.1, 0.0], 1002);
    let (peer, peer_connection) = spawn_player(&mut app, COPY, [158.0, 116.0, -35.1, 0.0], 1003);
    for (player, connection, id) in [
        (player, connection, 1001),
        (other, other_connection, 1002),
        (peer, peer_connection, 1003),
    ] {
        app.world_mut().entity_mut(player).insert((
            PlayerAchievementState::default(),
            crate::persistence::ClientId(id),
        ));
        attach_dungeon2_messages(app.world_mut(), connection);
    }
    let ordered: Vec<_> = data
        .maps
        .encounters_for_map(34, 1)
        .iter()
        .map(|row| (row.id, row.order_index, row.bit))
        .collect();
    assert_eq!(
        ordered,
        vec![(1144, 1000, 3), (1145, 2000, 4), (1146, 3000, 5)]
    );
    assert!(data.maps.encounters_for_map(34, 2).is_empty());
    let mut h = Harness {
        app,
        players: vec![
            (player, connection, Inbox::default()),
            (other, other_connection, Inbox::default()),
            (peer, peer_connection, Inbox::default()),
        ],
    };
    assert_initial_dungeon2_progress(&mut h, connection, peer_connection);
    drain::<DungeonProgress>(h.app.world_mut(), other_connection);
    let mut mask = 0;
    for (index, &(encounter, _, bit)) in ordered.iter().enumerate() {
        let (entry, boss_id) = match encounter {
            1144 => (NPC_HOGGER, DATA_HOGGER),
            1145 => (NPC_LORD_OVERHEAT, DATA_LORD_OVERHEAT),
            1146 => (NPC_RANDOLPH_MOLOCH, DATA_RANDOLPH_MOLOCH),
            _ => unreachable!(),
        };
        let boss = spawn_boss(&mut h.app, data, entry, COPY);
        h.advance(0.05);
        // Death is the real combat boundary; no direct SetBossState or achievement injection.
        h.app
            .world_mut()
            .entity_mut(boss)
            .insert(crate::combat::LastDamager(player));
        h.app.world_mut().get_mut::<Health>(boss).unwrap().current = 0.0;
        crate::combat::mark_npc_dead_world(h.app.world_mut(), boss);
        // Host receivers are frame-local. Drain each update, as the existing
        // encounter/chat inbox does, rather than losing the toast next frame.
        h.advance(0.05);
        let mut updates = drain::<AchievementStateUpdate>(h.app.world_mut(), connection);
        let mut dungeon_updates = drain::<DungeonProgress>(h.app.world_mut(), connection);
        let mut peer_updates = drain::<DungeonProgress>(h.app.world_mut(), peer_connection);
        h.advance(0.05);
        updates.extend(drain::<AchievementStateUpdate>(
            h.app.world_mut(),
            connection,
        ));
        dungeon_updates.extend(drain::<DungeonProgress>(h.app.world_mut(), connection));
        peer_updates.extend(drain::<DungeonProgress>(h.app.world_mut(), peer_connection));
        assert_eq!(peer_updates, dungeon_updates);
        assert_eq!(dungeon_updates.len(), 1);
        let expected: Vec<_> = ordered.iter().take(index + 1).map(|row| row.0).collect();
        assert_eq!(
            dungeon_updates[0]
                .encounters
                .iter()
                .filter(|e| e.defeated)
                .map(|e| e.encounter_id)
                .collect::<Vec<_>>(),
            expected
        );
        assert!(drain::<DungeonProgress>(h.app.world_mut(), other_connection).is_empty());
        mask |= 1 << bit;
        assert_eq!(h.boss_state(boss_id), Some(EncounterState::Done));
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(COPY),
            mask
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(OTHER_COPY),
            0
        );
        assert!(
            h.inbox(0)
                .ends
                .iter()
                .any(|end| end.encounter_id == encounter && end.success)
        );
        let earned = h
            .app
            .world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .earned_ids
            .contains(&633);
        assert_eq!(earned, index == 2);
        let toasts: Vec<_> = updates
            .iter()
            .filter_map(|update| update.completed.as_ref())
            .filter(|toast| toast.achievement_id == 633)
            .collect();
        if index == 2 {
            assert_eq!(toasts.len(), 1);
            assert_eq!(
                (toasts[0].name.as_str(), toasts[0].points),
                ("Stormwind Stockade", 10)
            );
            assert!(updates.iter().any(|update| {
                update
                    .snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.earned_ids.contains(&633))
            }));
        } else {
            assert!(toasts.is_empty());
        }
        assert!(drain::<AchievementStateUpdate>(h.app.world_mut(), other_connection).is_empty());
        // Repeating DONE cannot credit the encounter or creature again.
        assert!(!super::super::instance_script::set_boss_state(
            h.app.world_mut(),
            COPY,
            boss_id,
            EncounterState::Done
        ));
        if index == 0 {
            (player, connection) = reconnect_dungeon2_player(&mut h, player, connection);
            h.advance(0.05);
            assert_eq!(
                drain::<DungeonProgress>(h.app.world_mut(), connection),
                dungeon_updates
            );
            assert_eq!(
                drain::<DungeonProgress>(h.app.world_mut(), peer_connection),
                dungeon_updates
            );
        }
    }
    assert_stockade_catalog_response(&mut h, connection);
    assert!(
        h.app
            .world()
            .get::<PlayerAchievementState>(peer)
            .unwrap()
            .earned_ids
            .contains(&633)
    );
    let state = h.app.world().get::<PlayerAchievementState>(player).unwrap();
    assert_eq!(state.criteria_progress[&18527], 1);
    let db = &h
        .app
        .world()
        .resource::<crate::persistence::GameDatabase>()
        .db;
    let mut character_state = state.clone();
    assert!(character_state.account.take().is_some());
    assert_eq!(
        crate::persistence::load_achievements(db, 1001),
        character_state
    );
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(other)
            .unwrap()
            .earned_ids
            .contains(&633)
    );
    // Existing snapshot carries the dungeon achievement's real criterion progress.
    let snapshot = h
        .app
        .world()
        .resource::<AchievementCatalog>()
        .snapshot(state, 1);
    let progress = snapshot
        .progress
        .iter()
        .find(|progress| progress.achievement_id == 633)
        .unwrap();
    assert_eq!(
        (progress.current, progress.required, progress.completed),
        (1, 1, true)
    );
}

#[test]
fn dungeon2_leaving_instance_clears_objectives_despite_world_boss_rows() {
    let data = data().expect("Stockade and continent DB2 fixtures");
    assert_eq!(data.maps.encounters_for_map(0, 0)[0].id, 2416);
    let mut app = stockade_app(data);
    let (player, connection) = spawn_player(&mut app, COPY, [158.0, 116.0, -35.1, 0.0], 1001);
    attach_dungeon2_messages(app.world_mut(), connection);
    let mut h = Harness {
        app,
        players: vec![(player, connection, Inbox::default())],
    };
    h.advance(0.05);
    assert_eq!(
        drain::<DungeonProgress>(h.app.world_mut(), connection)[0]
            .encounters
            .len(),
        3
    );
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(InMap::continent(0));
    h.advance(0.05);
    let updates = drain::<DungeonProgress>(h.app.world_mut(), connection);
    assert_eq!(updates.len(), 1);
    assert_eq!(
        (
            updates[0].map_id,
            updates[0].instance_id,
            updates[0].difficulty_id
        ),
        (0, 0, 0)
    );
    assert!(updates[0].encounters.is_empty());
}

fn attach_dungeon2_messages(world: &mut World, connection: Entity) {
    world.entity_mut(connection).insert((
        MessageSender::<AchievementStateUpdate>::default(),
        MessageReceiver::<AchievementStateUpdate>::default(),
        MessageSender::<AchievementDelta>::default(),
        MessageReceiver::<AchievementDelta>::default(),
        MessageSender::<DungeonProgress>::default(),
        MessageReceiver::<DungeonProgress>::default(),
        MessageSender::<QueryAchievementCatalog>::default(),
        MessageReceiver::<QueryAchievementCatalog>::default(),
        MessageSender::<AchievementCatalogPage>::default(),
        MessageReceiver::<AchievementCatalogPage>::default(),
    ));
}

fn assert_initial_dungeon2_progress(h: &mut Harness, connection: Entity, peer: Entity) {
    h.advance(0.05);
    let initial = drain::<DungeonProgress>(h.app.world_mut(), connection);
    assert_eq!(initial.len(), 1);
    assert_eq!(drain::<DungeonProgress>(h.app.world_mut(), peer), initial);
    let snapshot = &initial[0];
    assert_eq!(
        (
            snapshot.map_id,
            snapshot.instance_id,
            snapshot.difficulty_id
        ),
        (34, COPY.instance_id, 1)
    );
    assert_eq!(
        snapshot
            .encounters
            .iter()
            .map(|row| (row.encounter_id, row.name.as_str(), row.defeated))
            .collect::<Vec<_>>(),
        vec![
            (1144, "Hogger", false),
            (1145, "Lord Overheat", false),
            (1146, "Randolph Moloch", false)
        ]
    );
    assert!(
        snapshot
            .encounters
            .iter()
            .all(|row| row.optional.is_none() && row.flags == 4)
    );
    assert!(
        drain::<AchievementCatalogPage>(h.app.world_mut(), connection).is_empty(),
        "catalog is request-only"
    );
}

fn reconnect_dungeon2_player(
    h: &mut Harness,
    player: Entity,
    connection: Entity,
) -> (Entity, Entity) {
    let state = crate::persistence::load_achievements(
        &h.app
            .world()
            .resource::<crate::persistence::GameDatabase>()
            .db,
        1001,
    );
    assert_eq!(state.criteria_progress[&18527], 1);
    h.app.world_mut().despawn(connection);
    h.app.world_mut().despawn(player);
    let (player, connection) = spawn_player(&mut h.app, COPY, [158.0, 116.0, -35.1, 0.0], 1001);
    h.app
        .world_mut()
        .entity_mut(player)
        .insert((state, crate::persistence::ClientId(1001)));
    attach_dungeon2_messages(h.app.world_mut(), connection);
    h.players[0] = (player, connection, Inbox::default());
    (player, connection)
}

fn request_catalog_page(
    h: &mut Harness,
    connection: Entity,
    request: QueryAchievementCatalog,
) -> AchievementCatalogPage {
    h.app
        .world_mut()
        .get_mut::<MessageSender<QueryAchievementCatalog>>(connection)
        .unwrap()
        .send::<shared::protocol::AchievementChannel>(request);
    let mut pages = Vec::new();
    for _ in 0..3 {
        h.advance(0.05);
        pages.extend(drain::<AchievementCatalogPage>(
            h.app.world_mut(),
            connection,
        ));
    }
    assert_eq!(pages.len(), 1);
    pages.pop().unwrap()
}

fn assert_stockade_catalog_response(h: &mut Harness, connection: Entity) {
    let page = request_catalog_page(
        h,
        connection,
        QueryAchievementCatalog::Category {
            category_id: 14808,
            after_id: 632,
        },
    );
    let AchievementCatalogPage::Category { achievements, .. } = page else {
        panic!("category response");
    };
    let stockade = achievements
        .iter()
        .find(|row| row.achievement_id == 633)
        .unwrap();
    assert_eq!(
        (
            stockade.name.as_str(),
            stockade.description.as_str(),
            stockade.points,
            stockade.icon_fdid,
            stockade.earned
        ),
        ("Stormwind Stockade", "Defeat Hogger.", 10, 134163, true)
    );
    assert_eq!(stockade.earned_at, Some(1_791_262_800));
    assert_eq!(stockade.criteria.len(), 1);
    let criterion = &stockade.criteria[0];
    assert_eq!(
        (
            criterion.criteria_id,
            criterion.current,
            criterion.required,
            criterion.completed
        ),
        (18527, 1, 1, true)
    );
    let criteria = request_catalog_page(
        h,
        connection,
        QueryAchievementCatalog::Criteria {
            achievement_id: 633,
            after_id: 0,
        },
    );
    assert_eq!(
        criteria,
        AchievementCatalogPage::Criteria {
            achievement_id: 633,
            criteria: stockade.criteria.clone(),
            next_id: None
        }
    );
    let categories = request_catalog_page(
        h,
        connection,
        QueryAchievementCatalog::Categories { after_id: 0 },
    );
    let AchievementCatalogPage::Categories {
        categories,
        next_id,
    } = categories
    else {
        panic!("categories response");
    };
    assert_eq!(
        categories.len(),
        shared::protocol::ACHIEVEMENT_CATEGORY_PAGE_SIZE
    );
    assert!(next_id.is_some());
}
