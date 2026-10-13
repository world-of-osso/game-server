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
    let difficulty = if map_id == 409 { 3 } else { 1 };
    bound_boss_at_difficulty(entry, map_id, bit, difficulty, catalog)
}

/// Use the production importer on a consistent isolated snapshot, never
/// canonical data. The TempDir stays alive until all fixture resources are loaded.
fn prepare_encounter_database(encounter: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let directory = tempfile::tempdir().expect("isolated encounter fixture directory");
    let path = directory.path().join("encounter-mirror.sqlite");
    let source = Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("read-only encounter source database");
    source
        .execute("VACUUM INTO ?1", [path.to_str().unwrap()])
        .expect("snapshot canonical encounter data without writing source");
    let script =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/encounter_summons.py");
    let result = std::process::Command::new("python3")
        .arg(script)
        .arg("--encounter")
        .arg(encounter)
        .arg("--db")
        .arg(&path)
        .output()
        .expect("run authenticated mirror template importer");
    assert!(
        result.status.success(),
        "mirror source preparation failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    (directory, path)
}

fn bound_boss_at_difficulty(
    entry: u32,
    map_id: u32,
    bit: u32,
    difficulty: u32,
    catalog: Option<bossmod::Catalog>,
) -> (Harness, Entity, InMap) {
    bound_boss_in_state(
        entry,
        map_id,
        bit,
        difficulty,
        catalog,
        EncounterState::NotStarted,
    )
}

fn bound_boss_in_state(
    entry: u32,
    map_id: u32,
    bit: u32,
    difficulty: u32,
    catalog: Option<bossmod::Catalog>,
    initial_state: EncounterState,
) -> (Harness, Entity, InMap) {
    let data = data().expect("world.db fixture");
    let mut app = stockade_app(data);
    if let Some(catalog) = catalog {
        app.insert_resource(catalog);
    }
    let prepared = match entry {
        234648 => Some(prepare_encounter_database("kystia")),
        135322 => Some(prepare_encounter_database("golden-serpent")),
        _ => None,
    };
    let path = prepared
        .as_ref()
        .map_or_else(world_db_path, |(_, path)| path.clone());
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    app.insert_resource(crate::conditions::load_spell_target_conditions(&path).unwrap());
    app.insert_resource(super::super::kings_rest::load_molten_gold_areas(&conn).unwrap());
    app.insert_resource(super::super::kings_rest::load_gold_model_reaches(&conn).unwrap());
    app.insert_resource(super::super::waypoint_path::load_script_paths(&conn).unwrap());
    app.insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    let expected = crate::world_data::load_expected_stats(&conn);
    let mut templates = crate::world_data::load_templates(&conn, &expected);
    templates.retain(|id, _| {
        *id == entry
            || (entry == 234648 && [234660, 255050].contains(id))
            || (entry == 135322 && *id == 135406)
    });
    crate::world_data::load_template_models(&conn, &mut templates);
    let difficulty_templates =
        crate::world_data::load_difficulty_templates(&conn, &expected, &templates);
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
        difficulty_templates,
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
    if initial_state != EncounterState::NotStarted {
        assert!(instance_script::set_boss_state(
            app.world_mut(),
            map,
            bit,
            initial_state
        ));
    }
    let (boss, spawn) = spawn_authored_actor(&mut app, &conn, entry, map, difficulty);
    let near = if entry == 135322 && initial_state != EncounterState::NotStarted {
        // Keep the relocated source branch idle until its fixture explicitly pulls.
        let position = app.world().get::<Position>(boss).unwrap();
        [
            position.x + 500.0,
            -position.z,
            position.y,
            std::f32::consts::PI,
        ]
    } else {
        [
            spawn.position_x + 3.0,
            spawn.position_y,
            spawn.position_z,
            std::f32::consts::PI,
        ]
    };
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

fn spawn_authored_actor(
    app: &mut App,
    conn: &Connection,
    entry: u32,
    map: InMap,
    difficulty: u32,
) -> (Entity, CreatureSpawn) {
    let spawn = conn.query_row(
        "SELECT guid,position_x,position_y,position_z,orientation,spawntimesecs FROM content_creature WHERE id1=? AND map=? ORDER BY guid LIMIT 1",
        [entry, map.map_id], |row| Ok(CreatureSpawn { guid: row.get(0)?, template_entry: entry,
            map: map.map_id as u16, zone_id: 0, position_x: row.get(1)?, position_y: row.get(2)?,
            position_z: row.get(3)?, orientation: row.get(4)?, spawntimesecs: row.get(5)?,
            wander_distance: 0.0, movement_type: 0, display_id: None, unit_flags: None,
            spawn_difficulties: vec![difficulty], equipment_id: 0 })).unwrap();
    let wd = app.world().resource::<WorldData>();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    let actor = crate::zones::spawn_creature(
        &mut commands,
        wd,
        wd.template_for(entry, difficulty)
            .expect("authored fixture difficulty"),
        map,
        false,
        &spawn,
        &mut rand::thread_rng(),
    );
    commands
        .entity(actor)
        .insert(crate::zones::CreatureDifficulty(difficulty));
    queue.apply(app.world_mut());
    (actor, spawn)
}

#[test]
fn encounter_coverage_kystia_mirror_tick_spawns_authored_copy_in_the_same_instance() {
    let (mut h, kystia, map) = bound_boss(234648, 2813, 0);
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Health {
        current: 1_000_000_000.0,
        max: 1_000_000_000.0,
    });
    h.engage(kystia);
    h.advance(14.4);
    assert!(!h.has_aura(kystia, 1264095));
    assert!(h.creatures_of(255050).is_empty());
    h.advance(0.3);
    assert!(h.has_aura(kystia, 1264095));
    // Local DB2 Attributes5/0x200 requests an extra tick on apply; subsequent
    // effect0 ticks are1500ms. Do not invent a2-second initial wait.
    assert_eq!(h.creatures_of(255050).len(), 1);
    h.advance(1.6);
    let mirrors = h.creatures_of(255050);
    assert_eq!(
        mirrors.len(),
        2,
        "initial plus authored1500ms periodic tick"
    );
    let center = *h.app.world().get::<Position>(kystia).unwrap();
    for mirror in mirrors {
        assert_eq!(*h.app.world().get::<InMap>(mirror).unwrap(), map);
        assert_eq!(
            h.app
                .world()
                .get::<crate::zones::TempSummon>(mirror)
                .unwrap()
                .summoner,
            kystia
        );
        let p = h.app.world().get::<Position>(mirror).unwrap();
        assert!((-30.0..=25.0).contains(&(p.x - center.x)));
        assert!((-26.0..=15.0).contains(&(p.z - center.z)));
        assert!(
            h.has_aura(mirror, 1264106),
            "authored SmartAI Respawn11 must cast Felstorm"
        );
    }
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), kystia);
    h.advance(0.1);
    assert!(
        h.creatures_of(255050).is_empty(),
        "evade must retire owned mirrors"
    );
}

#[test]
fn encounter_coverage_mirror_respawn_cast_is_not_an_ordinary_reset_cast() {
    let (mut h, kystia, map) = bound_boss(234648, 2813, 0);
    let mirror = crate::zones::summon_creature(
        h.app.world_mut(),
        kystia,
        255050,
        [8860.0, -4840.0, 13.0, 0.0],
    )
    .unwrap();
    h.advance(0.1);
    assert_eq!(*h.app.world().get::<InMap>(mirror).unwrap(), map);
    assert!(
        h.has_aura(mirror, 1264106),
        "appearance must run sourced Respawn11 cast"
    );
    let original_expiry = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(mirror)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1264106)
        .unwrap()
        .expires_at;
    h.advance(1.1);
    super::super::dispatch(h.app.world_mut(), mirror, Hook::Reset);
    h.advance(0.1);
    let expiry_after_reset = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(mirror)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1264106)
        .unwrap()
        .expires_at;
    assert_eq!(
        original_expiry, expiry_after_reset,
        "ordinary reset must not refresh Respawn11 Felstorm"
    );
}

#[test]
fn encounter_coverage_mirror_felstorm_removal_retires_actor_without_boss_credit() {
    let (mut h, kystia, map) = bound_boss(234648, 2813, 0);
    h.engage(kystia);
    let mirror = crate::zones::summon_creature(
        h.app.world_mut(),
        kystia,
        255050,
        [8860.0, -4840.0, 13.0, 0.0],
    )
    .unwrap();
    h.advance(0.1);
    assert!(h.has_aura(mirror, 1264106));
    crate::auras::remove_auras(
        h.app.world_mut(),
        mirror,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 1264106,
    );
    h.advance(0.1);
    assert!(
        h.app.world().get_entity(mirror).is_err(),
        "TC Felstorm OnRemove must DespawnOrUnsummon after1ms"
    );
    assert_eq!(
        super::super::instance_script::get_boss_state(h.app.world_mut(), map, 0),
        Some(EncounterState::InProgress)
    );
    assert!(h.app.world().get_entity(kystia).is_ok());
}

#[test]
fn encounter_coverage_golden_serpent_not_started_intro_is_friendly_and_uninteractible() {
    let (mut h, boss, map) = bound_boss(135322, 1762, 0);
    h.advance(1.0);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitFactionTemplate>(boss)
            .unwrap()
            .0,
        35,
        "TC NotStarted appearance is friendly until the landing path ends"
    );
    let flags = h
        .app
        .world()
        .get::<shared::components::UnitFlags>(boss)
        .unwrap()
        .0;
    assert_ne!(flags & shared::components::UnitFlags::NOT_SELECTABLE, 0);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, 0),
        Some(EncounterState::NotStarted)
    );
    assert!(h.inbox(0).starts.is_empty());
    assert!(h.inbox(0).engages.is_empty());
}

#[test]
fn encounter_coverage_golden_serpent_intro_flies_authored_paths_then_lands_hostile() {
    let (mut h, boss, map) = bound_boss(135322, 1762, 0);
    let player = h.player();
    let far = Position {
        x: h.app.world().get::<Position>(boss).unwrap().x + 500.0,
        ..*h.app.world().get::<Position>(boss).unwrap()
    };
    h.app.world_mut().entity_mut(player).insert(far);
    let spawn = h.wow_position(boss);
    // Each MovePath starts with WaypointMovementGenerator::DoInitialize's1000ms wait.
    h.advance(0.9);
    assert!(h.wow_position(boss).distance(spawn) < 0.01);
    assert!(h.has_aura(boss, 271598));
    let submerge_low = Vec3::new(-1065.97, 2641.51, 776.115);
    let submerge_apex = Vec3::new(-1065.97, 2641.51, 816.115);
    let flying = [
        Vec3::new(-1064.73, 2633.53, 819.27),
        Vec3::new(-1057.29, 2629.48, 820.627),
        Vec3::new(-1052.35, 2632.23, 821.548),
        Vec3::new(-1050.81, 2639.52, 821.841),
        Vec3::new(-1051.3, 2645.3, 822.053),
        Vec3::new(-1053.86, 2657.0, 821.205),
        Vec3::new(-1072.12, 2661.95, 820.305),
        Vec3::new(-1084.02, 2651.1, 820.091),
        Vec3::new(-1089.33, 2635.23, 819.845),
        Vec3::new(-1082.08, 2621.95, 819.27),
        Vec3::new(-1058.84, 2615.17, 815.878),
    ];
    let landing = Vec3::new(-1058.84, 2615.17, 810.152);
    let mut closest = [f32::MAX; 11];
    let mut lowest = f32::MAX;
    let mut emerge_removed = None;
    let mut landed = None;
    let mut elapsed = 0.9;
    for _ in 0..800 {
        h.advance(0.05);
        elapsed += 0.05;
        let position = h.wow_position(boss);
        lowest = lowest.min(position.z);
        for (node, best) in flying.iter().zip(&mut closest) {
            *best = best.min(position.distance(*node));
        }
        if emerge_removed.is_none() && !h.has_aura(boss, 271598) {
            assert!(
                position.distance(submerge_apex) < 0.01,
                "submerge path end removes emerge state: {position}"
            );
            emerge_removed = Some((elapsed, position));
        }
        let faction = h.app.world().get::<UnitFactionTemplate>(boss).unwrap().0;
        if faction != 35 {
            assert_eq!(faction, 16, "landing path end sets FACTION_MONSTER_2");
            landed = Some(elapsed);
            break;
        }
    }
    assert!(
        (lowest - submerge_low.z).abs() < 0.01,
        "path13532200 first descends to node0: lowest z {lowest}"
    );
    let (removed_at, apex) = emerge_removed.expect("submerge path ended");
    for (node, best) in flying.iter().zip(closest) {
        assert!(best < 0.75, "flying spline passes node {node}: {best}");
    }
    let landed_at = landed.expect("landing path ended");
    assert!(h.wow_position(boss).distance(landing) < 0.01);
    let flags = h.app.world().get::<UnitFlags>(boss).unwrap().0;
    assert_eq!(flags & UnitFlags::NOT_SELECTABLE, 0);
    // Template speed_run2.0 * baseMoveSpeed7: a Catmull-Rom path is never shorter
    // than its chords, and the two later MovePath calls each wait1000ms.
    let mut chords = apex.distance(flying[0]);
    for pair in flying.windows(2) {
        chords += pair[0].distance(pair[1]);
    }
    let landing_drop = flying[10].distance(landing);
    let minimum = 2.0 + (chords + landing_drop) / 14.0;
    let flight = landed_at - removed_at;
    assert!(
        flight >= minimum - 0.1 && flight <= minimum + 1.5,
        "flight {flight}s, chord minimum {minimum}s"
    );
    h.advance(2.0);
    assert!(h.wow_position(boss).distance(landing) < 0.01);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, 0),
        Some(EncounterState::NotStarted)
    );
}

fn defer_due_time_while_busy(mut due: f64, next_cast: f64, busy_windows: &[(f64, f64)]) -> f64 {
    for &(busy_start, busy_end) in busy_windows {
        if busy_start >= next_cast - 0.001 {
            break;
        }
        if busy_start <= due + 0.001 && due < busy_end {
            due = busy_end;
        }
    }
    due
}

#[test]
fn encounter_coverage_golden_serpent_tc_schedule_casts_at_authored_boundaries() {
    // Source InitializeAI explicitly handles every state other than NotStarted.
    // This is partial combat in that source branch, NOT fresh intro acceptance.
    let (mut h, boss, _) =
        bound_boss_in_state(135322, 1762, 0, 1, None, EncounterState::InProgress);
    let expected = Vec3::new(-1058.8403, 2615.1667, 810.1519);
    assert!(
        h.wow_position(boss).distance(expected) < 0.001,
        "position {:?}, state {:?}",
        h.wow_position(boss),
        h.app.world().get::<crate::creature_ai::CreatureState>(boss)
    );
    let player = h.player();
    let boss_position = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert((
        Position {
            x: boss_position.x + 3.0,
            ..boss_position
        },
        Health {
            current: 1_000_000_000.0,
            max: 1_000_000_000.0,
        },
    ));
    let started = crate::auras::aura_now(h.app.world());
    h.engage(boss);
    let mut previous_cast = None;
    let mut cast_started = 0.0;
    let mut busy_windows = Vec::new();
    let mut admissions: std::collections::HashMap<u32, Vec<f64>> = Default::default();
    for _ in 0..1_800 {
        h.advance(0.05);
        let now = crate::auras::aura_now(h.app.world()) - started;
        let current_cast = h
            .app
            .world()
            .get::<CastState>(boss)
            .map(|cast| cast.spell_id);
        if previous_cast != current_cast {
            if previous_cast.is_some() {
                busy_windows.push((cast_started, now));
            }
            cast_started = now;
        }
        if let Some(spell_id) = current_cast
            && previous_cast != current_cast
            && [265773, 265781, 265910, 265923].contains(&spell_id)
        {
            admissions.entry(spell_id).or_default().push(now);
        }
        previous_cast = current_cast;
    }
    let gust_cast = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(265781)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .cast_time
        .unwrap()
        .base_ms;
    let tail_first = 12.5 + f64::from(gust_cast) / 1_000.0;
    for (spell_id, first, repeat) in [
        (265773, 9.0, 11.0),
        (265781, 12.5, 21.5),
        (265910, tail_first, 17.0),
        (265923, 40.0, 41.5),
    ] {
        let times = admissions
            .get(&spell_id)
            .unwrap_or_else(|| panic!("authored spell {spell_id} never admitted"));
        assert!(
            (times[0] - first).abs() <= 0.1,
            "spell {spell_id}: actual {times:?}, first {first}"
        );
        assert!(
            times.len() >= 2,
            "authored spell {spell_id} did not repeat: {times:?}"
        );
        for pair in times.windows(2) {
            let expected_due = defer_due_time_while_busy(pair[0] + repeat, pair[1], &busy_windows);
            assert!(
                (pair[1] - expected_due).abs() <= 0.1,
                "spell {spell_id}: actual {times:?}, due {expected_due}, busy {busy_windows:?}"
            );
        }
    }
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.spell_id == Some(265773)
                && event.kind == CombatLogKind::Damage
                && event.periodic)
    );
    assert!(
        h.inbox(0).timeline.is_empty(),
        "partial TC combat must not invent Retail alerts"
    );
}

#[test]
fn encounter_coverage_golden_serpent_rejects_obsolete_bossmod_alert_catalog() {
    // Authentic legacy LittleWigs GoldenSerpent.lua OnEngage CDBar265773/9s.
    // Current Retail never invokes it, so even a stale catalog must stay silent.
    let catalog_db = Connection::open_in_memory().unwrap();
    catalog_db
        .execute_batch(include_str!("bossmod_diremaul.sql"))
        .unwrap();
    catalog_db
        .execute(
            "INSERT INTO bossmod_authority VALUES(2139,'LittleWigs')",
            [],
        )
        .unwrap();
    catalog_db
        .execute(
            "INSERT INTO bossmod_rule(id,addon,encounter_id,trigger,action,spell_id,delay_ms)
         VALUES(100,'LittleWigs',2139,'engage','timer',265773,9000)",
            [],
        )
        .unwrap();
    let catalog = bossmod::load_catalog(&catalog_db).unwrap();
    let (mut h, boss, _) = bound_boss_in_state(
        135322,
        1762,
        0,
        1,
        Some(catalog),
        EncounterState::InProgress,
    );
    let player = h.player();
    let position = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: position.x + 3.0,
        ..position
    });
    h.engage(boss);
    h.advance(0.1);
    assert!(
        h.inbox(0)
            .engages
            .iter()
            .any(|frame| frame.unit == boss.to_bits())
    );
    assert!(
        h.inbox(0).timeline.is_empty(),
        "stale legacy timer is not Retail authority"
    );
}

#[test]
fn encounter_coverage_golden_serpent_evade_retires_and_respawns_without_credit() {
    let (mut h, boss, map) =
        bound_boss_in_state(135322, 1762, 0, 1, None, EncounterState::InProgress);
    let player = h.player();
    let position = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: position.x + 3.0,
        ..position
    });
    h.engage(boss);
    let gold = crate::zones::summon_creature(
        h.app.world_mut(),
        boss,
        135406,
        [-1058.8403, 2615.1667, 810.1519, 0.0],
    )
    .unwrap();
    h.advance(0.1);
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    h.advance(0.05);
    let retirement = h
        .app
        .world()
        .get::<crate::combat::DeathState>(boss)
        .expect("TC evade must retire the population actor, not just reset combat");
    assert!(retirement.respawn_timer <= 30.0 && retirement.respawn_timer >= 29.8);
    assert!(h.app.world().get_entity(gold).is_err());
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0
    );
    assert!(h.inbox(0).ends.iter().all(|end| !end.success));
    h.app.world_mut().entity_mut(player).insert(Position {
        x: position.x + 500.0,
        ..position
    });
    h.advance(30.5);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(boss)
            .is_none()
    );
    assert!(
        h.wow_position(boss)
            .distance(Vec3::new(-1058.8403, 2615.1667, 810.1519))
            < 0.001,
        "position {:?}, state {:?}",
        h.wow_position(boss),
        h.app.world().get::<crate::creature_ai::CreatureState>(boss)
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitFactionTemplate>(boss)
            .unwrap()
            .0,
        16
    );
}

#[test]
fn encounter_coverage_animated_gold_lethal_spell_hit_becomes_molten_without_death() {
    let (mut h, boss, map) = bound_boss(135322, 1762, 0);
    let position = h.wow_position(boss);
    let gold = crate::zones::summon_creature(
        h.app.world_mut(),
        boss,
        135406,
        [position.x, position.y, position.z, 0.0],
    )
    .unwrap();
    h.advance(0.1);
    let amount = h.health(gold).max * 2_000.0;
    h.hit(gold, amount);
    assert_eq!(
        h.health(gold).current,
        1.0,
        "TC Animated Gold intercepts lethal loss before death"
    );
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(gold)
            .is_none()
    );
    assert!(
        h.has_aura(gold, 265915),
        "lethal interception must cast authored Molten Gold"
    );
    let flags = h
        .app
        .world()
        .get::<shared::components::UnitFlags>(gold)
        .unwrap();
    assert_ne!(flags.0 & shared::components::UnitFlags::IMMUNE_TO_PC, 0);
    assert_ne!(flags.0 & shared::components::UnitFlags::NOT_SELECTABLE, 0);
    assert_eq!(*h.app.world().get::<InMap>(gold).unwrap(), map);
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0
    );
}

#[test]
fn encounter_coverage_animated_gold_direct_self_loss_also_intercepts_lethal_damage() {
    let (mut h, boss, map) = bound_boss(135322, 1762, 0);
    let position = h.wow_position(boss);
    let gold = crate::zones::summon_creature(
        h.app.world_mut(),
        boss,
        135406,
        [position.x, position.y, position.z, 0.0],
    )
    .unwrap();
    h.advance(0.1);
    let amount = h.health(gold).max * 2_000.0;
    crate::combat::deal_direct_damage(h.app.world_mut(), gold, gold, amount);
    assert_eq!(
        h.health(gold).current,
        1.0,
        "script clamp includes self, unlike built-in unkillable"
    );
    assert!(h.has_aura(gold, 265915));
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(gold)
            .is_none()
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        gold,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 265915,
    );
    let flags = h
        .app
        .world()
        .get::<shared::components::UnitFlags>(gold)
        .unwrap();
    assert_eq!(
        flags.0
            & (shared::components::UnitFlags::IMMUNE_TO_PC
                | shared::components::UnitFlags::NOT_SELECTABLE),
        0
    );
}

#[test]
fn encounter_coverage_animated_gold_melee_lethal_hit_is_not_a_kill() {
    let (mut h, boss, _) = bound_boss(135322, 1762, 0);
    let position = h.wow_position(boss);
    let gold = crate::zones::summon_creature(
        h.app.world_mut(),
        boss,
        135406,
        [position.x, position.y, position.z, 0.0],
    )
    .unwrap();
    h.advance(0.1);
    let player = h.player();
    let location = *h.app.world().get::<Position>(gold).unwrap();
    h.app.world_mut().entity_mut(player).insert((
        location,
        crate::stats::CharacterStats {
            main_hand: crate::item_stats::WeaponDamage {
                min: 100_000.0,
                max: 100_000.0,
                ..crate::item_stats::WeaponDamage::UNARMED
            },
            ..Default::default()
        },
        crate::combat::AutoAttack::new(gold),
    ));
    // Reproduce the failed boundary deterministically with native Dark Luck's
    // -100 melee-hit field, rather than changing or seeding production RNG.
    let before = h.health(gold).current;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: player,
            target: player,
            spell_id: 194356,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.advance(0.2);
    assert_eq!(h.health(gold).current, before);
    assert!(!h.has_aura(gold, 265915));
    assert!(h.inbox(0).log.iter().any(|event| {
        event.source == Some(player.to_bits())
            && event.target == Some(gold.to_bits())
            && matches!(event.kind, shared::protocol::CombatLogKind::Miss(_))
    }));
    crate::auras::remove_auras(
        h.app.world_mut(),
        player,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 194356,
    );
    h.advance(1.8);
    // Require an observed landed hit, not a lucky first roll.
    // CharacterStats keeps the ordinary native hit table after debuff removal.
    let mut landed = false;
    for _ in 0..32 {
        let location = *h.app.world().get::<Position>(gold).unwrap();
        h.app.world_mut().entity_mut(player).insert(location);
        h.advance(0.2);
        landed = h.inbox(0).log.iter().any(|event| {
            event.source == Some(player.to_bits())
                && event.target == Some(gold.to_bits())
                && event.spell_id.is_none()
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0
        });
        if landed {
            break;
        }
        h.advance(1.8); // Keep the native two-second weapon cadence between attempts.
    }
    let receipts: Vec<_> = h
        .inbox(0)
        .log
        .iter()
        .filter(|event| {
            event.source == Some(player.to_bits()) && event.target == Some(gold.to_bits())
        })
        .collect();
    assert!(landed, "no landed melee hit: {receipts:?}");
    assert_eq!(h.health(gold).current, 1.0, "{receipts:?}");
    assert!(h.has_aura(gold, 265915));
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(gold)
            .is_none()
    );
}

#[test]
fn encounter_coverage_animated_gold_periodic_loss_intercepts_and_exact_one_hp_is_not_lethal() {
    let (mut h, boss, _) = bound_boss(135322, 1762, 0);
    let position = h.wow_position(boss);
    let gold = crate::zones::summon_creature(
        h.app.world_mut(),
        boss,
        135406,
        [position.x, position.y, position.z, 0.0],
    )
    .unwrap();
    h.advance(0.1);
    let player = h.player();
    let before = h.health(gold).current;
    crate::combat::deal_direct_damage(h.app.world_mut(), gold, gold, before - 1.0);
    assert_eq!(h.health(gold).current, 1.0);
    assert!(
        !h.has_aura(gold, 265915),
        "leaving exactly1HP is not the source lethal predicate"
    );
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(133)
        .unwrap()
        .clone();
    let ctx = EffectCtx {
        caster: player,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    deal_damage(h.app.world_mut(), &ctx, gold, 100_000.0, false, true);
    // At1HP the source clamps the lethal loss to0 BEFORE its authored heal136.
    // Do not clamp again after the heal or invent a permanently-one-HP actor.
    assert_eq!(h.health(gold).current, h.health(gold).max);
    assert!(h.has_aura(gold, 265915));
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(gold)
            .is_none()
    );
}

#[test]
fn encounter_coverage_animated_gold_molten_transition_stops_active_motion() {
    let (mut h, boss, _) = bound_boss(135322, 1762, 0);
    let position = h.wow_position(boss);
    let gold = crate::zones::summon_creature(
        h.app.world_mut(),
        boss,
        135406,
        [position.x, position.y, position.z, 0.0],
    )
    .unwrap();
    h.advance(0.1);
    let start = *h.app.world().get::<Position>(gold).unwrap();
    let destination = Position {
        x: start.x + 30.0,
        ..start
    };
    super::super::movement::move_point(h.app.world_mut(), gold, 1, destination);
    super::super::movement::set_walk(h.app.world_mut(), gold, true);
    h.advance(0.2);
    let moving_position = *h.app.world().get::<Position>(gold).unwrap();
    assert!(moving_position.x > start.x);
    let lethal_loss = h.health(gold).max * 2_000.0;
    crate::combat::deal_direct_damage(h.app.world_mut(), gold, gold, lethal_loss);
    assert!(h.has_aura(gold, 265915));
    let molten_position = *h.app.world().get::<Position>(gold).unwrap();
    h.advance(0.5);
    assert_eq!(
        *h.app.world().get::<Position>(gold).unwrap(),
        molten_position,
        "source lethal transition clears its generator and stops moving"
    );
    assert_eq!(
        *h.app
            .world()
            .get::<shared::components::CreatureMotion>(gold)
            .unwrap(),
        shared::components::CreatureMotion::Still
    );
}

fn molten_gold_fixture() -> (Harness, Entity, Entity, Position, InMap) {
    molten_gold_fixture_in_state(EncounterState::NotStarted)
}

fn molten_gold_fixture_in_state(
    initial_state: EncounterState,
) -> (Harness, Entity, Entity, Position, InMap) {
    let (mut h, boss, map) = bound_boss_in_state(135322, 1762, 0, 1, None, initial_state);
    let position = h.wow_position(boss);
    let gold = crate::zones::summon_creature(
        h.app.world_mut(),
        boss,
        135406,
        [position.x, position.y, position.z, 0.0],
    )
    .unwrap();
    let center = *h.app.world().get::<Position>(gold).unwrap();
    let player = h.player();
    let player_distance = if initial_state == EncounterState::NotStarted {
        20.0
    } else {
        500.0
    };
    h.app.world_mut().entity_mut(player).insert(Position {
        x: center.x + player_distance,
        ..center
    });
    h.advance(0.1);
    let loss = h.health(gold).max * 2_000.0;
    crate::combat::deal_direct_damage(h.app.world_mut(), gold, gold, loss);
    assert!(h.has_aura(gold, 265915));
    (h, gold, player, center, map)
}

#[test]
fn encounter_coverage_molten_gold_source_sphere_enters_damages_and_exits() {
    let (mut h, gold, player, center, _) = molten_gold_fixture();
    // TC sphere radius5 plus Player combat reach1.5; exact distance is 3D.
    h.app.world_mut().entity_mut(player).insert(Position {
        y: center.y + 6.51,
        ..center
    });
    h.advance(0.1);
    assert!(!h.has_aura(player, 265914));
    h.app.world_mut().entity_mut(player).insert(Position {
        y: center.y + 6.49,
        ..center
    });
    h.advance(0.1);
    assert!(
        h.has_aura(player, 265914),
        "source OnUnitEnter self-casts damage aura"
    );
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(player)
        .unwrap();
    assert!(
        aura.iter()
            .any(|a| a.spell_id == 265914 && a.caster == player)
    );
    let health = h.health(player).current;
    h.advance(1.1);
    assert!(
        h.health(player).current < health,
        "actual authored periodic loss"
    );
    assert!(
        !h.has_aura(gold, 265914),
        "NPCs do not receive this player-only trigger"
    );
    h.app.world_mut().entity_mut(player).insert(Position {
        x: center.x + 6.51,
        ..center
    });
    h.advance(0.1);
    assert!(
        !h.has_aura(player, 265914),
        "source OnUnitExit removes damage aura"
    );
}

#[test]
fn encounter_coverage_molten_gold_attached_area_is_copy_local_and_removed_with_aura() {
    let (mut h, gold, player, center, map) = molten_gold_fixture();
    h.app.world_mut().entity_mut(player).insert(center);
    let remote_map = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    let wow = h.wow_position(gold);
    let (remote, _) = spawn_player(&mut h.app, remote_map, [wow.x, wow.y, wow.z, 0.0], 999);
    h.advance(0.1);
    assert!(h.has_aura(player, 265914));
    assert!(!h.has_aura(remote, 265914));
    let moved = Position {
        x: center.x + 30.0,
        ..center
    };
    h.app.world_mut().entity_mut(gold).insert(moved);
    h.advance(0.1);
    assert!(
        !h.has_aura(player, 265914),
        "area follows its aura target, not cast origin"
    );
    h.app.world_mut().entity_mut(player).insert(moved);
    h.advance(0.1);
    assert!(h.has_aura(player, 265914));
    crate::auras::remove_auras(
        h.app.world_mut(),
        gold,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 265915,
    );
    h.advance(0.1);
    assert!(
        !h.has_aura(player, 265914),
        "aura395 removal removes the attached area"
    );
}

#[test]
fn encounter_coverage_molten_gold_scripted_cast_keeps_source_player_selection() {
    let (mut h, gold, player, center, map) = molten_gold_fixture();
    let wow = h.wow_position(gold);
    let (selected, _) = spawn_player(&mut h.app, map, [wow.x + 20.0, wow.y, wow.z, 0.0], 998);
    h.app
        .world_mut()
        .entity_mut(player)
        .insert((center, CombatTarget(Some(selected))));
    h.advance(0.1);
    assert!(
        h.has_aura(selected, 265914),
        "TC null-target cast uses a valid Player selection first"
    );
    assert!(!h.has_aura(player, 265914));
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(selected)
        .unwrap();
    assert!(
        aura.iter()
            .any(|a| a.spell_id == 265914 && a.caster == player)
    );
}

#[test]
fn encounter_coverage_molten_gold_source_reach_scales_and_actor_retirement_exits() {
    let (mut h, gold, player, center, _) = molten_gold_fixture();
    h.app.world_mut().entity_mut(player).insert((
        crate::zones::NativeObjectScale(2.0),
        shared::components::UnitScale(2.0),
        Position {
            x: center.x + 8.01,
            ..center
        },
    ));
    h.advance(0.1);
    assert!(!h.has_aura(player, 265914));
    h.app.world_mut().entity_mut(player).insert(Position {
        x: center.x + 7.99,
        ..center
    });
    h.advance(0.1);
    assert!(
        h.has_aura(player, 265914),
        "source Player combat reach follows object scale"
    );
    crate::zones::unsummon(h.app.world_mut(), gold);
    h.advance(0.1);
    assert!(h.app.world().get_entity(gold).is_err());
    assert!(
        !h.has_aura(player, 265914),
        "removing the aura target exits its area members"
    );
}

#[test]
fn encounter_coverage_lucres_call_removes_real_molten_aura_only_in_its_copy() {
    let (mut h, gold, _, _, map) = molten_gold_fixture();
    let boss = h.creatures_of(135322)[0];
    let position = h.wow_position(gold);
    let remote = crate::zones::summon_creature(
        h.app.world_mut(),
        boss,
        135406,
        [position.x, position.y, position.z, 0.0],
    )
    .unwrap();
    h.app.world_mut().entity_mut(remote).insert(InMap {
        instance_id: map.instance_id + 1,
        ..map
    });
    h.advance(0.1);
    let loss = h.health(remote).max * 2_000.0;
    crate::combat::deal_direct_damage(h.app.world_mut(), remote, remote, loss);
    assert!(h.has_aura(remote, 265915));
    let cast_ms = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(265923)
        .unwrap()
        .misc
        .as_ref()
        .and_then(|misc| misc.cast_time)
        .map_or(0, |time| time.base_ms);
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, 265923, None, false)
        .expect("real local Lucre effect164 and target8 must be admitted");
    h.advance(cast_ms as f32 / 1000.0 + 0.1);
    assert!(
        !h.has_aura(gold, 265915),
        "source effect164 removes TriggerSpell265915"
    );
    assert!(
        h.has_aura(remote, 265915),
        "source area selection cannot cross copies"
    );
}

fn cast_real_lucre(h: &mut Harness, boss: Entity) {
    let cast_ms = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(265923)
        .unwrap()
        .misc
        .as_ref()
        .and_then(|misc| misc.cast_time)
        .map_or(0, |time| time.base_ms);
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, 265923, None, false)
        .expect("real Lucre cast");
    h.advance(cast_ms as f32 / 1000.0 + 0.1);
}

#[test]
fn encounter_coverage_gold_lucre_delays_walk_and_rearms_molten_after_three_seconds() {
    let (mut h, gold, _, center, _) = molten_gold_fixture();
    let boss = h.creatures_of(135322)[0];
    h.app.world_mut().entity_mut(gold).insert(Position {
        x: center.x + 25.0,
        ..center
    });
    let waiting_position = *h.app.world().get::<Position>(gold).unwrap();
    cast_real_lucre(&mut h, boss);
    assert!(!h.has_aura(gold, 265915));
    h.advance(2.7);
    assert!(
        h.app
            .world()
            .get::<super::super::movement::Walking>(gold)
            .is_none()
    );
    h.advance(0.3);
    assert!(
        h.app
            .world()
            .get::<super::super::movement::Walking>(gold)
            .is_some(),
        "source task sets walk after3s, not when Lucre first hits"
    );
    assert_eq!(
        *h.app.world().get::<Position>(gold).unwrap(),
        waiting_position,
        "missing bake must not grant straight-line movement"
    );
    let loss = h.health(gold).max * 2_000.0;
    crate::combat::deal_direct_damage(h.app.world_mut(), gold, gold, loss);
    assert!(
        h.has_aura(gold, 265915),
        "source task rearms the first-lethal Molten cast"
    );
}

#[test]
fn encounter_coverage_gold_lucre_task_checks_molten_at_callback_not_at_hit() {
    let (mut h, gold, _, _, _) = molten_gold_fixture();
    let boss = h.creatures_of(135322)[0];
    cast_real_lucre(&mut h, boss);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: gold,
            target: gold,
            spell_id: 265915,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.advance(3.1);
    assert!(h.has_aura(gold, 265915));
    assert!(
        h.app
            .world()
            .get::<super::super::movement::Walking>(gold)
            .is_none(),
        "restored Molten blocks the delayed source task"
    );
}

fn insert_kings_rest_navigation(h: &mut Harness) {
    let nav_dir = crate::ground::ground_data_dir().join("navmesh");
    assert!(
        nav_dir.join("1762").read_dir().unwrap().any(|entry| entry
            .unwrap()
            .path()
            .extension()
            .is_some_and(|extension| extension == "nav")),
        "real Kings' Rest bake is required for this integration boundary"
    );
    h.app
        .insert_resource(crate::navigation::Navigation::new(&nav_dir));
}

#[test]
fn encounter_coverage_gold_lucre_baked_follow_absorbs_then_retires_without_credit() {
    // Source NotStarted spawn is in the unported flying intro, not a walkable start.
    let (mut h, gold, _, center, map) = molten_gold_fixture_in_state(EncounterState::Fail);
    let boss = h.creatures_of(135322)[0];
    insert_kings_rest_navigation(&mut h);
    // This west ray has an actual nav corridor; the east fixture ray was outside the chamber.
    let start = Position {
        x: center.x - 25.0,
        ..center
    };
    h.app.world_mut().entity_mut(gold).insert(start);
    cast_real_lucre(&mut h, boss);
    h.advance(2.7);
    assert_eq!(*h.app.world().get::<Position>(gold).unwrap(), start);
    h.advance(0.5);
    let moving = *h.app.world().get::<Position>(gold).unwrap();
    assert_ne!(
        moving, start,
        "source task starts actual walking over baked navigation"
    );
    assert_eq!(
        *h.app.world().get::<CreatureMotion>(gold).unwrap(),
        CreatureMotion::Walk
    );
    let mut absorbed = false;
    for _ in 0..200 {
        h.advance(0.1);
        if h.app
            .world()
            .get::<shared::components::ModelDisplay>(gold)
            .unwrap()
            .display_id
            == 11686
        {
            absorbed = true;
            break;
        }
    }
    assert!(absorbed, "real nav arrival must run Follow MovementInform");
    assert_eq!(
        *h.app.world().get::<CreatureMotion>(gold).unwrap(),
        CreatureMotion::Still
    );
    assert_ne!(
        h.app.world().get::<UnitFlags>(gold).unwrap().0 & UnitFlags::NOT_SELECTABLE,
        0
    );
    let stopped = *h.app.world().get::<Position>(gold).unwrap();
    h.advance(3.8);
    assert!(h.app.world().get_entity(gold).is_ok());
    assert_eq!(*h.app.world().get::<Position>(gold).unwrap(), stopped);
    assert!(
        h.has_aura(boss, 265991),
        "actual triggered Luster lands on the source primary"
    );
    h.advance(0.3);
    assert!(h.app.world().get_entity(gold).is_err());
    assert_ne!(
        get_boss_state(h.app.world_mut(), map, 0),
        Some(EncounterState::Done)
    );
}

#[test]
fn encounter_coverage_gold_follow_checks_source_three_dimensional_reach() {
    let (mut h, gold, _, center, _) = molten_gold_fixture_in_state(EncounterState::Fail);
    let boss = h.creatures_of(135322)[0];
    h.app.world_mut().entity_mut(gold).insert(Position {
        y: center.y + 8.01,
        ..center
    });
    cast_real_lucre(&mut h, boss);
    h.advance(3.3);
    assert_ne!(
        h.app
            .world()
            .get::<shared::components::ModelDisplay>(gold)
            .unwrap()
            .display_id,
        11686,
        "horizontal overlap cannot absorb outside source reach1+5+2 in3D"
    );
    h.app.world_mut().entity_mut(gold).insert(Position {
        y: center.y + 7.99,
        ..center
    });
    h.advance(0.1);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::ModelDisplay>(gold)
            .unwrap()
            .display_id,
        11686,
        "source100ms position check informs once inside3D reach"
    );
}

#[test]
fn encounter_coverage_gold_follow_repaths_subyard_primary_displacement() {
    let (mut h, gold, _, center, _) = molten_gold_fixture_in_state(EncounterState::Fail);
    let boss = h.creatures_of(135322)[0];
    insert_kings_rest_navigation(&mut h);
    h.app.world_mut().entity_mut(gold).insert(Position {
        x: center.x - 25.0,
        ..center
    });
    cast_real_lucre(&mut h, boss);
    h.advance(4.0);
    let displaced = Position {
        x: center.x + 0.5,
        ..center
    };
    h.app.world_mut().entity_mut(boss).insert(displaced);
    let mut absorbed_at = None;
    for _ in 0..200 {
        h.advance(0.1);
        if h.app
            .world()
            .get::<shared::components::ModelDisplay>(gold)
            .unwrap()
            .display_id
            == 11686
        {
            absorbed_at = h.app.world().get::<Position>(gold).copied();
            break;
        }
    }
    let absorbed_at = absorbed_at.expect("subyard displacement retains actual follow arrival");
    let separation = (displaced.x - absorbed_at.x).abs();
    assert!(
        separation <= 8.02,
        "chase's1yard route tolerance must not end follow at stale8.5yard endpoint: {separation}"
    );
}

fn walking_gold_fixture() -> (Harness, Entity, Entity) {
    let (mut h, gold, _, center, _) = molten_gold_fixture_in_state(EncounterState::Fail);
    let boss = h.creatures_of(135322)[0];
    insert_kings_rest_navigation(&mut h);
    h.app.world_mut().entity_mut(gold).insert(Position {
        x: center.x - 25.0,
        ..center
    });
    cast_real_lucre(&mut h, boss);
    h.advance(3.5);
    assert_eq!(
        *h.app.world().get::<CreatureMotion>(gold).unwrap(),
        CreatureMotion::Walk
    );
    (h, gold, boss)
}

#[test]
fn encounter_coverage_gold_follow_root_stops_then_resumes_the_real_path() {
    let (mut h, gold, _) = walking_gold_fixture();
    // Actual root122 aura through Unit's aura boundary, not a fabricated movement flag.
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: gold,
            target: gold,
            spell_id: 122,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(h.has_aura(gold, 122));
    let rooted_at = *h.app.world().get::<Position>(gold).unwrap();
    h.advance(0.5);
    assert_eq!(
        *h.app.world().get::<Position>(gold).unwrap(),
        rooted_at,
        "source UNIT_STATE_NOT_MOVE must prevent follow path advancement"
    );
    assert_eq!(
        *h.app.world().get::<CreatureMotion>(gold).unwrap(),
        CreatureMotion::Still
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        gold,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 122,
    );
    h.advance(0.2);
    assert_ne!(*h.app.world().get::<Position>(gold).unwrap(), rooted_at);
    assert_eq!(
        *h.app.world().get::<CreatureMotion>(gold).unwrap(),
        CreatureMotion::Walk
    );
}

#[test]
fn encounter_coverage_gold_follow_respects_real_movement_interrupting_cast() {
    let (mut h, gold, _) = walking_gold_fixture();
    // Genuine4s Gust is a Unit-level cast fixture; Gold's authored AI never casts it.
    crate::spell_cast::creature_cast(h.app.world_mut(), gold, 265781, Some(gold), false)
        .expect("real movement-interrupting NPC cast");
    let casting_at = *h.app.world().get::<Position>(gold).unwrap();
    h.advance(0.5);
    assert_eq!(
        *h.app.world().get::<Position>(gold).unwrap(),
        casting_at,
        "source IsMovementPreventedByCasting stops the following owner"
    );
    h.advance(3.8);
    assert_ne!(*h.app.world().get::<Position>(gold).unwrap(), casting_at);
    assert_eq!(
        *h.app.world().get::<CreatureMotion>(gold).unwrap(),
        CreatureMotion::Walk
    );
}

#[test]
fn encounter_coverage_golden_absorption_announcement_is_once_until_source_reset() {
    let (mut h, boss, _) = bound_boss(135322, 1762, 0);
    for _ in 0..2 {
        super::super::dispatch(h.app.world_mut(), boss, super::super::Hook::DoAction(1));
    }
    h.advance(0.1);
    let announcements = |h: &Harness| {
        h.players[0]
            .2
            .chat
            .iter()
            .filter(|message| message.content.contains("absorbs an Animated Gold"))
            .count()
    };
    assert_eq!(announcements(&h), 1);
    super::super::dispatch(h.app.world_mut(), boss, super::super::Hook::Reset);
    super::super::dispatch(h.app.world_mut(), boss, super::super::Hook::DoAction(1));
    h.advance(0.1);
    assert_eq!(announcements(&h), 2);
}

#[test]
fn encounter_coverage_gold_luster_uses_real_nearby_entry_condition() {
    let (mut h, gold, player, center, map) = molten_gold_fixture();
    let boss = h.creatures_of(135322)[0];
    let remote = crate::zones::summon_creature(
        h.app.world_mut(),
        boss,
        135322,
        [center.x, -center.z, center.y, 0.0],
    )
    .unwrap();
    h.app.world_mut().entity_mut(remote).insert(InMap {
        instance_id: map.instance_id + 1,
        ..map
    });
    // A nearer unrelated player must not crowd out the valid entry.
    h.app.world_mut().entity_mut(player).insert(Position {
        x: center.x + 1.0,
        ..center
    });
    h.app.world_mut().entity_mut(boss).insert(Position {
        x: center.x + 3.0,
        ..center
    });
    crate::auras::remove_auras(
        h.app.world_mut(),
        gold,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 265915,
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), gold, 265991, None, true)
        .expect("real Luster target38 admission");
    // Actual Speed18, minimum missile distance5: primary hit lands after278ms.
    h.advance(0.4);
    assert!(
        h.has_aura(boss, 265991),
        "authored entry135322 condition selects the primary"
    );
    assert!(
        h.has_aura(gold, 265991),
        "unconditioned effect2 selects the nearby caster separately"
    );
    let primary_aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(boss)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 265991)
        .unwrap();
    assert_eq!(
        primary_aura
            .effects
            .iter()
            .map(|effect| effect.aura_type)
            .collect::<Vec<_>>(),
        vec![69, 79]
    );
    let gold_aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(gold)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 265991)
        .unwrap();
    assert_eq!(
        gold_aura
            .effects
            .iter()
            .map(|effect| effect.aura_type)
            .collect::<Vec<_>>(),
        vec![4]
    );
    assert!(!h.has_aura(player, 265991));
    assert!(
        !h.has_aura(remote, 265991),
        "nearby lookup cannot cross map copies"
    );
}

#[test]
fn encounter_coverage_source_conditions_preserve_legacy_and_current_object_types() {
    let (mut h, boss, _) = bound_boss(135322, 1762, 0);
    let conditions = crate::conditions::load_spell_target_conditions(&world_db_path()).unwrap();
    let player = h.player();
    assert!(conditions.target_meets(h.app.world(), 265991, 0, boss));
    assert!(!conditions.target_meets(h.app.world(), 265991, 0, player));
    h.app
        .world_mut()
        .get_mut::<shared::components::Npc>(boss)
        .unwrap()
        .template_id = 15263;
    // Actual retained TDB3730 row is legacy31/type3/entry15263.
    assert!(conditions.target_meets(h.app.world(), 3730, 0, boss));
    assert!(!conditions.target_meets(h.app.world(), 3730, 0, player));
    assert!(!conditions.target_meets(h.app.world(), 265991, 0, boss));
}

#[test]
fn encounter_coverage_spit_gold_summons_only_on_expiry_or_target_death() {
    let (mut h, boss, map) = bound_boss(135322, 1762, 0);
    let player = h.player();
    for (reason, expected) in [
        (crate::auras::RemoveReason::Dispelled, 0),
        (crate::auras::RemoveReason::Expired, 1),
        (crate::auras::RemoveReason::Death, 2),
    ] {
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: boss,
                target: player,
                spell_id: 265773,
                effect_mask: 1,
                source: crate::auras::AuraSource::Cast,
            },
        );
        assert!(h.has_aura(player, 265773));
        let location = *h.app.world().get::<Position>(player).unwrap();
        crate::auras::remove_auras(h.app.world_mut(), player, reason, |aura| {
            aura.spell_id == 265773
        });
        let actors = h.creatures_of(135406);
        assert_eq!(actors.len(), expected, "source removal reason {reason:?}");
        for actor in actors {
            assert_eq!(*h.app.world().get::<InMap>(actor).unwrap(), map);
            assert_eq!(*h.app.world().get::<Position>(actor).unwrap(), location);
        }
    }
}

#[test]
fn encounter_coverage_nibbles_survives_lethal_hit_without_encounter_credit() {
    let (mut h, kystia, map) = bound_boss(234648, 2813, 0);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (nibbles, _) = spawn_authored_actor(&mut h.app, &conn, 234660, map, 1);
    h.engage(kystia);
    h.advance(0.1);
    h.hit_pct(nibbles, 2_000.0);
    assert_eq!(
        h.health(nibbles).current,
        1.0,
        "TC Nibbles SetUnkillable must clamp a lethal hit"
    );
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(nibbles)
            .is_none()
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0
    );
    assert!(
        h.inbox(0).ends.is_empty(),
        "companion damage cannot complete encounter3101"
    );
}

#[test]
fn encounter_coverage_nibbles_health_check_starts_authored_light_infusion_phase() {
    let (mut h, kystia, map) = bound_boss(234648, 2813, 0);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (nibbles, _) = spawn_authored_actor(&mut h.app, &conn, 234660, map, 1);
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Health {
        current: 1_000_000_000.0,
        max: 1_000_000_000.0,
    });
    h.engage(kystia);
    h.advance(0.1);
    assert!(
        h.inbox(0)
            .engages
            .iter()
            .any(|frame| frame.unit == nibbles.to_bits() && frame.target_frame_priority == 2)
    );
    h.hit_pct(nibbles, 2_000.0);
    h.advance(0.8);
    assert_ne!(
        h.app.world().get::<UnitFactionTemplate>(nibbles).unwrap().0,
        35,
        "source health check is not due before1s"
    );
    h.advance(0.2);
    assert_eq!(
        h.app.world().get::<UnitFactionTemplate>(nibbles).unwrap().0,
        35
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::ModelDisplay>(nibbles)
            .unwrap()
            .display_id,
        128332
    );
    assert!(
        matches!(
            h.app
                .world()
                .get::<crate::creature_ai::CreatureState>(nibbles),
            Some(crate::creature_ai::CreatureState::Pursuing { .. })
        ),
        "friendly scripted phase cannot be generic evade: {:?}",
        h.app
            .world()
            .get::<crate::creature_ai::CreatureState>(nibbles)
    );
    let cast = h
        .app
        .world()
        .get::<CastState>(nibbles)
        .expect("Light Infusion must start at the1s health check");
    assert_eq!(cast.spell_id, 1230304);
    assert_eq!(cast.duration, 2.0, "local CastingTimeIndex5 is2000ms");
    h.advance(2.2);
    let channel = h
        .app
        .world()
        .get::<CastState>(nibbles)
        .expect("Light Infusion must become the authored channel");
    assert_eq!(channel.spell_id, 1230304);
    assert_eq!(channel.cast_type, CastType::Channel);
    assert_eq!(channel.duration, 15.0, "local DurationIndex8 is15000ms");
    assert!(
        h.has_aura(kystia, 1265412),
        "Light Infusion dummy must apply Destabilized to Kystia"
    );
    assert!(
        !h.has_aura(kystia, 1217989),
        "Light Infusion203 must remove the initial shield"
    );
    // Named source gap, NOT successful cycle acceptance: partial flags0x40020
    // omit IGNORE_CASTER_AURAS.20s Destabilized still stuns Kystia after15s Light.
    h.advance(14.9);
    assert_eq!(
        crate::spell_cast::creature_cast_ignoring_in_progress(
            h.app.world_mut(),
            kystia,
            474238,
            Some(nibbles)
        ),
        Err(crate::spell_cast::CastFailure::CrowdControlled(
            crate::auras::CastBlock::Stunned
        ))
    );
    assert_eq!(
        h.app.world().get::<CastState>(kystia).unwrap().spell_id,
        1265412
    );
    assert!(
        !h.has_aura(kystia, 1217989),
        "blocked source callback must not invent shield renewal"
    );
    assert!(
        h.health(nibbles).current > h.health(nibbles).max * 0.2,
        "authored10% healing ticks must restore the companion"
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0
    );
    assert!(
        h.inbox(1).engages.is_empty(),
        "companion frame cannot leak to another copy"
    );
}

#[test]
#[ignore = "authentic infusion cycle blocked: local15s Light/20s self-stun; no approved timer/flag correction"]
fn encounter_coverage_kystia_full_infusion_cycle_acceptance_gate() {
    let (mut h, kystia, map) = bound_boss(234648, 2813, 0);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (nibbles, _) = spawn_authored_actor(&mut h.app, &conn, 234660, map, 1);
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Health {
        current: 1_000_000_000.0,
        max: 1_000_000_000.0,
    });
    h.engage(kystia);
    h.advance(0.1);
    h.hit_pct(nibbles, 2_000.0);
    h.advance(18.1);
    let infusion = h
        .app
        .world()
        .get::<CastState>(kystia)
        .expect("Illicit Infusion required for full cycle");
    assert_eq!(infusion.spell_id, 474238);
    assert_eq!(infusion.cast_type, CastType::Channel);
    assert_eq!(infusion.duration, 3.0);
    h.advance(3.2);
    let auras = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(kystia)
        .unwrap();
    assert_eq!(
        auras
            .iter()
            .find(|aura| aura.spell_id == 1217989)
            .unwrap()
            .stacks,
        3
    );
    assert_eq!(
        h.app.world().get::<UnitFactionTemplate>(nibbles).unwrap().0,
        16
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::ModelDisplay>(nibbles)
            .unwrap()
            .display_id,
        126199
    );
}

#[test]
fn encounter_coverage_kystia_uses_authored_chaos_barrage_schedule_and_lifecycle() {
    // TC a352b1fa boss_kystia_manaheart.cpp: 1s initial, 3500ms repeat.
    // Primary actor only completes encounter3101; Nibbles never grants boss credit.
    let (mut h, boss, map) = bound_boss(234648, 2813, 0);
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Health {
        current: 1_000_000_000.0,
        max: 1_000_000_000.0,
    });
    h.engage(boss);
    h.advance(0.9);
    assert_eq!(h.casts(0, boss, 1230298), 0);
    h.advance(0.2);
    let cast = h
        .app
        .world()
        .get::<CastState>(boss)
        .expect("authored1s Chaos Barrage missing");
    assert_eq!(cast.spell_id, 1230298);
    assert_eq!(cast.duration, 3.0, "DB2 CastingTimeIndex14 is3000ms");
    assert_eq!(
        h.casts(0, boss, 1230298),
        0,
        "source trigger flags do not bypass cast time"
    );
    h.advance(2.8);
    assert_eq!(h.casts(0, boss, 1230298), 0);
    h.advance(0.2);
    assert_eq!(h.casts(0, boss, 1230298), 1);
    h.advance(0.3);
    assert!(h.app.world().get::<CastState>(boss).is_none());
    h.advance(0.2);
    assert_eq!(
        h.app.world().get::<CastState>(boss).unwrap().spell_id,
        1230298
    );
    h.advance(3.0);
    assert_eq!(
        h.casts(0, boss, 1230298),
        2,
        "authored3500ms repeat missing"
    );
    assert_eq!(h.inbox(0).starts[0].encounter_id, 3101);
    assert!(h.inbox(1).starts.is_empty());
    assert!(
        h.inbox(1)
            .log
            .iter()
            .all(|e| e.source != Some(boss.to_bits()))
    );
    // Mirror is due14s, but TC waits for the11.5s Chaos cast to finish14.5s.
    h.advance(6.8);
    assert!(
        !h.has_aura(boss, 1264095),
        "Mirror Images interrupted Chaos Barrage"
    );
    h.advance(0.3);
    assert!(
        h.has_aura(boss, 1264095),
        "due Mirror Images aura did not follow cast completion"
    );
    assert_eq!(
        h.casts(0, boss, 474240),
        0,
        "Normal must not schedule Fel Nova"
    );
    // Controlled death proof includes the authored four-stack damage reduction.
    h.hit_pct(boss, 2_000.0);
    h.advance(0.05);
    assert!(h.inbox(0).ends.last().unwrap().success);
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        1
    );
    let casts = h.casts(0, boss, 1230298);
    h.advance(4.0);
    assert_eq!(h.casts(0, boss, 1230298), casts, "dead boss cast again");
}

#[test]
fn encounter_coverage_kystia_appears_with_sourced_four_stack_felshield() {
    let (mut h, boss, _) = bound_boss(234648, 2813, 0);
    let shield_stacks = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(boss)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == 1217989))
        .map(|aura| aura.stacks);
    assert_eq!(
        shield_stacks,
        Some(4),
        "TC JustAppeared/FelCrazed must supply four stacks"
    );
    let before = h.health(boss).current;
    h.hit(boss, 100_000.0);
    let protected_loss = before - h.health(boss).current;
    assert!(protected_loss > 0.0);
    crate::auras::remove_auras(
        h.app.world_mut(),
        boss,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 1217989,
    );
    let before = h.health(boss).current;
    h.hit(boss, 100_000.0);
    let unprotected_loss = before - h.health(boss).current;
    // DB2 aura87 authors -20% for each of the four stacks.
    assert!((protected_loss / unprotected_loss - 0.2).abs() < 0.001);
}

#[test]
fn encounter_coverage_kystia_source_trigger_flags_do_not_bypass_school_lockout() {
    let (mut h, boss, _) = bound_boss(234648, 2813, 0);
    let now = h.app.world().resource::<Time>().elapsed_secs_f64();
    let mut history = crate::spell_cast::SpellHistory::default();
    history.lock_schools(127, now + 30.0);
    h.app.world_mut().entity_mut(boss).insert(history);
    h.engage(boss);
    h.advance(1.1);
    assert_eq!(
        h.casts(0, boss, 1230298),
        0,
        "TC ignores only cast-in-progress, not school lockout"
    );
}

#[test]
fn encounter_coverage_kystia_heroic_fel_nova_schedule_and_evade_retirement() {
    let (mut h, boss, _) = bound_boss_at_difficulty(234648, 2813, 0, 2, None);
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Health {
        current: 1_000_000_000.0,
        max: 1_000_000_000.0,
    });
    h.engage(boss);
    // Due12s, admitted only after the11.5s Chaos cast completes14.5s.
    h.advance(14.4);
    let nova_hits = |h: &Harness| {
        h.inbox(0)
            .log
            .iter()
            .filter(|e| {
                e.kind == CombatLogKind::Damage
                    && e.source == Some(boss.to_bits())
                    && e.spell_id == Some(1223906)
            })
            .count()
    };
    assert_eq!(nova_hits(&h), 0);
    h.advance(0.3);
    let cast = h
        .app
        .world()
        .get::<CastState>(boss)
        .expect("Heroic selector→Blink→Fel Nova must start the sourced cast");
    assert_eq!(cast.spell_id, 1223906);
    assert_eq!(cast.duration, 3.5, "DB2 CastingTimeIndex22 is3500ms");
    h.advance(3.5);
    assert!(
        nova_hits(&h) > 0,
        "Heroic Fel Nova must deal actual DB2 damage"
    );
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    h.advance(0.05);
    assert!(!h.inbox(0).ends.last().unwrap().success);
    let hits = nova_hits(&h);
    h.advance(16.0);
    assert_eq!(nova_hits(&h), hits, "evaded boss retained its15s schedule");
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
            EncounterTimelineEvent::Cast { caster, spell_id, severity:alert_severity, warning_text }
            if *caster == boss.to_bits() && *spell_id == spell && *alert_severity == severity
                && warning_text == if entry == 11490 { "Casting Intense Pain" } else { "Riptide" })));
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
