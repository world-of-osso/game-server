use super::*;
use crate::achievement::{AchievementCatalog, PlayerAchievementState, load_achievement_catalog};
use shared::protocol::{
    AchievementCatalogPage, AchievementStateUpdate, DungeonProgress, QueryAchievementCatalog,
};

#[test]
fn dungeon2_stockade_entry_kills_reconnect_and_earned_category() {
    let data = data().expect("Stockade DB2 and script fixtures");
    let mut app = stockade_app(data);
    let conn = Connection::open(world_db_path()).unwrap();
    app.insert_resource(load_achievement_catalog(&conn).unwrap());
    app.add_plugins(crate::achievement::AchievementPlugin);
    app.insert_resource(crate::maps::ServerClock {
        offset_secs: 0,
        fixed_unix: Some(1_791_262_800),
    });
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    crate::persistence::create_tables(&db);
    app.insert_resource(crate::persistence::GameDatabase { db });
    let (player, connection) = spawn_player(&mut app, COPY, [158.0, 116.0, -35.1, 0.0], 1001);
    let (other, other_connection) =
        spawn_player(&mut app, OTHER_COPY, [158.0, 116.0, -35.1, 0.0], 1002);
    for (player, connection, id) in [(player, connection, 1001), (other, other_connection, 1002)] {
        app.world_mut().entity_mut(player).insert((
            PlayerAchievementState::default(),
            crate::persistence::ClientId(id),
        ));
        app.world_mut().entity_mut(connection).insert((
            MessageSender::<AchievementStateUpdate>::default(),
            MessageReceiver::<AchievementStateUpdate>::default(),
            MessageSender::<DungeonProgress>::default(),
            MessageReceiver::<DungeonProgress>::default(),
            MessageSender::<QueryAchievementCatalog>::default(),
            MessageReceiver::<QueryAchievementCatalog>::default(),
            MessageSender::<AchievementCatalogPage>::default(),
            MessageReceiver::<AchievementCatalogPage>::default(),
        ));
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
        ],
    };
    h.advance(0.05);
    let initial = drain::<DungeonProgress>(h.app.world_mut(), connection);
    assert_eq!(initial.len(), 1);
    assert_eq!(
        (
            initial[0].map_id,
            initial[0].instance_id,
            initial[0].difficulty_id
        ),
        (34, COPY.instance_id, 1)
    );
    assert_eq!(
        initial[0]
            .encounters
            .iter()
            .map(|e| (e.encounter_id, e.name.as_str(), e.defeated))
            .collect::<Vec<_>>(),
        vec![
            (1144, "Hogger", false),
            (1145, "Lord Overheat", false),
            (1146, "Randolph Moloch", false)
        ]
    );
    assert!(
        initial[0]
            .encounters
            .iter()
            .all(|row| row.optional.is_none() && row.flags == 4)
    );
    assert!(
        drain::<AchievementCatalogPage>(h.app.world_mut(), connection).is_empty(),
        "catalog is request-only"
    );
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
        h.advance(0.05);
        updates.extend(drain::<AchievementStateUpdate>(
            h.app.world_mut(),
            connection,
        ));
        dungeon_updates.extend(drain::<DungeonProgress>(h.app.world_mut(), connection));
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
        if index == 0 {
            // Reconnect the same character to the running copy, preserving its state.
            h.app.world_mut().entity_mut(connection).remove::<InWorld>();
            h.advance(0.05);
            h.app
                .world_mut()
                .entity_mut(connection)
                .insert(InWorld { character_id: 1001 });
            h.advance(0.05);
            let resumed = drain::<DungeonProgress>(h.app.world_mut(), connection);
            assert_eq!(resumed, dungeon_updates);
        }
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
    }
    h.app
        .world_mut()
        .get_mut::<MessageSender<QueryAchievementCatalog>>(connection)
        .unwrap()
        .send::<shared::protocol::AchievementChannel>(QueryAchievementCatalog::Category {
            category_id: 14808,
            after_id: 632,
        });
    let mut pages = Vec::new();
    for _ in 0..3 {
        h.advance(0.05);
        pages.extend(drain::<AchievementCatalogPage>(
            h.app.world_mut(),
            connection,
        ));
    }
    assert_eq!(pages.len(), 1);
    let AchievementCatalogPage::Category { achievements, .. } = &pages[0] else {
        panic!("category response");
    };
    let stockade = achievements
        .iter()
        .find(|a| a.achievement_id == 633)
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
    assert_eq!(
        (
            stockade.criteria[0].criteria_id,
            stockade.criteria[0].current,
            stockade.criteria[0].required,
            stockade.criteria[0].completed
        ),
        (18527, 1, 1, true)
    );
    let state = h.app.world().get::<PlayerAchievementState>(player).unwrap();
    assert_eq!(state.criteria_progress[&18527], 1);
    let db = &h
        .app
        .world()
        .resource::<crate::persistence::GameDatabase>()
        .db;
    assert_eq!(crate::persistence::load_achievements(db, 1001), *state);
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
        .snapshot(state);
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
