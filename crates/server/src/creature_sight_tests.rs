use std::collections::HashMap;
use std::time::Duration;

use bevy::time::TimeUpdateStrategy;
use shared::components::{CombatStatus, Player};

use super::*;
use crate::combat::{AutoAttack, CombatPlugin};
use crate::creature_aggro::{AggroMode, CreatureAggroPlugin, insert_aggro_components};
use crate::creature_ai::CreatureAiPlugin;
use crate::faction_reaction::FactionTemplateEntry;
use crate::networking::CombatTarget;

/// FactionTemplate 2321 (Stormwind Infantry 49869): Stormwind, Alliance, enemy of Horde
/// and monsters.
const INFANTRY: u32 = 2321;
/// FactionTemplate 14 (Blackrock Invader 42937): monster, enemy of players.
const INVADER: u32 = 14;
/// FactionTemplate 32 (Blackrock Worg 49871): "Beast - Wolf", no groups, enemy of Prey.
const WORG: u32 = 32;
/// FactionTemplate 1 (Human player).
const HUMAN: u32 = 1;

/// The Retail 12.1.0.69933 rows of those templates.
fn faction_templates() -> FactionTemplates {
    let entry = |id, faction, flags, faction_group, friend_group, enemy_group, enemy| {
        (
            id,
            FactionTemplateEntry {
                id,
                faction,
                flags,
                faction_group,
                friend_group,
                enemy_group,
                enemies: [enemy, 0, 0, 0, 0, 0, 0, 0],
                ..Default::default()
            },
        )
    };
    FactionTemplates(HashMap::from([
        entry(INFANTRY, 72, 2080, 3, 2, 12, 0),
        entry(INVADER, 14, 0, 8, 0, 1, 0),
        entry(WORG, 29, 16, 0, 0, 0, 28),
        entry(HUMAN, 1, 72, 3, 2, 12, 0),
    ]))
}

fn sight_app() -> App {
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        50,
    )));
    app.insert_resource(crate::spell_cast::test_harness::load_native_creature_world(
        &[30],
    ));
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    app.insert_resource(crate::scripted_ai::model_reach::load_model_reaches(&conn).unwrap());
    app.insert_resource(faction_templates());
    app.add_plugins((CombatPlugin, CreatureAiPlugin, CreatureAggroPlugin));
    app.update();
    app
}

fn at(x: f32) -> Position {
    Position { x, y: 0.0, z: 0.0 }
}

struct Fixture {
    name: &'static str,
    faction: u32,
    mode: AggroMode,
    health: f32,
}

const SOLDIER: Fixture = Fixture {
    name: "Stormwind Infantry",
    faction: INFANTRY,
    mode: AggroMode::Aggressive,
    health: 60.0,
};

const ORC: Fixture = Fixture {
    name: "Blackrock Invader",
    faction: INVADER,
    mode: AggroMode::Aggressive,
    health: 60.0,
};

/// A civilian (`flags_extra` 2) like the Blackrock Worg.
const CIVILIAN_ORC: Fixture = Fixture {
    mode: AggroMode::Reactive,
    ..ORC
};

const WORG_FIXTURE: Fixture = Fixture {
    name: "Blackrock Worg",
    faction: WORG,
    mode: AggroMode::Reactive,
    health: 60.0,
};

/// A level 10 creature spawned through the world spawn component path, its swings
/// never missing.
fn spawn(app: &mut App, fixture: &Fixture, position: Position) -> Entity {
    let display_id = app.world().resource::<WorldData>().templates[&30].models[0].display_id;
    let world = app.world_mut();
    let mut commands = world.commands();
    let mut entity = commands.spawn((
        position,
        InMap::continent(0),
        Npc {
            template_id: 30,
            name: fixture.name.into(),
        },
        shared::components::ModelDisplay { display_id },
        Health {
            current: fixture.health,
            max: fixture.health,
        },
        UnitLevel(10),
        UnitFactionTemplate(fixture.faction),
        SpawnPoint {
            x: position.x,
            y: position.y,
            z: position.z,
            wander_distance: 0.0,
            speed_walk: 1.0,
            speed_run: 1.14286,
        },
        crate::auras::AuraStatMods {
            melee_hit_chance_pct: 100.0,
            ..Default::default()
        },
        // A corpse stays a corpse while the test watches it.
        crate::combat::RespawnDelay(600.0),
    ));
    insert_aggro_components(&mut entity, fixture.mode);
    let id = entity.id();
    world.flush();
    id
}

fn spawn_player(app: &mut App, position: Position) -> Entity {
    app.world_mut()
        .spawn((
            position,
            InMap::continent(0),
            Player {
                name: "Anduin".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            Health {
                current: 1000.0,
                max: 1000.0,
            },
            UnitLevel(10),
            UnitFactionTemplate(HUMAN),
            CombatTarget::default(),
        ))
        .id()
}

fn run(app: &mut App, updates: usize) {
    for _ in 0..updates {
        app.update();
    }
}

/// Ticks of one notify period at 50 ms.
const NOTIFY_TICKS: usize = 20;

fn state(app: &App, creature: Entity) -> CreatureState {
    *app.world().get::<CreatureState>(creature).unwrap()
}

fn pursuing(target: Entity) -> CreatureState {
    CreatureState::Pursuing {
        target: target.to_bits(),
    }
}

fn health(app: &App, unit: Entity) -> f32 {
    app.world().get::<Health>(unit).unwrap().current
}

fn dead(app: &App, unit: Entity) -> bool {
    app.world().get::<DeathState>(unit).is_some()
}

/// Whether `attacker` swung at `target`: a logged swing of any outcome. A creature
/// victim dodges or parries 9% of swings (`melee::swing_chances`) and MOD_HIT_CHANCE
/// does not remove that, so a landed swing is chance; the swing itself is not, and
/// an avoided one still engages the victim (`record_swing_events` → `add_threat`).
fn swung_at(app: &App, attacker: Entity, target: Entity) -> bool {
    app.world()
        .resource::<crate::combat::CombatLog>()
        .range(0.0, f64::MAX)
        .iter()
        .any(|entry| {
            entry.event.attacker == attacker.to_bits() && entry.event.target == target.to_bits()
        })
}

/// Move `unit` as a movement system does: new position, relocation marked.
fn move_to(app: &mut App, unit: Entity, position: Position) {
    *app.world_mut().get_mut::<Position>(unit).unwrap() = position;
    app.world().resource::<InterestDirtyQueue>().mark(unit);
}

#[test]
fn a_soldier_and_a_hostile_orc_in_range_fight_until_one_dies_and_the_other_goes_home() {
    let mut app = sight_app();
    let soldier = spawn(&mut app, &SOLDIER, at(0.0));
    let orc = spawn(&mut app, &ORC, at(10.0));

    run(&mut app, NOTIFY_TICKS + 1);
    assert_eq!(state(&app, soldier), pursuing(orc), "the soldier attacks");
    assert_eq!(state(&app, orc), pursuing(soldier), "the orc attacks");
    assert_eq!(
        app.world()
            .get::<AutoAttack>(soldier)
            .map(|swing| swing.victim),
        Some(orc)
    );
    assert_eq!(
        app.world().get::<CombatStatus>(soldier),
        Some(&CombatStatus(true))
    );

    run(&mut app, 60);
    assert!(
        swung_at(&app, orc, soldier),
        "the orc swings at the soldier"
    );
    assert!(
        swung_at(&app, soldier, orc),
        "the soldier swings at the orc"
    );

    let mut ticks = 0;
    while !dead(&app, soldier) && !dead(&app, orc) {
        run(&mut app, 1);
        ticks += 1;
        assert!(ticks < 2_000, "the fight ends");
    }
    let (survivor, corpse) = if dead(&app, orc) {
        (soldier, orc)
    } else {
        (orc, soldier)
    };
    run(&mut app, 2);
    assert_ne!(
        state(&app, survivor),
        pursuing(corpse),
        "nobody fights a corpse"
    );
    run(&mut app, 100);
    assert_eq!(state(&app, survivor), CreatureState::Idle, "it went home");
    assert!(app.world().get::<AutoAttack>(survivor).is_none());
}

#[test]
fn a_soldier_and_a_worg_are_neutral_and_never_fight() {
    let mut app = sight_app();
    let soldier = spawn(&mut app, &SOLDIER, at(0.0));
    let worg = spawn(&mut app, &WORG_FIXTURE, at(3.0));

    run(&mut app, 3 * NOTIFY_TICKS);
    assert_eq!(state(&app, soldier), CreatureState::Idle);
    assert_eq!(state(&app, worg), CreatureState::Idle);
    assert_eq!(health(&app, worg), 60.0);
}

#[test]
fn an_orc_beyond_the_aggro_radius_is_attacked_once_it_walks_in() {
    let mut app = sight_app();
    let soldier = spawn(&mut app, &SOLDIER, at(0.0));
    let orc = spawn(&mut app, &CIVILIAN_ORC, at(25.0));

    run(&mut app, 2 * NOTIFY_TICKS);
    assert_eq!(
        state(&app, soldier),
        CreatureState::Idle,
        "outside TC GetAttackDistance"
    );

    move_to(&mut app, orc, at(15.0));
    run(&mut app, NOTIFY_TICKS + 1);
    assert_eq!(state(&app, soldier), pursuing(orc));
}

#[test]
fn a_civilian_orc_never_starts_a_fight_but_fights_back() {
    let mut app = sight_app();
    let soldier = spawn(&mut app, &SOLDIER, at(0.0));
    let orc = spawn(&mut app, &CIVILIAN_ORC, at(3.0));
    app.world_mut()
        .entity_mut(soldier)
        .insert(crate::scripted_ai::ReactState::Passive);

    run(&mut app, 2 * NOTIFY_TICKS);
    assert_eq!(
        state(&app, orc),
        CreatureState::Idle,
        "a civilian never starts"
    );

    app.world_mut()
        .entity_mut(soldier)
        .insert(crate::scripted_ai::ReactState::Aggressive);
    move_to(&mut app, soldier, at(0.5));
    run(&mut app, NOTIFY_TICKS + 1);
    assert_eq!(state(&app, soldier), pursuing(orc));
    run(&mut app, 60);
    assert_eq!(
        state(&app, orc),
        pursuing(soldier),
        "it fights back when hit"
    );
}

#[test]
fn creatures_immune_to_npcs_neither_attack_nor_are_attacked() {
    let mut app = sight_app();
    let soldier = spawn(&mut app, &SOLDIER, at(0.0));
    let orc = spawn(&mut app, &ORC, at(4.0));
    app.world_mut()
        .entity_mut(soldier)
        .insert(UnitFlags(UNIT_FLAG_IMMUNE_TO_NPC));

    run(&mut app, 2 * NOTIFY_TICKS);
    assert_eq!(state(&app, soldier), CreatureState::Idle);
    assert_eq!(state(&app, orc), CreatureState::Idle);
}

#[test]
fn a_player_hitting_an_orc_that_fights_a_soldier_keeps_swinging_until_it_dies() {
    let mut app = sight_app();
    let soldier = spawn(
        &mut app,
        &Fixture {
            health: 100_000.0,
            ..SOLDIER
        },
        at(0.0),
    );
    let orc = spawn(
        &mut app,
        &Fixture {
            health: 400.0,
            ..CIVILIAN_ORC
        },
        at(3.0),
    );
    run(&mut app, NOTIFY_TICKS + 30);
    assert_eq!(state(&app, orc), pursuing(soldier), "the fight is on");

    let player = spawn_player(&mut app, at(4.5));
    app.world_mut().get_mut::<CombatTarget>(player).unwrap().0 = Some(orc);
    app.world_mut()
        .entity_mut(player)
        .insert(AutoAttack::new(orc));
    let mut ticks = 0;
    while !dead(&app, orc) {
        run(&mut app, 1);
        ticks += 1;
        assert!(ticks < 4_000, "the orc dies");
        if dead(&app, orc) {
            break;
        }
        assert_eq!(
            app.world()
                .get::<AutoAttack>(player)
                .map(|swing| swing.victim),
            Some(orc),
            "the player's auto-attack is still on the orc after {ticks} ticks"
        );
    }
    let log = app.world().resource::<crate::combat::CombatLog>();
    let player_swings = log
        .range(0.0, f64::MAX)
        .into_iter()
        .filter(|entry| entry.event.attacker == player.to_bits())
        .count();
    assert!(player_swings > 1, "the player swung {player_swings} times");
}

#[test]
fn dormant_resize_population_fits_one_catch_up_frame() {
    let mut app = App::new();
    app.init_resource::<NpcGrid>().init_resource::<SightPass>();
    app.add_systems(Update, resize_npcs);
    app.world_mut().spawn_batch((0..250_000).map(|_| {
        (
            Npc {
                template_id: 30,
                name: "Forest Spider".into(),
            },
            GridDormant,
            at(0.0),
            InMap::continent(0),
            shared::components::ModelDisplay { display_id: 0 },
            shared::components::UnitScale(1.0),
        )
    }));
    app.update();
    let fastest = (0..3)
        .map(|_| {
            let start = std::time::Instant::now();
            for _ in 0..5 {
                app.update();
            }
            start.elapsed()
        })
        .min()
        .unwrap();
    eprintln!("250000 dormant resize units/five steps: {fastest:?}");
    assert!(
        fastest < Duration::from_millis(50),
        "resize scans exhaust frame: {fastest:?}"
    );
}

#[test]
fn waking_resized_npc_updates_spatial_search_after_dormant_changes_settle() {
    let mut app = sight_app();
    let npc = spawn(&mut app, &ORC, at(200.0));
    run(&mut app, 1);
    let map = InMap::continent(0);
    assert!(
        !app.world()
            .resource::<NpcGrid>()
            .near(map, &at(0.0))
            .any(|unit| unit == npc)
    );
    app.world_mut()
        .entity_mut(npc)
        .insert((GridDormant, shared::components::UnitScale(1000.0)));
    run(&mut app, 3);
    app.world_mut().entity_mut(npc).remove::<GridDormant>();
    assert!(
        app.world()
            .resource::<NpcGrid>()
            .near(map, &at(0.0))
            .any(|unit| unit == npc),
        "waking a resized creature immediately expands neighbour search to its authored scaled reach"
    );
}
