//! Authentic JournalEncounterCreature joins exercised through real creature spawning.
use super::*;

fn bound_boss(entry: u32, map_id: u32, bit: u32) -> (Harness, Entity, InMap) {
    bound_boss_with_catalog(entry, map_id, bit, None)
}

fn bound_boss_with_catalog(
    entry: u32,
    map_id: u32,
    bit: u32,
    catalog: Option<bossmod::Catalog>,
) -> (Harness, Entity, InMap) {
    let data = data().expect("world.db fixture");
    let mut app = stockade_app(data);
    if let Some(catalog) = catalog {
        app.insert_resource(catalog);
    }
    let path = world_db_path();
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let expected = crate::world_data::load_expected_stats(&conn);
    let mut templates = crate::world_data::load_templates(&conn, &expected);
    templates.retain(|id, _| *id == entry);
    crate::world_data::load_template_models(&conn, &mut templates);
    let hp = conn
        .prepare("SELECT level,class,basehp0,basehp1,basehp2 FROM creature_classlevelstats")
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
        class_level_base_hp: hp,
        class_level_base_mana: data.base_mana.clone(),
        ..Default::default()
    });
    let mut scripts = smart_ai::load_smart_scripts(&path).unwrap();
    if entry == 3674 {
        // Pin the successful branch of Skum's authored80% chance; keep the
        // real3–4s selected timer, spell and target. Production is unchanged.
        for row in std::sync::Arc::make_mut(scripts.0.get_mut(&entry).unwrap()) {
            if row.action_type == 11 {
                assert_eq!(row.chance, 80);
                row.chance = 100;
            }
        }
    }
    // Use production's complete DB2 store, including absence of legacy SAI IDs.
    // A script's unknown spell must fail normal admission, not panic the fixture.
    let all = crate::spell_cast::test_harness::world_db().unwrap();
    app.insert_resource(all.store.clone());
    app.insert_resource(scripts);
    let mut maps = data.maps.clone();
    maps.raid_creature_encounters.insert((map_id, entry), bit);
    let difficulty = if map_id == 409 { 3 } else { 1 };
    let mut instances = crate::maps::InstanceManager::default();
    let entrant = crate::maps::instances::InstanceEntrant {
        character_id: 2001,
        leader: 2001,
        difficulty_id: difficulty,
        ..Default::default()
    };
    let map = instances
        .create_map(
            &maps,
            map_id,
            entrant,
            crate::maps::instances::Now {
                elapsed: 0.0,
                unix: 0,
            },
        )
        .unwrap()
        .map;
    app.insert_resource(instances);
    app.insert_resource(maps);
    let spawn = conn.query_row(
        "SELECT guid,position_x,position_y,position_z,orientation,spawntimesecs FROM content_creature WHERE id1=? AND map=? ORDER BY guid LIMIT 1",
        [entry, map_id], |row| Ok(CreatureSpawn { guid: row.get(0)?, template_entry: entry,
            map: map_id as u16, zone_id: 0, position_x: row.get(1)?, position_y: row.get(2)?,
            position_z: row.get(3)?, orientation: row.get(4)?, spawntimesecs: row.get(5)?,
            wander_distance: 0.0, movement_type: 0, display_id: None, unit_flags: None,
            spawn_difficulties: vec![difficulty], equipment_id: 0 })).unwrap();
    let wd = app.world().resource::<WorldData>();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    let boss = crate::zones::spawn_creature(
        &mut commands,
        wd,
        &wd.templates[&entry],
        map,
        false,
        &spawn,
        &mut rand::thread_rng(),
    );
    queue.apply(app.world_mut());
    let near = [
        spawn.position_x + 3.0,
        spawn.position_y,
        spawn.position_z,
        std::f32::consts::PI,
    ];
    let (player, connection) = spawn_player(&mut app, map, near, 1);
    app.world_mut()
        .entity_mut(player)
        .insert(crate::persistence::ClientId(2001));
    let other_map = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    let (other, other_connection) = spawn_player(&mut app, other_map, near, 2);
    let mut h = Harness {
        app,
        players: vec![
            (player, connection, Inbox::default()),
            (other, other_connection, Inbox::default()),
        ],
    };
    h.advance(0.05);
    (h, boss, map)
}

#[test]
fn boss_timeline_data_bound_skum_has_no_smartai_alert_fallback() {
    let (mut h, boss, _) = bound_boss(3674, 43, 4);
    h.engage(boss);
    h.advance(5.0);
    assert!(h.inbox(0).timeline.is_empty());
    assert!(h.inbox(1).timeline.is_empty());
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.spell_id == Some(6254)),
        "Skum combat casts must remain independent of alert authority"
    );
}

#[test]
fn bossmod_timeline_diremaul_imported_bars_reset_on_real_casts_with_copy_scoped_warnings() {
    use shared::protocol::{EncounterSeverity, EncounterTimelineEvent};
    for (entry, encounter, bit, spell, initial, repeat, severity) in [
        (13280, 344, 1, 22419, 6900, 6100, EncounterSeverity::High),
        (11490, 343, 0, 22478, 3700, 8500, EncounterSeverity::Medium),
    ] {
        let catalog_db = Connection::open_in_memory().unwrap();
        catalog_db
            .execute_batch(include_str!("bossmod_diremaul.sql"))
            .unwrap();
        let catalog = bossmod::load_catalog(&catalog_db).unwrap();
        let (mut h, boss, _) = bound_boss_with_catalog(entry, 429, bit, Some(catalog));
        h.engage(boss);
        h.advance(0.1);
        assert_eq!(h.inbox(0).starts[0].encounter_id, encounter);
        let initial_timer = h
            .inbox(0)
            .timeline
            .iter()
            .find_map(|event| match event {
                EncounterTimelineEvent::Timers { timers, .. } => {
                    timers.iter().find(|timer| timer.spell_id == spell)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(initial_timer.duration_ms, initial);
        assert!(
            initial_timer.remaining_ms <= initial && initial_timer.remaining_ms >= initial - 100
        );
        assert!(
            !h.inbox(0)
                .timeline
                .iter()
                .any(|event| matches!(event, EncounterTimelineEvent::Cast { .. }))
        );
        let mut admitted_casts = Vec::new();
        for _ in 0..320 {
            h.advance(0.05);
            if let Some(cast) = h.app.world().get::<CastState>(boss) {
                if admitted_casts.last() != Some(&cast.spell_id) {
                    admitted_casts.push(cast.spell_id);
                }
            }
        }
        if entry == 11490 {
            assert!(
                admitted_casts.contains(&spell),
                "Zevrim's real SmartAI cast must enter CastState before its start warning"
            );
        }
        assert!(
            h.inbox(0).log.iter().any(|event| matches!(
                event.kind,
                CombatLogKind::Damage | CombatLogKind::AuraApplied
            ) && event.spell_id == Some(spell)),
            "entry {entry} spell {spell}: actual spell effect must resolve, not an injected warning"
        );
        assert!(h.inbox(0).timeline.iter().any(|event| matches!(event,
            EncounterTimelineEvent::Cast { caster, spell_id, severity:alert_severity }
            if *caster == boss.to_bits() && *spell_id == spell && *alert_severity == severity)));
        assert!(
            h.inbox(0).timeline.iter().any(|event| match event {
                EncounterTimelineEvent::Timers { timers, .. } => timers
                    .iter()
                    .any(|timer| timer.spell_id == spell && timer.duration_ms == repeat),
                _ => false,
            }),
            "cast must reset to addon delay, not the SmartAI repeat range"
        );
        assert!(
            h.inbox(1).timeline.is_empty(),
            "other copy cannot receive alerts"
        );
        crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
        h.advance(0.1);
        assert!(!h.inbox(0).ends.last().unwrap().success);
        let warnings_before = h
            .inbox(0)
            .timeline
            .iter()
            .filter(|event| matches!(event, EncounterTimelineEvent::Cast { .. }))
            .count();
        h.advance(5.0);
        assert_eq!(
            h.inbox(0)
                .timeline
                .iter()
                .filter(|event| matches!(event, EncounterTimelineEvent::Cast { .. }))
                .count(),
            warnings_before,
            "evade must retire the alert schedule"
        );
    }
}

#[test]
fn data_bound_dungeon_smartai_and_scriptless_raid_get_copy_scoped_lifecycle() {
    // Authentic local journal/display/map joins; neither has Rust registration.
    for (entry, map_id, bit, encounter) in [(6910, 70, 0, 547), (12118, 409, 0, 663)] {
        let (mut h, boss, map) = bound_boss(entry, map_id, bit);
        let database_file = tempfile::NamedTempFile::new().unwrap();
        h.app.insert_resource(crate::persistence::GameDatabase {
            db: redb::Database::create(database_file.path()).unwrap(),
        });
        h.engage(boss);
        h.advance(0.05);
        assert_eq!(
            h.inbox(0).starts.len(),
            1,
            "entry {entry} encounter missing"
        );
        assert_eq!(h.inbox(0).starts[0].encounter_id, encounter);
        assert_eq!(h.inbox(0).engages.len(), 1);
        assert!(h.inbox(1).starts.is_empty());
        if entry == 6910 {
            h.advance(3.2);
            assert!(
                h.casts(0, boss, 15801) > 0,
                "authored Revelosh Lightning Bolt did not cast"
            );
        }
        crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
        h.advance(0.05);
        assert_eq!(h.inbox(0).ends.len(), 1);
        assert!(!h.inbox(0).ends[0].success);
        for _ in 0..100 {
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
        h.advance(0.05);
        assert_eq!(h.inbox(0).starts.len(), 2);
        h.hit_pct(boss, 200.0);
        h.advance(0.05);
        assert_eq!(h.inbox(0).ends.len(), 2);
        assert!(h.inbox(0).ends[1].success);
        assert_eq!(h.inbox(0).disengages.len(), 2);
        assert!(h.inbox(1).ends.is_empty());
        let instances = h.app.world().resource::<crate::maps::InstanceManager>();
        assert_eq!(instances.completed_encounters(map), 1 << bit);
        let done = instances.initial_done_bosses(map);
        assert_eq!(
            instance_script::done_boss_entries(h.app.world().resource::<MapStore>(), map_id, done),
            vec![entry]
        );
        if map_id == 409 {
            use redb::ReadableDatabase;
            let txn = h
                .app
                .world()
                .resource::<crate::persistence::GameDatabase>()
                .db
                .begin_read()
                .unwrap();
            let table = txn
                .open_table(redb::TableDefinition::<&str, &[u8]>::new(
                    "raid_first_kills_v1",
                ))
                .unwrap();
            let difficulty = instances.copy_difficulty(map).unwrap().difficulty_id;
            let key = format!("character:2001:409:{difficulty}:663");
            let row = table.get(key.as_str()).unwrap().unwrap();
            let credit: serde_json::Value = serde_json::from_slice(row.value()).unwrap();
            assert_eq!(credit["participants"][0]["character_id"], "2001");
            assert_eq!(credit["participants"].as_array().unwrap().len(), 1);
        }
    }
}

#[test]
fn data_selected_grim_batol_preserves_owned_summon_cleanup_on_evade() {
    let (mut h, boss, _) = bound_boss(39625, 670, 0);
    // Exercise the existing JustSummoned boundary; no new summon ability is authored.
    let summon = h
        .app
        .world_mut()
        .spawn(crate::zones::TempSummon { summoner: boss })
        .id();
    dispatch(h.app.world_mut(), boss, Hook::JustSummoned(summon));
    h.engage(boss);
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    assert!(
        h.app.world().get_entity(summon).is_err(),
        "owned summon survived boss reset"
    );
}
