//! Northshire creatures fighting each other, spawned from the imported world.db
//! (`cargo test -p server npc_combat_tests -- --ignored`).

use std::path::PathBuf;
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
    DATA.get_or_init(|| {
        load_world_data(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db"))
    })
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
    app.insert_resource(load_world_data(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db"),
    ));
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

#[test]
#[ignore]
fn northshire_infantry_and_the_worg_beside_it_are_neutral_and_never_fight() {
    let infantry = spawn_of(INFANTRY);
    let worg = spawn_of(WORG_BY_INFANTRY);
    assert_eq!((infantry.template_entry, worg.template_entry), (49_869, 49_871));
    let Some(mut app) = fight_app(vec![placed(infantry, 49_869, 0.0), placed(worg, 49_871, 2.0)])
    else {
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
#[ignore]
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
    assert_eq!(state(&app, soldier), pursuing(invader), "the soldier attacks");
    assert_eq!(state(&app, invader), pursuing(soldier), "the invader attacks");

    run(&mut app, 100);
    assert!(health(&app, soldier).0 < health(&app, soldier).1, "the invader hits");
    assert!(health(&app, invader).0 < health(&app, invader).1, "the soldier hits");

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
    let survivor = if dead(&app, invader) { soldier } else { invader };
    run(&mut app, 200);
    assert_eq!(state(&app, survivor), CreatureState::Idle, "the survivor is home");
    assert!(app.world().get::<AutoAttack>(survivor).is_none());
    assert!(
        app.world().get::<ThreatTable>(survivor).unwrap().top_threat().is_none(),
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
            app.world().get::<AutoAttack>(player).map(|swing| swing.victim),
            Some(invader),
            "the player's auto-attack is still on after {ticks} ticks"
        );
    }
    assert_eq!(
        app.world().get::<CreatureTap>(invader).map(|tap| tap.tappers.clone()),
        Some(vec![7]),
        "the player's own damage tapped it"
    );
}
