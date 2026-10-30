//! Manually clocked gameplay App. No server, protocol, replication or socket plugins.
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use shared::components::Position;
use std::time::Duration;

use super::graph::{Graph, Pack};

const TICK: Duration = Duration::from_millis(50);

pub(super) fn output_directory() -> std::path::PathBuf {
    std::env::var_os("DUNGEON_SIMULATOR_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/dungeon-simulator")
        })
}

pub(super) fn write_report(report: &super::campaign::Report) -> Result<(), String> {
    let directory = output_directory();
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let events = report
        .performance
        .get("combat_events")
        .ok_or("final simulator report lacks its native combat-event trace")?;
    let export = serde_json::json!({"seed":report.seed,"status":report.status,
        "reason":report.detail,"events":events});
    std::fs::write(
        directory.join(format!("combat-events-{}.json", report.seed)),
        serde_json::to_vec_pretty(&export).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("write native combat-event export: {error}"))?;
    std::fs::write(
        directory.join(format!("run-{}.json", report.seed)),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .map_err(|error| error.to_string())
}

pub(super) fn print_report(report: &super::campaign::Report) {
    println!(
        "RUN seed={} status={} bosses={:?} pack={:?} sim_s={:.2} wall_s={:.3} detail={}",
        report.seed,
        report.status,
        report.bosses_killed,
        report.failed_pack,
        report.simulated_seconds,
        report.wall_seconds,
        report.detail
    );
}

pub(super) fn run_stockade(seed: u64) -> Result<usize, String> {
    let report = super::campaign::run(seed)?;
    write_report(&report)?;
    print_report(&report);
    if report.status != "cleared" {
        return Err(format!("{}: {}", report.status, report.detail));
    }
    Ok(report.bosses_killed.len())
}

#[test]
fn native_evade_at_home_completes_ordinary_return_lifecycle() {
    let mut app = build_app();
    let copy = create_copy(&mut app);
    let actors = spawn_population(&mut app, copy);
    let actor = actors[&20375744]; // Actual Stockade Rumbling Earth from the stalled seed12.
    crate::creature_aggro::enter_evade_mode(app.world_mut(), actor);
    assert_eq!(
        app.world().get::<crate::creature_ai::CreatureState>(actor),
        Some(&crate::creature_ai::CreatureState::Returning)
    );
    app.update();
    app.update();
    assert!(
        app.world()
            .get::<shared::components::Health>(actor)
            .is_some_and(|health| health.current > 0.0)
    );
    assert_ne!(
        app.world().get::<crate::creature_ai::CreatureState>(actor),
        Some(&crate::creature_ai::CreatureState::Returning),
        "a real living actor already at its native home must complete return instead of permanently blocking the active-fight history"
    );
}

#[test]
fn native_evade_walks_frozen_home_route_before_completing_return() {
    let mut app = build_app();
    let copy = create_copy(&mut app);
    let actors = spawn_population(&mut app, copy);
    let actor = actors[&20375744];
    let home = *app.world().get::<Position>(actor).unwrap();
    crate::creature_aggro::enter_evade_mode(app.world_mut(), actor);
    app.world_mut().entity_mut(actor).insert(Position {
        x: home.x + 4.0,
        ..home
    });
    app.update();
    let first_step = *app.world().get::<Position>(actor).unwrap();
    assert!(
        first_step.x < home.x + 4.0,
        "Returning must enter the frozen movement adapter"
    );
    assert!(
        first_step.x > home.x,
        "returning four yards cannot teleport home in one50ms tick"
    );
    for _ in 0..80 {
        app.update();
    }
    assert_eq!(*app.world().get::<Position>(actor).unwrap(), home);
    assert_eq!(
        app.world().get::<crate::creature_ai::CreatureState>(actor),
        Some(&crate::creature_ai::CreatureState::Idle)
    );
}

pub(super) fn build_app() -> App {
    build_cached_app(&mut None)
}

pub(super) fn build_cached_app(catalogs: &mut Option<super::catalogs::Catalogs>) -> App {
    build_runtime_app(catalogs, &crate::test_data::world_db_path())
}

#[cfg(test)]
pub(super) fn build_app_with_world_db(path: &std::path::Path) -> App {
    build_runtime_app(&mut None, path)
}

fn build_runtime_app(
    catalogs: &mut Option<super::catalogs::Catalogs>,
    path: &std::path::Path,
) -> App {
    let mut app = App::new();
    app.add_plugins(server_spell_scripts::SpellScriptsPlugin);
    app.insert_resource(super::Space(super::graph::read_graph().unwrap()));
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(TICK));
    app.insert_resource(Time::<Fixed>::from_duration(TICK));
    app.add_plugins((
        crate::combat::CombatPlugin,
        crate::spell_cast::SpellCastPlugin,
        crate::auras::AuraPlugin,
        crate::creature_aggro::CreatureAggroPlugin,
        crate::creature_scripts::CreatureScriptsPlugin,
        crate::scripted_ai::ScriptedAiPlugin,
        crate::followers::FollowerPlugin,
        crate::hunter_pet::HunterPetPlugin,
        crate::pet_control::PetControlPlugin,
        crate::stats::StatsPlugin,
    ));
    server::test_support::install_player_death(&mut app);
    if app.world().resource::<super::Space>().0.map_id == 2825 {
        let db =
            rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .expect("open interaction objective catalog");
        server::test_support::objectives::install(&mut app, &db)
            .expect("install interaction objectives");
    }
    app.add_systems(FixedUpdate, crate::zones::apply_addon_auras);
    app.init_resource::<crate::maps::InstanceManager>();
    app.init_resource::<crate::spell_cast::test_harness::ObservedCombatEvents>();
    app.init_resource::<crate::spell_cast::holy_metrics::HolyMetrics>();
    app.add_systems(
        FixedUpdate,
        (
            crate::followers::simulator::update_leader,
            super::movement::update,
            crate::creature_ai::no_path_evade_system,
        )
            .chain()
            .after(crate::followers::update_ai),
    );
    for label in [
        Startup.intern(),
        FixedPreUpdate.intern(),
        FixedUpdate.intern(),
        FixedPostUpdate.intern(),
        Update.intern(),
    ] {
        app.edit_schedule(label, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
    }
    let reused = catalogs.is_some();
    if let Some(catalogs) = catalogs.take() {
        catalogs.install(app.world_mut());
        // Plugin Build still creates fresh dynamic state and systems. Startup only
        // loads definitions/registers function pointers in this transport-free App.
        app.add_schedule(bevy::ecs::schedule::Schedule::new(Startup));
    }
    app.finish();
    app.cleanup();
    app.update();
    if !reused {
        load_gameplay_data(&mut app, path);
    }
    if !app.world().resource::<super::Space>().0.flights.is_empty() {
        let db =
            rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .expect("open flight catalogs");
        let root = super::graph::extraction::asset_root();
        let nav = std::env::var_os("DUNGEON_SIMULATOR_NAVMESH")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| root.join("navmesh"));
        server::test_support::flight::install(&mut app, &db, &nav, &root)
            .expect("install native simulator flight");
        app.add_systems(
            FixedUpdate,
            server::test_support::flight::update.after(super::movement::update),
        );
    }
    let has_accessory_boss = app
        .world()
        .resource::<super::Space>()
        .0
        .follower_boss_sources
        .iter()
        .any(|(boss, source)| boss != source);
    let has_taxi = app
        .world()
        .resource::<super::Space>()
        .0
        .transports
        .iter()
        .any(|edge| {
            matches!(
                edge.action,
                server::test_support::transport::Action::SpellClick
                    | server::test_support::transport::Action::TaxiProximity
            )
        });
    let has_drop = app
        .world()
        .resource::<super::Space>()
        .0
        .transports
        .iter()
        .any(|edge| edge.action == server::test_support::transport::Action::CollisionDrop);
    if has_drop {
        let db =
            rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .expect("open collision drop catalog");
        let nav = std::env::var_os("DUNGEON_SIMULATOR_NAVMESH")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| super::graph::extraction::asset_root().join("navmesh"));
        server::test_support::transport::install_drop_system(&mut app, &db, &nav)
            .expect("install native collision drop system");
    }
    if has_taxi {
        let db =
            rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .expect("open taxi transport catalog");
        server::test_support::transport::install_taxi_system(&mut app, &db)
            .expect("install native simulator taxi transport");
    }
    if has_accessory_boss {
        let db =
            rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .expect("open vehicle accessory catalog");
        server::test_support::install_vehicle_accessories(
            &mut app,
            &super::graph::extraction::asset_root(),
            &db,
        )
        .expect("install native simulator vehicle accessories");
    }
    app
}

fn load_gameplay_data(app: &mut App, path: &std::path::Path) {
    let db =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    app.insert_resource(
        crate::item_proc_data::load(&db).expect("load native equipped-producer catalogs"),
    );
    let data = crate::world_data::load_world_data(path);
    app.insert_resource(data.armor_constants.clone());
    app.insert_resource(data);
    let (spells, classes) = crate::spell_info::load_spell_data(&db).unwrap();
    app.insert_resource(spells);
    app.insert_resource(classes);
    app.insert_resource(crate::item_stats::load_item_scaling_tables(path).unwrap());
    app.insert_resource(crate::item_templates::load_item_templates(path).unwrap());
    app.insert_resource(crate::stats::NativeItemInstances {
        bonuses: server::item_instance::load::load_bonus_tables(&db).unwrap(),
        enchants: server::item_instance::load::load_enchantment_tables(&db).unwrap(),
        effects: server::item_instance::load::load_item_effect_tables(&db).unwrap(),
    });
    app.insert_resource(crate::faction::load_faction_templates(&db).unwrap());
    app.insert_resource(crate::maps::store::load_map_store(&db).unwrap());
    app.insert_resource(crate::scripted_ai::model_reach::load_model_reaches(&db).unwrap());
    app.insert_resource(crate::unit_creature_type::load_race_types(&db).unwrap());
    app.insert_resource(crate::conditions::load_spell_target_conditions(path).unwrap());
}

pub(super) fn create_copy(app: &mut App) -> crate::maps::InMap {
    let graph = &app.world().resource::<super::Space>().0;
    let (map_id, difficulty_id) = (graph.map_id, graph.difficulty_id());
    app.world_mut()
        .resource_scope(|world, maps: Mut<crate::maps::MapStore>| {
            world
                .resource_mut::<crate::maps::InstanceManager>()
                .create_map(
                    &maps,
                    map_id,
                    crate::maps::instances::InstanceEntrant {
                        character_id: 1,
                        group: None,
                        leader: 1,
                        difficulty_id,
                        ..default()
                    },
                    crate::maps::instances::Now {
                        elapsed: 0.0,
                        unix: 1_792_454_400,
                    },
                )
                .unwrap()
                .map
        })
}

pub(super) fn spawn_population(
    app: &mut App,
    copy: crate::maps::InMap,
) -> std::collections::BTreeMap<u32, Entity> {
    let difficulty_id = app
        .world()
        .resource::<crate::maps::InstanceManager>()
        .copy_difficulty(copy)
        .expect("native simulator copy difficulty")
        .difficulty_id;
    let mut queue = bevy::ecs::world::CommandQueue::default();
    app.world_mut()
        .resource_scope(|world, data: Mut<crate::world_data::WorldData>| {
            let mut commands = Commands::new(&mut queue, world);
            crate::zones::spawn_map_creatures(
                &mut commands,
                &data,
                crate::zones::MapContent {
                    map: copy,
                    difficulty_id,
                    dead_bosses: Vec::new(),
                },
                false,
                None,
            );
        });
    queue.apply(app.world_mut());
    live_population(app.world_mut(), copy)
}

pub(super) fn live_population(
    world: &mut World,
    copy: crate::maps::InMap,
) -> std::collections::BTreeMap<u32, Entity> {
    world
        .query::<(Entity, &crate::zones::CreatureOrigin, &crate::maps::InMap)>()
        .iter(world)
        .filter(|(_, source, map)| **map == copy && source.guid != 0)
        .map(|(actor, source, _)| (source.guid, actor))
        .collect()
}

fn first_hostile_pack<'a>(app: &mut App, graph: &'a Graph, tank: Entity) -> &'a Pack {
    let data = app.world().resource::<crate::world_data::WorldData>();
    let factions = app.world().resource::<crate::faction::FactionTemplates>();
    let friendly = app
        .world()
        .get::<shared::components::UnitFactionTemplate>(tank)
        .unwrap()
        .0;
    graph
        .packs
        .iter()
        .filter(|pack| {
            pack.spawns.iter().any(|spawn| {
                let template = data.template_for(spawn.entry, 1).unwrap();
                factions.reaction(u32::from(template.faction), friendly)
                    == crate::faction::Reaction::Hostile
                    && template.aggro_mode != crate::creature_aggro::AggroMode::Passive
            })
        })
        .min_by(|a, b| {
            let distance = |pack: &Pack| {
                (pack.position[0] - graph.entrance[0]).hypot(pack.position[1] - graph.entrance[1])
            };
            distance(a).total_cmp(&distance(b)).then(a.id.cmp(&b.id))
        })
        .expect("Stockade has hostile packs")
}

pub(super) fn run_first_pack() -> Result<(), String> {
    let _rng_scope = crate::random::seed_scope(1);
    let graph = super::graph::read_graph()?;
    let mut app = build_app();
    let copy = create_copy(&mut app);
    let party = crate::followers::simulator::spawn_party(
        app.world_mut(),
        copy,
        Position {
            x: graph.entrance[0],
            y: graph.entrance_height,
            z: graph.entrance[1],
        },
    )?;
    let pack = first_hostile_pack(&mut app, &graph, party.tank).clone();
    // Step-one qualification deliberately isolates one pack, not a dungeon clear.
    let selected: std::collections::BTreeSet<_> =
        pack.spawns.iter().map(|spawn| spawn.guid).collect();
    {
        let mut data = app
            .world_mut()
            .resource_mut::<crate::world_data::WorldData>();
        let indices = data.spawns_by_map[&34]
            .iter()
            .copied()
            .filter(|&index| selected.contains(&data.spawns[index].guid))
            .collect();
        data.spawns_by_map.insert(34, indices);
    }
    let population = spawn_population(&mut app, copy);
    let enemies: Vec<_> = pack
        .spawns
        .iter()
        .map(|spawn| population[&spawn.guid])
        .collect();
    let at = Position {
        x: pack.position[0] - 2.0,
        y: pack.height,
        z: pack.position[1],
    };
    for unit in party.members.iter().copied().chain([party.owner]) {
        app.world_mut().entity_mut(unit).insert(at);
    }
    let guids = pack.spawns.iter().map(|spawn| spawn.guid).collect();
    if !crate::followers::simulator::pull(app.world_mut(), party.tank, &guids)?.0 {
        return Err(format!(
            "source pull budget refused opening pack{}",
            pack.id
        ));
    }
    let first = enemies[0];
    crate::combat_refs::set_in_combat_with(app.world_mut(), party.tank, first);
    crate::creature_aggro::add_threat(app.world_mut(), first, party.tank, 1.0);
    for _ in 0..12_000 {
        app.update();
        if enemies
            .iter()
            .all(|enemy| !crate::spell_cast::targets::is_alive(app.world(), *enemy))
        {
            let events = &app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0;
            if !events.iter().any(|event| {
                event.kind == shared::protocol::CombatLogKind::Damage && event.amount > 0
            }) {
                return Err("pack death lacked real damage events".into());
            }
            println!(
                "PACK_CLEARED {} at {:.2}s; {} real combat events",
                pack.id,
                app.world().resource::<Time<Virtual>>().elapsed_secs_f64(),
                events.len()
            );
            return Ok(());
        }
        if party
            .members
            .iter()
            .copied()
            .chain([party.owner])
            .all(|unit| !crate::spell_cast::targets::is_alive(app.world(), unit))
        {
            return Err(format!("party wiped on pack{}", pack.id));
        }
    }
    Err(format!("one-pack simulation stalled on pack{}", pack.id))
}
