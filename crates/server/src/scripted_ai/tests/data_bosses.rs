//! Authentic JournalEncounterCreature joins exercised through real creature spawning.
use super::*;

#[path = "azure_absolute_zero.rs"]
mod azure_absolute_zero;
#[path = "azure_combat.rs"]
mod azure_combat;
#[path = "azure_explosive_brand.rs"]
mod azure_explosive_brand;
#[path = "azure_frost_bomb.rs"]
mod azure_frost_bomb;
#[path = "azure_frozen_ground.rs"]
mod azure_frozen_ground;
#[path = "azure_intro.rs"]
mod azure_intro;
#[path = "azure_jump_charge.rs"]
mod azure_jump_charge;
#[path = "azure_leymor_children.rs"]
mod azure_leymor_children;
#[path = "azure_mechanic_power.rs"]
mod azure_mechanic_power;
#[path = "azure_rune_callbacks.rs"]
mod azure_rune_callbacks;
#[path = "azure_rune_fields.rs"]
mod azure_rune_fields;
#[path = "azure_rune_strip.rs"]
mod azure_rune_strip;
#[path = "azure_sprout_death.rs"]
mod azure_sprout_death;
#[path = "azure_sprout_field_actions.rs"]
mod azure_sprout_field_actions;
#[path = "azure_sprout_fields.rs"]
mod azure_sprout_fields;
#[path = "azure_sprouts.rs"]
mod azure_sprouts;
#[path = "azure_telash.rs"]
mod azure_telash;
#[path = "azure_telash_return.rs"]
mod azure_telash_return;
#[path = "celebras.rs"]
mod celebras;
#[path = "gargolmar.rs"]
mod gargolmar;
#[path = "gargolmar_speech.rs"]
mod gargolmar_speech;
#[path = "gizlock.rs"]
mod gizlock;
#[path = "gnomeregan.rs"]
mod gnomeregan;
#[path = "data_bosses_group_criteria.rs"]
mod group_criteria;
#[path = "herald_intro.rs"]
mod herald_intro;
#[path = "landslide.rs"]
mod landslide;
#[path = "maraudon.rs"]
mod maraudon;
#[path = "noxxion.rs"]
mod noxxion;
#[path = "omor.rs"]
mod omor;
#[path = "omor_pushback.rs"]
mod omor_pushback;
#[path = "omor_slay.rs"]
mod omor_slay;
#[path = "theradras.rs"]
mod theradras;
#[path = "vyletongue.rs"]
mod vyletongue;

#[path = "ragefire_completion.rs"]
mod ragefire_completion;

#[path = "shadowfang_keep.rs"]
mod shadowfang_keep;
#[path = "wailing_aggro.rs"]
mod wailing_aggro;
#[path = "wailing_awakening.rs"]
mod wailing_awakening;
#[path = "wailing_escort.rs"]
mod wailing_escort;
#[path = "wailing_physical.rs"]
mod wailing_physical;
#[path = "wailing_progress.rs"]
mod wailing_progress;
#[path = "wailing_ritual_waves.rs"]
mod wailing_ritual_waves;
#[path = "wailing_start.rs"]
mod wailing_start;

fn bound_boss(entry: u32, map_id: u32, bit: u32) -> (Harness, Entity, InMap) {
    bound_boss_with_catalog(entry, map_id, bit, None)
}

fn bound_boss_with_catalog(
    entry: u32,
    map_id: u32,
    bit: u32,
    catalog: Option<bossmod::Catalog>,
) -> (Harness, Entity, InMap) {
    // Source Lucifron spawn20056605 supports9/18; map409 difficulty9 supplies CT1004.
    // Legacy difficulty3 has no installed MapDifficulty row and is not that spawn's context.
    let difficulty = if map_id == 409 { 9 } else { 1 };
    bound_boss_at_difficulty(entry, map_id, bit, difficulty, catalog)
}

/// Use the production importer on a consistent isolated snapshot, never
/// canonical data. The TempDir stays alive until all fixture resources are loaded.
fn prepare_encounter_database(encounter: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    // Concurrent full-world snapshots must not consume the shared /tmp tmpfs.
    let fixture_root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/encounter-fixtures");
    std::fs::create_dir_all(&fixture_root).expect("disk-backed encounter fixture root");
    let directory =
        tempfile::tempdir_in(fixture_root).expect("isolated encounter fixture directory");
    let path = directory.path().join("encounter-mirror.sqlite");
    let source = Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("read-only encounter source database");
    source
        .execute("VACUUM INTO ?1", [path.to_str().unwrap()])
        .expect("snapshot canonical encounter data without writing source");
    if ["gargolmar", "omor"].contains(&encounter) {
        let dump = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql");
        run_preparation_script(
            "encounter_serverside_spells.py",
            &[
                "--encounter".as_ref(),
                encounter.as_ref(),
                "--db".as_ref(),
                path.as_os_str(),
                "--source-dump".as_ref(),
                dump.as_os_str(),
            ],
        );
        if encounter == "gargolmar" {
            return (directory, path);
        }
    }
    if ["adarogg", "slagmaw"].contains(&encounter) {
        // encounter_serverside_spells.py's actual TDB rows precede the summon import.
        let copy = Connection::open(&path).unwrap();
        let prepared: bool = copy
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'tdb_serverside_spell')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        if !prepared {
            crate::spell_info::serverside_tests::write_ragefire_serverside_tables(&copy);
        }
    }
    if encounter == "slagmaw" {
        let dump = world_db_path()
            .parent()
            .unwrap()
            .join("reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql");
        run_preparation_script(
            "encounter_model_geometry.py",
            &[
                "--db".as_ref(),
                path.as_os_str(),
                "--source-dump".as_ref(),
                dump.as_os_str(),
            ],
        );
    } else {
        run_preparation_script(
            "encounter_summons.py",
            &[
                "--encounter".as_ref(),
                encounter.as_ref(),
                "--db".as_ref(),
                path.as_os_str(),
            ],
        );
    }
    (directory, path)
}

/// Run a copy-only source producer from scripts/ against the fixture snapshot.
fn run_preparation_script(script: &str, args: &[&std::ffi::OsStr]) {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts")
        .join(script);
    let result = std::process::Command::new("python3")
        .arg(&script)
        .args(args)
        .output()
        .expect("run authenticated source preparation");
    assert!(
        result.status.success(),
        "{} failed: {}",
        script.display(),
        String::from_utf8_lossy(&result.stderr)
    );
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
    let (h, boss, map) = encounter_fixture(
        entry,
        map_id,
        bit,
        difficulty,
        catalog,
        initial_state,
        false,
    );
    (h, boss.expect("authored boss spawn"), map)
}

/// A Kings' Rest copy filled by production copy fill from the real map1762 rows.
fn kings_rest_copy(difficulty: u32) -> (Harness, InMap) {
    let (h, _, map) = encounter_fixture(
        135322,
        1762,
        0,
        difficulty,
        None,
        EncounterState::NotStarted,
        true,
    );
    (h, map)
}

fn encounter_fixture(
    entry: u32,
    map_id: u32,
    bit: u32,
    difficulty: u32,
    catalog: Option<bossmod::Catalog>,
    initial_state: EncounterState,
    populate: bool,
) -> (Harness, Option<Entity>, InMap) {
    let data = data().expect("world.db fixture");
    let mut app = stockade_app_with(data, |app| {
        app.add_systems(FixedUpdate, crate::zones::apply_addon_auras);
        // Install ingress and its source faction resource before the first update.
        app.insert_resource(
            crate::faction::world_db_faction_templates().expect("world.db faction templates"),
        );
        app.add_plugins(crate::npc_interaction::NpcInteractionPlugin);
        if populate {
            let connection =
                Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .unwrap();
            app.insert_resource(crate::gathering::load_data(&connection).unwrap());
        }
    });
    if let Some(catalog) = catalog {
        app.insert_resource(catalog);
    }
    let prepared = match entry {
        234648 => Some(prepare_encounter_database("kystia")),
        135322 => Some(prepare_encounter_database("golden-serpent")),
        ADAROGG => Some(prepare_encounter_database("adarogg")),
        SLAGMAW => Some(prepare_encounter_database("slagmaw")),
        13282 => Some(prepare_encounter_database("noxxion")),
        12225 => Some(prepare_encounter_database("celebras")),
        12201 => Some(prepare_encounter_database("theradras")),
        17306 => Some(prepare_encounter_database("gargolmar")),
        17308 => Some(prepare_encounter_database("omor")),
        186737 => Some(prepare_encounter_database("telash")),
        _ => None,
    };
    let path = prepared
        .as_ref()
        .map_or_else(world_db_path, |(_, path)| path.clone());
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    app.insert_resource(crate::conditions::load_spell_target_conditions(&path).unwrap());
    app.insert_resource(super::super::kings_rest::load_molten_gold_areas(&conn).unwrap());
    app.insert_resource(super::super::kings_rest::load_gust_slash_shape(&conn).unwrap());
    app.insert_resource(crate::spell_cast::area_trigger::load_aura_field_catalog(&conn).unwrap());
    app.insert_resource(
        super::super::ragefire_chasm::intro_triggers::load_triggers(&conn).unwrap(),
    );
    app.insert_resource(super::super::kings_rest::load_gold_model_reaches(&conn).unwrap());
    app.insert_resource(super::super::waypoint_path::load_script_paths(&conn).unwrap());
    app.insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    app.insert_resource(crate::spell_cast::teleport_positions::load_positions(&conn).unwrap());
    // Prepared copies have distinct imported authority; only canonical input is cached.
    let canonical = prepared.is_none().then(crate::test_data::world_data);
    let expected = canonical.map_or_else(
        || crate::world_data::load_expected_stats(&conn),
        |data| data.expected_stats.clone(),
    );
    let spawns: Vec<_> = if populate {
        crate::world_data::load_spawns(&conn)
            .into_iter()
            .filter(|spawn| u32::from(spawn.map) == map_id)
            .collect()
    } else {
        Vec::new()
    };
    let mut templates = canonical.map_or_else(
        || crate::world_data::load_templates(&conn, &expected),
        |data| data.templates.clone(),
    );
    templates.retain(|id, _| {
        *id == entry
            || (entry == 234648 && [234660, 255050].contains(id))
            || (entry == 135322 && [135406, 137020].contains(id))
            || (entry == ADAROGG && *id == DEMONIC_LEAP)
            || (entry == 7800 && *id == 7915)
            || (entry == 13282 && *id == 13456)
            || (entry == 12225 && [13743, 13716].contains(id))
            || (entry == 12203 && *id == 11783)
            || (entry == 12201 && *id == 12238)
            || (entry == 17306 && *id == 17309)
            || (entry == 17308 && *id == 17540)
            || (entry == 186737 && *id == 197204)
            || (map_id == 43 && [3636, 3654, 3678, 3679, 5048, 5762, 5763].contains(id))
            || spawns.iter().any(|spawn| spawn.template_entry == *id)
    });
    let difficulty_templates = if let Some(data) = canonical {
        data.difficulty_templates
            .iter()
            .filter(|((entry, _), _)| templates.contains_key(entry))
            .map(|(key, template)| (*key, template.clone()))
            .collect()
    } else {
        crate::world_data::load_template_models(&conn, &mut templates);
        crate::world_data::load_difficulty_templates(&conn, &expected, &templates)
    };
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
    let (template_string_ids, spawn_string_ids, script_spawn_groups) = if populate {
        (
            crate::world_data::load_template_string_ids(&conn).unwrap(),
            crate::world_data::load_spawn_string_ids(&conn).unwrap(),
            crate::world_data::load_script_spawn_groups(&conn, super::super::SCRIPT_SPAWN_GROUPS)
                .unwrap(),
        )
    } else {
        Default::default()
    };
    app.insert_resource(WorldData {
        templates,
        difficulty_templates,
        spawns_by_map: crate::world_data::build_spawns_by_map(&spawns),
        spawns,
        template_string_ids,
        spawn_string_ids,
        script_spawn_groups,
        // The Azure intro depends on the real non-following tender formation.
        formations: if populate && map_id == 2515 {
            crate::test_data::world_data().formations.clone()
        } else {
            Default::default()
        },
        addons: crate::world_data::load_spawn_addons(&conn),
        template_addons: crate::world_data::load_template_addons(&conn),
        class_level_base_hp: hp,
        class_level_base_mana: data.base_mana.clone(),
        expected_stats: expected,
        summon_levels: crate::world_data::summon_levels::SummonLevels::load(&conn).unwrap(),
        jump_charge_params: crate::spell_cast::jump_charge::load_params(&conn).unwrap(),
        ..Default::default()
    });
    let mut scripts = smart_ai::load_smart_scripts(&path).unwrap();
    if [3674, 3669, 3670].contains(&entry) {
        // Pin the successful branch of the authored80% chance; keep the
        // selected source timer, spell and target. Production is unchanged.
        for row in std::sync::Arc::make_mut(scripts.0.get_mut(&entry).unwrap()) {
            if row.action_type == 11 && row.chance == 80 {
                row.chance = 100;
            }
        }
    }
    // Use production's complete DB2 store, including absence of legacy SAI IDs.
    // A script's unknown spell must fail normal admission, not panic the fixture.
    let all = crate::spell_cast::test_harness::world_db().unwrap();
    let mut store = all.store.clone();
    if [17306, 17308].contains(&entry) || store.get(119297).is_none() {
        // Production's loader over the prepared copy's server-side tables.
        store.append_serverside(&conn).unwrap();
    }
    app.insert_resource(store);
    app.insert_resource(scripts);
    let mut maps = data.maps.clone();
    if entry == 234648 {
        // Mirror Image and Blink resolve their authored ground destinations.
        let wdt = maps.maps.get(&map_id).unwrap().wdt_file_data_id;
        app.insert_resource(crate::ground::GroundMap::new(
            &crate::ground::ground_data_dir(),
            std::collections::HashMap::from([(map_id, wdt)]),
        ));
    }
    maps.raid_creature_encounters.insert((map_id, entry), bit);
    if map_id == 43 {
        // Source-authenticated Wailing Normal bindings (producer profiles), fixture-only.
        for (entry, bit) in [
            (3671, 0),
            (3669, 1),
            (3653, 2),
            (3670, 3),
            (3674, 4),
            (3673, 5),
            (5775, 6),
            (3654, 7),
        ] {
            maps.raid_creature_encounters.insert((43, entry), bit);
        }
    }
    let mut instances = crate::maps::InstanceManager::default();
    // Raid copies use the entrant's raid difficulty; dungeons its dungeon difficulty.
    let entrant = crate::maps::instances::InstanceEntrant {
        character_id: 2001,
        leader: 2001,
        difficulty_id: difficulty,
        raid_difficulty: crate::maps::RaidDifficulty {
            primary: difficulty,
            ..Default::default()
        },
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
    let (boss, near) = if populate {
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, app.world());
        let content = crate::zones::MapContent {
            map,
            difficulty_id: difficulty,
            dead_bosses: Vec::new(),
        };
        crate::zones::spawn_map_creatures(
            &mut commands,
            app.world().resource::<WorldData>(),
            content,
            false,
            Some(&mut crate::test_data::ground_map()),
        );
        // Object spawns and their lock sidecar are unchanged by the template import.
        let canonical =
            Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let objects = crate::game_objects::load_game_object_spawns(&canonical).unwrap();
        crate::game_objects::spawn_map_game_objects(&mut commands, &objects, map, difficulty, None);
        queue.apply(app.world_mut());
        // 500 yards east of the Serpent's spawn, outside every aggro radius.
        (None, [-565.97, 2641.51, 796.115, std::f32::consts::PI])
    } else {
        let (boss, spawn) = spawn_authored_actor(&mut app, &conn, entry, map, difficulty);
        let near = if [
            ADAROGG, 61412, SLAGMAW, 61528, 13601, 12236, 12225, 12203, 12201, 17306, 17308,
        ]
        .contains(&entry)
            || (entry == 135322 && initial_state != EncounterState::NotStarted)
        {
            // Keep these hostile source actors idle until their fixture explicitly pulls.
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
        (Some(boss), near)
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
        &mut crate::zones::CreatureSpawnContext {
            map: map,
            difficulty_id: difficulty,
            ground: Some(&mut crate::test_data::ground_map()),
            temporary: false,
        },
        false,
        &spawn,
        &mut rand::thread_rng(),
    )
    .expect("authored actor level context");
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

#[test]
fn encounter_coverage_serpentine_seal_raises_temple_event_then_the_serpent() {
    let (mut h, map) = kings_rest_copy(23);
    let temple = |h: &mut Harness| -> Vec<Entity> {
        h.app
            .world_mut()
            .query_filtered::<(Entity, &InMap, &crate::zones::CreatureStringIds), Without<crate::combat::DeathState>>()
            .iter(h.app.world())
            .filter(|(_, other, ids)| **other == map && ids.has("TempleEvent"))
            .map(|(entity, _, _)| entity)
            .collect()
    };
    assert!(temple(&mut h).is_empty(), "manual group1255 is withheld");
    assert!(
        h.creatures_of(135322).is_empty(),
        "manual group1256 is withheld"
    );
    let seal = h
        .app
        .world_mut()
        .query::<(Entity, &shared::protocol::GameObjectInfo)>()
        .iter(h.app.world())
        .find(|(_, info)| info.entry == 288466)
        .map(|(entity, _)| entity)
        .expect("Serpentine Seal spawned in the copy");
    let pool = h
        .app
        .world_mut()
        .query::<(Entity, &shared::protocol::GameObjectInfo, &InMap)>()
        .iter(h.app.world())
        .find(|(_, info, copy)| info.entry == 289347 && **copy == map)
        .map(|(entity, _, _)| entity)
        .expect("liquid pool289347 from the copy's production spawns");
    let collision = |h: &Harness| {
        h.app
            .world()
            .get::<shared::protocol::GameObjectCollision>(pool)
            .map(|collision| collision.0)
    };
    assert_eq!(collision(&h), Some(true), "pool starts GO_STATE_READY");
    assert_eq!(
        h.app
            .world()
            .get::<crate::game_objects::GameObjectState>(pool),
        Some(&crate::game_objects::GameObjectState(1))
    );
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let sources = crate::game_objects::load_game_object_spawns(&conn).unwrap();
    let source_pool = sources
        .0
        .iter()
        .find(|spawn| spawn.info.entry == 289347)
        .unwrap()
        .clone();
    let other_pool = h
        .app
        .world_mut()
        .spawn((
            crate::game_objects::game_object_bundle(&source_pool),
            InMap {
                instance_id: map.instance_id + 1,
                ..map
            },
        ))
        .id();
    let seal_position = *h.app.world().get::<Position>(seal).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: seal_position.x + 2.0,
        ..seal_position
    });
    let use_seal = |h: &mut Harness| {
        let connection = h.players[0].1;
        h.app
            .world_mut()
            .get_mut::<MessageSender<shared::protocol::UseGameObject>>(connection)
            .unwrap()
            .send::<shared::protocol::InteractionChannel>(shared::protocol::UseGameObject {
                object: seal.to_bits(),
            });
    };
    use_seal(&mut h);
    h.advance(0.1);
    assert!(
        h.app
            .world()
            .get::<crate::game_objects::GameObjectInUse>(seal)
            .is_some(),
        "UseDoorOrButton marks the seal in use"
    );
    use_seal(&mut h);
    h.advance(2.7);
    assert!(
        h.creatures_of(137020).is_empty(),
        "Zul waits the seal's 3s task"
    );
    h.advance(0.3);
    let zuls = h.creatures_of(137020);
    assert_eq!(zuls.len(), 1, "an in-use seal cannot be used again");
    let zul = zuls[0];
    assert!(
        h.app
            .world()
            .get::<crate::zones::CreatureStringIds>(zul)
            .is_some_and(|ids| ids.has("SerpentBossEvent"))
    );
    h.advance(0.8);
    assert_eq!(collision(&h), Some(true), "Zul waits +1s before SetGoState");
    h.advance(0.3);
    assert_eq!(
        collision(&h),
        Some(false),
        "GO_STATE_ACTIVE disables collision"
    );
    assert_eq!(
        h.app
            .world()
            .get::<crate::game_objects::GameObjectState>(pool),
        Some(&crate::game_objects::GameObjectState(0))
    );
    assert_eq!(
        h.app
            .world()
            .get::<crate::game_objects::GameObjectState>(other_pool),
        Some(&crate::game_objects::GameObjectState(1)),
        "the same source pool in another copy stays READY"
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::protocol::GameObjectCollision>(other_pool),
        Some(&shared::protocol::GameObjectCollision(true))
    );
    assert!(
        h.inbox(0)
            .chat
            .iter()
            .any(|chat| chat.content.contains("G'huun")),
        "Zul's +1s Talk0"
    );
    h.advance(8.0);
    assert!(temple(&mut h).is_empty(), "temple spawns wait for +10s");
    h.advance(1.0);
    let raised = temple(&mut h);
    assert_eq!(raised.len(), 17, "SpawnGroupSpawn1255");
    assert!(h.has_aura(zul, 269058), "shadowform until +15s");
    h.advance(5.0);
    assert!(!h.has_aura(zul, 269058));
    h.advance(1.0);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::ModelDisplay>(zul)
            .map(|display| display.display_id),
        Some(11686),
        "+16s DISPLAY_INVISIBLE_ZUL, not yet unsummoned"
    );
    h.advance(1.0);
    assert!(
        h.app.world().get_entity(zul).is_err()
            || h.app
                .world()
                .get::<crate::combat::DeathState>(zul)
                .is_some(),
        "DespawnOrUnsummon(1s) after +16s"
    );
    for &creature in &raised[..16] {
        h.hit_pct(creature, 1_000.0);
    }
    h.advance(0.1);
    assert!(h.creatures_of(135322).is_empty());
    h.hit_pct(raised[16], 1_000.0);
    h.advance(0.1);
    let serpents = h.creatures_of(135322);
    assert_eq!(serpents.len(), 1);
    let serpent = serpents[0];
    assert_eq!(*h.app.world().get::<InMap>(serpent).unwrap(), map);
    assert_eq!(
        h.app.world().get::<UnitFactionTemplate>(serpent).unwrap().0,
        35,
        "spawned Serpent starts its NotStarted intro"
    );
    assert!(h.has_aura(serpent, 271598));
}

#[test]
fn encounter_coverage_zul_despawns_only_untagged_cosmetic_spirits() {
    let (mut h, map) = kings_rest_copy(23);
    let seal = h
        .app
        .world_mut()
        .query::<(Entity, &shared::protocol::GameObjectInfo)>()
        .iter(h.app.world())
        .find(|(_, info)| info.entry == 288466)
        .map(|(entity, _)| entity)
        .expect("Serpentine Seal spawned in the copy");
    // go_kings_rest_serpentine_seal's summon point.
    let zul = crate::zones::summon_creature(
        h.app.world_mut(),
        seal,
        137020,
        [-1066.0365, 2643.302, 819.37024, 5.028946],
    )
    .expect("Shadow of Zul template");
    let spirits: Vec<(Entity, bool)> = h
        .app
        .world_mut()
        .query::<(
            Entity,
            &InMap,
            &shared::components::Npc,
            Option<&crate::zones::CreatureStringIds>,
        )>()
        .iter(h.app.world())
        .filter(|(_, other, npc, _)| {
            **other == map && [137650, 137651, 137652].contains(&npc.template_id)
        })
        .map(|(entity, _, _, ids)| {
            let kept = ids.is_some_and(|ids| ids.has("DontForceRespawn"));
            (entity, kept)
        })
        .collect();
    let kept = spirits.iter().filter(|(_, kept)| *kept).count();
    assert_eq!(
        (spirits.len(), kept),
        (132, 27),
        "map1762 content spirit rows"
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), zul, 270692, None, false)
        .expect("real 270692 dest-area cast");
    h.advance(2.9);
    assert!(
        spirits.iter().all(|(spirit, _)| h
            .app
            .world()
            .get::<crate::combat::DeathState>(*spirit)
            .is_none()),
        "authored SmartAI waits 3s after the hit"
    );
    h.advance(0.3);
    for (spirit, kept) in spirits {
        assert_eq!(
            h.app
                .world()
                .get::<crate::combat::DeathState>(spirit)
                .is_none(),
            kept,
            "condition58 spares DontForceRespawn; others despawn"
        );
    }
}

/// TC kings_rest.cpp Shadow-Borne witch doctor/champion: temple_basic summon visual on
/// appearance, then their EventMap schedules from JustEngagedWith.
#[test]
fn encounter_coverage_shadow_borne_temple_trash_appear_and_cast_authored_schedules() {
    let (mut h, map) = kings_rest_copy(23);
    let raised = crate::zones::spawn_group_spawn(
        h.app.world_mut(),
        map,
        23,
        false,
        super::super::kings_rest::SPAWN_GROUP_PRE_FIRST_BOSS,
    );
    assert_eq!(raised.len(), 17);
    h.advance(0.1);
    let doctor = h.creatures_of(134174)[0];
    let champion = h.creatures_of(134158)[0];
    for unit in [doctor, champion] {
        assert!(h.has_aura(unit, 269619), "temple_basic JustAppeared visual");
    }
    let player = h.player();
    let at_doctor = *h.app.world().get::<Position>(doctor).unwrap();
    h.app.world_mut().entity_mut(player).insert((
        at_doctor,
        Health {
            current: 1_000_000_000.0,
            max: 1_000_000_000.0,
        },
    ));
    h.engage(doctor);
    let casting = |h: &Harness, unit: Entity| {
        h.app
            .world()
            .get::<CastState>(unit)
            .map(|cast| cast.spell_id)
    };
    h.advance(4.9);
    assert_eq!(casting(&h, doctor), None);
    h.advance(0.2);
    assert_eq!(
        casting(&h, doctor),
        Some(269973),
        "Deathly Chill at 5s on the victim"
    );
    // 1500ms cast; the event repeated 2.5s after it fired, at 7.5s.
    h.advance(2.3);
    assert_eq!(h.casts(0, doctor, 269973), 1);
    assert_eq!(casting(&h, doctor), None);
    h.advance(0.2);
    assert_eq!(casting(&h, doctor), Some(269973), "repeat 2.5s");
    let at_champion = *h.app.world().get::<Position>(champion).unwrap();
    h.app.world_mut().entity_mut(player).insert(at_champion);
    h.engage(champion);
    h.advance(20.9);
    assert!(!h.has_aura(champion, 269928));
    h.advance(0.3);
    assert!(h.has_aura(champion, 269928), "Vigilant Defense at 21s");
}

/// TC at_lava_guard_gordoth_intro7899: a normal player's actual DB2 box entry
/// triggers only its copy, once per instance, not on repeated polls or re-entry.
#[test]
fn encounter_coverage_gordoth_static_intro_trigger_is_box_scoped_and_once() {
    let (mut h, _, map) =
        encounter_fixture(61528, 389, 3, 1, None, EncounterState::NotStarted, true);
    h.advance(0.1);
    let boss = h.creatures_of(61528)[0];
    let original = *h.app.world().get::<Position>(boss).unwrap();
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let trigger: (f32, f32, f32, f32, f32) = conn
        .query_row(
            "SELECT Pos_0,Pos_1,Pos_2,Box_length,Box_yaw FROM area_trigger WHERE ID=7899",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap();
    let (x, y, z, length, yaw) = trigger;
    let player = h.player();
    let outside_x = x + yaw.cos() * (length / 2.0 + 0.5);
    let outside_y = y + yaw.sin() * (length / 2.0 + 0.5);
    let (px, py, pz) = crate::terrain_height::wow_to_bevy(outside_x, outside_y, z);
    h.app.world_mut().entity_mut(player).insert(Position {
        x: px,
        y: py,
        z: pz,
    });
    h.advance(0.1);
    assert_eq!(
        *h.app.world().get::<Position>(boss).unwrap(),
        original,
        "outside the rotated narrow axis"
    );
    let (px, py, pz) = crate::terrain_height::wow_to_bevy(x, y, z);
    let inside = Position {
        x: px,
        y: py,
        z: pz,
    };
    let other_player = h.players[1].0;
    h.app.world_mut().entity_mut(other_player).insert(inside);
    h.advance(0.1);
    assert_eq!(
        *h.app.world().get::<Position>(boss).unwrap(),
        original,
        "entry in an unpopulated other copy cannot use this copy's boss"
    );
    h.app.world_mut().entity_mut(player).insert(inside);
    h.advance(0.25);
    let moved = *h.app.world().get::<Position>(boss).unwrap();
    assert!(
        Vec3::new(
            moved.x - original.x,
            moved.y - original.y,
            moved.z - original.z
        )
        .length()
            > 5.0,
        "actual AT entry dispatches sourced jump, without direct DoAction"
    );
    h.advance(0.8);
    // Give a later repeated action an observable destination delta. Successful
    // OnlyOnce must retain the new position through leave/re-enter.
    let landing = *h.app.world().get::<Position>(boss).unwrap();
    let displaced = Position {
        x: landing.x + 10.0,
        ..landing
    };
    h.app.world_mut().entity_mut(boss).insert(displaced);
    h.advance(0.1);
    assert_eq!(
        *h.app.world().get::<Position>(boss).unwrap(),
        displaced,
        "no repeat while inside"
    );
    h.app.world_mut().entity_mut(player).insert(Position {
        x: px + 100.0,
        ..inside
    });
    h.advance(0.1);
    h.app.world_mut().entity_mut(player).insert(inside);
    h.advance(0.25);
    assert_eq!(
        *h.app.world().get::<Position>(boss).unwrap(),
        displaced,
        "instance OnlyOnce survives re-entry"
    );
    assert_eq!(*h.app.world().get::<InMap>(boss).unwrap(), map);
    // The failed other-copy entry must not consume OnlyOnce: create that real
    // copy and its authored boss, then leave/re-enter to retry successfully.
    let maps = h.app.world().resource::<crate::maps::MapStore>().clone();
    let other_map = h
        .app
        .world_mut()
        .resource_mut::<crate::maps::InstanceManager>()
        .create_map(
            &maps,
            389,
            crate::maps::instances::InstanceEntrant {
                character_id: 2002,
                leader: 2002,
                difficulty_id: 1,
                ..Default::default()
            },
            crate::maps::instances::Now {
                elapsed: 0.0,
                unix: 0,
            },
        )
        .unwrap()
        .map;
    assert_eq!(
        *h.app.world().get::<InMap>(other_player).unwrap(),
        other_map
    );
    let (other_boss, _) = spawn_authored_actor(&mut h.app, &conn, 61528, other_map, 1);
    h.app.world_mut().entity_mut(other_player).insert(Position {
        x: px + 500.0,
        ..inside
    });
    h.advance(0.1);
    let other_original = *h.app.world().get::<Position>(other_boss).unwrap();
    h.app.world_mut().entity_mut(other_player).insert(inside);
    h.advance(0.25);
    assert_ne!(
        *h.app.world().get::<Position>(other_boss).unwrap(),
        other_original,
        "missing-boss retry succeeds and first copy's OnlyOnce cannot consume another copy"
    );
    assert_eq!(
        *h.app.world().get::<Position>(boss).unwrap(),
        displaced,
        "second copy jump leaves first copy unchanged"
    );
}

/// Canonical binds only Koranthal on map389: a populated copy initializes the
/// unbound Gordoth script without a boss state, so it stays at its spawn.
#[test]
fn encounter_coverage_gordoth_unbound_copy_initializes_at_spawn() {
    let (mut h, _, _) = encounter_fixture(61412, 389, 1, 1, None, EncounterState::NotStarted, true);
    h.advance(0.1);
    let gordoth = h.creatures_of(61528);
    assert_eq!(gordoth.len(), 1);
    let at = h.wow_position(gordoth[0]);
    assert!(
        at.distance(Vec3::new(-369.501, 166.212, -18.5202)) < 0.01,
        "authored spawn, no started relocation: {at:?}"
    );
}

/// Source DoAction(ACTION_JUMP): MoveJump50 speed/min-height4, then effect-motion
/// landing sets the vials DESTROYED and the nearby researchers cast Bloody Suicide.
/// The pinned UpdateAI does not pump its separately scheduled cage task.
#[test]
fn encounter_coverage_gordoth_source_jump_lands_breaks_vials_and_researchers_die() {
    let (mut h, _, map) =
        encounter_fixture(61528, 389, 3, 1, None, EncounterState::NotStarted, true);
    h.advance(0.1);
    let boss = h.creatures_of(61528)[0];
    let from = *h.app.world().get::<Position>(boss).unwrap();
    let origin = Vec3::new(from.x, from.y, from.z);
    let landing = Vec3::new(-363.5392, -22.005634, -203.36604);
    let vials: Vec<_> = h
        .app
        .world_mut()
        .query::<(Entity, &shared::protocol::GameObjectInfo, &InMap)>()
        .iter(h.app.world())
        .filter(|(_, info, copy)| info.entry == 211784 && **copy == map)
        .map(|(entity, _, _)| entity)
        .collect();
    assert_eq!(vials.len(), 3);
    let researchers = h.creatures_of(61644);
    assert_eq!(researchers.len(), 7);
    let ready = |h: &Harness, object| {
        h.app
            .world()
            .get::<crate::game_objects::GameObjectState>(object)
            .is_some_and(|state| state.0 == 1)
    };
    assert!(vials.iter().all(|&vial| ready(&h, vial)));
    dispatch(h.app.world_mut(), boss, Hook::DoAction(1));
    h.advance(0.35);
    let at = *h.app.world().get::<Position>(boss).unwrap();
    let duration_ms = 1 + (origin.distance(landing) * 1000.0 / 50.0) as u32;
    let t = 350.0 / duration_ms as f32;
    let mut expected = origin.lerp(landing, t);
    expected.y += 4.0 * 4.0 * t * (1.0 - t);
    assert!(
        Vec3::new(at.x, at.y, at.z).distance(expected) < 0.001,
        "authentic linear spline plus parabolic height, not relocation: {at:?}"
    );
    assert!(
        vials.iter().all(|&vial| ready(&h, vial)),
        "not destroyed before arrival"
    );
    assert!(researchers.iter().all(|&researcher| {
        h.app
            .world()
            .get::<crate::combat::DeathState>(researcher)
            .is_none()
    }));
    h.advance(0.55);
    let at = *h.app.world().get::<Position>(boss).unwrap();
    assert!(Vec3::new(at.x, at.y, at.z).distance(landing) < 0.001);
    let orientation = h
        .app
        .world()
        .get::<shared::components::Rotation>(boss)
        .unwrap()
        .y
        - std::f32::consts::FRAC_PI_2;
    assert!(
        (orientation - 0.30812347).abs() < 0.001,
        "source facing after landing"
    );
    for vial in vials {
        assert_eq!(
            h.app
                .world()
                .get::<crate::game_objects::GameObjectState>(vial),
            Some(&crate::game_objects::GameObjectState(2))
        );
        assert_eq!(
            h.app
                .world()
                .get::<shared::protocol::GameObjectCollision>(vial),
            Some(&shared::protocol::GameObjectCollision(false))
        );
    }
    for researcher in researchers {
        assert!(
            h.app
                .world()
                .get::<crate::combat::DeathState>(researcher)
                .is_some(),
            "real120022 extra initial periodic tick ->51744 INSTAKILL"
        );
        assert_eq!(
            h.app
                .world()
                .get::<crate::combat::LastDamager>(researcher)
                .map(|killer| killer.0),
            Some(researcher)
        );
    }
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 3),
        Some(EncounterState::NotStarted),
        "researcher self-deaths never grant Gordoth credit"
    );
    let original_spawn = h
        .app
        .world()
        .get::<crate::creature_ai::SpawnPoint>(boss)
        .unwrap()
        .clone();
    // Exercise HomeMovementGenerator after source SetHomePosition, not a field or
    // controller-selection assertion. Original respawn coordinates must survive.
    h.app
        .world_mut()
        .entity_mut(boss)
        .insert(crate::creature_ai::CreatureState::Returning);
    h.advance(0.2);
    let returned = *h.app.world().get::<Position>(boss).unwrap();
    assert!(
        Vec3::new(returned.x, returned.y, returned.z).distance(landing) < 0.001,
        "source home is landing point, not the original cage spawn"
    );
    assert!(
        (h.app
            .world()
            .get::<shared::components::Rotation>(boss)
            .unwrap()
            .y
            - std::f32::consts::FRAC_PI_2
            - 0.30812347)
            .abs()
            < 0.001
    );
    let spawn = h
        .app
        .world()
        .get::<crate::creature_ai::SpawnPoint>(boss)
        .unwrap();
    assert_eq!(
        (spawn.x, spawn.y, spawn.z),
        (original_spawn.x, original_spawn.y, original_spawn.z)
    );
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    h.advance(30.2);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(boss)
            .is_none()
    );
    let respawned = *h.app.world().get::<Position>(boss).unwrap();
    let spawn_point = Vec3::new(original_spawn.x, original_spawn.y, original_spawn.z);
    h.app
        .world_mut()
        .entity_mut(boss)
        .insert(crate::creature_ai::CreatureState::Returning);
    h.advance(0.2);
    let returning = *h.app.world().get::<Position>(boss).unwrap();
    assert!(
        Vec3::new(returning.x, returning.y, returning.z).distance(spawn_point)
            < Vec3::new(respawned.x, respawned.y, respawned.z).distance(spawn_point),
        "respawn resets home to spawn before source InitializeAI relocates the actor"
    );
}

/// A real Mortimer POINT1 callback schedules self-death7s later. A competing
/// point must not arrive while the highest-priority source jump owns movement.
#[test]
fn encounter_coverage_jump_suspends_lower_point_until_after_landing() {
    let (mut h, parent) = Harness::new(NPC_RANDOLPH_MOLOCH, &[]).unwrap();
    let player = h.player();
    let at_parent = *h.app.world().get::<Position>(parent).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: at_parent.x + 500.0,
        ..at_parent
    });
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), parent);
    let at = h.wow_position(parent);
    let observer =
        crate::zones::summon_creature(h.app.world_mut(), parent, 46482, [at.x, at.y, at.z, 0.0])
            .unwrap();
    let from = *h.app.world().get::<Position>(observer).unwrap();
    super::super::movement::set_walk(h.app.world_mut(), observer, false);
    super::super::movement::move_point(
        h.app.world_mut(),
        observer,
        1,
        Position {
            x: from.x + 0.1,
            ..from
        },
    );
    super::super::jump_movement::move_jump(
        h.app.world_mut(),
        observer,
        7,
        Position {
            x: from.x + 10.0,
            ..from
        },
        1.0,
        4.0,
    );
    h.advance(7.5);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(observer)
            .is_none(),
        "lower POINT1 never arrived and scheduled Mortimer death during the jump"
    );
    assert!(h.health(observer).current > 0.0);
    h.advance(2.6);
    let run_speed = h
        .app
        .world()
        .get::<crate::creature_ai::SpawnPoint>(observer)
        .unwrap()
        .speed_run
        * 7.0;
    h.advance(10.0 / run_speed + 6.5);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(observer)
            .is_none(),
        "after resuming the source point, its7s death callback has not yet fired"
    );
    h.advance(1.0);
    assert!(
        h.app.world().get_entity(observer).is_err()
            || h.app
                .world()
                .get::<crate::combat::DeathState>(observer)
                .is_some(),
        "lower point resumes, reaches its target and executes the real callback"
    );
}

#[test]
fn encounter_coverage_gordoth_death_interrupts_jump_without_landing_effects() {
    let (mut h, _, map) =
        encounter_fixture(61528, 389, 3, 1, None, EncounterState::NotStarted, true);
    h.advance(0.1);
    let boss = h.creatures_of(61528)[0];
    dispatch(h.app.world_mut(), boss, Hook::DoAction(1));
    h.advance(0.2);
    h.hit_pct(boss, 1_000.0);
    h.advance(1.1);
    let vials: Vec<_> = h
        .app
        .world_mut()
        .query::<(Entity, &shared::protocol::GameObjectInfo, &InMap)>()
        .iter(h.app.world())
        .filter(|(_, info, copy)| info.entry == 211784 && **copy == map)
        .map(|(entity, _, _)| entity)
        .collect();
    assert_eq!(vials.len(), 3);
    for vial in vials {
        assert_eq!(
            h.app
                .world()
                .get::<crate::game_objects::GameObjectState>(vial),
            Some(&crate::game_objects::GameObjectState(1))
        );
    }
    for researcher in h.creatures_of(61644) {
        assert!(
            h.app
                .world()
                .get::<crate::combat::DeathState>(researcher)
                .is_none()
        );
    }
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 3),
        Some(EncounterState::Done)
    );
}

/// TC Gordoth's already-started InitializeAI branch relocates to the source jump
/// landing point, without inventing an intro bypass.
#[test]
fn encounter_coverage_gordoth_relocates_started_actor_and_casts_source_schedule() {
    let (mut h, boss, _) =
        encounter_fixture(61528, 389, 3, 1, None, EncounterState::InProgress, false);
    let boss = boss.unwrap();
    assert!(
        h.wow_position(boss)
            .distance(Vec3::new(-363.5392, 203.36604, -22.005634))
            < 0.001
    );
    let player = h.player();
    let position = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: position.x + 3.0,
        ..position
    });
    h.engage(boss);
    let casting = |h: &Harness| {
        h.app
            .world()
            .get::<CastState>(boss)
            .map(|cast| cast.spell_id)
    };
    h.advance(7.2);
    assert_eq!(casting(&h), None);
    h.advance(0.2);
    assert_eq!(
        casting(&h),
        Some(119999),
        "Ground Rupture at7.3s, real1s cast"
    );
    h.advance(1.1);
    assert_eq!(h.casts(0, boss, 119999), 1);
    // Restore the victim after authentic knockback; category bypass is covered
    // separately through actual native cast/cooldown history.
    h.app.world_mut().entity_mut(player).insert(Position {
        x: position.x + 3.0,
        ..position
    });
    h.advance(4.7);
    assert!(
        !h.inbox(0)
            .log
            .iter()
            .any(|event| event.spell_id == Some(120024)),
        "Seismic waits13.3s"
    );
    h.advance(0.2);
    assert!(
        h.inbox(0).log.iter().any(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(120024)
                && event.source == Some(boss.to_bits())
                && event.target == Some(player.to_bits())
        }),
        "Seismic13.3s source cast performs normal damage"
    );
}

#[test]
fn encounter_coverage_gordoth_enrages_once_below31_and_evade_resets_it() {
    let (mut h, boss, map) =
        encounter_fixture(61528, 389, 3, 1, None, EncounterState::InProgress, false);
    let boss = boss.unwrap();
    h.set_health_pct(boss, 36.0);
    h.hit_pct(boss, 1.0);
    assert!(!h.has_aura(boss, 50420), "still above31 percent");
    h.hit_pct(boss, 6.0);
    assert!(h.health(boss).current < h.health(boss).max * 0.31);
    assert!(
        h.has_aura(boss, 50420),
        "projected loss crossed source31 percent"
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        boss,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 50420,
    );
    h.hit_pct(boss, 1.0);
    assert!(!h.has_aura(boss, 50420), "source one-shot _enrageTriggered");
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 3),
        Some(EncounterState::Fail)
    );
    h.advance(30.2);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(boss)
            .is_none()
    );
    h.set_health_pct(boss, 32.0);
    h.hit_pct(boss, 3.0);
    assert!(
        h.has_aura(boss, 50420),
        "fresh source AI after30s respawn resets enrage"
    );
}

/// TC boss_dark_shaman_koranthal: appearance Vortex, engage Talk/remove Vortex,
/// Twisted6s/7.3s, Storm20.5s/47.2s; Storm reschedules Twisted to +15.7s.
#[test]
fn encounter_coverage_koranthal_uses_source_combat_schedule_and_storm_damage() {
    let (mut h, boss, map) =
        encounter_fixture(61412, 389, 1, 1, None, EncounterState::NotStarted, false);
    let boss = boss.unwrap();
    h.advance(0.1);
    assert!(h.has_aura(boss, 119928), "Shadow Vortex on appearance");
    let player = h.player();
    let position = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: position.x + 3.0,
        ..position
    });
    h.engage(boss);
    h.advance(0.1);
    assert!(!h.has_aura(boss, 119928));
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 1),
        Some(EncounterState::InProgress)
    );
    assert!(
        h.inbox(0)
            .chat
            .iter()
            .any(|chat| chat.content.contains("overwhelm"))
    );
    let casting = |h: &Harness| {
        h.app
            .world()
            .get::<CastState>(boss)
            .map(|cast| cast.spell_id)
    };
    h.advance(5.8);
    assert_eq!(casting(&h), None, "Twisted waits6s");
    h.advance(0.2);
    assert_eq!(casting(&h), Some(119300));
    h.advance(2.1);
    assert_eq!(
        h.casts(0, boss, 119300),
        1,
        "real2s cast completes on victim"
    );
    h.advance(5.0);
    assert_eq!(casting(&h), None);
    h.advance(0.2);
    assert_eq!(casting(&h), Some(119300), "repeat at13.3s");
    h.advance(7.0);
    assert!(!h.has_aura(boss, 119971), "Storm waits20.5s");
    h.advance(0.2);
    assert!(h.has_aura(boss, 119971));
    assert!(
        h.inbox(0)
            .chat
            .iter()
            .any(|chat| chat.content.contains("Shadow Storm"))
    );
    h.advance(3.2);
    assert_eq!(
        koranthal_storm_damage_count(&h, boss, player),
        0,
        "first3s periodic tick launches the missile; its authored5s flight has not landed"
    );
    let before = h.health(player).current;
    h.advance(5.0);
    assert!(
        koranthal_storm_damage_count(&h, boss, player) > 0,
        "119973 dummy ->119965 five-second missile -> actual119964 damage"
    );
    assert!(
        h.health(player).current < before,
        "actual storm health loss"
    );
    assert_eq!(
        h.casts(0, boss, 119300),
        2,
        "Storm reschedules, no Twisted at20.6s"
    );
    // The extra5s impact observation must not shift the36.2/67.7 source-clock guards.
    h.advance(7.3);
    assert_eq!(casting(&h), None);
    h.advance(0.2);
    assert_eq!(casting(&h), Some(119300), "rescheduled to36.2s");
    h.advance(31.3);
    assert!(!h.has_aura(boss, 119971), "next Storm waits until67.7s");
    h.advance(0.2);
    assert!(h.has_aura(boss, 119971), "Storm repeats47.2s after20.5s");
}

/// TC119973 effect0 dummy reads its effect value119965 and casts that triggered;
/// the actual missile chain hits with119964, independently of119971's other effect.
#[test]
fn encounter_coverage_koranthal_shadow_storm_dummy_launches_authored_damage() {
    let (mut h, boss, _) =
        encounter_fixture(61412, 389, 1, 1, None, EncounterState::NotStarted, false);
    let boss = boss.unwrap();
    let target = h.player();
    let position = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(target).insert(Position {
        x: position.x + 3.0,
        ..position
    });
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, 119973, None, true)
        .expect("real119973 source-area-enemy dummy");
    // Native119965 Speed5/Attr9 speed-as-delay; TC Spell.cpp:880–896 gives5000ms.
    h.advance(4.95);
    assert_eq!(
        koranthal_storm_damage_count(&h, boss, target),
        0,
        "no119964 damage before the authored5s destination flight"
    );
    let before = h.health(target).current;
    h.advance(0.1);
    assert_eq!(
        koranthal_storm_damage_count(&h, boss, target),
        1,
        "one actual119964 hit at5s, within one50ms fixture frame"
    );
    assert!(
        h.health(target).current < before,
        "actual child health loss, not unrelated earlier melee damage"
    );
}

/// Recipient-owned positive119964 hits, not generic HP loss from melee or other spells.
fn koranthal_storm_damage_count(h: &Harness, boss: Entity, target: Entity) -> usize {
    h.inbox(0)
        .log
        .iter()
        .filter(|event| {
            let authored_hit =
                event.kind == CombatLogKind::Damage && event.spell_id == Some(119964);
            let owned_hit =
                event.source == Some(boss.to_bits()) && event.target == Some(target.to_bits());
            authored_hit && owned_hit && event.amount > 0
        })
        .count()
}

#[test]
fn encounter_coverage_koranthal_death_and_evade_use_source_lifecycle() {
    for killed in [false, true] {
        let (mut h, boss, map) =
            encounter_fixture(61412, 389, 1, 1, None, EncounterState::NotStarted, false);
        let boss = boss.unwrap();
        let player = h.player();
        let position = *h.app.world().get::<Position>(boss).unwrap();
        h.app.world_mut().entity_mut(player).insert(Position {
            x: position.x + 3.0,
            ..position
        });
        h.engage(boss);
        h.advance(0.1);
        if killed {
            h.hit_pct(boss, 1_000.0);
            h.advance(0.1);
            assert_eq!(
                get_boss_state(h.app.world_mut(), map, 1),
                Some(EncounterState::Done)
            );
            assert!(
                h.inbox(0)
                    .chat
                    .iter()
                    .any(|chat| chat.content.contains("My death"))
            );
        } else {
            crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
            h.advance(0.1);
            assert_eq!(
                get_boss_state(h.app.world_mut(), map, 1),
                Some(EncounterState::Fail)
            );
            assert!(
                h.app
                    .world()
                    .get::<crate::combat::DeathState>(boss)
                    .is_some(),
                "_DespawnAtEvade retires actor without a kill"
            );
            assert!(h.inbox(0).ends.iter().all(|end| !end.success));
            h.app.world_mut().entity_mut(player).insert(Position {
                x: position.x + 500.0,
                ..position
            });
            h.advance(30.1);
            assert!(
                h.app
                    .world()
                    .get::<crate::combat::DeathState>(boss)
                    .is_none(),
                "authored30s respawn"
            );
            assert!(h.has_aura(boss, 119928), "reappearance Shadow Vortex");
        }
    }
}

/// Actual Cobrahn SAI: first Lightning Bolt20805 is due immediately, repeats6–8s.
/// Pin only its80% chance successful branch; all timings/targets/spell effects are real.
/// Generic BossAi completes encounter586; this does NOT prove SET_INST_DATA34 support.
#[test]
fn encounter_coverage_cobrahn_lightning_damage_repeat_and_generic_lifecycle() {
    let (mut h, boss, map) = bound_boss_at_difficulty(3669, 43, 1, 1, None);
    let player = h.player();
    let position = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: position.x + 1.0,
        ..position
    });
    h.engage(boss);
    h.advance(0.1);
    assert_eq!(casting(&h, boss), Some(20805));
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 0),
        Some(EncounterState::InProgress)
    );
    let before = h.health(player).current;
    // Three-second cast plus max(distance,5)/24 missile flight and fixed-tick margin.
    h.advance(3.4);
    assert_eq!(h.casts(0, boss, 20805), 1);
    assert!(h.health(player).current < before);
    assert!(
        h.inbox(0).log.iter().any(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(20805)
                && event.source == Some(boss.to_bits())
                && event.target == Some(player.to_bits())
        }),
        "combat events: {:?}; player {:?}; caster {:?}",
        h.inbox(0).log,
        h.health(player),
        h.app.world().get::<shared::components::Position>(boss)
    );
    h.advance(2.2);
    assert_eq!(casting(&h, boss), None, "no repeat before6s");
    h.advance(2.6);
    assert_eq!(casting(&h, boss), Some(20805), "repeat selected within6–8s");
    h.hit_pct(boss, 1_000.0);
    h.advance(0.1);
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 0),
        Some(EncounterState::Done)
    );
    assert!(
        h.inbox(0)
            .ends
            .iter()
            .any(|end| end.encounter_id == 586 && end.success)
    );
    assert!(h.inbox(0).ends.iter().all(|end| end.encounter_id != 585));
}

/// Actual Pythas SmartAI: immediate20805/repeat6–8s (successful80% branch),
/// Thunderclap8147 first6–8s/repeat12–30s; casting defers its due event.
#[test]
fn encounter_coverage_pythas_real_lightning_thunderclap_and_source_death_target() {
    let (mut h, boss, map) = bound_boss_at_difficulty(3670, 43, 3, 1, None);
    let player = h.player();
    let position = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: position.x + 1.0,
        ..position
    });
    h.engage(boss);
    h.advance(0.1);
    assert_eq!(casting(&h, boss), Some(20805));
    h.advance(5.6);
    assert_eq!(h.casts(0, boss, 8147), 0, "Thunderclap waits at least6s");
    h.advance(6.0);
    assert!(h.casts(0, boss, 20805) >= 1);
    assert_eq!(h.casts(0, boss, 8147), 1);
    assert!(h.inbox(0).log.iter().any(|event| {
        event.kind == CombatLogKind::Damage
            && event.spell_id == Some(8147)
            && event.source == Some(boss.to_bits())
            && event.target == Some(player.to_bits())
    }));
    h.advance(5.0);
    assert_eq!(
        h.casts(0, boss, 8147),
        1,
        "repeat waits at least12s after first attempt"
    );
    // First attempt by11s + repeat at most30s + a3s Lightning cast deferral.
    h.advance(28.0);
    assert!(
        h.casts(0, boss, 8147) >= 2,
        "source repeat actually executes"
    );
    h.hit_pct(boss, 1_000.0);
    h.advance(0.1);
    let pythas = instance_script::boss_id_for_encounter(h.app.world_mut(), map, 588).unwrap();
    let anacondra = instance_script::boss_id_for_encounter(h.app.world_mut(), map, 585).unwrap();
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, pythas),
        Some(EncounterState::Done)
    );
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, anacondra),
        Some(EncounterState::NotStarted)
    );
    assert!(
        h.inbox(0)
            .ends
            .iter()
            .any(|end| end.encounter_id == 588 && end.success)
    );
    assert!(h.inbox(0).ends.iter().all(|end| end.encounter_id != 585));
}

/// Actual Skum6254 row: first3–4s/repeat3–4s, successful80% branch only pinned.
#[test]
fn encounter_coverage_skum_real_chain_damage_and_repeat() {
    let (mut h, boss, map) = bound_boss_at_difficulty(3674, 43, 4, 1, None);
    let player = h.player();
    let position = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: position.x + 1.0,
        ..position
    });
    h.engage(boss);
    h.advance(2.7);
    assert_ne!(casting(&h, boss), Some(6254), "first event waits3s");
    assert_eq!(h.casts(0, boss, 6254), 0);
    h.advance(3.3);
    assert_eq!(h.casts(0, boss, 6254), 1, "first4s maximum plus1.8s cast");
    assert!(h.inbox(0).log.iter().any(|event| {
        event.kind == CombatLogKind::Damage
            && event.spell_id == Some(6254)
            && event.source == Some(boss.to_bits())
            && event.target == Some(player.to_bits())
    }));
    h.advance(4.5);
    assert!(h.casts(0, boss, 6254) >= 2);
    h.hit_pct(boss, 1_000.0);
    h.advance(0.1);
    let skum = instance_script::boss_id_for_encounter(h.app.world_mut(), map, 589).unwrap();
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, skum),
        Some(EncounterState::Done)
    );
    assert!(
        h.inbox(0)
            .ends
            .iter()
            .any(|end| end.encounter_id == 589 && end.success)
    );
}

/// Verdan's sourced8142 first20–35s applies actual5s MOD_ROOT26, not merely a cast log.
#[test]
fn encounter_coverage_verdan_real_vines_root_and_expire() {
    let (mut h, boss, map) = bound_boss_at_difficulty(5775, 43, 6, 1, None);
    let player = h.player();
    let position = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: position.x + 1.0,
        ..position
    });
    h.engage(boss);
    h.advance(19.5);
    assert_eq!(h.casts(0, boss, 8142), 0, "first event waits20s");
    let mut waited = 0.0;
    while !h.has_aura(player, 8142) {
        assert!(waited < 18.0, "first35s maximum plus1.5s cast");
        h.advance(0.1);
        waited += 0.1;
    }
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(player)
            .unwrap()
            .movement_speed,
        0.0
    );
    assert!(h.inbox(0).log.iter().any(|event| {
        event.kind == CombatLogKind::Damage
            && event.spell_id == Some(8142)
            && event.source == Some(boss.to_bits())
            && event.target == Some(player.to_bits())
    }));
    h.advance(5.5);
    assert!(
        !h.has_aura(player, 8142),
        "source5s root expires before20s repeat"
    );
    assert!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(player)
            .unwrap()
            .movement_speed
            > 0.0
    );
    h.hit_pct(boss, 1_000.0);
    h.advance(0.1);
    let verdan = instance_script::boss_id_for_encounter(h.app.world_mut(), map, 591).unwrap();
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, verdan),
        Some(EncounterState::Done)
    );
    assert!(
        h.inbox(0)
            .ends
            .iter()
            .any(|end| end.encounter_id == 591 && end.success)
    );
}

/// Actual Anacondra row7 casts8148 at7s and repeats every63s. This proves
/// aura application/reapplication only, not DAMAGE_SHIELD15 retaliation or missingSpell2.
#[test]
fn encounter_coverage_anacondra_thorns_application_and_source_repeat() {
    let (mut h, boss, _) = bound_boss_at_difficulty(3671, 43, 0, 1, None);
    h.engage(boss);
    h.advance(6.6);
    assert!(!h.has_aura(boss, 8148), "Thorns waits7s");
    h.advance(0.7);
    assert!(h.has_aura(boss, 8148), "actual self/party aura");
    let removed = crate::auras::remove_auras(
        h.app.world_mut(),
        boss,
        crate::auras::RemoveReason::Dispelled,
        |aura| aura.spell_id == 8148,
    );
    assert_eq!(removed.len(), 1);
    h.advance(62.0);
    assert!(!h.has_aura(boss, 8148), "no repeat before63s");
    h.advance(1.5);
    assert!(
        h.has_aura(boss, 8148),
        "source repeat reapplies removed buff"
    );
}

/// Thorns must retaliate against damaging melee swings, not arbitrary spells.
#[test]
fn encounter_coverage_anacondra_thorns_retaliates_only_while_applied() {
    let (mut h, boss, _) = bound_boss_at_difficulty(3671, 43, 0, 1, None);
    let player = h.player();
    let position = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: position.x + 1.0,
        ..position
    });
    h.engage(boss);
    h.advance(7.3);
    assert!(h.has_aura(boss, 8148));
    h.app
        .world_mut()
        .entity_mut(boss)
        .insert(ReactState::Passive);
    h.advance(0.1);
    h.hit(boss, 1.0);
    h.advance(0.1);
    let retaliations = |h: &Harness| {
        h.inbox(0)
            .log
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(8148)
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
            })
            .count()
    };
    assert_eq!(
        retaliations(&h),
        0,
        "spell damage does not trigger melee shield"
    );
    let health = h.health(player).current;
    let boss_health = h.health(boss).current;
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(crate::combat::AutoAttack::new(boss));
    h.advance(8.5);
    assert!(
        h.health(boss).current < boss_health,
        "real melee swings damaged shield owner"
    );
    assert!(
        retaliations(&h) > 0,
        "actual Thorns Nature damage reaches attacker"
    );
    assert!(h.health(player).current < health);
    let before_removal = retaliations(&h);
    crate::auras::remove_auras(
        h.app.world_mut(),
        boss,
        crate::auras::RemoveReason::Dispelled,
        |aura| aura.spell_id == 8148,
    );
    h.advance(8.5);
    assert_eq!(
        retaliations(&h),
        before_removal,
        "removed Thorns cannot retaliate"
    );
    // Controlled native reapplication isolates Unit::Kill's death-aura boundary,
    // not a new AI cast schedule. Existing auto-attacks deliver the lethal swing.
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, 8148, Some(boss), true).unwrap();
    assert!(h.has_aura(boss, 8148));
    h.app.world_mut().get_mut::<Health>(boss).unwrap().current = 1.0;
    h.advance(8.5);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(boss)
            .is_some()
    );
    assert_eq!(
        retaliations(&h),
        before_removal,
        "death-removed Thorns cannot retaliate after lethal melee"
    );
}

/// Real deaths/evades use each bound encounter, never another Wailing boss.
/// Generic BossAi lifecycle is not proof of source action34. Mutanus has no
/// content_creature placement and must instead use its source summon group.
#[test]
fn encounter_coverage_wailing_kresh_serpentis_generic_lifecycle() {
    for (entry, bit, encounter) in [(3653, 2, 587), (3673, 5, 590)] {
        for killed in [false, true] {
            let (mut h, boss, map) = bound_boss_at_difficulty(entry, 43, bit, 1, None);
            h.engage(boss);
            h.advance(0.1);
            let slot =
                instance_script::boss_id_for_encounter(h.app.world_mut(), map, encounter).unwrap();
            assert_eq!(
                get_boss_state(h.app.world_mut(), map, slot),
                Some(EncounterState::InProgress)
            );
            if killed {
                h.hit_pct(boss, 1_000.0);
            } else {
                crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
            }
            h.advance(0.1);
            // Generic BossAi::_Reset returns to NOT_STARTED; no invented FAIL state.
            let expected = if killed {
                EncounterState::Done
            } else {
                EncounterState::NotStarted
            };
            assert_eq!(get_boss_state(h.app.world_mut(), map, slot), Some(expected));
            assert!(
                h.inbox(0)
                    .ends
                    .iter()
                    .any(|end| end.encounter_id == encounter && end.success == killed)
            );
            assert!(
                h.inbox(0)
                    .ends
                    .iter()
                    .all(|end| end.encounter_id == encounter)
            );
            assert!(
                h.inbox(1).ends.is_empty(),
                "other instance receives no completion"
            );
        }
    }
}

/// The source Disciple combat clock is independent of the unavailable escort activation.
/// First Sleep1090 is due5–10s; Druid's Potion8141 only below80% health.
#[test]
fn encounter_coverage_wailing_disciple_sleep_and_conditional_potion() {
    let (mut h, enemy, map) = bound_boss_at_difficulty(3669, 43, 1, 1, None);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (disciple, _) = spawn_authored_actor(&mut h.app, &conn, 3678, map, 1);
    let position = *h.app.world().get::<Position>(disciple).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(position);
    // Source EVENT_PROGRESS_1 sets FACTION_ESCORTEE_N_NEUTRAL_ACTIVE250.
    // Base faction35 cannot attack players. Activation/gossip is not simulated.
    h.app
        .world_mut()
        .entity_mut(disciple)
        .insert(UnitFactionTemplate(250));
    h.app.world_mut().entity_mut(enemy).insert((
        Position {
            x: position.x + 10.0,
            ..position
        },
        ReactState::Passive,
    ));
    h.advance(0.1);
    let threshold = h.health(disciple).max * 0.8;
    h.app
        .world_mut()
        .get_mut::<Health>(disciple)
        .unwrap()
        .current = threshold;
    crate::creature_aggro::add_threat(h.app.world_mut(), disciple, enemy, 1.0);
    h.advance(4.7);
    assert_ne!(casting(&h, disciple), Some(1090), "Sleep waits at least5s");
    assert!(!h.has_aura(enemy, 1090));
    let mut waited = 0.0;
    while !h.has_aura(enemy, 1090) {
        assert!(waited < 7.5, "actual Sleep aura after10s plus1.5s cast");
        h.advance(0.05);
        waited += 0.05;
    }
    h.advance(6.0);
    assert_eq!(
        h.health(disciple).current,
        threshold,
        "exactly80% forbids potion"
    );
    h.set_health_pct(disciple, 75.0);
    let wounded = h.health(disciple).current;
    let mut healed = false;
    let mut repeated_sleep = false;
    for _ in 0..640 {
        h.advance(0.05);
        healed |= h.health(disciple).current > wounded;
        repeated_sleep |= casting(&h, disciple) == Some(1090);
    }
    assert!(
        healed,
        "real potion healing, including cast-collision retry"
    );
    assert!(
        repeated_sleep,
        "20–30s Sleep repeat starts another actual cast"
    );
}

/// Source EVENT_PROGRESS_20 summons3678/group4 (Mutanus), not a fabricated
/// fixed placement. Stage the source pre-wave endpoint/faction; no escort-route proof.
#[test]
fn encounter_coverage_wailing_mutanus_authored_group_target_credit_and_corpse_lifetime() {
    let (mut h, unrelated, map) = bound_boss_at_difficulty(3669, 43, 1, 1, None);
    h.app
        .world_mut()
        .entity_mut(unrelated)
        .insert(ReactState::Passive);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (disciple, _) = spawn_authored_actor(&mut h.app, &conn, 3678, map, 1);
    let endpoint: (f32, f32, f32) = conn.query_row(
        "SELECT PositionX,PositionY,PositionZ FROM tdb_waypoint_path_node WHERE PathId=367803 ORDER BY NodeId DESC LIMIT 1",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).unwrap();
    let (x, y, z) = crate::terrain_height::wow_to_bevy(endpoint.0, endpoint.1, endpoint.2);
    h.app.world_mut().entity_mut(disciple).insert((
        Position { x, y, z },
        UnitFactionTemplate(250),
        ReactState::Passive,
        Health {
            current: 1_000_000_000.0,
            max: 1_000_000_000.0,
        },
    ));
    let player = h.player();
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(Position { x: x + 1.0, y, z });
    h.advance(0.1);
    let summons = Ctx {
        world: h.app.world_mut(),
        me: disciple,
    }
    .summon_creature_group(4)
    .expect("authored Disciple group4");
    assert_eq!(summons.len(), 1);
    let mutanus = summons[0];
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Npc>(mutanus)
            .unwrap()
            .template_id,
        3654
    );
    assert_eq!(h.app.world().get::<InMap>(mutanus), Some(&map));
    assert_eq!(
        h.app
            .world()
            .get::<crate::zones::TempSummon>(mutanus)
            .unwrap()
            .summoner,
        disciple
    );
    let position = h.wow_position(mutanus);
    assert!((position.x - 150.943).abs() < 0.001);
    assert!((position.y - 262.797).abs() < 0.001);
    assert!((position.z + 103.904).abs() < 0.001);
    h.advance(0.1);
    assert!(
        matches!(h.app.world().get::<crate::creature_ai::CreatureState>(mutanus),
        Some(crate::creature_ai::CreatureState::Pursuing { target }) if *target == disciple.to_bits())
    );
    h.advance(301.0);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(mutanus)
            .is_none(),
        "corpse timer does not expire a living summon"
    );
    let slot = instance_script::boss_id_for_encounter(h.app.world_mut(), map, 592).unwrap();
    h.hit_pct(mutanus, 1_000.0);
    h.advance(0.1);
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, slot),
        Some(EncounterState::Done)
    );
    assert!(
        h.inbox(0)
            .ends
            .iter()
            .any(|end| end.encounter_id == 592 && end.success)
    );
    assert!(h.inbox(1).ends.iter().all(|end| end.encounter_id != 592));
    h.advance(298.0);
    assert!(
        h.app.world().get_entity(mutanus).is_ok(),
        "authored300s corpse lifetime"
    );
    h.advance(3.0);
    assert!(
        h.app.world().get_entity(mutanus).is_err(),
        "corpse timer unsummons instead of respawning"
    );
}

/// Instance source sends ACTION_ALL_DONE0 to its own Disciple when all four
/// druids become DONE. This proves the state condition/notification, not four
/// complete fights or the Anacondra mode0 source-data discrepancy.
#[test]
fn encounter_coverage_wailing_four_druids_notify_only_their_copy_once() {
    let (mut h, _boss, map) = bound_boss_at_difficulty(3669, 43, 1, 1, None);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (disciple, _) = spawn_authored_actor(&mut h.app, &conn, 3678, map, 1);
    let other = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    let (other_disciple, _) = spawn_authored_actor(&mut h.app, &conn, 3678, other, 1);
    let position = *h.app.world().get::<Position>(disciple).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(position);
    let other_player = h.players[1].0;
    let other_position = *h.app.world().get::<Position>(other_disciple).unwrap();
    h.app
        .world_mut()
        .entity_mut(other_player)
        .insert(other_position);
    h.advance(0.1);
    let announcements = |h: &Harness, index: usize| {
        h.inbox(index)
            .chat
            .iter()
            .filter(|message| message.content.contains("Naralex can be awakened"))
            .count()
    };
    let slots = [586, 588, 585, 590].map(|encounter| {
        instance_script::boss_id_for_encounter(h.app.world_mut(), map, encounter).unwrap()
    });
    for &slot in &slots[..3] {
        assert!(instance_script::set_boss_state(
            h.app.world_mut(),
            map,
            slot,
            EncounterState::Done
        ));
    }
    h.advance(0.1);
    assert_eq!(announcements(&h, 0), 0, "three druids are insufficient");
    assert!(instance_script::set_boss_state(
        h.app.world_mut(),
        map,
        slots[3],
        EncounterState::Done
    ));
    h.advance(0.1);
    assert_eq!(announcements(&h, 0), 1);
    assert_eq!(
        announcements(&h, 1),
        0,
        "another copy cannot receive the notification"
    );
    assert!(!instance_script::set_boss_state(
        h.app.world_mut(),
        map,
        slots[3],
        EncounterState::Done
    ));
    h.advance(0.1);
    assert_eq!(announcements(&h, 0), 1, "repeated DONE cannot notify twice");
}

const ADAROGG: u32 = 61408;
const DEMONIC_LEAP: u32 = 61409;
const SLAGMAW: u32 = 61463;

/// Adarogg engaged by the first player (the tank) 3 yards away; `healer` adds a
/// second threat-list player 8 yards away in the same copy.
fn engaged_adarogg(healer: bool) -> (Harness, Entity, InMap, Option<Entity>) {
    let (mut h, boss, map) =
        encounter_fixture(ADAROGG, 389, 0, 1, None, EncounterState::NotStarted, false);
    let boss = boss.unwrap();
    let position = *h.app.world().get::<Position>(boss).unwrap();
    let tank = h.player();
    h.app.world_mut().entity_mut(tank).insert(Position {
        x: position.x + 3.0,
        ..position
    });
    let healer = healer.then(|| {
        let wow = [position.x + 8.0, -position.z, position.y, 0.0];
        let (healer, connection) = spawn_player(&mut h.app, map, wow, 3);
        h.players.push((healer, connection, Inbox::default()));
        healer
    });
    h.engage(boss);
    if let Some(healer) = healer {
        crate::creature_aggro::add_threat(h.app.world_mut(), boss, healer, 0.5);
    }
    h.advance(0.1);
    (h, boss, map, healer)
}

fn casting(h: &Harness, unit: Entity) -> Option<u32> {
    h.app
        .world()
        .get::<CastState>(unit)
        .map(|cast| cast.spell_id)
}

/// TC boss_adarogg: Inferno Charge at10s picks a random non-tank player, summons
/// server-side119297 Demonic Leap there and channels119405 onto it; Fire Breath
/// 119420 hits the victim at20s. Both repeat15-20s.
#[test]
fn encounter_coverage_adarogg_source_inferno_charge_and_fire_breath() {
    let (mut h, boss, map, healer) = engaged_adarogg(true);
    let healer = healer.unwrap();
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 0),
        Some(EncounterState::InProgress)
    );
    h.advance(9.7);
    assert!(h.creatures_of(DEMONIC_LEAP).is_empty(), "Inferno waits10s");
    h.advance(0.3);
    let leaps = h.creatures_of(DEMONIC_LEAP);
    assert_eq!(leaps.len(), 1, "server-side119297 summons one Demonic Leap");
    let leap = leaps[0];
    assert!(
        h.wow_position(leap).distance(h.wow_position(healer)) < 0.5,
        "DEST_TARGET_ENEMY: summoned at the non-tank player"
    );
    assert_eq!(casting(&h, boss), Some(119405), "channels Inferno Charge");
    assert!(
        h.has_aura(leap, 119405),
        "NEARBY_ENTRY condition selects the Leap"
    );
    assert_eq!(h.casts(2, boss, 119297), 1, "logged to the targeted player");
    h.advance(3.1);
    assert_eq!(casting(&h, boss), None, "3s channel ends");
    assert!(
        h.inbox(0)
            .log
            .iter()
            .all(|event| event.spell_id != Some(119299)),
        "TC EffectCharge without a unit target never casts119299"
    );
    h.advance(2.0);
    assert!(h.creatures_of(DEMONIC_LEAP).is_empty(), "5s summon expires");
    h.advance(4.6);
    assert_eq!(casting(&h, boss), None, "Fire Breath waits20s");
    h.advance(0.3);
    assert_eq!(casting(&h, boss), Some(119420));
    let tank = h.player();
    let before = h.health(tank).current;
    h.advance(1.6);
    assert!(
        h.health(tank).current < before,
        "1.5s cast damages the victim"
    );
    assert!(h.health(healer).current == h.health(healer).max);
    h.advance(10.5);
    assert_eq!(
        h.casts(2, boss, 119297),
        2,
        "Inferno repeats within15-20s of10s"
    );
}

fn spell_gos(h: &Harness, caster: Entity, spell_id: u32) -> Vec<shared::protocol::SpellGo> {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == caster.to_bits() && go.spell_id == spell_id)
        .cloned()
        .collect()
}

fn hits_entry(h: &mut Harness, go: &shared::protocol::SpellGo, entry: u32) -> bool {
    let entries = h.creatures_of(entry);
    go.hit_targets
        .iter()
        .any(|hit| entries.iter().any(|unit| unit.to_bits() == *hit))
}

/// TC at_adarogg_intro7904: an ordinary player's actual DB2 box entry makes both
/// StringId houndmasters yell, then Adjules Chow Time at +2s/+1s/+1.2s with the
/// two authored MovePoints. Only once per instance.
#[test]
fn encounter_coverage_adarogg_intro_trigger_yells_and_chows_once() {
    let (mut h, _, _) =
        encounter_fixture(ADAROGG, 389, 0, 1, None, EncounterState::NotStarted, true);
    h.advance(0.1);
    let boss = h.creatures_of(ADAROGG)[0];
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (x, y, z, length): (f32, f32, f32, f32) = conn
        .query_row(
            "SELECT Pos_0,Pos_1,Pos_2,Box_length FROM area_trigger WHERE ID=7904 AND Box_yaw=0",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    let at = |wx: f32| {
        let (px, py, pz) = crate::terrain_height::wow_to_bevy(wx, y, z);
        Position {
            x: px,
            y: py,
            z: pz,
        }
    };
    let player = h.player();
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(at(x + length / 2.0 + 0.5));
    h.advance(0.1);
    let yells = |h: &Harness, text: &str| {
        h.inbox(0)
            .chat
            .iter()
            .filter(|chat| chat.content.contains(text))
            .count()
    };
    assert_eq!(yells(&h, "cornered"), 0, "outside the box length");
    h.app.world_mut().entity_mut(player).insert(at(x));
    h.advance(0.1);
    assert_eq!(yells(&h, "He's cornered!"), 1, "houndmaster_1 group0");
    assert_eq!(yells(&h, "We've got him now!"), 1, "houndmaster_2 group1");
    h.advance(1.8);
    assert!(spell_gos(&h, boss, 120113).is_empty(), "first chow waits2s");
    h.advance(0.2);
    let chows = spell_gos(&h, boss, 120113);
    assert_eq!(chows.len(), 1);
    assert!(
        hits_entry(&mut h, &chows[0], 61666),
        "condition: Corrupted Houndmaster"
    );
    let target = |h: &Harness| {
        h.app
            .world()
            .get::<crate::scripted_ai::movement::PointMovement>(boss)
            .map(|movement| {
                crate::terrain_height::bevy_to_wow(
                    movement.target.x,
                    movement.target.y,
                    movement.target.z,
                )
            })
    };
    let first = target(&h).expect("MovePoint AdaroggIntroPoints[0]");
    assert!((first.0 + 282.31488).abs() < 0.001 && (first.1 + 53.24906).abs() < 0.001);
    h.advance(1.0);
    assert_eq!(spell_gos(&h, boss, 120113).len(), 2);
    let second = target(&h).expect("MovePoint AdaroggIntroPoints[1]");
    assert!((second.0 + 281.1894).abs() < 0.001 && (second.1 + 54.73433).abs() < 0.001);
    h.advance(1.2);
    assert_eq!(
        spell_gos(&h, boss, 120113).len(),
        3,
        "third chow with MovePath"
    );
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(at(x + length / 2.0 + 0.5));
    h.advance(0.1);
    h.app.world_mut().entity_mut(player).insert(at(x));
    h.advance(2.5);
    assert_eq!(yells(&h, "He's cornered!"), 1, "OnlyOnce survives re-entry");
    assert_eq!(spell_gos(&h, boss, 120113).len(), 3);
}

/// Source WaypointPathEnded(PATH_INTRO2): Eat Trogg at +1s then once more after
/// 2-3s, never a third time. Map389 has no navigation for the ground path itself,
/// so the fixture places Adarogg at that path's authored last node.
#[test]
fn encounter_coverage_adarogg_path_end_eats_two_troggs() {
    let (mut h, _, _) =
        encounter_fixture(ADAROGG, 389, 0, 1, None, EncounterState::NotStarted, true);
    h.advance(0.1);
    let boss = h.creatures_of(ADAROGG)[0];
    let (px, py, pz) = crate::terrain_height::wow_to_bevy(-286.479, -74.899, -60.5504);
    h.app.world_mut().entity_mut(boss).insert(Position {
        x: px,
        y: py,
        z: pz,
    });
    dispatch(
        h.app.world_mut(),
        boss,
        Hook::WaypointPathEnded {
            node_id: 2,
            path_id: 6_140_801,
        },
    );
    h.advance(0.9);
    assert!(spell_gos(&h, boss, 120064).is_empty(), "Eat Trogg waits1s");
    h.advance(0.2);
    let eats = spell_gos(&h, boss, 120064);
    assert_eq!(eats.len(), 1);
    assert!(
        hits_entry(&mut h, &eats[0], 61659),
        "condition: Slain Trogg"
    );
    h.advance(1.8);
    assert_eq!(
        spell_gos(&h, boss, 120064).len(),
        1,
        "repeat waits at least2s"
    );
    h.advance(1.3);
    assert_eq!(spell_gos(&h, boss, 120064).len(), 2, "repeat within3s");
    h.advance(6.0);
    assert_eq!(
        spell_gos(&h, boss, 120064).len(),
        2,
        "_eatCounter stops at two"
    );
}

/// TDB spell_target_position (map389) of the three teleports Slagmaw may pick after
/// its initial `_lastTeleportSpell` West: North119424, East119425, South119426.
const SLAGMAW_FIRST_TELEPORTS: [[f32; 3]; 3] = [
    [-222.94, 165.703, -19.721],
    [-226.477, 135.704, -19.721],
    [-263.212, 136.244, -19.721],
];

fn engaged_slagmaw() -> (Harness, Entity, InMap) {
    let (mut h, boss, map) =
        encounter_fixture(SLAGMAW, 389, 2, 1, None, EncounterState::NotStarted, false);
    let boss = boss.unwrap();
    let position = *h.app.world().get::<Position>(boss).unwrap();
    let tank = h.player();
    h.app.world_mut().entity_mut(tank).insert(Position {
        x: position.x + 3.0,
        ..position
    });
    h.engage(boss);
    h.advance(0.05);
    (h, boss, map)
}

fn slagmaw_missile_hits(h: &Harness, boss: Entity, target: Entity) -> usize {
    h.inbox(0)
        .log
        .iter()
        .filter(|event| {
            let missile_damage =
                event.kind == CombatLogKind::Damage && event.spell_id == Some(119433);
            let owned_target =
                event.source == Some(boss.to_bits()) && event.target == Some(target.to_bits());
            missile_damage && owned_target && event.amount > 0
        })
        .count()
}

/// TC boss_slagmaw: Lava Spit119434 at a random threat target from1s, re-armed1s
/// (one event per UpdateAI, waiting out each3s cast); after five, Magnaw Submerge
/// 120384, a server-side teleport other than the last (initially West) 3s later
/// and emerge1s after that, then Lava Spit resumes1s later.
#[test]
fn encounter_coverage_slagmaw_spits_five_then_submerges_and_teleports() {
    let (mut h, boss, map) = engaged_slagmaw();
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 2),
        Some(EncounterState::InProgress)
    );
    h.advance(0.85);
    assert_eq!(casting(&h, boss), None, "Lava Spit waits1s");
    h.advance(0.15);
    assert_eq!(casting(&h, boss), Some(119434));
    let tank = h.player();
    let tick = h.app.world().resource::<Time<Fixed>>().timestep();
    let cast = h.app.world().get::<CastState>(boss).unwrap();
    assert_eq!(cast.duration, 3.0, "DB2 CastingTimeIndex14");
    let remaining = Duration::from_secs_f32(cast.duration - cast.elapsed);
    let now = h.app.world().resource::<Time<Fixed>>().elapsed();
    let complete_by = now + remaining + tick;
    // Preserve the original fourth-spit checkpoint, independent of this flight check.
    let fourth_spit_at = now + Duration::from_millis(14_900);
    while h.casts(0, boss, 119434) == 0 {
        assert!(
            h.app.world().resource::<Time<Fixed>>().elapsed() < complete_by,
            "three-second cast completes within its fixed-tick quantization"
        );
        h.advance(tick.as_secs_f32());
        assert_eq!(slagmaw_missile_hits(&h, boss, tank), 0);
    }
    let launched_at = h.app.world().resource::<Time<Fixed>>().elapsed();
    assert!(launched_at >= now + remaining, "no early cast completion");
    let spell = h.app.world().resource::<SpellStore>().get(119434).unwrap();
    let misc = spell.misc.as_ref().unwrap();
    assert_eq!(misc.attributes[9] & 0x10, 0x10, "speed is delay in seconds");
    assert_eq!(misc.speed, 1.0);
    assert_eq!((misc.launch_delay, misc.min_duration), (0.0, 0.0));
    let impact_at = launched_at + Duration::from_secs_f32(misc.speed);
    while h.app.world().resource::<Time<Fixed>>().elapsed() + tick < impact_at {
        h.advance(tick.as_secs_f32());
        assert_eq!(
            slagmaw_missile_hits(&h, boss, tank),
            0,
            "119433 cannot damage before the authored missile delay"
        );
    }
    h.advance(tick.as_secs_f32());
    assert_eq!(h.app.world().resource::<Time<Fixed>>().elapsed(), impact_at);
    assert_eq!(
        slagmaw_missile_hits(&h, boss, tank),
        1,
        "119433 damages the target exactly when its missile is due, not on a melee roll"
    );
    // Each due event runs the update after the previous3s cast ends.
    while h.app.world().resource::<Time<Fixed>>().elapsed() < fourth_spit_at {
        h.advance(tick.as_secs_f32());
    }
    assert_eq!(h.casts(0, boss, 119434), 4);
    let mut waited = 0.0;
    while h.casts(0, boss, 119434) < 5 {
        assert!(waited < 0.6, "fifth spit by ~16s");
        h.advance(0.05);
        waited += 0.05;
    }
    h.advance(0.1);
    assert!(
        h.has_aura(boss, 120384),
        "sixth event submerges instead of spitting"
    );
    assert_ne!(casting(&h, boss), Some(119434));
    let submerged = h.wow_position(boss);
    h.advance(2.8);
    assert!(
        h.wow_position(boss).distance(submerged) < 1.0,
        "teleport waits3s"
    );
    let mut waited = 0.0;
    while h.wow_position(boss).distance(submerged) < 10.0 {
        assert!(waited < 0.4, "teleport by3s");
        h.advance(0.05);
        waited += 0.05;
    }
    // Chase resumes after the teleport: within one 0.05s update, at most two fixed
    // ticks of run speed (7 x speed_run1.28571 x 0.1s = 0.9yd).
    let landed = h.wow_position(boss);
    assert!(
        SLAGMAW_FIRST_TELEPORTS
            .iter()
            .any(|point| landed.distance(Vec3::from_array(*point)) < 1.0),
        "server-side teleport to an authored non-West position: {landed:?}"
    );
    assert!(h.has_aura(boss, 120384));
    h.advance(0.9);
    assert!(h.has_aura(boss, 120384), "still submerged");
    h.advance(0.15);
    assert!(!h.has_aura(boss, 120384), "emerges1s after teleport");
    assert_eq!(h.casts(0, boss, 119434), 5);
    h.advance(1.1);
    assert_eq!(casting(&h, boss), Some(119434), "Lava Spit resumes1s later");
}

/// The2.5s boundary check (deferred, like every event, until the1-4s Lava Spit
/// cast ends) evades a victim farther than50yd (_DespawnAtEvade).
#[test]
fn encounter_coverage_slagmaw_boundary_check_evades_a_distant_victim() {
    let (mut h, boss, map) = engaged_slagmaw();
    h.advance(2.3);
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 2),
        Some(EncounterState::InProgress),
        "victim within50yd at no check yet"
    );
    let position = *h.app.world().get::<Position>(boss).unwrap();
    let tank = h.player();
    h.app.world_mut().entity_mut(tank).insert(Position {
        x: position.x + 200.0,
        ..position
    });
    h.advance(1.5);
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 2),
        Some(EncounterState::InProgress),
        "check waits for the cast; no other leash"
    );
    h.advance(0.4);
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 2),
        Some(EncounterState::Fail)
    );
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(boss)
            .is_some(),
        "despawned without a kill"
    );
    assert!(!h.inbox(0).disengages.is_empty());
}

/// Magnaw Teleport East119425 lands ~48yd from Slagmaw's spawn. TC has no creature
/// leash in a dungeon (`CanCreatureAttack`, Creature.cpp:2731), so it stays engaged and
/// its Submerge (AuraInterruptFlags EnteringCombat) survives the relocation.
#[test]
fn encounter_coverage_slagmaw_east_teleport_keeps_it_engaged() {
    let (mut h, boss, map) = engaged_slagmaw();
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, 120384, Some(boss), true).unwrap();
    h.advance(0.1);
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, 119425, Some(boss), true).unwrap();
    h.advance(0.3);
    assert_ne!(
        h.app.world().get::<crate::creature_ai::CreatureState>(boss),
        Some(&crate::creature_ai::CreatureState::Returning)
    );
    let east = Vec3::new(-226.477, 135.704, -19.721);
    assert!(
        h.wow_position(boss).distance(east) < 3.0,
        "{:?}",
        h.wow_position(boss)
    );
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 2),
        Some(EncounterState::InProgress)
    );
    assert!(h.has_aura(boss, 120384), "submerged until EVENT_EMERGE");
}

/// `GetDistance` subtracts Slagmaw's display42247 CombatReach24 and the player's1.5:
/// a victim held 70yd away (44.5) stays engaged through every check; 80yd (54.5) evades.
/// Frost Nova122 (6s MOD_ROOT_2) holds Slagmaw at home so only the boundary is measured.
#[test]
fn encounter_coverage_slagmaw_boundary_measures_beyond_both_combat_reaches() {
    for (offset, evades) in [(70.0, false), (80.0, true)] {
        let (mut h, boss, map) = engaged_slagmaw();
        let tank = h.player();
        let mut failed_at = None;
        for step in 0..160 {
            if step % 100 == 0 {
                root(&mut h, tank, boss);
            }
            let Some(position) = h.app.world().get::<Position>(boss).copied() else {
                break;
            };
            h.app.world_mut().entity_mut(tank).insert(Position {
                x: position.x + offset,
                ..position
            });
            h.advance(0.05);
            if get_boss_state(h.app.world_mut(), map, 2) == Some(EncounterState::Fail) {
                failed_at = Some(step);
                break;
            }
        }
        assert_eq!(failed_at.is_some(), evades, "victim {offset}yd from center");
    }
}

fn root(h: &mut Harness, caster: Entity, target: Entity) {
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(122)
        .cloned()
        .unwrap();
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: 122,
            effect_mask: crate::auras::aura_effect_mask(&spell),
            source: crate::auras::AuraSource::Cast,
        },
    );
}

#[test]
fn encounter_coverage_slagmaw_death_completes_its_boss_state() {
    let (mut h, boss, map) = engaged_slagmaw();
    h.hit_pct(boss, 1_000.0);
    h.advance(0.1);
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 2),
        Some(EncounterState::Done)
    );
    assert!(!h.inbox(0).disengages.is_empty());
}

/// NonTankTargetSelector(me): only players other than the current victim qualify.
#[test]
fn encounter_coverage_adarogg_inferno_charge_never_targets_the_tank() {
    let (mut h, boss, _, _) = engaged_adarogg(false);
    h.advance(10.2);
    assert!(h.creatures_of(DEMONIC_LEAP).is_empty());
    assert_eq!(h.casts(0, boss, 119297), 0);
    assert_ne!(casting(&h, boss), Some(119405));
}

#[test]
fn encounter_coverage_adarogg_death_and_evade_use_source_lifecycle() {
    for killed in [false, true] {
        let (mut h, boss, map, _) = engaged_adarogg(true);
        h.advance(10.1);
        assert_eq!(h.creatures_of(DEMONIC_LEAP).len(), 1);
        if killed {
            h.hit_pct(boss, 1_000.0);
            h.advance(0.1);
            assert_eq!(
                get_boss_state(h.app.world_mut(), map, 0),
                Some(EncounterState::Done)
            );
        } else {
            crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
            h.advance(0.1);
            assert_eq!(
                get_boss_state(h.app.world_mut(), map, 0),
                Some(EncounterState::Fail)
            );
            assert!(
                h.app
                    .world()
                    .get::<crate::combat::DeathState>(boss)
                    .is_some(),
                "_DespawnAtEvade retires actor without a kill"
            );
        }
        if !killed {
            // _DespawnAtEvade does not DespawnAll; the wild summon keeps its 5s expiry.
            assert_eq!(h.creatures_of(DEMONIC_LEAP).len(), 1);
            h.advance(5.0);
        }
        assert!(
            h.creatures_of(DEMONIC_LEAP).is_empty(),
            "_JustDied DespawnAll, or the evaded summon's own expiry"
        );
        assert!(!h.inbox(0).disengages.is_empty(), "frame disengage");
    }
}

/// TC Executed371652 effect0 apply sets three independent flag fields, preserves
/// their other bits, never performs a real death, and has no removal callback.
#[test]
fn encounter_coverage_ruby_executed_sets_source_flags_without_killing() {
    use shared::components::{UnitFlags, UnitFlags2, UnitFlags3};
    let (mut h, actor, _) =
        encounter_fixture(189162, 2521, 0, 23, None, EncounterState::NotStarted, false);
    let actor = actor.unwrap();
    h.app.world_mut().entity_mut(actor).insert((
        UnitFlags(UnitFlags::IMMUNE_TO_PC),
        UnitFlags2(4),
        UnitFlags3(2),
    ));
    let health = h.health(actor);
    crate::spell_cast::creature_cast(h.app.world_mut(), actor, 371652, None, true)
        .expect("realExecuted371652 self cast");
    let expected = (
        UnitFlags(UnitFlags::IMMUNE_TO_PC | UnitFlags::PREVENT_EMOTES_FROM_CHAT_TEXT),
        UnitFlags2(4 | UnitFlags2::FEIGN_DEATH),
        UnitFlags3(2 | UnitFlags3::FAKE_DEAD),
    );
    assert_eq!(h.app.world().get::<UnitFlags>(actor), Some(&expected.0));
    assert_eq!(h.app.world().get::<UnitFlags2>(actor), Some(&expected.1));
    assert_eq!(h.app.world().get::<UnitFlags3>(actor), Some(&expected.2));
    assert_eq!(h.health(actor), health);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(actor)
            .is_none()
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 371652,
    );
    h.advance(0.1);
    assert_eq!(
        h.app.world().get::<UnitFlags2>(actor),
        Some(&expected.1),
        "source has no cleanup hook"
    );
    assert_eq!(h.app.world().get::<UnitFlags3>(actor), Some(&expected.2));
    // A hit containing only effect1 (MOD_UNATTACKABLE) must not run effect0's hook.
    let other = h.player();
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: other,
            target: other,
            spell_id: 371652,
            effect_mask: 2,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(h.app.world().get::<UnitFlags2>(other).is_none());
    assert!(h.app.world().get::<UnitFlags3>(other).is_none());
}

/// TC spell_ruby_life_pools_storm_infusion offsets effect1 TARGET_DEST_DEST by
/// Position::RelocateOffset(local9,0,4), relative to the destination's orientation.
#[test]
fn encounter_coverage_ruby_storm_infusion_creates_trigger_at_oriented_offset() {
    let (mut h, _, map) =
        encounter_fixture(195119, 2521, 0, 23, None, EncounterState::NotStarted, false);
    let caster = h.player();
    let origin = Position {
        x: 12.0,
        y: 20.0,
        z: 31.0,
    };
    for (orientation, expected) in [
        (0.0, Vec3::new(21.0, 24.0, 31.0)),
        (std::f32::consts::FRAC_PI_2, Vec3::new(12.0, 24.0, 22.0)),
    ] {
        h.app.world_mut().entity_mut(caster).insert((
            origin,
            shared::components::Rotation {
                x: 0.0,
                y: orientation + std::f32::consts::FRAC_PI_2,
                z: 0.0,
            },
        ));
        crate::spell_cast::creature_cast(h.app.world_mut(), caster, 395029, None, true)
            .expect("real395029 self/destination cast");
        let triggers: Vec<_> = h
            .app
            .world_mut()
            .query::<(
                Entity,
                &crate::spell_cast::area_trigger::SpellAreaTrigger,
                &Position,
                &InMap,
            )>()
            .iter(h.app.world())
            .filter(|(_, trigger, _, copy)| trigger.spell_id == 395029 && **copy == map)
            .map(|(entity, _, position, _)| (entity, Vec3::new(position.x, position.y, position.z)))
            .collect();
        assert_eq!(triggers.len(), 1);
        assert!(
            triggers[0].1.distance(expected) < 0.001,
            "oriented9yd forward/4yd up: {:?}",
            triggers[0].1
        );
        h.app.world_mut().despawn(triggers[0].0);
    }
}

/// TC spell_ruby_life_pools_ice_shield ticks every1s and refreshes an existing
/// 372749 aura, without reapplying/refilling its absorb or creating a missing shield.
#[test]
fn encounter_coverage_ruby_ice_shield_refreshes_existing_duration_only() {
    let (mut h, target, _) =
        encounter_fixture(188011, 2521, 0, 23, None, EncounterState::NotStarted, false);
    let target = target.unwrap();
    let caster = h.player();
    for spell_id in [372749, 384933] {
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster,
                target,
                spell_id,
                effect_mask: 1,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    let shield = |h: &Harness| {
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(target)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 372749)
            .unwrap()
            .clone()
    };
    let initial = shield(&h);
    h.advance(0.1);
    let first_tick = shield(&h);
    assert!(
        first_tick.expires_at.unwrap() > initial.expires_at.unwrap(),
        "DB2 EXTRA_INITIAL_PERIOD ticks immediately"
    );
    h.advance(0.8);
    assert_eq!(
        shield(&h).expires_at,
        first_tick.expires_at,
        "next tick waits 1s"
    );
    h.advance(0.2);
    let refreshed = shield(&h);
    assert!(
        refreshed.expires_at.unwrap() >= initial.expires_at.unwrap() + 0.95,
        "1s periodic tick refreshes max duration"
    );
    assert_eq!(refreshed.instance_id, initial.instance_id);
    assert_eq!(
        refreshed.effects, initial.effects,
        "no absorb reset/reapplication"
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        target,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 372749,
    );
    h.advance(1.1);
    assert!(
        !h.has_aura(target, 372749),
        "periodic callback never creates a missing shield"
    );
}

/// TC spell_ruby_life_pools_excavate: effect0 periodic dummy casts373497 triggered
/// from its original caster on its bearer, and stops when that aura is removed.
#[test]
fn encounter_coverage_ruby_excavate_periodic_cast_uses_original_caster_and_target() {
    let (mut h, target, _) =
        encounter_fixture(189162, 2521, 0, 23, None, EncounterState::NotStarted, false);
    let target = target.unwrap();
    let caster = h.player();
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: 372793,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.advance(0.1);
    assert_eq!(h.casts(0, caster, 373497), 1, "DB2 EXTRA_INITIAL_PERIOD");
    h.advance(0.8);
    assert_eq!(h.casts(0, caster, 373497), 1, "next tick waits1s");
    let casts: Vec<_> = h
        .inbox(0)
        .log
        .iter()
        .filter(|event| event.kind == CombatLogKind::CastSuccess && event.spell_id == Some(373497))
        .collect();
    assert_eq!(
        casts[0].target,
        Some(target.to_bits()),
        "not the caster/self"
    );
    h.advance(0.2);
    assert_eq!(h.casts(0, caster, 373497), 2);
    crate::auras::remove_auras(
        h.app.world_mut(),
        target,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 372793,
    );
    h.advance(1.1);
    assert_eq!(h.casts(0, caster, 373497), 2, "no more ticks after removal");
}

/// TC kings_rest.cpp npc_kings_rest_minion_of_zul: only TempleEvent minions run the
/// temple_basic visual; every minion appears with Bound by Shadow 269935.
#[test]
fn encounter_coverage_minion_of_zul_appears_bound_by_shadow() {
    let (mut h, map) = kings_rest_copy(23);
    h.advance(0.1);
    let resident = h.creatures_of(133943);
    assert_eq!(
        resident.len(),
        8,
        "untagged map1762 minions; group1255 withheld"
    );
    for &minion in &resident {
        assert!(
            h.has_aura(minion, 269935),
            "Bound by Shadow on JustAppeared"
        );
        assert!(!h.has_aura(minion, 269619), "no TempleEvent StringId");
    }
    crate::zones::spawn_group_spawn(
        h.app.world_mut(),
        map,
        23,
        false,
        super::super::kings_rest::SPAWN_GROUP_PRE_FIRST_BOSS,
    );
    h.advance(0.1);
    let raised: Vec<Entity> = h
        .creatures_of(133943)
        .into_iter()
        .filter(|minion| !resident.contains(minion))
        .collect();
    assert_eq!(raised.len(), 6);
    for minion in raised {
        assert!(h.has_aura(minion, 269935));
        assert!(
            h.has_aura(minion, 269619),
            "TempleEvent temple_basic visual"
        );
    }
}

/// TC spell_kings_rest_bound_by_shadow: only AURA_REMOVE_BY_ENEMY_SPELL (the absorb
/// used up) kills the minion.
#[test]
fn encounter_coverage_minion_of_zul_dies_when_bound_by_shadow_is_broken() {
    let (mut h, _) = kings_rest_copy(23);
    h.advance(0.1);
    let minions = h.creatures_of(133943);
    let (broken, cancelled) = (minions[0], minions[1]);
    let absorb = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(broken)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == 269935))
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map(|effect| effect.amount)
        .expect("269935 effect0 SCHOOL_ABSORB");
    assert!(absorb > 0.0 && absorb < h.health(broken).max / 2.0);
    crate::auras::remove_auras(
        h.app.world_mut(),
        cancelled,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 269935,
    );
    h.hit(broken, absorb + 1.0);
    h.advance(0.1);
    assert!(!h.has_aura(broken, 269935));
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(broken)
            .is_some(),
        "KillSelf once the shield is used up"
    );
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(cancelled)
            .is_none(),
        "other removal modes keep it alive"
    );
}

/// TC npc_kings_rest_minion_of_zul::JustEngagedWith casts Fixate 269936 on one enemy
/// within 300yd; HandleAuraModFixate makes ReselectVictim prefer it over higher threat.
#[test]
fn encounter_coverage_minion_of_zul_fixates_one_enemy_over_threat() {
    let (mut h, map) = kings_rest_copy(23);
    h.advance(0.1);
    let minion = h.creatures_of(133943)[0];
    let at = h.wow_position(minion);
    let first = h.player();
    let (second, _) = super::spawn_player(&mut h.app, map, [at.x + 400.0, at.y, at.z, 0.0], 2);
    let near = *h.app.world().get::<Position>(minion).unwrap();
    h.app.world_mut().entity_mut(first).insert(Position {
        x: near.x + 2.0,
        ..near
    });
    crate::creature_aggro::add_threat(h.app.world_mut(), minion, first, 1.0);
    h.advance(0.1);
    let fixated: Vec<Entity> = [first, second]
        .into_iter()
        .filter(|&player| {
            h.app
                .world()
                .get::<crate::auras::ActiveAuras>(player)
                .is_some_and(|auras| {
                    auras
                        .iter()
                        .any(|aura| aura.spell_id == 269936 && aura.caster == minion)
                })
        })
        .collect();
    assert_eq!(fixated.len(), 1, "MaxTargets1 fixate");
    let fixated = fixated[0];
    assert_eq!(fixated, first, "only the engaged player is within 300yd");
    let other = second;
    h.app.world_mut().entity_mut(other).insert(Position {
        x: near.x + 3.0,
        ..near
    });
    crate::creature_aggro::add_threat(h.app.world_mut(), minion, other, 1_000_000.0);
    h.advance(0.5);
    assert_eq!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(minion)
            .and_then(|target| target.0),
        Some(fixated),
        "the fixate target outranks a far larger threat"
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        fixated,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 269936,
    );
    h.advance(0.5);
    assert_eq!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(minion)
            .and_then(|target| target.0),
        Some(other),
        "ClearFixate returns to threat order"
    );
}

/// TC at_kings_rest_gust_slash: 269931 creates AT17933 5yd ahead of the warrior; it
/// orbits its creation point (TDB orbit radius5, speed5, blending from radius0,
/// clockwise, looping) and an entering player casts 269932 once with no explicit
/// target, which InitExplicitTargets resolves to that player's valid hostile selection.
#[test]
fn encounter_coverage_gust_slash_orbits_and_entering_player_casts_source_damage() {
    let (mut h, map) = kings_rest_copy(23);
    crate::zones::spawn_group_spawn(
        h.app.world_mut(),
        map,
        23,
        false,
        super::super::kings_rest::SPAWN_GROUP_PRE_FIRST_BOSS,
    );
    h.advance(0.1);
    let warrior = h.creatures_of(134157)[0];
    crate::spell_cast::creature_cast(h.app.world_mut(), warrior, 269931, None, false)
        .expect("real Gust Slash cast");
    h.advance(1.6);
    let triggers: Vec<(Entity, [f32; 4], InMap)> = h
        .app
        .world_mut()
        .query::<(Entity, &super::super::kings_rest::GustSlashTrigger, &InMap)>()
        .iter(h.app.world())
        .map(|(entity, trigger, copy)| (entity, trigger.center, *copy))
        .collect();
    assert_eq!(
        triggers.len(),
        1,
        "DB2 effect179 creates create properties13339"
    );
    let (at, center, at_map) = triggers[0];
    assert_eq!(at_map, map);
    let warrior_at = h.wow_position(warrior);
    let ahead = Vec3::new(center[0] - warrior_at.x, center[1] - warrior_at.y, 0.0);
    assert!(
        (ahead.length() - 5.0).abs() < 0.05,
        "target47 radius8 = 5yd: {ahead:?}"
    );
    let player = h.player();
    let (x, y, z) = crate::terrain_height::wow_to_bevy(center[0], center[1], center[2]);
    h.app.world_mut().entity_mut(player).insert((
        Position { x, y, z },
        crate::networking::CombatTarget(Some(warrior)),
    ));
    let before = h.health(warrior).current;
    h.advance(0.1);
    assert_eq!(h.casts(0, player, 269932), 1, "OnUnitEnter casts once");
    assert!(
        h.health(warrior).current < before,
        "selection takes 269932 damage"
    );
    h.advance(0.3);
    assert_eq!(h.casts(0, player, 269932), 1, "no repeat while inside");
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(Position { x: x + 500.0, y, z });
    h.advance(1.2);
    let wow_at = |h: &Harness| {
        let p = h.app.world().get::<Position>(at).unwrap();
        let (x, y, _) = crate::terrain_height::bevy_to_wow(p.x, p.y, p.z);
        Vec3::new(x - center[0], y - center[1], 0.0)
    };
    let first = wow_at(&h);
    assert!(
        (first.length() - 5.0).abs() < 0.01,
        "blend done by 25% of 6s: {first:?}"
    );
    h.advance(0.1);
    let second = wow_at(&h);
    assert!((second.length() - 5.0).abs() < 0.01);
    assert!(
        first.cross(second).z < 0.0,
        "CounterClockwise0 orbits clockwise"
    );
    h.advance(27.5);
    assert!(
        h.app.world().get_entity(at).is_ok(),
        "duration index9 is 30s"
    );
    h.advance(1.2);
    assert!(h.app.world().get_entity(at).is_err(), "removed at 30s");
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

/// Animated Gold as Lucre's Call leaves it: its TDB template addon spawns it molten
/// (265915) and the call's aura removal animates it (covered by the Lucre's Call test).
fn animated_gold(h: &mut Harness, boss: Entity, position: Vec3) -> Entity {
    let gold = crate::zones::summon_creature(
        h.app.world_mut(),
        boss,
        135406,
        [position.x, position.y, position.z, 0.0],
    )
    .unwrap();
    h.advance(0.1);
    assert!(
        h.has_aura(gold, 265915),
        "TDB creature_template_addon spawns Animated Gold molten"
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        gold,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 265915,
    );
    gold
}

#[test]
fn encounter_coverage_animated_gold_lethal_spell_hit_becomes_molten_without_death() {
    let (mut h, boss, map) = bound_boss(135322, 1762, 0);
    let position = h.wow_position(boss);
    let gold = animated_gold(&mut h, boss, position);
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
    let gold = animated_gold(&mut h, boss, position);
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
    let gold = animated_gold(&mut h, boss, position);
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
    let gold = animated_gold(&mut h, boss, position);
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
    let gold = animated_gold(&mut h, boss, position);
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
        h.app.insert_resource(crate::persistence::GameDatabase::new(
            redb::Database::create(database_file.path()).unwrap(),
        ));
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

/// Skorpyron (Nighthold1530, encounter1849, bit1): DefeatEncounter165 leaves gated by TC
/// DungeonDifficulty68 credit only a native defeat in a copy of their difficulty
/// (31656 Normal14, 31666 Heroic15, 31454 Mythic16, 64394 LFR7/17).
#[test]
fn achievements_criteria_dungeon_difficulty_credits_only_the_copy_difficulty() {
    for (difficulty, credited) in [(14, 31656), (15, 31666)] {
        assert_copy_difficulty_credit(
            (102263, 1530, 1),
            difficulty,
            credited,
            &[31656, 31666, 31454, 64394],
        );
    }
}

#[test]
fn achievements_criteria_native_encounter_loot_credit_precedes_lock_commit() {
    use crate::achievement::PlayerAchievementState;
    let (mut h, boss, map) = bound_boss_at_difficulty(102263, 1530, 1, 14, None);
    h.app
        .insert_resource(crate::test_data::achievement_catalog());
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    crate::persistence::create_tables(&db);
    h.app
        .insert_resource(crate::persistence::GameDatabase::new(db));
    let player = h.player();
    let character = h
        .app
        .world()
        .get::<crate::persistence::ClientId>(player)
        .unwrap()
        .0;
    // The generic fixture's transport PeerId1 differs from its character ClientId2001.
    // Native authority is the authenticated in-world character, not the transport ID.
    h.app
        .world_mut()
        .entity_mut(h.players[0].1)
        .insert(crate::auth::InWorld {
            character_id: character,
        });
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(PlayerAchievementState::default());
    let other = h.players[1].0;
    h.app
        .world_mut()
        .entity_mut(other)
        .insert(PlayerAchievementState::default());
    h.engage(boss);
    h.advance(0.05);
    h.hit_pct(boss, 200.0);
    h.advance(0.05);
    assert_eq!(h.inbox(0).ends.last().map(|end| end.success), Some(true));
    let manager = h.app.world().resource::<crate::maps::InstanceManager>();
    assert_eq!(manager.completed_encounters(map), 2);
    let difficulty = manager.copy_difficulty(map).unwrap();
    let now = crate::maps::difficulty::now(h.app.world()).unix;
    let lock = manager
        .locks
        .find_active_lock(character, difficulty, now)
        .unwrap();
    assert_eq!(
        lock.data.completed_encounters & 2,
        2,
        "DONE saved the actual loot lock"
    );
    let state = h.app.world().get::<PlayerAchievementState>(player).unwrap();
    assert_eq!(
        state.criteria_progress.get(&35448),
        Some(&1),
        "first eligible defeat must credit before its new lock is saved"
    );
    assert!(!state.criteria_progress.contains_key(&35458));
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(other)
            .unwrap()
            .criteria_progress
            .contains_key(&35448)
    );
    let slot = instance_script::boss_id_for_encounter(h.app.world_mut(), map, 1849).unwrap();
    assert!(
        !instance_script::set_boss_state(h.app.world_mut(), map, slot, EncounterState::Done),
        "an already-DONE native transition cannot replay credit"
    );
    h.advance(0.05);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&35448),
        Some(&1)
    );
    drop(h);
    let db = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        crate::persistence::load_achievements(&db, character)
            .criteria_progress
            .get(&35448),
        Some(&1)
    );
}

/// Erudax (Grim Batol670, encounter1049, bit1): KillCreature0 leaves gated by TC
/// LegacyDungeonDifficulty20 compare the copy's Difficulty.db2 OldEnumValue
/// (16783 Normal0 -> Erudax kills5732; 16784 Heroic1 -> Heroic: Grim Batol5062).
#[test]
fn achievements_criteria_legacy_dungeon_difficulty_credits_only_the_copy_difficulty() {
    for (difficulty, credited) in [(1, 16783), (2, 16784)] {
        assert_copy_difficulty_credit((40484, 670, 1), difficulty, credited, &[16783, 16784]);
    }
}

fn assert_copy_difficulty_credit(
    (entry, map_id, bit): (u32, u32, u32),
    difficulty: u32,
    credited: u32,
    criteria: &[u32],
) {
    use crate::achievement::PlayerAchievementState;
    let (mut h, boss, map) = bound_boss_at_difficulty(entry, map_id, bit, difficulty, None);
    h.app
        .insert_resource(crate::test_data::achievement_catalog());
    let database_file = tempfile::NamedTempFile::new().unwrap();
    h.app.insert_resource(crate::persistence::GameDatabase::new(
        redb::Database::create(database_file.path()).unwrap(),
    ));
    crate::persistence::create_tables(
        &h.app
            .world()
            .resource::<crate::persistence::GameDatabase>()
            .db,
    );
    let player = h.player();
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(PlayerAchievementState::default());
    let other = h.players[1].0;
    h.app
        .world_mut()
        .entity_mut(other)
        .insert(PlayerAchievementState::default());
    h.engage(boss);
    h.advance(0.05);
    h.hit_pct(boss, 200.0);
    h.advance(0.05);
    assert_eq!(h.inbox(0).ends.last().map(|end| end.success), Some(true));
    let instances = h.app.world().resource::<crate::maps::InstanceManager>();
    assert_eq!(instances.completed_encounters(map), 1 << bit);
    assert_eq!(
        instances.copy_difficulty(map).unwrap().difficulty_id,
        difficulty
    );
    let progress = |h: &Harness, entity: Entity, criterion: u32| {
        h.app
            .world()
            .get::<PlayerAchievementState>(entity)
            .unwrap()
            .criteria_progress
            .get(&criterion)
            .copied()
    };
    for &criterion in criteria {
        let expected = (criterion == credited).then_some(1);
        assert_eq!(
            progress(&h, player, criterion),
            expected,
            "{difficulty}:{criterion}"
        );
        assert_eq!(progress(&h, other, criterion), None, "other copy");
    }
    h.advance(0.05);
    assert_eq!(progress(&h, player, credited), Some(1), "idle tick");
}
