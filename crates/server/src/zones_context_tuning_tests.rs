//! Source17536 fixtures through ordinary copy spawning, never canonical placements.
use super::*;
use rusqlite::{Connection, OpenFlags};

const NAZAN: u32 = 17_536;
const RAMPARTS: u32 = 543;

#[test]
fn herald_context_startup_publishes_ground_before_ordinary_continent_stats() {
    let mut data = sourced_nazan_fixture();
    // A sourced17536 fixture at an actual imported continent spawn, not a canonical Nazan placement.
    let source_spawn = crate::test_data::world_data()
        .spawns
        .iter()
        .find(|spawn| spawn.template_entry == 448 && spawn.map == 0)
        .unwrap()
        .clone();
    data.spawns = vec![CreatureSpawn {
        template_entry: NAZAN,
        ..source_spawn
    }];
    data.spawns_by_map = HashMap::from([(0, vec![0])]);
    let connection = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let maps = crate::maps::store::load_map_store(&connection).unwrap();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: std::time::Duration::from_millis(50),
    });
    app.add_plugins(shared::protocol::ProtocolPlugin);
    app.insert_resource(maps).insert_resource(data);
    // Same plugin ordering as production; ground is published by a deferred command in PostStartup.
    app.add_plugins((crate::ground::GroundPlugin, ZonePlugin));
    app.finish();
    app.cleanup();
    assert!(!app.world().contains_resource::<crate::ground::GroundMap>());
    app.update();
    let world = app.world_mut();
    let actor = fixture_actor(world);
    let scaling = world.get::<LevelScaling>(actor).unwrap();
    assert_eq!(
        scaling.content_tuning_id, 73,
        "actual Elwynn/Northshire terrain, not templateCT0"
    );
    assert_eq!(
        world.get::<UnitLevel>(actor).unwrap().0,
        scaling.native_level()
    );
    assert!(world.get::<ScaledStats>(actor).is_some());
    assert_eq!(
        world.resource::<WorldData>().spawns.len(),
        1,
        "fixture population only"
    );
}

#[test]
fn herald_context_authored_yala_keeps_provenance_selected_fixed_stats() {
    let data = crate::test_data::world_data();
    let spawn = data
        .spawns
        .iter()
        .find(|spawn| spawn.template_entry == 249363)
        .unwrap();
    let template = &data.templates[&249363];
    assert_eq!(template.min_level, 5);
    assert!(
        template.tuning.is_none(),
        "authored CT0, like unresolved Retail CT0"
    );
    let mut world = World::new();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, &world);
    let actor = spawn_creature(
        &mut commands,
        data,
        template,
        &mut CreatureSpawnContext {
            map: crate::maps::InMap::continent(2991),
            difficulty_id: 0,
            ground: None,
            temporary: false,
        },
        false,
        spawn,
        &mut rand::thread_rng(),
    )
    .expect("authored fixed-level creature");
    queue.apply(&mut world);
    assert_eq!(world.get::<UnitLevel>(actor).unwrap().0, 5);
    assert_eq!(world.get::<Health>(actor).unwrap().max, 102.0);
    assert!(world.get::<LevelScaling>(actor).is_none());
    assert!(world.get::<ScaledStats>(actor).is_none());
}

fn sourced_nazan_fixture() -> WorldData {
    let source = crate::test_data::world_db_path();
    let connection = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    // Temporary views project actual TDB base/difficulty fields; the source stays read-only.
    // CT0 intentionally has no stored range: the ordinary spawn must resolve its context.
    connection
        .execute_batch(
            "CREATE TEMP VIEW content_creature_template AS
         SELECT t.entry,COALESCE(l.minlevel,1) minlevel,COALESCE(l.maxlevel,1) maxlevel,
           t.faction,t.npcflag,t.speed_walk,d.HealthModifier,t.name,t.Classification rank,
           t.ExperienceModifier,t.type,t.flags_extra,t.speed_run,t.ScriptName,t.unit_class,
           d.HealthScalingExpansion exp,t.unit_flags,d.ContentTuningID,d.HealthScalingExpansion,
           d.LevelScalingDeltaMin,d.LevelScalingDeltaMax,d.DamageModifier,d.ArmorModifier,
           t.subname,d.ManaModifier,t.unit_flags2,t.RegenHealth,t.family,d.TypeFlags type_flags,
           NULL detection_range
         FROM tdb_creature_template t JOIN tdb_creature_template_difficulty d ON d.Entry=t.entry
         LEFT JOIN content_tuning_level l ON l.ID=d.ContentTuningID
         WHERE t.entry=17536 AND d.DifficultyID=0;
         CREATE TEMP VIEW content_creature_template_difficulty AS
         SELECT d.*,COALESCE(l.minlevel,1) minlevel,COALESCE(l.maxlevel,1) maxlevel,
           d.HealthScalingExpansion exp
         FROM tdb_creature_template_difficulty d
         LEFT JOIN content_tuning_level l ON l.ID=d.ContentTuningID WHERE d.Entry=17536;
         CREATE TEMP VIEW content_creature_template_model AS
         SELECT * FROM tdb_creature_template_model WHERE CreatureID=17536;",
        )
        .unwrap();
    let expected_stats = crate::world_data::load_expected_stats(&connection);
    let mut templates = crate::world_data::load_templates(&connection, &expected_stats);
    crate::world_data::load_template_models(&connection, &mut templates);
    let difficulty_templates =
        crate::world_data::load_difficulty_templates(&connection, &expected_stats, &templates);
    assert!(templates[&NAZAN].tuning.is_none(), "source base CT0");
    assert!(
        difficulty_templates[&(NAZAN, 2)].tuning.is_none(),
        "source Heroic CT0"
    );
    assert_eq!(templates[&NAZAN].health_modifier, 9.0);
    assert_eq!(difficulty_templates[&(NAZAN, 2)].health_modifier, 12.0);
    // Pinned Herald CenterPos, used only as a fixture spawn; no population is imported.
    let spawn = CreatureSpawn {
        guid: 1,
        template_entry: NAZAN,
        map: RAMPARTS as u16,
        zone_id: 0,
        position_x: -1410.5204,
        position_y: 1740.5852,
        position_z: 89.05998,
        orientation: 0.0,
        wander_distance: 0.0,
        movement_type: 0,
        spawntimesecs: 120,
        display_id: None,
        unit_flags: None,
        spawn_difficulties: vec![1, 2],
        equipment_id: 0,
    };
    WorldData {
        summon_levels: crate::world_data::summon_levels::SummonLevels::load(&connection).unwrap(),
        templates,
        difficulty_templates,
        expected_stats,
        class_level_base_hp: crate::test_data::world_data().class_level_base_hp.clone(),
        spawns: vec![spawn],
        spawns_by_map: HashMap::from([(RAMPARTS as u16, vec![0])]),
        ..Default::default()
    }
}

fn area_fixture(area_id: u32) -> crate::ground::GroundMap {
    let terrain = crate::terrain_height::test_helpers::flat_terrain_in_area(0.0, 0.0, 0.0, area_id);
    let tile = crate::ground::bevy_to_tile(-1.0, 1.0);
    crate::ground::GroundMap::with_terrain(tile, terrain)
}

struct SpawnLogWriter(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for SpawnLogWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn spawn_continent_fixture(mut data: WorldData, mut ground: crate::ground::GroundMap) -> App {
    data.spawns[0].map = 0;
    data.spawns[0].position_x = -1.0;
    data.spawns[0].position_y = -1.0;
    data.spawns[0].position_z = 0.0;
    data.spawns[0].zone_id = 9; // Source area9 has CT73; it must not replace actual terrain area1221.
    data.spawns[0].spawn_difficulties = vec![0];
    data.spawns_by_map = HashMap::from([(0, vec![0])]);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    assert_eq!(
        spawn_map_creatures(
            &mut commands,
            &data,
            MapContent::continent(0),
            false,
            Some(&mut ground)
        ),
        1
    );
    queue.apply(app.world_mut());
    app
}

fn fixture_actor(world: &mut World) -> Entity {
    world
        .query_filtered::<Entity, With<Npc>>()
        .single(world)
        .unwrap()
}

#[test]
fn herald_context_ordinary_spawn_reads_actual_terrain_parent_not_spawn_zone() {
    // Installed source area1221 has CT0, parent16 has CT2; zone9 instead has CT73.
    let mut app = spawn_continent_fixture(sourced_nazan_fixture(), area_fixture(1221));
    let world = app.world_mut();
    let actor = fixture_actor(world);
    let scaling = world
        .get::<LevelScaling>(actor)
        .expect("actual terrain parent supplies CT2");
    assert_eq!(scaling.content_tuning_id, 2);
    assert_eq!(scaling.delta, 0, "source17536 delta survives parent tuning");
    assert_eq!(
        world.get::<UnitLevel>(actor).unwrap().0,
        scaling.native_level()
    );
    assert!(world.get::<ScaledStats>(actor).is_some());
}

#[test]
fn herald_context_missing_ordinary_area_is_explicit_not_legacy_level() {
    let mut data = sourced_nazan_fixture();
    data.spawns[0].map = 0;
    data.spawns[0].spawn_difficulties = vec![0];
    let hogger = crate::test_data::world_data().templates[&448].clone();
    data.templates.insert(448, hogger);
    data.spawns.push(CreatureSpawn {
        guid: 2,
        template_entry: 448,
        ..data.spawns[0].clone()
    });
    data.spawns_by_map = HashMap::from([(0, vec![0, 1])]);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    let bytes = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let writer = bytes.clone();
    let subscriber = bevy::log::tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(move || SpawnLogWriter(writer.clone()))
        .finish();
    let count = bevy::log::tracing::subscriber::with_default(subscriber, || {
        spawn_map_creatures(
            &mut commands,
            &data,
            MapContent::continent(0),
            false,
            Some(&mut crate::ground::GroundMap::default()),
        )
    });
    let logs = String::from_utf8(bytes.lock().unwrap().clone()).unwrap();
    assert!(logs.contains("CREATURE_LEVEL_CONTEXT_GAP"), "{logs}");
    assert!(
        logs.contains("creature17536 spawn1 map0 copy0 difficulty0"),
        "{logs}"
    );
    assert_eq!(
        count, 1,
        "unknown context refuses only the unresolved spawn"
    );
    queue.apply(app.world_mut());
    let world = app.world_mut();
    let actor = fixture_actor(world);
    assert_eq!(world.get::<Npc>(actor).unwrap().template_id, 448);
    assert_eq!(
        world.get::<LevelScaling>(actor).unwrap().content_tuning_id,
        73
    );
    assert_eq!(world.query::<&Npc>().iter(world).count(), 1);
}

#[test]
fn herald_context_critter_and_known_zero_area_do_not_inherit_region() {
    let mut known_zero = spawn_continent_fixture(sourced_nazan_fixture(), area_fixture(0));
    let world = known_zero.world_mut();
    let actor = fixture_actor(world);
    assert!(world.get::<LevelScaling>(actor).is_none());
    assert!(world.get::<ScaledStats>(actor).is_none());
    assert_eq!(world.get::<UnitLevel>(actor).unwrap().0, 1);

    let mut data = sourced_nazan_fixture();
    let critter = crate::test_data::world_data().templates[&1932].clone(); // Source Black Sheep.
    assert_eq!(critter.creature_type, 8);
    assert!(critter.tuning.is_none());
    let level = critter.min_level;
    data.templates.insert(1932, critter);
    data.spawns[0].template_entry = 1932;
    let mut app = spawn_continent_fixture(data, area_fixture(1221));
    let world = app.world_mut();
    let actor = fixture_actor(world);
    assert!(
        world.get::<LevelScaling>(actor).is_none(),
        "critters do not take CT2"
    );
    assert_eq!(world.get::<UnitLevel>(actor).unwrap().0, level);
}

#[test]
fn herald_context_explicit_template_precedes_actual_area_tuning() {
    let mut data = sourced_nazan_fixture();
    let hogger = crate::test_data::world_data().templates[&448].clone();
    assert_eq!(hogger.tuning.as_ref().unwrap().content_tuning_id, 73);
    data.templates.insert(448, hogger);
    data.spawns[0].template_entry = 448;
    let mut app = spawn_continent_fixture(data.clone(), area_fixture(1221));
    let world = app.world_mut();
    let actor = fixture_actor(world);
    assert_eq!(
        world.get::<LevelScaling>(actor).unwrap().content_tuning_id,
        73,
        "source Hogger CT73 precedes terrain parent CT2"
    );
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    let content = MapContent {
        map: crate::maps::InMap {
            map_id: RAMPARTS,
            instance_id: 1,
        },
        difficulty_id: 1,
        dead_bosses: vec![],
    };
    assert_eq!(
        spawn_map_creatures(&mut commands, &data, content, false, None),
        1
    );
    queue.apply(app.world_mut());
    let world = app.world_mut();
    let mut actors = world.query::<(&crate::maps::InMap, &LevelScaling)>();
    let (_, scaling) = actors
        .iter(world)
        .find(|(map, _)| map.map_id == RAMPARTS)
        .unwrap();
    assert_eq!(
        scaling.content_tuning_id, 73,
        "Hogger CT73 precedes map543 Normal CT220"
    );
}

#[test]
fn herald_context_ordinary_nazan_spawn_uses_authored_normal_and_heroic_map_tuning() {
    use crate::level_scaling::{LevelStats, TuningInputs};
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let data = sourced_nazan_fixture();
    for (difficulty, tuning_id, health_modifier, damage_modifier) in
        [(1, 220, 9.0, 1.0), (2, 1065, 12.0, 13.0)]
    {
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, app.world());
        let content = MapContent {
            map: crate::maps::InMap {
                map_id: RAMPARTS,
                instance_id: difficulty,
            },
            difficulty_id: difficulty,
            dead_bosses: vec![],
        };
        assert_eq!(
            spawn_map_creatures(&mut commands, &data, content.clone(), false, None),
            1
        );
        queue.apply(app.world_mut());
        let world = app.world_mut();
        let mut actors = world.query::<(Entity, &Npc, &crate::maps::InMap)>();
        let actor = actors
            .iter(world)
            .find(|(_, npc, map)| npc.template_id == NAZAN && **map == content.map)
            .unwrap()
            .0;
        let scaling = world
            .get::<LevelScaling>(actor)
            .expect("ordinary sourced CT0 Nazan must resolve actual map difficulty tuning");
        assert_eq!(scaling.content_tuning_id, tuning_id);
        assert_eq!(scaling.delta, 0, "Nazan retains its authored zero delta");
        let level = scaling.native_level();
        assert!(level > 1, "map range, not unresolved template minimum");
        assert_eq!(world.get::<UnitLevel>(actor).unwrap().0, level);
        let stats = LevelStats::build(
            &data.expected_stats,
            &TuningInputs {
                content_tuning_id: tuning_id,
                health_scaling_expansion: 1,
                unit_class: 1,
                health_modifier,
                damage_modifier,
                armor_modifier: 1.0,
            },
        );
        assert_eq!(
            world.get::<Health>(actor).unwrap().max,
            stats.max_health(level)
        );
        assert_eq!(
            world.get::<CreatureArmor>(actor).unwrap().0,
            stats.armor(level)
        );
        let scaled = world
            .get::<ScaledStats>(actor)
            .expect("regional stats are retained");
        assert_eq!(scaled.0.max_health(10), stats.max_health(10));
        assert_eq!(
            scaled.0.swing_damage_range(level, 2.0),
            stats.swing_damage_range(level, 2.0)
        );
    }
}
