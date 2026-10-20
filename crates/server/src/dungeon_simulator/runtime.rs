//! Manually clocked gameplay App. No server, protocol, replication or socket plugins.
use bevy::prelude::*;
use rand::{SeedableRng, rngs::StdRng};
use shared::components::{Health, Position};
use std::time::Duration;

use super::graph::{Graph, Pack};

const TICK: Duration = Duration::from_millis(50);

pub(super) fn run_stockade(_seed: u64) -> Result<usize, String> {
    Err("full frozen-graph Stockade route is not implemented".into())
}

pub(super) fn build_app() -> App {
    let mut app = App::new();
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
        crate::stats::StatsPlugin,
    ));
    app.init_resource::<crate::maps::InstanceManager>();
    app.init_resource::<crate::spell_cast::test_harness::ObservedCombatEvents>();
    app.add_systems(
        FixedUpdate,
        (crate::followers::simulator::update_leader, move_combatants)
            .chain()
            .after(crate::followers::update_ai),
    );
    app.finish();
    app.cleanup();
    app.update();
    load_gameplay_data(&mut app);
    app
}

fn load_gameplay_data(app: &mut App) {
    let source = crate::spell_cast::test_harness::world_db().unwrap();
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mut data = crate::test_data::world_data().clone();
    // The approved simulation geometry is a plane. Native combat/stat inputs stay intact.
    for spawn in data.spawns.iter_mut().filter(|spawn| spawn.map == 34) {
        spawn.position_z = 0.0;
    }
    app.insert_resource(data.armor_constants.clone());
    app.insert_resource(data);
    app.insert_resource(source.store.clone());
    app.insert_resource(source.classes.clone());
    app.insert_resource(crate::spell_cast::test_harness::power_table(source));
    app.insert_resource(crate::spell_cast::test_harness::shapeshift_forms(source));
    app.insert_resource(crate::item_stats::load_item_scaling_tables(&source.powers_db).unwrap());
    app.insert_resource(crate::faction::load_faction_templates(&db).unwrap());
    app.insert_resource(crate::maps::store::load_map_store(&db).unwrap());
    app.insert_resource(crate::scripted_ai::model_reach::load_model_reaches(&db).unwrap());
    app.insert_resource(crate::unit_creature_type::load_race_types(&db).unwrap());
    app.insert_resource(
        crate::conditions::load_spell_target_conditions(&source.powers_db).unwrap(),
    );
}

fn create_copy(app: &mut App) -> crate::maps::InMap {
    let maps = app.world().resource::<crate::maps::MapStore>().clone();
    app.world_mut()
        .resource_mut::<crate::maps::InstanceManager>()
        .create_map(
            &maps,
            34,
            crate::maps::instances::InstanceEntrant {
                character_id: 1,
                group: None,
                leader: 1,
                difficulty_id: 1,
                ..default()
            },
            crate::maps::instances::Now {
                elapsed: 0.0,
                unix: 1_792_454_400,
            },
        )
        .unwrap()
        .map
}

fn spawn_pack(app: &mut App, copy: crate::maps::InMap, pack: &Pack, seed: u64) -> Vec<Entity> {
    let data = app
        .world()
        .resource::<crate::world_data::WorldData>()
        .clone();
    let mut rng = StdRng::seed_from_u64(seed);
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    let mut context = crate::zones::CreatureSpawnContext {
        map: copy,
        difficulty_id: 1,
        ground: None,
        temporary: false,
    };
    let actors = pack
        .spawns
        .iter()
        .map(|member| {
            let spawn = data
                .spawns
                .iter()
                .find(|spawn| spawn.guid == member.guid)
                .unwrap();
            crate::zones::spawn_creature(
                &mut commands,
                &data,
                data.template_for(member.entry, 1).unwrap(),
                &mut context,
                false,
                spawn,
                &mut rng,
            )
            .expect("source Stockade creature spawn")
        })
        .collect();
    queue.apply(app.world_mut());
    actors
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
    let graph = super::graph::read_graph()?;
    let mut app = build_app();
    let copy = create_copy(&mut app);
    let party = crate::followers::simulator::spawn_party(
        app.world_mut(),
        copy,
        Position {
            x: graph.entrance[0],
            y: 0.0,
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
    let enemies = spawn_pack(&mut app, copy, &pack, 1);
    let at = Position {
        x: pack.position[0] - 2.0,
        y: 0.0,
        z: pack.position[1],
    };
    for unit in party.members.iter().copied().chain([party.owner]) {
        app.world_mut().entity_mut(unit).insert(at);
    }
    let guids = pack.spawns.iter().map(|spawn| spawn.guid).collect();
    if !crate::followers::simulator::pull(app.world_mut(), party.tank, &guids)? {
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

/// Physical movement is replaced, not emulated: all runtime positions are 2D.
/// The production AI and kits still choose pursuit targets, ranges and holds.
fn move_combatants(world: &mut World) {
    let dt = world.resource::<Time>().delta_secs();
    let mut query = world.query::<(
        Entity,
        &Position,
        &Health,
        &crate::creature_ai::CreatureState,
    )>();
    let moves: Vec<_> = query
        .iter(world)
        .filter_map(|(unit, from, health, state)| {
            if health.current <= 0.0
                || world
                    .get::<shared::casting::CastState>(unit)
                    .is_some_and(|cast| !cast.cast_while_moving)
            {
                return None;
            }
            let crate::creature_ai::CreatureState::Pursuing { target } = state else {
                return None;
            };
            let target = Entity::try_from_bits(*target)?;
            let to = world.get::<Position>(target)?;
            let reach = world
                .get::<crate::followers::FollowerReach>(unit)
                .map_or(2.0, |reach| reach.0);
            let speed = world
                .get::<crate::creature_ai::SpawnPoint>(unit)
                .map_or(1.0, |spawn| spawn.speed_run)
                * crate::creature_ai::BASE_RUN_SPEED
                * world
                    .get::<crate::auras::AuraStatMods>(unit)
                    .map_or(1.0, |mods| mods.movement_speed);
            let delta = Vec2::new(to.x - from.x, to.z - from.z);
            let travel = (delta.length() - reach).clamp(0.0, speed * dt);
            let step = delta.normalize_or_zero() * travel;
            Some((
                unit,
                Position {
                    x: from.x + step.x,
                    y: 0.0,
                    z: from.z + step.y,
                },
            ))
        })
        .collect();
    for (unit, to) in moves {
        crate::creature_ai::move_follower(world, unit, to, 7.0);
    }
}
