use std::collections::HashMap;
use std::time::Duration;

use bevy::time::TimeUpdateStrategy;
use shared::components::{CombatStatus, Npc};

use super::*;
use crate::combat::{AutoAttack, CombatPlugin};
use crate::creature_ai::{CreatureAiPlugin, SpawnPoint};
use crate::creature_scripts::TrainingDummy;
use crate::faction_reaction::FactionTemplateEntry;
use crate::world_data::WorldData;

/// FactionTemplate 1 (Human): player + Alliance groups, enemy of Horde and monsters.
const HUMAN: u32 = 1;
/// FactionTemplate 22 (Forest Spider): monster group, enemy of players.
const MONSTER: u32 = 22;
/// FactionTemplate 189 (Stonetusk Boar): no groups, neutral to players.
const NEUTRAL_BEAST: u32 = 189;
/// Forest Spider `detection_range`.
const SPIDER_DETECTION: f32 = 18.0;

fn faction_templates() -> FactionTemplates {
    let entry = |id, faction, flags, faction_group, friend_group, enemy_group| {
        (
            id,
            FactionTemplateEntry {
                id,
                faction,
                flags,
                faction_group,
                friend_group,
                enemy_group,
                ..Default::default()
            },
        )
    };
    FactionTemplates(HashMap::from([
        entry(HUMAN, 1, 72, 3, 2, 12),
        entry(MONSTER, 22, 0, 8, 0, 1),
        entry(NEUTRAL_BEAST, 7, 1024, 0, 0, 0),
    ]))
}

fn aggro_app() -> App {
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        50,
    )));
    app.insert_resource(WorldData::default());
    app.insert_resource(faction_templates());
    app.add_plugins((CombatPlugin, CreatureAiPlugin, CreatureAggroPlugin));
    app.update();
    app
}

fn at(x: f32, z: f32) -> Position {
    Position { x, y: 0.0, z }
}

fn spawn_player(app: &mut App, position: Position, level: u8) -> Entity {
    app.world_mut()
        .spawn((
            position,
            Player {
                name: "Anduin".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            UnitLevel(level),
            UnitFactionTemplate(HUMAN),
            CombatTarget::default(),
        ))
        .id()
}

struct CreatureFixture {
    faction: u32,
    level: u8,
    mode: AggroMode,
    health: f32,
}

const FOREST_SPIDER: CreatureFixture = CreatureFixture {
    faction: MONSTER,
    level: 5,
    mode: AggroMode::Aggressive,
    health: 100.0,
};

const STONETUSK_BOAR: CreatureFixture = CreatureFixture {
    faction: NEUTRAL_BEAST,
    level: 5,
    mode: AggroMode::Aggressive,
    health: 100.0,
};

/// Spawned through the same component path as world spawns (`zones.rs`).
fn spawn_creature(app: &mut App, fixture: &CreatureFixture, position: Position) -> Entity {
    let world = app.world_mut();
    let mut commands = world.commands();
    let mut entity = commands.spawn((
        position,
        Npc {
            template_id: 30,
            name: "Forest Spider".into(),
        },
        Health {
            current: fixture.health,
            max: fixture.health,
        },
        UnitLevel(fixture.level),
        UnitFactionTemplate(fixture.faction),
        SpawnPoint {
            x: position.x,
            y: position.y,
            z: position.z,
            wander_distance: 0.0,
            speed_walk: 1.0,
            speed_run: 1.14286,
        },
    ));
    insert_aggro_components(&mut entity, fixture.mode, SPIDER_DETECTION);
    let id = entity.id();
    world.flush();
    id
}

fn run(app: &mut App, updates: usize) {
    for _ in 0..updates {
        app.update();
    }
}

fn state(app: &App, creature: Entity) -> Option<CreatureState> {
    app.world().get::<CreatureState>(creature).cloned()
}

fn pursuing(target: Entity) -> Option<CreatureState> {
    Some(CreatureState::Pursuing {
        target: target.to_bits(),
    })
}

fn health(app: &App, unit: Entity) -> f32 {
    app.world().get::<Health>(unit).unwrap().current
}

fn attack(app: &mut App, player: Entity, target: Entity) {
    app.world_mut().get_mut::<CombatTarget>(player).unwrap().0 = Some(target);
}

#[test]
fn aggro_radius_follows_level_difference_with_clamps() {
    assert_eq!(aggro_radius(18.0, 5, 5), 18.0);
    assert_eq!(aggro_radius(18.0, 5, 10), 13.0);
    assert_eq!(aggro_radius(18.0, 10, 5), 23.0);
    assert_eq!(
        aggro_radius(20.0, 60, 1),
        45.0,
        "capped at MAX_AGGRO_RADIUS"
    );
    assert_eq!(aggro_radius(20.0, 1, 60), 5.0, "floor of 5 yards");
    assert_eq!(
        aggro_radius(0.0, 5, 5),
        0.0,
        "no detection range never aggroes"
    );
}

#[test]
fn aggro_mode_follows_template_flags() {
    assert_eq!(AggroMode::for_template(1, 0, 0, ""), AggroMode::Aggressive);
    assert_eq!(AggroMode::for_template(1, 0x2, 0, ""), AggroMode::Reactive);
    assert_eq!(AggroMode::for_template(8, 0x2, 0, ""), AggroMode::Passive);
    assert_eq!(AggroMode::for_template(7, 0x80, 0, ""), AggroMode::Passive);
    assert_eq!(AggroMode::for_template(7, 0, 0x100, ""), AggroMode::Passive);
    assert_eq!(
        AggroMode::for_template(9, 0x40000, 0x20000, "npc_training_dummy"),
        AggroMode::Passive
    );
}

#[test]
fn hostile_spider_aggroes_player_in_radius_chases_and_hits() {
    let mut app = aggro_app();
    let spider = spawn_creature(&mut app, &FOREST_SPIDER, at(0.0, 0.0));
    let player = spawn_player(&mut app, at(15.0, 0.0), 5);

    run(&mut app, 1);
    assert_eq!(state(&app, spider), pursuing(player));
    assert_eq!(
        app.world().get::<CombatTarget>(spider).unwrap().0,
        Some(player)
    );
    assert_eq!(
        app.world().get::<CombatStatus>(player),
        Some(&CombatStatus(true)),
        "aggro puts the player in combat"
    );

    run(&mut app, 40);
    let spider_pos = *app.world().get::<Position>(spider).unwrap();
    assert!(
        (spider_pos.x - 13.0).abs() < 0.01,
        "spider stops 2 yd short of the player: {spider_pos:?}"
    );
    assert!(
        health(&app, player) < 100.0,
        "spider auto-attacks the player"
    );
}

#[test]
fn player_outside_level_scaled_radius_is_ignored() {
    let mut app = aggro_app();
    let spider = spawn_creature(&mut app, &FOREST_SPIDER, at(0.0, 0.0));
    // Level 10 vs level 5 spider: radius 18 - 5 = 13 yards.
    spawn_player(&mut app, at(15.0, 0.0), 10);
    run(&mut app, 10);
    assert_eq!(state(&app, spider), Some(CreatureState::Idle));

    // Level 1: radius 18 + 4 = 22 yards.
    let low = spawn_player(&mut app, at(0.0, 20.0), 1);
    run(&mut app, 1);
    assert_eq!(state(&app, spider), pursuing(low));
}

#[test]
fn tuned_creature_aggro_radius_uses_its_level_against_the_player() {
    let mut app = aggro_app();
    // Native level 30 (ContentTuning 1-30): unscaled, a level 10 player would
    // be 20 levels below and aggroed from 38 yards.
    let spider = spawn_creature(
        &mut app,
        &CreatureFixture {
            level: 30,
            ..FOREST_SPIDER
        },
        at(0.0, 0.0),
    );
    app.world_mut().entity_mut(spider).insert(LevelScaling {
        content_tuning_id: 73,
        min_level: 1,
        max_level: 30,
        delta: 0,
    });
    // Scaled to level 10 against a level 10 player: radius 18 yards.
    spawn_player(&mut app, at(20.0, 0.0), 10);
    run(&mut app, 10);
    assert_eq!(state(&app, spider), Some(CreatureState::Idle));
    let near = spawn_player(&mut app, at(0.0, 17.0), 10);
    run(&mut app, 1);
    assert_eq!(state(&app, spider), pursuing(near));
}

#[test]
fn nearest_hostile_player_is_attacked_first() {
    let mut app = aggro_app();
    let spider = spawn_creature(&mut app, &FOREST_SPIDER, at(0.0, 0.0));
    spawn_player(&mut app, at(12.0, 0.0), 5);
    let near = spawn_player(&mut app, at(0.0, -6.0), 5);
    run(&mut app, 1);
    assert_eq!(state(&app, spider), pursuing(near));
}

#[test]
fn dead_ghost_and_flying_players_are_not_aggroed() {
    let mut app = aggro_app();
    let spider = spawn_creature(&mut app, &FOREST_SPIDER, at(0.0, 0.0));
    let corpse = spawn_player(&mut app, at(4.0, 0.0), 5);
    app.world_mut().get_mut::<Health>(corpse).unwrap().current = 0.0;
    let ghost = spawn_player(&mut app, at(0.0, 4.0), 5);
    app.world_mut().entity_mut(ghost).insert(PlayerDeathState {
        state: shared::death::DeathState::Ghost,
        ..Default::default()
    });
    let flying = spawn_player(&mut app, at(-4.0, 0.0), 5);
    app.world_mut().entity_mut(flying).insert(TaxiFlight {
        points: vec![Vec3::ZERO],
        next: 0,
    });
    run(&mut app, 10);
    assert_eq!(state(&app, spider), Some(CreatureState::Idle));
}

#[test]
fn neutral_boar_ignores_player_until_hit_then_fights_back() {
    let mut app = aggro_app();
    let boar = spawn_creature(&mut app, &STONETUSK_BOAR, at(0.0, 0.0));
    let player = spawn_player(&mut app, at(3.0, 0.0), 5);
    run(&mut app, 10);
    assert_eq!(state(&app, boar), Some(CreatureState::Idle));

    attack(&mut app, player, boar);
    run(&mut app, 2);
    assert!(health(&app, boar) < 100.0, "player swing lands");
    assert_eq!(state(&app, boar), pursuing(player));
    let threat = app.world().get::<ThreatTable>(boar).unwrap();
    assert_eq!(
        threat.threat_for(player.to_bits()),
        100.0 - health(&app, boar)
    );

    run(&mut app, 20);
    assert!(health(&app, player) < 100.0, "boar hits back");
}

#[test]
fn training_dummy_never_aggroes_or_fights_back() {
    let mut app = aggro_app();
    let dummy_fixture = CreatureFixture {
        mode: AggroMode::for_template(9, 0x40000, 0x20000, "npc_training_dummy"),
        ..FOREST_SPIDER
    };
    let dummy = spawn_creature(&mut app, &dummy_fixture, at(0.0, 0.0));
    app.world_mut()
        .entity_mut(dummy)
        .insert(TrainingDummy::default());
    let player = spawn_player(&mut app, at(3.0, 0.0), 5);
    run(&mut app, 10);
    assert_eq!(state(&app, dummy), None);

    attack(&mut app, player, dummy);
    run(&mut app, 40);
    assert_eq!(state(&app, dummy), None, "dummy never chases");
    assert_eq!(*app.world().get::<Position>(dummy).unwrap(), at(0.0, 0.0));
    assert_eq!(health(&app, player), 100.0, "dummy never swings");
    assert!(app.world().get::<AutoAttack>(dummy).is_none());
}

#[test]
fn highest_threat_attacker_takes_aggro_past_the_threshold() {
    let mut app = aggro_app();
    let spider = spawn_creature(&mut app, &FOREST_SPIDER, at(0.0, 0.0));
    let tank = spawn_player(&mut app, at(1.0, 0.0), 5);
    let mage = spawn_player(&mut app, at(0.0, 30.0), 5);
    run(&mut app, 1);
    assert_eq!(state(&app, spider), pursuing(tank));

    add_threat(app.world_mut(), spider, tank, 100.0);
    // Ranged challenger (30 yd) needs 130%: 120 is not enough.
    add_threat(app.world_mut(), spider, mage, 120.0);
    run(&mut app, 1);
    assert_eq!(state(&app, spider), pursuing(tank));

    add_threat(app.world_mut(), spider, mage, 15.0);
    run(&mut app, 1);
    assert_eq!(state(&app, spider), pursuing(mage));
    assert_eq!(
        app.world().get::<CombatTarget>(spider).unwrap().0,
        Some(mage)
    );
}

#[test]
fn killing_the_only_target_makes_the_creature_evade_home_at_full_health() {
    let mut app = aggro_app();
    let spider = spawn_creature(&mut app, &FOREST_SPIDER, at(0.0, 0.0));
    let player = spawn_player(&mut app, at(10.0, 0.0), 5);
    run(&mut app, 1);
    add_threat(app.world_mut(), spider, player, 30.0);
    app.world_mut().get_mut::<Health>(spider).unwrap().current = 70.0;
    run(&mut app, 20);
    assert_eq!(state(&app, spider), pursuing(player));

    app.world_mut().get_mut::<Health>(player).unwrap().current = 0.0;
    run(&mut app, 1);
    assert_eq!(state(&app, spider), Some(CreatureState::Returning));
    assert_eq!(health(&app, spider), 100.0, "evade restores health");
    assert!(app.world().get::<ThreatTable>(spider).unwrap().is_empty());
    assert_eq!(app.world().get::<CombatTarget>(spider).unwrap().0, None);
    assert!(app.world().get::<AutoAttack>(spider).is_none());

    run(&mut app, 40);
    assert_eq!(state(&app, spider), Some(CreatureState::Idle));
    assert_eq!(*app.world().get::<Position>(spider).unwrap(), at(0.0, 0.0));
}

#[test]
fn leashed_creature_evades_and_its_attackers_stop_swinging() {
    let mut app = aggro_app();
    let spider = spawn_creature(&mut app, &FOREST_SPIDER, at(0.0, 0.0));
    let player = spawn_player(&mut app, at(3.0, 0.0), 5);
    attack(&mut app, player, spider);
    run(&mut app, 2);
    assert_eq!(state(&app, spider), pursuing(player));
    assert!(health(&app, spider) < 100.0);

    // Player runs 60 yd away; the spider chases past its 40 yd leash.
    *app.world_mut().get_mut::<Position>(player).unwrap() = at(60.0, 0.0);
    let mut evaded = false;
    for _ in 0..100 {
        run(&mut app, 1);
        if state(&app, spider) == Some(CreatureState::Returning) {
            evaded = true;
            break;
        }
    }
    assert!(evaded, "spider evades beyond its leash");
    assert_eq!(health(&app, spider), 100.0);
    assert!(
        app.world().get::<AutoAttack>(player).is_none(),
        "evade stops the player's auto-attack"
    );

    // Evading creatures ignore damage and do not re-aggro on the way home.
    add_threat(app.world_mut(), spider, player, 50.0);
    run(&mut app, 1);
    assert_eq!(state(&app, spider), Some(CreatureState::Returning));
}

#[test]
fn killed_creature_forgets_its_fight() {
    let mut app = aggro_app();
    let weak_spider = CreatureFixture {
        health: 5.0,
        ..FOREST_SPIDER
    };
    let spider = spawn_creature(&mut app, &weak_spider, at(0.0, 0.0));
    app.world_mut()
        .entity_mut(spider)
        .insert(crate::combat::RespawnDelay(60.0));
    let player = spawn_player(&mut app, at(3.0, 0.0), 5);
    attack(&mut app, player, spider);
    run(&mut app, 2);
    assert!(app.world().get::<DeathState>(spider).is_some());
    assert_eq!(state(&app, spider), Some(CreatureState::Idle));
    assert!(app.world().get::<ThreatTable>(spider).unwrap().is_empty());
    let position = *app.world().get::<Position>(spider).unwrap();
    run(&mut app, 10);
    assert_eq!(*app.world().get::<Position>(spider).unwrap(), position);
}

const FIREBALL: u32 = 133;
const SILENCE: u32 = 15487;

/// A world-spawned neutral creature (Stonetusk Boar template) `x` yards from
/// the harness caster.
fn spawn_harness_boar(h: &mut crate::spell_cast::test_harness::Harness, x: f32) -> Entity {
    let world = h.app.world_mut();
    let mut commands = world.commands();
    let mut entity = commands.spawn((
        Npc {
            template_id: 113,
            name: "Stonetusk Boar".into(),
        },
        UnitFactionTemplate(NEUTRAL_BEAST),
        Position { x, y: 0.0, z: 0.0 },
        Health {
            current: 5000.0,
            max: 5000.0,
        },
    ));
    insert_aggro_components(&mut entity, AggroMode::Aggressive, 18.0);
    let id = entity.id();
    world.flush();
    id
}

#[test]
fn spell_damage_on_neutral_creature_puts_the_damage_on_its_threat_list() {
    let Some(mut h) = crate::spell_cast::test_harness::harness(Vec::new(), &[FIREBALL], 8) else {
        return;
    };
    let boar = spawn_harness_boar(&mut h, 20.0);
    h.cast_and_deliver(FIREBALL, Some(boar));
    h.advance(40);
    // SP coefficient 3.8812 × spell power 100 = 388 ± 2.5%.
    let damage = 5000.0 - h.health(boar);
    assert!(
        crate::spell_cast::test_harness::varied(388.12).contains(&damage),
        "{damage}"
    );
    assert_eq!(state(&h.app, boar), pursuing(h.caster));
    let threat = h.app.world().get::<ThreatTable>(boar).unwrap();
    assert_eq!(threat.threat_for(h.caster.to_bits()), damage);
}

#[test]
fn harmful_spell_without_damage_engages_a_neutral_creature() {
    let Some(mut h) = crate::spell_cast::test_harness::harness(Vec::new(), &[SILENCE], 5) else {
        return;
    };
    let boar = spawn_harness_boar(&mut h, 20.0);
    h.cast_and_deliver(SILENCE, Some(boar));
    assert!(
        h.inbox.failed.is_empty(),
        "silence lands: {:?}",
        h.inbox.failed
    );
    assert_eq!(h.health(boar), 5000.0);
    assert_eq!(state(&h.app, boar), pursuing(h.caster));
}

#[test]
fn hostile_civilian_ignores_nearby_player_until_hit() {
    let mut app = aggro_app();
    let civilian = CreatureFixture {
        mode: AggroMode::for_template(7, 0x2, 0, ""),
        ..FOREST_SPIDER
    };
    let kobold = spawn_creature(&mut app, &civilian, at(0.0, 0.0));
    let player = spawn_player(&mut app, at(3.0, 0.0), 5);
    run(&mut app, 10);
    assert_eq!(state(&app, kobold), Some(CreatureState::Idle));

    attack(&mut app, player, kobold);
    run(&mut app, 20);
    assert_eq!(state(&app, kobold), pursuing(player));
    assert!(health(&app, player) < 100.0, "civilian hits back");
}
