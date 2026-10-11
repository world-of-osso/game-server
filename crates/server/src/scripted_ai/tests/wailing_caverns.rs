//! Lord Cobrahn's encounter on canonical spawn, SmartAI, spell and text data.
use super::*;

const COBRAHN: u32 = 3669;
const LIGHTNING_BOLT: u32 = 20805;
const WC: InMap = InMap {
    map_id: 43,
    instance_id: 1,
};
const WC_OTHER: InMap = InMap {
    map_id: 43,
    instance_id: 2,
};

fn cobrahn() -> (Harness, Entity) {
    let data = data().expect("world.db fixture");
    let mut app = stockade_app(data);
    let path = world_db_path();
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let expected = crate::world_data::load_expected_stats(&conn);
    let mut templates = crate::world_data::load_templates(&conn, &expected);
    templates.retain(|entry, _| *entry == COBRAHN);
    crate::world_data::load_template_models(&conn, &mut templates);
    let class_level_base_hp = conn
        .prepare("SELECT level, class, basehp0, basehp1, basehp2 FROM creature_classlevelstats")
        .unwrap()
        .query_map([], |row| {
            Ok((
                (row.get(0)?, row.get(1)?),
                [row.get(2)?, row.get(3)?, row.get(4)?],
            ))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    app.insert_resource(WorldData {
        templates,
        class_level_base_hp,
        class_level_base_mana: data.base_mana.clone(),
        ..Default::default()
    });
    let all = crate::spell_cast::test_harness::world_db().unwrap();
    app.insert_resource(SpellStore::fixture(
        [LIGHTNING_BOLT, 133]
            .iter()
            .map(|id| all.store.get(*id).unwrap().clone())
            .collect(),
    ));
    let mut scripts = smart_ai::load_smart_scripts(&path).unwrap();
    // Pin the successful branch of the authored 80% roll. Timing, spell, target
    // and all other row values remain canonical; production retains 80%.
    for event in std::sync::Arc::make_mut(scripts.0.get_mut(&COBRAHN).unwrap()) {
        if event.action_type == 11 {
            assert_eq!(event.chance, 80);
            event.chance = 100;
        }
    }
    app.insert_resource(scripts);
    let spawn = conn.query_row(
        "SELECT guid, position_x, position_y, position_z, orientation, spawntimesecs FROM content_creature WHERE id1 = 3669 AND map = 43 ORDER BY guid LIMIT 1",
        [], |row| Ok(CreatureSpawn { guid: row.get(0)?, template_entry: COBRAHN, map: 43, zone_id: 718,
            position_x: row.get(1)?, position_y: row.get(2)?, position_z: row.get(3)?, orientation: row.get(4)?,
            wander_distance: 0.0, movement_type: 0, spawntimesecs: row.get(5)?, display_id: None, unit_flags: None,
            spawn_difficulties: vec![1], equipment_id: 0 }),
    ).unwrap();
    let entrant = crate::maps::instances::InstanceEntrant {
        character_id: 2001,
        group: None,
        leader: 2001,
        difficulty_id: 1,
    };
    let mut instances = crate::maps::InstanceManager::default();
    assert_eq!(
        instances
            .create_map(
                &data.maps,
                43,
                entrant,
                crate::maps::instances::Now {
                    elapsed: 0.0,
                    unix: 0
                }
            )
            .unwrap()
            .map,
        WC
    );
    app.insert_resource(instances);
    let wd = app.world().resource::<WorldData>();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    let boss = crate::zones::spawn_creature(
        &mut commands,
        wd,
        &wd.templates[&COBRAHN],
        WC,
        false,
        &spawn,
        &mut rand::thread_rng(),
    );
    queue.apply(app.world_mut());
    let near = [
        spawn.position_x + 25.0,
        spawn.position_y,
        spawn.position_z,
        std::f32::consts::PI,
    ];
    let (player, connection) = spawn_player(&mut app, WC, near, 1);
    let (other, other_connection) = spawn_player(&mut app, WC_OTHER, near, 2);
    let mut h = Harness {
        app,
        players: vec![
            (player, connection, Inbox::default()),
            (other, other_connection, Inbox::default()),
        ],
    };
    h.advance(0.05);
    (h, boss)
}

#[test]
fn cobrahn_authored_cast_and_encounter_messages_are_copy_scoped() {
    let (mut h, boss) = cobrahn();
    h.engage(boss);
    h.advance(3.2);
    assert_eq!(
        get_boss_state(h.app.world_mut(), WC, 0),
        Some(EncounterState::InProgress)
    );
    assert_eq!(h.inbox(0).starts.len(), 1);
    let start = &h.inbox(0).starts[0];
    assert_eq!(
        (start.encounter_id, start.difficulty_id, start.group_size),
        (586, 1, 1)
    );
    assert_eq!(h.inbox(0).engages[0].unit, boss.to_bits());
    assert_eq!(h.inbox(0).engages[0].target_frame_priority, 1);
    assert!(h.inbox(1).starts.is_empty());
    assert!(h.inbox(1).engages.is_empty());
    assert_eq!(h.casts(0, boss, LIGHTNING_BOLT), 1);
    assert!(
        h.inbox(0)
            .chat
            .iter()
            .any(|chat| chat.content.contains("dreamer"))
    );
    h.hit_pct(boss, 110.0);
    h.advance(0.1);
    assert_eq!(
        get_boss_state(h.app.world_mut(), WC, 0),
        Some(EncounterState::Done)
    );
    assert_eq!(h.inbox(0).ends.len(), 1);
    assert_eq!(h.inbox(0).ends[0].encounter_id, 586);
    assert!(h.inbox(0).ends[0].success);
    assert_eq!(h.inbox(0).disengages[0].unit, boss.to_bits());
    assert!(h.inbox(1).ends.is_empty());
    assert!(h.inbox(1).disengages.is_empty());
}

#[test]
fn cobrahn_evade_fails_encounter_and_repull_restarts_authored_timer() {
    let (mut h, boss) = cobrahn();
    h.engage(boss);
    h.advance(3.2);
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    h.advance(0.1);
    assert_eq!(
        get_boss_state(h.app.world_mut(), WC, 0),
        Some(EncounterState::NotStarted)
    );
    assert_eq!(h.inbox(0).ends.len(), 1);
    assert!(!h.inbox(0).ends[0].success);
    assert_eq!(h.inbox(0).disengages.len(), 1);
    h.engage(boss);
    h.advance(3.2);
    assert_eq!(h.inbox(0).starts.len(), 2);
    assert_eq!(h.casts(0, boss, LIGHTNING_BOLT), 2);
}
