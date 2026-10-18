//! Northshire creatures fighting each other, spawned from the imported world.db
//! (`cargo test -p server npc_combat_tests -- --ignored`).

use std::sync::OnceLock;

use bevy::time::TimeUpdateStrategy;
use shared::components::{Player, ThreatTable};

use super::*;
use crate::combat::{AutoAttack, DeathState};
use crate::creature_ai::CreatureState;
use crate::creature_tap::CreatureTap;
use crate::networking::CombatTarget;
use crate::world_data::load_world_data;

/// Retail spawns are TrinityCore guid + `RETAIL_GUID_OFFSET` (retail_content.py).
const RETAIL: u32 = 20_000_000;
/// Stormwind Infantry (49869, faction 2321, TrinityCore guid 279928) and the Blackrock
/// Worg (49871, faction 32, civilian, guid 279929) spawned 1.3 yd from it.
const INFANTRY: u32 = RETAIL + 279_928;
const WORG_BY_INFANTRY: u32 = RETAIL + 279_929;
/// A Blackrock Invader (42937, faction 14) and a Northshire Guard (1642, faction 11,
/// level 90: a 45 yd aggro radius against the invader).
const INVADER: u32 = RETAIL + 279_831;
const NORTHSHIRE_GUARD_ENTRY: u32 = 1642;
/// Ticks of one relocation notify period (1 s at 50 ms).
const NOTIFY_TICKS: usize = 20;

fn world_data() -> &'static WorldData {
    static DATA: OnceLock<WorldData> = OnceLock::new();
    DATA.get_or_init(|| load_world_data(&crate::test_data::world_db_path()))
}

fn spawn_of(guid: u32) -> &'static CreatureSpawn {
    world_data()
        .spawns
        .iter()
        .find(|spawn| spawn.guid == guid)
        .expect("spawn in world.db")
}

/// `spawn` (or a creature of `entry` in its place) moved `dx` WoW yards along x,
/// standing still.
fn placed(spawn: &CreatureSpawn, entry: u32, dx: f32) -> CreatureSpawn {
    CreatureSpawn {
        guid: spawn.guid,
        template_entry: entry,
        map: spawn.map,
        zone_id: spawn.zone_id,
        position_x: spawn.position_x + dx,
        position_y: spawn.position_y,
        position_z: spawn.position_z,
        orientation: spawn.orientation,
        wander_distance: 0.0,
        movement_type: 0,
        spawntimesecs: spawn.spawntimesecs,
        display_id: None,
        unit_flags: None,
        spawn_difficulties: Vec::new(),
        equipment_id: 0,
    }
}

#[derive(Resource)]
struct Spawned(Vec<Entity>);

/// The server's creature, combat and aggro plugins with real templates and factions,
/// running only `spawns`.
fn fight_app(spawns: Vec<CreatureSpawn>) -> Option<App> {
    let templates = crate::faction::world_db_faction_templates()?;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(50),
    ));
    app.insert_resource(Time::<Fixed>::from_duration(
        std::time::Duration::from_millis(50),
    ));
    app.insert_resource(load_world_data(&crate::test_data::world_db_path()));
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    app.insert_resource(crate::scripted_ai::model_reach::load_model_reaches(&conn).unwrap());
    app.insert_resource(templates);
    app.add_plugins((
        crate::creature_ai::CreatureAiPlugin,
        crate::combat::CombatPlugin,
        crate::creature_aggro::CreatureAggroPlugin,
    ));
    app.add_systems(
        PostStartup,
        move |mut commands: Commands, world_data: Res<WorldData>| {
            let spawned = spawn_creatures(
                &mut commands,
                &world_data,
                crate::maps::InMap::continent(0),
                0,
                false,
                spawns.iter(),
                Some(&mut crate::test_data::ground_map()),
            );
            let entities = spawns.iter().map(|spawn| spawned[&spawn.guid]).collect();
            commands.insert_resource(Spawned(entities));
        },
    );
    app.update();
    Some(app)
}

fn spawned(app: &App, index: usize) -> Entity {
    app.world().resource::<Spawned>().0[index]
}

fn run(app: &mut App, updates: usize) {
    for _ in 0..updates {
        app.update();
    }
}

fn state(app: &App, creature: Entity) -> CreatureState {
    *app.world().get::<CreatureState>(creature).unwrap()
}

fn pursuing(target: Entity) -> CreatureState {
    CreatureState::Pursuing {
        target: target.to_bits(),
    }
}

fn health(app: &App, unit: Entity) -> (f32, f32) {
    let health = app.world().get::<Health>(unit).unwrap();
    (health.current, health.max)
}

fn dead(app: &App, unit: Entity) -> bool {
    app.world().get::<DeathState>(unit).is_some()
}

fn assert_melee_attempts(app: &App, attacker: Entity, target: Entity) {
    use shared::protocol::CombatEventType;
    let log = app.world().resource::<crate::combat::CombatLog>();
    let attempts: Vec<_> = log
        .range(0.0, f64::MAX)
        .into_iter()
        .filter(|entry| {
            entry.event.attacker == attacker.to_bits()
                && entry.event.target == target.to_bits()
                && entry.event.spell_id == 0
                && matches!(
                    entry.event.event_type,
                    CombatEventType::MeleeDamage
                        | CombatEventType::CriticalHit
                        | CombatEventType::Block
                        | CombatEventType::Miss
                        | CombatEventType::Dodge
                        | CombatEventType::Parry
                )
        })
        .collect();
    assert!(
        !attempts.is_empty(),
        "{attacker:?} never swung at {target:?}"
    );
    for entry in attempts {
        let event = &entry.event;
        match event.event_type {
            CombatEventType::Miss | CombatEventType::Dodge | CombatEventType::Parry => {
                assert_eq!(event.amount, 0.0, "{event:?}");
            }
            _ => {
                assert!(event.amount > 0.0 && event.amount.is_finite(), "{event:?}");
                assert!(health(app, target).0 < health(app, target).1, "{event:?}");
            }
        }
    }
}

#[test]
#[ignore]
fn northshire_infantry_and_the_worg_beside_it_are_neutral_and_never_fight() {
    let infantry = spawn_of(INFANTRY);
    let worg = spawn_of(WORG_BY_INFANTRY);
    assert_eq!(
        (infantry.template_entry, worg.template_entry),
        (49_869, 49_871)
    );
    let Some(mut app) = fight_app(vec![
        placed(infantry, 49_869, 0.0),
        placed(worg, 49_871, 2.0),
    ]) else {
        return;
    };
    let (soldier, worg) = (spawned(&app, 0), spawned(&app, 1));
    run(&mut app, 5 * NOTIFY_TICKS);
    assert_eq!(state(&app, soldier), CreatureState::Idle);
    assert_eq!(state(&app, worg), CreatureState::Idle);
    let (current, max) = health(&app, worg);
    assert_eq!(current, max);
}

#[test]
fn northshire_infantry_and_a_blackrock_invader_fight_until_one_dies_or_evades() {
    let infantry = spawn_of(INFANTRY);
    assert_eq!(spawn_of(INVADER).template_entry, 42_937);
    let Some(mut app) = fight_app(vec![
        placed(infantry, 49_869, 0.0),
        CreatureSpawn {
            guid: INVADER,
            ..placed(infantry, 42_937, 12.0)
        },
    ]) else {
        return;
    };
    let (soldier, invader) = (spawned(&app, 0), spawned(&app, 1));

    run(&mut app, NOTIFY_TICKS + 1);
    assert_eq!(
        state(&app, soldier),
        pursuing(invader),
        "the soldier attacks"
    );
    assert_eq!(
        state(&app, invader),
        pursuing(soldier),
        "the invader attacks"
    );

    run(&mut app, 100);
    assert_melee_attempts(&app, invader, soldier);
    assert_melee_attempts(&app, soldier, invader);

    let mut ticks = 0;
    while !dead(&app, soldier) && !dead(&app, invader) {
        run(&mut app, 1);
        ticks += 1;
        if ticks > 20 * 600 {
            panic!(
                "no death after 10 minutes: soldier {:?}, invader {:?}",
                health(&app, soldier),
                health(&app, invader)
            );
        }
    }
    let survivor = if dead(&app, invader) {
        soldier
    } else {
        invader
    };
    run(&mut app, 200);
    assert_eq!(
        state(&app, survivor),
        CreatureState::Idle,
        "the survivor is home"
    );
    assert!(app.world().get::<AutoAttack>(survivor).is_none());
    assert!(
        app.world()
            .get::<ThreatTable>(survivor)
            .unwrap()
            .top_threat()
            .is_none(),
        "its fight is over"
    );
}

#[test]
#[ignore]
fn a_northshire_guard_attacks_a_blackrock_invader_that_comes_close() {
    let infantry = spawn_of(INFANTRY);
    let Some(mut app) = fight_app(vec![
        placed(infantry, NORTHSHIRE_GUARD_ENTRY, 0.0),
        CreatureSpawn {
            guid: INVADER,
            ..placed(infantry, 42_937, 50.0)
        },
    ]) else {
        return;
    };
    let (guard, invader) = (spawned(&app, 0), spawned(&app, 1));
    run(&mut app, 2 * NOTIFY_TICKS);
    assert_eq!(
        state(&app, guard),
        CreatureState::Idle,
        "50 yd: beyond any aggro radius"
    );

    let guard_position = *app.world().get::<Position>(guard).unwrap();
    *app.world_mut().get_mut::<Position>(invader).unwrap() = Position {
        x: guard_position.x + 10.0,
        ..guard_position
    };
    app.world()
        .resource::<crate::interest::InterestDirtyQueue>()
        .mark(invader);
    run(&mut app, NOTIFY_TICKS + 1);
    assert_eq!(state(&app, guard), pursuing(invader));
}

/// The level 10 Human player of character 7, `dx` WoW yards from `spawn`.
fn spawn_player_near(app: &mut App, spawn: &CreatureSpawn, dx: f32) -> Entity {
    let (x, y, z) = crate::terrain_height::wow_to_bevy(
        spawn.position_x + dx,
        spawn.position_y,
        spawn.position_z,
    );
    app.world_mut()
        .spawn((
            Position { x, y, z },
            crate::maps::InMap::continent(0),
            Player {
                name: "Anduin".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            Health {
                current: 100_000.0,
                max: 100_000.0,
            },
            UnitLevel(10),
            UnitFactionTemplate(crate::faction::faction_template_for_race(1)),
            CombatTarget::default(),
            crate::persistence::ClientId(7),
        ))
        .id()
}

#[test]
#[ignore]
fn a_player_hitting_an_invader_that_fights_infantry_taps_it_and_keeps_swinging() {
    let infantry = spawn_of(INFANTRY);
    let Some(mut app) = fight_app(vec![
        placed(infantry, 49_869, 0.0),
        CreatureSpawn {
            guid: INVADER,
            ..placed(infantry, 42_937, 4.0)
        },
    ]) else {
        return;
    };
    let (soldier, invader) = (spawned(&app, 0), spawned(&app, 1));
    // The soldier must not win the race: it barely scratches the invader.
    app.world_mut().get_mut::<Health>(soldier).unwrap().max = 1.0e9;
    app.world_mut().get_mut::<Health>(soldier).unwrap().current = 1.0e9;
    run(&mut app, NOTIFY_TICKS + 40);
    assert_eq!(state(&app, invader), pursuing(soldier), "the fight is on");

    let player = spawn_player_near(&mut app, infantry, 6.0);
    app.world_mut().get_mut::<CombatTarget>(player).unwrap().0 = Some(invader);
    app.world_mut()
        .entity_mut(player)
        .insert(AutoAttack::new(invader));
    let mut ticks = 0;
    while !dead(&app, invader) {
        run(&mut app, 1);
        ticks += 1;
        assert!(ticks < 20 * 600, "the invader dies");
        if dead(&app, invader) {
            break;
        }
        assert_eq!(
            app.world()
                .get::<AutoAttack>(player)
                .map(|swing| swing.victim),
            Some(invader),
            "the player's auto-attack is still on after {ticks} ticks"
        );
    }
    assert_eq!(
        app.world()
            .get::<CreatureTap>(invader)
            .map(|tap| tap.tappers.clone()),
        Some(vec![7]),
        "the player's own damage tapped it"
    );
}

/// Mean wall time of one 50 ms tick of the creature, combat and aggro plugins running
/// the world.db spawns within 300 yd of WoW `(x, y)` on map 0 and a level 1 human player
/// at the first spawn, after a warm-up; with the line of sight bake loaded when `los`,
/// and then the LOS segments tested per tick.
fn tick_cost(x: f32, y: f32, los: bool) -> Option<(usize, std::time::Duration, Option<f64>)> {
    let spawns: Vec<CreatureSpawn> = world_data()
        .spawns
        .iter()
        .filter(|spawn| {
            spawn.map == 0
                && (spawn.position_x - x).abs() < 300.0
                && (spawn.position_y - y).abs() < 300.0
        })
        .map(|spawn| CreatureSpawn {
            wander_distance: spawn.wander_distance,
            movement_type: spawn.movement_type,
            display_id: spawn.display_id,
            unit_flags: spawn.unit_flags,
            equipment_id: spawn.equipment_id,
            ..placed(spawn, spawn.template_entry, 0.0)
        })
        .collect();
    let count = spawns.len();
    let first = (
        spawns[0].position_x,
        spawns[0].position_y,
        spawns[0].position_z,
    );
    let mut app = if los {
        los_fight_app(spawns)?
    } else {
        fight_app(spawns)?
    };
    player_at(&mut app, first.0, first.1, first.2);
    run(&mut app, 20 * 30);
    let ticks = 20 * 120;
    let rays = |app: &App| {
        app.world()
            .get_resource::<crate::line_of_sight::LineOfSight>()
            .map(|los| los.rays())
    };
    let rays_before = rays(&app);
    let start = std::time::Instant::now();
    run(&mut app, ticks);
    let elapsed = start.elapsed();
    let per_tick = rays_before
        .zip(rays(&app))
        .map(|(before, after)| (after - before) as f64 / ticks as f64);
    if let Some(warm_up) = rays_before {
        println!("LOS rays in the {} warm-up ticks: {warm_up}", 20 * 30);
    }
    Some((count, elapsed / ticks as u32, per_tick))
}

fn print_tick_cost(zone: &str, x: f32, y: f32) {
    for los in [false, true] {
        if let Some((count, mean, rays)) = tick_cost(x, y, los) {
            println!(
                "{zone}_tick_cost: {count} spawns, LOS {los}: {mean:?} per tick, LOS rays per \
                 tick {rays:?}"
            );
        }
    }
}

/// `cargo test --release -p server _tick_cost -- --ignored --nocapture --test-threads 1`
#[test]
#[ignore]
fn northshire_tick_cost() {
    let center = spawn_of(INFANTRY);
    print_tick_cost("northshire", center.position_x, center.position_y);
}

/// Stormwind's Trade District.
#[test]
#[ignore]
fn stormwind_tick_cost() {
    print_tick_cost("stormwind", -8830.0, 630.0);
}

/// `fight_app` with the line of sight baked in the ground directory (`los-bake --map 0`).
fn los_fight_app(spawns: Vec<CreatureSpawn>) -> Option<App> {
    let mut app = fight_app(spawns)?;
    let dir = crate::ground::ground_data_dir().join("los");
    app.insert_resource(crate::line_of_sight::LineOfSight::load(&dir).unwrap());
    Some(app)
}

/// A level 60 Blackrock Invader (faction 14, hostile to all) standing at WoW `(x, y, z)`:
/// a 45 yd aggro radius against a level 1 player.
fn invader_at(x: f32, y: f32, z: f32) -> CreatureSpawn {
    CreatureSpawn {
        position_x: x,
        position_y: y,
        position_z: z,
        ..placed(spawn_of(INVADER), 42_937, 0.0)
    }
}

fn level_60(app: &mut App, creature: Entity) {
    app.world_mut()
        .entity_mut(creature)
        .insert(UnitLevel(60))
        .remove::<shared::level_scaling::LevelScaling>();
}

/// A level 1 human male player standing at WoW `(x, y, z)`.
fn player_at(app: &mut App, x: f32, y: f32, z: f32) -> Entity {
    let (x, y, z) = crate::terrain_height::wow_to_bevy(x, y, z);
    app.world_mut()
        .spawn((
            Position { x, y, z },
            crate::maps::InMap::continent(0),
            Player {
                name: "Anduin".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            Health {
                current: 100_000.0,
                max: 100_000.0,
            },
            UnitLevel(1),
            UnitFactionTemplate(crate::faction::faction_template_for_race(1)),
            CombatTarget::default(),
        ))
        .id()
}

/// Stormwind's outer wall west of the main gate: the invader on the street behind it
/// (floor heights from the LOS bake), the player on the Elwynn side 40 yd away.
const STREET: (f32, f32, f32) = (-8831.9, 492.8, 98.26);
const BEHIND_THE_WALL: (f32, f32, f32) = (-8862.27, 471.05, 83.10);
/// The same street 20 yd from the invader, nothing between them.
const UP_THE_STREET: (f32, f32, f32) = (-8845.9, 506.8, 98.26);

fn aggro_at(creature: (f32, f32, f32), player: (f32, f32, f32)) -> Option<bool> {
    let mut app = los_fight_app(vec![invader_at(creature.0, creature.1, creature.2)])?;
    let invader = spawned(&app, 0);
    level_60(&mut app, invader);
    let player = player_at(&mut app, player.0, player.1, player.2);
    run(&mut app, NOTIFY_TICKS);
    Some(state(&app, invader) == pursuing(player))
}

#[test]
#[ignore]
fn a_creature_behind_the_stormwind_wall_does_not_aggro_a_player_it_would_in_the_open() {
    let Some(open) = aggro_at(STREET, UP_THE_STREET) else {
        return;
    };
    assert!(open, "the invader attacks the player up the street");
    assert_eq!(
        aggro_at(STREET, BEHIND_THE_WALL),
        Some(false),
        "the wall hides the player"
    );
}

#[test]
#[ignore]
fn a_creature_in_northshire_abbey_aggroes_a_player_through_the_doorway() {
    // The invader in the hall, the player on the steps of the west door 30 yd away.
    assert_eq!(
        aggro_at((-8896.5, -173.8, 81.58), (-8909.2, -146.6, 81.95)),
        Some(true)
    );
    // The same line 4 yd to the side runs through the wall beside the door.
    assert_eq!(
        aggro_at((-8892.875, -172.11, 81.58), (-8905.575, -144.91, 81.96)),
        Some(false)
    );
}
