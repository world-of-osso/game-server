use super::*;
use crate::achievement::{AchievementCatalog, PlayerAchievementState, load_achievement_catalog};
use shared::protocol::AchievementStateUpdate;

#[test]
fn dungeonach_stockade_real_boss_deaths_record_ordered_progress_and_send_final_toast() {
    let data = data().expect("Stockade DB2 and script fixtures");
    let mut app = stockade_app(data);
    let conn = Connection::open(world_db_path()).unwrap();
    app.insert_resource(load_achievement_catalog(&conn).unwrap());
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
        h.advance(0.10);
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
        let updates = drain::<AchievementStateUpdate>(h.app.world_mut(), connection);
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
