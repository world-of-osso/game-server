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
    cobrahn_with_events(Vec::new())
}

fn cobrahn_with_events(extra_events: Vec<smart_ai::SmartEvent>) -> (Harness, Entity) {
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
    std::sync::Arc::make_mut(scripts.0.get_mut(&COBRAHN).unwrap()).extend(extra_events);
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
        raid_difficulty: Default::default(),
        raid_group: false,
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
        &mut crate::zones::CreatureSpawnContext {
            map: WC,
            difficulty_id: crate::zones::map_difficulty_id(app.world(), WC),
            ground: Some(&mut crate::test_data::ground_map()),
            temporary: false,
        },
        false,
        &spawn,
        &mut rand::thread_rng(),
    )
    .expect("source Cobrahn level context");
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

fn timeline(h: &mut Harness, player: usize) -> Vec<shared::protocol::EncounterTimelineEvent> {
    std::mem::take(&mut h.players[player].2.timeline)
}

#[test]
fn boss_timeline_smartai_phases_are_not_addon_alert_authority() {
    use shared::protocol::EncounterTimelineEvent;
    // Generic phase lifecycle fixture, NOT invented Cobrahn mechanics. Cast
    // rows/timers stay authentic; only reset/aggro phase actions are added.
    let phase_event = |id, event_type, phase| smart_ai::SmartEvent {
        id,
        link: 0,
        event_type,
        phase_mask: 0,
        chance: 100,
        flags: 0,
        event_params: [0; 5],
        action_type: 22,
        action_params: [phase, 0, 0, 0, 0, 0, 0],
        target_type: 0,
        target_params: [0; 4],
        target_position: [0.0; 4],
    };
    let (mut h, boss) =
        cobrahn_with_events(vec![phase_event(1001, 25, 1), phase_event(1002, 4, 2)]);
    assert!(
        !timeline(&mut h, 0)
            .iter()
            .any(|event| matches!(event, EncounterTimelineEvent::Phase { .. })),
        "spawn/reset outside an encounter must not warn"
    );
    h.engage(boss);
    h.advance(0.1);
    assert!(
        !timeline(&mut h, 0)
            .iter()
            .any(|event| matches!(event, EncounterTimelineEvent::Phase { .. })),
        "AI phase changes cannot substitute for addon-authored alert phases"
    );
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    h.advance(0.1);
    assert!(!h.inbox(0).ends.last().unwrap().success);
    assert!(
        !timeline(&mut h, 0)
            .iter()
            .any(|event| matches!(event, EncounterTimelineEvent::Phase { .. })),
        "reset must not recreate a warning after EncounterEnd"
    );
}

#[test]
fn cobrahn_timeline_does_not_fallback_to_smartai_without_addon_data() {
    let (mut h, boss) = cobrahn();
    h.engage(boss);
    h.advance(9.0);
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.spell_id == Some(LIGHTNING_BOLT)),
        "combat must still execute authored SmartAI casts"
    );
    assert!(
        timeline(&mut h, 0).is_empty(),
        "AI deadlines are not addon alert schedules"
    );
    assert!(timeline(&mut h, 1).is_empty());
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    h.advance(0.1);
    assert!(!h.inbox(0).ends.last().unwrap().success);
    assert!(timeline(&mut h, 0).is_empty());
}

fn assert_authored_cobrahn_loot(path: &std::path::Path) {
    use ::rand::SeedableRng;
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let tables = crate::loot::load_loot_tables(&conn).unwrap();
    let items = crate::item_templates::load_item_templates(path).unwrap();
    let mut rng = rand::rngs::StdRng::seed_from_u64(3669);
    let loot = tables.roll_creature(COBRAHN, 1, &items, |_| false, &mut rng);
    // TDB Normal LootID3669: GoldMin87 / GoldMax168; group1 totals100%.
    assert!(
        (87..=168).contains(&loot.copper),
        "authored money absent: {loot:?}"
    );
    let boss_items: Vec<_> = loot
        .items
        .iter()
        .filter(|item| [6460, 6465, 10410].contains(&item.item_id))
        .collect();
    assert_eq!(boss_items.len(), 1, "authored group1 boss item: {loot:?}");
    assert_eq!(boss_items[0].count, 1);
    assert!(
        !loot.items.iter().any(|item| item.item_id == 9738),
        "quest-only Gem of Cobrahn is not unrestricted loot"
    );
}

#[test]
fn cobrahn_canonical_normal_loot_has_authored_item_and_money() {
    assert_authored_cobrahn_loot(&world_db_path());
}

#[test]
#[ignore = "requires explicit read-only population copy; acceptance gate for its loot bindings"]
fn cobrahn_population_copy_normal_loot_has_authored_item_and_money() {
    let path = std::env::var_os("DUNGEON_ENCOUNTER_POPULATION_DB")
        .expect("set DUNGEON_ENCOUNTER_POPULATION_DB to the prepared copy");
    assert_authored_cobrahn_loot(std::path::Path::new(&path));
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
    // A real repull happens after evade has reached home, not mid-return.
    for _ in 0..40 {
        if h.app.world().get::<crate::creature_ai::CreatureState>(boss)
            == Some(&crate::creature_ai::CreatureState::Idle)
        {
            break;
        }
        h.advance(0.05);
    }
    assert_eq!(
        h.app.world().get::<crate::creature_ai::CreatureState>(boss),
        Some(&crate::creature_ai::CreatureState::Idle)
    );
    h.engage(boss);
    h.advance(3.2);
    assert_eq!(h.inbox(0).starts.len(), 2);
    assert_eq!(h.casts(0, boss, LIGHTNING_BOLT), 2);
}
