use super::*;
use crate::stats::CharacterStats;
use bevy::ecs::system::RunSystemOnce;
use shared::components::{CombatStatus, Player, UnitLevel};

#[path = "combat_tests/attack_requests.rs"]
mod attack_requests;
#[path = "combat_tests/death.rs"]
mod death;
#[path = "combat_tests/line_of_sight.rs"]
mod line_of_sight;
#[path = "combat_tests/outcomes.rs"]
mod outcomes;
#[path = "combat_tests/player_damage.rs"]
mod player_damage;
#[path = "combat_tests/rage.rs"]
mod rage;

fn pos(x: f32, y: f32, z: f32) -> Position {
    Position { x, y, z }
}

fn setup_app() -> App {
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.init_resource::<CombatLog>();
    app.init_resource::<FactionTemplates>();
    app.update();
    app.update();
    app
}

fn build_replication_app() -> App {
    let mut app = setup_app();
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: std::time::Duration::from_millis(50),
    });
    app.add_plugins(shared::ProtocolPlugin);
    app.add_message::<crate::death::PlayerDeathTransition>();
    app
}

fn finish_replication_app(mut app: App) -> App {
    app.finish();
    app.cleanup();
    app.update();
    app
}

fn setup_replication_app() -> App {
    finish_replication_app(build_replication_app())
}

fn setup_combat_app() -> App {
    let mut app = build_replication_app();
    app.add_plugins(CombatPlugin);
    finish_replication_app(app)
}

fn spawn_target(app: &mut App, position: Position, hp: f32) -> Entity {
    app.world_mut()
        .spawn((
            position,
            Health {
                current: hp,
                max: 100.0,
            },
        ))
        .id()
}

fn spawn_attacker(app: &mut App, position: Position, target: Entity) -> Entity {
    app.world_mut()
        .spawn((
            position,
            CombatTarget(Some(target)),
            AutoAttack::new(target),
            crate::maps::InMap::continent(0),
            Player {
                name: "test".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
        ))
        .id()
}

fn run_expire_combat_refs_tick(app: &mut App) {
    crate::combat_refs::expire_combat_refs_system(app.world_mut());
}

/// Run auto-attack logic without needing lightyear MessageSender.
fn run_attack_tick(app: &mut App) {
    let _ = app.world_mut().run_system_once(
        |time: Res<Time>,
         mut commands: Commands,
         mut attackers: Query<(Entity, &mut AutoAttack, &Position, Option<&CharacterStats>)>,
         mut targets: AutoAttackTargets,
         scaling: Query<ScalingItem<'static>>,
         summon_modifiers: crate::auras::summon_damage::SummonDamageModifiers,
         armor_constants: Option<Res<ArmorConstants>>,
         sight: SwingSight| {
            let dt = time.delta_secs();
            let context = SwingContext {
                scaling: &scaling,
                summon_modifiers: &summon_modifiers,
                sight: &sight,
                armor_constants: armor_constants.as_deref(),
                spells: None,
                creature_types: crate::unit_creature_type::CreatureTypes {
                    races: None,
                    forms: None,
                    data: None,
                },
            };
            let mut redirect_health = std::collections::HashMap::new();
            for (entity, mut aa, position, stats) in &mut attackers {
                let te = aa.victim;
                aa.timer -= dt;
                aa.off_hand_timer -= dt;
                let attacker = Attacker {
                    entity,
                    position,
                    mods: None,
                    stats,
                    haste_pct: 0.0,
                    enemy_dodge_pct: 0.0,
                    expertise_pct: 0.0,
                    creature: false,
                    authored_attack_time: None,
                    creature_flags: None,
                    temp_summoner: None,
                    spirit_wolf: None,
                };
                for hand in [Hand::Main, Hand::Off] {
                    if try_swing(
                        &attacker,
                        hand,
                        &mut targets,
                        &context,
                        te,
                        &mut aa,
                        &mut commands,
                        &mut redirect_health,
                    )
                    .is_some()
                    {
                        commands.queue(move |world: &mut World| {
                            crate::combat_refs::set_in_combat_with(world, entity, te);
                        });
                    }
                }
            }
        },
    );
}

/// Hogger's per-level stats (level_scaling_tests fixture: ExpectedStat 10 and 30).
fn tuned_hogger(app: &mut App, entity: Entity) {
    app.world_mut().entity_mut(entity).insert((
        UnitLevel(30),
        crate::level_scaling::tests::HOGGER_SCALING,
        crate::level_scaling::tests::hogger_stats(),
    ));
}

#[test]
fn scaled_overkill_melee_uses_logged_damage_units() {
    for (health_before, expected_health, expected_overkill) in
        [(46.0, 0.0, 6), (230.0, 115.0, 0), (115.0, 0.0, 0)]
    {
        let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[], 1).unwrap();
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .remove::<CharacterStats>();
        h.app.world_mut().entity_mut(h.caster).insert(UnitLevel(10));
        tuned_hogger(&mut h.app, h.npc);
        h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = health_before;
        h.attack(h.npc);
        h.advance(6);
        let event = h
            .inbox
            .log
            .iter()
            .find(|event| event.kind == CombatLogKind::Damage && event.spell_id.is_none())
            .expect("melee damage delivered to owner");
        // Logged 10 damage removes 115 native health: 46 / 115 needs 4 damage.
        assert_eq!(event.amount, 10);
        assert_eq!(
            event.overflow, expected_overkill,
            "health before {health_before}"
        );
        assert!((0..=event.amount).contains(&event.overflow));
        assert_eq!(h.health(h.npc), expected_health);
    }
}

#[test]
fn level_10_player_hit_takes_the_level_10_share_of_a_tuned_creatures_health() {
    let mut app = setup_app();
    let hogger = spawn_target(&mut app, pos(1.0, 0.0, 0.0), 13_137.0);
    tuned_hogger(&mut app, hogger);
    let player = spawn_attacker(&mut app, pos(0.0, 0.0, 0.0), hogger);
    app.world_mut().entity_mut(player).insert(UnitLevel(10));
    run_attack_tick(&mut app);
    // 10 damage against a level 10 Hogger's 1,133 health is 115 of its 13,137.
    let health = app.world().get::<Health>(hogger).unwrap();
    assert_eq!(health.current, 13_137.0 - 115.0);
}

#[test]
fn level_40_player_hit_on_a_tuned_creature_is_unscaled() {
    let mut app = setup_app();
    let hogger = spawn_target(&mut app, pos(1.0, 0.0, 0.0), 13_137.0);
    tuned_hogger(&mut app, hogger);
    let player = spawn_attacker(&mut app, pos(0.0, 0.0, 0.0), hogger);
    app.world_mut().entity_mut(player).insert(UnitLevel(40));
    run_attack_tick(&mut app);
    assert_eq!(app.world().get::<Health>(hogger).unwrap().current, 13_127.0);
}

#[test]
fn tuned_creature_swings_expected_stat_damage_at_the_targets_level() {
    for (player_level, dps) in [(10, 17.05213f32), (40, 162.6229)] {
        let mut app = setup_app();
        let player = spawn_target(&mut app, pos(1.0, 0.0, 0.0), 100_000.0);
        app.world_mut()
            .entity_mut(player)
            .insert(UnitLevel(player_level));
        let hogger = spawn_attacker(&mut app, pos(0.0, 0.0, 0.0), player);
        app.world_mut().entity_mut(hogger).remove::<Player>();
        tuned_hogger(&mut app, hogger);
        run_attack_tick(&mut app);
        // DPS × DamageModifier 1.7 × 2 s, rolled up to 1.5×, at the player's level.
        let min = dps * 1.7 * 2.0;
        let taken = 100_000.0 - app.world().get::<Health>(player).unwrap().current;
        assert!(
            taken >= min.round() - 1.0 && taken <= (min * 1.5).round(),
            "level {player_level}: {taken} outside {min}..{}",
            min * 1.5
        );
    }
}

#[test]
fn test_in_melee_range() {
    assert!(in_melee_range(
        &pos(0.0, 0.0, 0.0),
        &pos(3.0, 4.0, 0.0),
        0.0
    ));
    assert!(!in_melee_range(
        &pos(0.0, 0.0, 0.0),
        &pos(10.0, 0.0, 0.0),
        0.0
    ));
}

#[test]
fn test_auto_attack_deals_damage() {
    let mut app = setup_app();
    let target = spawn_target(&mut app, pos(1.0, 0.0, 0.0), 100.0);
    spawn_attacker(&mut app, pos(0.0, 0.0, 0.0), target);

    run_attack_tick(&mut app);

    let health = app.world().entity(target).get::<Health>().unwrap();
    assert_eq!(health.current, 90.0, "target should have taken 10 damage");
}

#[test]
fn test_auto_attack_out_of_range() {
    let mut app = setup_app();
    let target = spawn_target(&mut app, pos(100.0, 0.0, 0.0), 100.0);
    spawn_attacker(&mut app, pos(0.0, 0.0, 0.0), target);

    run_attack_tick(&mut app);

    let health = app.world().entity(target).get::<Health>().unwrap();
    assert_eq!(health.current, 100.0, "target should not have taken damage");
}

#[test]
fn test_auto_attack_timer_resets() {
    let mut app = setup_app();
    let target = spawn_target(&mut app, pos(1.0, 0.0, 0.0), 100.0);
    let player = spawn_attacker(&mut app, pos(0.0, 0.0, 0.0), target);

    run_attack_tick(&mut app);

    let auto = app.world().entity(player).get::<AutoAttack>().unwrap();
    assert!(
        (auto.timer - AUTO_ATTACK_INTERVAL).abs() < 0.01,
        "timer should reset to {AUTO_ATTACK_INTERVAL}, got {}",
        auto.timer
    );
}

#[test]
fn auto_attack_fires_at_correct_interval() {
    let mut app = setup_app();
    let target = spawn_target(&mut app, pos(1.0, 0.0, 0.0), 100.0);
    let attacker = spawn_attacker(&mut app, pos(0.0, 0.0, 0.0), target);

    // First tick: timer=0, swings immediately -> damage dealt, timer resets to 2.0
    run_attack_tick(&mut app);
    let hp_after_first = app.world().entity(target).get::<Health>().unwrap().current;
    assert_eq!(hp_after_first, 90.0, "first swing deals damage");

    // Second tick: timer > 0 (just reset to 2.0), no swing
    run_attack_tick(&mut app);
    let hp_after_second = app.world().entity(target).get::<Health>().unwrap().current;
    assert_eq!(hp_after_second, 90.0, "no swing during cooldown");

    // Manually expire the timer to simulate 2s passing
    app.world_mut()
        .entity_mut(attacker)
        .get_mut::<AutoAttack>()
        .unwrap()
        .timer = 0.0;

    // Third tick: timer expired, swings again
    run_attack_tick(&mut app);
    let hp_after_third = app.world().entity(target).get::<Health>().unwrap().current;
    assert_eq!(hp_after_third, 80.0, "second swing after interval");

    // Verify interval constant
    assert_eq!(AUTO_ATTACK_INTERVAL, 2.0, "interval is 2 seconds");
}

#[test]
fn auto_attack_stops_when_target_dies() {
    let mut app = setup_app();
    let target = spawn_target(&mut app, pos(1.0, 0.0, 0.0), 10.0); // 10 HP = dies in 1 hit
    let attacker = spawn_attacker(&mut app, pos(0.0, 0.0, 0.0), target);

    // First swing kills the target
    run_attack_tick(&mut app);
    let hp = app.world().entity(target).get::<Health>().unwrap().current;
    assert_eq!(hp, 0.0, "target is dead");

    // Expire timer and try again - no further damage (target is dead)
    app.world_mut()
        .entity_mut(attacker)
        .get_mut::<AutoAttack>()
        .unwrap()
        .timer = 0.0;
    run_attack_tick(&mut app);
    let hp = app.world().entity(target).get::<Health>().unwrap().current;
    assert_eq!(hp, 0.0, "no damage dealt to dead target");
}

#[test]
fn auto_attack_stops_when_out_of_melee_range() {
    let mut app = setup_app();
    let target = spawn_target(&mut app, pos(1.0, 0.0, 0.0), 100.0);
    let attacker = spawn_attacker(&mut app, pos(0.0, 0.0, 0.0), target);

    // In range: swing connects
    run_attack_tick(&mut app);
    assert_eq!(
        app.world().entity(target).get::<Health>().unwrap().current,
        90.0,
    );

    // Move target out of melee range (>5yd)
    app.world_mut()
        .entity_mut(target)
        .get_mut::<Position>()
        .unwrap()
        .x = 100.0;

    // Expire timer and try again - out of range, no damage
    app.world_mut()
        .entity_mut(attacker)
        .get_mut::<AutoAttack>()
        .unwrap()
        .timer = 0.0;
    run_attack_tick(&mut app);
    assert_eq!(
        app.world().entity(target).get::<Health>().unwrap().current,
        90.0,
        "no damage when out of range",
    );

    // Move target back in range - swing resumes
    app.world_mut()
        .entity_mut(target)
        .get_mut::<Position>()
        .unwrap()
        .x = 1.0;
    run_attack_tick(&mut app);
    assert_eq!(
        app.world().entity(target).get::<Health>().unwrap().current,
        80.0,
        "swing resumes when back in range",
    );
}

#[test]
fn entering_combat_sets_replicated_flag() {
    let mut app = setup_app();
    let target = spawn_target(&mut app, pos(1.0, 0.0, 0.0), 100.0);
    let attacker = spawn_attacker(&mut app, pos(0.0, 0.0, 0.0), target);

    run_attack_tick(&mut app);

    let combat = app
        .world()
        .entity(attacker)
        .get::<CombatStatus>()
        .copied()
        .expect("combat flag should exist after dealing damage");
    assert!(combat.0);
}

#[test]
fn active_combat_maintenance_does_not_dirty_true_status() {
    let mut app = setup_app();
    let target = spawn_target(&mut app, pos(1.0, 0.0, 0.0), 100.0);
    let attacker = spawn_attacker(&mut app, pos(0.0, 0.0, 0.0), target);
    crate::combat_refs::set_in_combat_with(app.world_mut(), attacker, target);
    app.update();

    let tick_before = app
        .world()
        .entity(attacker)
        .get_ref::<CombatStatus>()
        .unwrap()
        .last_changed();

    run_expire_combat_refs_tick(&mut app);

    assert!(
        app.world()
            .entity(attacker)
            .get::<CombatStatus>()
            .is_some_and(|status| status.0)
    );
    assert_eq!(
        app.world()
            .entity(attacker)
            .get_ref::<CombatStatus>()
            .unwrap()
            .last_changed(),
        tick_before,
        "maintaining active combat must not dirty an unchanged true status"
    );
}

#[test]
fn combat_with_a_threatless_target_expires_five_seconds_after_the_last_swing() {
    let mut app = setup_app();
    let target = spawn_target(&mut app, pos(1.0, 0.0, 0.0), 100.0);
    let attacker = spawn_attacker(&mut app, pos(0.0, 0.0, 0.0), target);

    run_attack_tick(&mut app);
    attack_stop(app.world_mut(), attacker);
    assert!(app.world().entity(attacker).get::<AutoAttack>().is_none());

    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f64(
            crate::combat_refs::PVP_COMBAT_TIMEOUT_SECS - 1.0,
        ));
    run_expire_combat_refs_tick(&mut app);
    assert!(
        app.world()
            .entity(attacker)
            .get::<CombatStatus>()
            .is_some_and(|flag| flag.0),
        "combat flag should stay active for 5 s after the swing"
    );

    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f64(2.0));
    run_expire_combat_refs_tick(&mut app);
    assert!(
        app.world()
            .entity(attacker)
            .get::<CombatStatus>()
            .is_some_and(|flag| !flag.0),
        "combat flag should clear 5 s after the swing"
    );
}

#[test]
fn overkill_damage_clamps_to_zero() {
    let mut app = setup_app();
    // Target has only 3 HP but BASE_DAMAGE is 10
    let target = spawn_target(&mut app, pos(1.0, 0.0, 0.0), 3.0);
    spawn_attacker(&mut app, pos(0.0, 0.0, 0.0), target);

    run_attack_tick(&mut app);

    let hp = app.world().entity(target).get::<Health>().unwrap().current;
    assert_eq!(hp, 0.0, "HP clamped to 0, not negative");
}

#[test]
fn zero_health_without_damage_transition_is_not_discovered_by_death_poll() {
    let mut app = setup_combat_app();
    let npc = app
        .world_mut()
        .spawn((
            pos(0.0, 0.0, 0.0),
            Health {
                current: 0.0,
                max: 100.0,
            },
            Npc {
                template_id: 1,
                name: "Fixture wolf".into(),
            },
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();

    app.world_mut().run_schedule(FixedUpdate);

    let entity = app.world().entity(npc);
    assert!(
        entity.get::<DeathState>().is_none(),
        "a zero-health NPC without an authoritative damage transition must not be discovered by a global death poll"
    );
    assert!(
        entity.get::<Replicate>().is_some(),
        "a zero-health NPC without an authoritative damage transition must remain replicated"
    );
    assert!(
        app.world()
            .resource::<CombatLog>()
            .range(0.0, f64::MAX)
            .is_empty(),
        "a global death poll must not emit a death event for an untracked zero-health NPC"
    );
}

#[test]
fn nonlethal_auto_attack_keeps_npc_alive_and_replicated() {
    let mut app = setup_replication_app();
    let target = app
        .world_mut()
        .spawn((
            pos(1.0, 0.0, 0.0),
            Health {
                current: 100.0,
                max: 100.0,
            },
            Npc {
                template_id: 1,
                name: "Fixture wolf".into(),
            },
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    spawn_attacker(&mut app, pos(0.0, 0.0, 0.0), target);

    app.world_mut()
        .run_system_once(auto_attack_system)
        .expect("auto-attack system should run");
    app.update();

    let entity = app.world().entity(target);
    assert_eq!(entity.get::<Health>().unwrap().current, 90.0);
    assert!(entity.get::<DeathState>().is_none());
    assert!(entity.get::<Replicate>().is_some());
    assert!(
        app.world()
            .resource::<CombatLog>()
            .range(0.0, f64::MAX)
            .iter()
            .all(|entry| entry.event.event_type != CombatEventType::Death),
        "a nonlethal auto-attack must not produce a death combat event"
    );
}

#[test]
fn lethal_auto_attack_leaves_a_replicated_corpse_and_a_death_event() {
    let mut app = setup_replication_app();
    let target = app
        .world_mut()
        .spawn((
            pos(1.0, 0.0, 0.0),
            Health {
                current: BASE_DAMAGE,
                max: 100.0,
            },
            Npc {
                template_id: 1,
                name: "Fixture wolf".into(),
            },
            Replicate::to_clients(NetworkTarget::All),
            CorpseDelay(60.0),
        ))
        .id();
    spawn_attacker(&mut app, pos(0.0, 0.0, 0.0), target);

    app.world_mut()
        .run_system_once(auto_attack_system)
        .expect("auto-attack system should run");
    app.update();

    let entity = app.world().entity(target);
    assert!(entity.get::<DeathState>().is_some());
    assert!(
        entity.get::<Replicate>().is_some(),
        "the corpse stays visible"
    );
    assert_eq!(entity.get::<Corpse>(), Some(&Corpse { remaining: 60.0 }));
    assert!(
        app.world()
            .resource::<CombatLog>()
            .range(0.0, f64::MAX)
            .iter()
            .any(|entry| {
                entry.event.target == target.to_bits()
                    && entry.event.event_type == CombatEventType::Death
            }),
        "a lethal auto-attack must produce a death combat event"
    );
}

#[test]
fn test_respawn_restores_health() {
    let health = Health {
        current: 0.0,
        max: 100.0,
    };
    let position = pos(50.0, 50.0, 50.0);
    let spawn = SpawnPoint {
        x: 10.0,
        y: 20.0,
        z: 30.0,
        wander_distance: 5.0,
        speed_walk: 1.0,
        speed_run: 1.0,
    };

    let (health, position) = respawned_state(health, position, &spawn);

    assert_eq!(health.current, 100.0, "health should be restored to max");
    assert_eq!(position.x, 10.0);
    assert_eq!(position.y, 20.0);
    assert_eq!(position.z, 30.0);
}

#[test]
fn respawn_at_spawn_with_full_health_does_not_dirty_replicated_state() {
    let mut app = setup_replication_app();
    let spawn = SpawnPoint {
        x: 10.0,
        y: 20.0,
        z: 30.0,
        wander_distance: 0.0,
        speed_walk: 1.0,
        speed_run: 1.0,
    };
    let position = pos(spawn.x, spawn.y, spawn.z);
    let entity = app
        .world_mut()
        .spawn((
            DeathState { respawn_timer: 0.0 },
            Health {
                current: 100.0,
                max: 100.0,
            },
            spawn,
            position,
        ))
        .id();
    app.update();

    let health_tick = app
        .world()
        .entity(entity)
        .get_ref::<Health>()
        .unwrap()
        .last_changed();
    let position_tick = app
        .world()
        .entity(entity)
        .get_ref::<Position>()
        .unwrap()
        .last_changed();

    app.world_mut()
        .run_system_once(respawn_system)
        .expect("respawn system should run");
    app.update();

    let entity_ref = app.world().entity(entity);
    assert!(entity_ref.get::<DeathState>().is_none());
    assert!(entity_ref.get::<Replicate>().is_some());
    assert_eq!(entity_ref.get::<Health>().unwrap().current, 100.0);
    assert_eq!(*entity_ref.get::<Position>().unwrap(), position);
    assert_eq!(
        entity_ref.get_ref::<Health>().unwrap().last_changed(),
        health_tick,
        "respawning at full health must not dirty Health"
    );
    assert_eq!(
        entity_ref.get_ref::<Position>().unwrap().last_changed(),
        position_tick,
        "respawning at spawn must not dirty Position"
    );
}

#[test]
fn dead_creature_in_dormant_grid_respawns_on_its_timer() {
    use bevy::time::TimeUpdateStrategy;
    let mut app = setup_combat_app();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(50),
    ));
    let spawn = SpawnPoint {
        x: 10.0,
        y: 20.0,
        z: 30.0,
        wander_distance: 0.0,
        speed_walk: 1.0,
        speed_run: 1.0,
    };
    let wolf = app
        .world_mut()
        .spawn((
            DeathState { respawn_timer: 3.0 },
            crate::grid_activity::GridDormant,
            Health {
                current: 0.0,
                max: 100.0,
            },
            spawn,
            // Died 15 yd from its spawn point.
            pos(25.0, 20.0, 30.0),
        ))
        .id();

    // 2.5 s: still dead.
    for _ in 0..50 {
        app.update();
    }
    assert!(app.world().get::<DeathState>(wolf).is_some());

    // 3.5 s: back at its spawn point with full health, still dormant.
    for _ in 0..20 {
        app.update();
    }
    let wolf = app.world().entity(wolf);
    assert!(wolf.get::<DeathState>().is_none());
    assert_eq!(wolf.get::<Health>().unwrap().current, 100.0);
    assert_eq!(*wolf.get::<Position>().unwrap(), pos(10.0, 20.0, 30.0));
    assert!(wolf.get::<crate::grid_activity::GridDormant>().is_some());
}

/// AzerothCore `Unit::setDeathState` runs `CombatStop` on death, so a killer
/// standing at the spawn point does not swing at the respawned NPC.
#[test]
fn a_killed_npc_respawns_in_its_killers_melee_range_at_full_health() {
    use bevy::time::TimeUpdateStrategy;
    let mut app = setup_combat_app();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(50),
    ));
    let kobold = app
        .world_mut()
        .spawn((
            pos(1.0, 0.0, 0.0),
            // Three swings: the kill lands well after the attack started.
            Health {
                current: BASE_DAMAGE * 3.0,
                max: 100.0,
            },
            Npc {
                template_id: 6,
                name: "Kobold Vermin".into(),
            },
            SpawnPoint {
                x: 1.0,
                y: 0.0,
                z: 0.0,
                wander_distance: 0.0,
                speed_walk: 1.0,
                speed_run: 1.0,
            },
            RespawnDelay(3.0),
            CorpseDelay(1.0),
        ))
        .id();
    let player = spawn_attacker(&mut app, pos(0.0, 0.0, 0.0), kobold);

    for _ in 0..(AUTO_ATTACK_INTERVAL * 2.0 * 20.0) as usize + 10 {
        app.update();
    }
    assert!(app.world().get::<DeathState>(kobold).is_some(), "killed");

    // 3 s respawn plus another auto-attack interval.
    for _ in 0..(60.0 + AUTO_ATTACK_INTERVAL * 20.0) as usize {
        app.update();
    }
    assert!(app.world().get::<DeathState>(kobold).is_none(), "respawned");
    assert_eq!(app.world().get::<Health>(kobold).unwrap().current, 100.0);
    assert!(app.world().get::<AutoAttack>(player).is_none());
}

#[test]
fn respawn_delay_is_spawntimesecs_plus_rank_corpse_decay() {
    // Northshire Guard: spawntimesecs 430, normal rank.
    assert_eq!(RespawnDelay::for_spawn(430, 0), RespawnDelay(490.0));
    assert_eq!(
        RespawnDelay::for_spawn(430, 1),
        RespawnDelay(730.0),
        "elite"
    );
    assert_eq!(RespawnDelay::for_spawn(430, 4), RespawnDelay(730.0), "rare");
    assert_eq!(
        RespawnDelay::for_spawn(430, 3),
        RespawnDelay(1030.0),
        "world boss"
    );
}

fn make_event(event_type: CombatEventType, amount: f32) -> CombatEvent {
    CombatEvent {
        attacker: 1,
        target: 2,
        amount,
        spell_id: 0,
        event_type,
    }
}

#[test]
fn combat_log_records_and_retrieves() {
    let mut log = CombatLog::default();
    assert!(log.entries.is_empty());

    log.record(1.0, make_event(CombatEventType::MeleeDamage, 100.0));
    log.record(2.0, make_event(CombatEventType::Death, 0.0));

    assert_eq!(log.entries.len(), 2);
    assert_eq!(log.entries[0].timestamp, 1.0);
    assert_eq!(log.entries[1].event.event_type, CombatEventType::Death);
}

#[test]
fn combat_log_evicts_oldest_at_capacity() {
    let mut log = CombatLog {
        entries: VecDeque::new(),
        capacity: 3,
    };
    log.record(1.0, make_event(CombatEventType::MeleeDamage, 10.0));
    log.record(2.0, make_event(CombatEventType::MeleeDamage, 20.0));
    log.record(3.0, make_event(CombatEventType::MeleeDamage, 30.0));
    log.record(4.0, make_event(CombatEventType::MeleeDamage, 40.0));

    assert_eq!(log.entries.len(), 3);
    assert_eq!(
        log.entries[0].timestamp, 2.0,
        "oldest entry should be evicted"
    );
}

#[test]
fn combat_log_range_filters_by_time() {
    let mut log = CombatLog::default();
    log.record(1.0, make_event(CombatEventType::MeleeDamage, 10.0));
    log.record(5.0, make_event(CombatEventType::SpellDamage, 20.0));
    log.record(10.0, make_event(CombatEventType::Death, 0.0));

    let range = log.range(4.0, 6.0);
    assert_eq!(range.len(), 1);
    assert_eq!(range[0].event.event_type, CombatEventType::SpellDamage);
}

#[test]
fn combat_log_clear_resets() {
    let mut log = CombatLog::default();
    log.record(1.0, make_event(CombatEventType::MeleeDamage, 10.0));
    log.entries.clear();
    assert!(log.entries.is_empty());
}

#[test]
fn combat_log_records_all_event_types() {
    let mut log = CombatLog::default();
    let all_types = [
        CombatEventType::MeleeDamage,
        CombatEventType::SpellDamage,
        CombatEventType::SpellHeal,
        CombatEventType::PeriodicDamage,
        CombatEventType::PeriodicHeal,
        CombatEventType::Absorb,
        CombatEventType::Miss,
        CombatEventType::Dodge,
        CombatEventType::Parry,
        CombatEventType::Block,
        CombatEventType::CriticalHit,
        CombatEventType::Interrupt,
        CombatEventType::Death,
        CombatEventType::Respawn,
    ];

    for (i, event_type) in all_types.iter().enumerate() {
        log.record(i as f64, make_event(event_type.clone(), i as f32));
    }

    assert_eq!(log.entries.len(), all_types.len());
    for (i, event_type) in all_types.iter().enumerate() {
        assert_eq!(
            &log.entries[i].event.event_type, event_type,
            "entry {i} should be {event_type:?}",
        );
        assert_eq!(log.entries[i].event.amount, i as f32);
    }
}

#[test]
fn human_melee_on_training_dummy_logs_every_swing_and_never_kills_it() {
    let mut app = setup_combat_app();
    let Some(templates) = crate::faction::world_db_faction_templates() else {
        return;
    };
    app.insert_resource(templates);
    // Grandmaster's Training Dummy (31144): ceil(12600 basehp2 × 0.000187) hp, faction 7.
    let health = Health {
        current: 3.0,
        max: 3.0,
    };
    let dummy = app
        .world_mut()
        .spawn((
            pos(1.0, 0.0, 0.0),
            health,
            Npc {
                template_id: 31144,
                name: "Grandmaster's Training Dummy".into(),
            },
            UnitFactionTemplate(7),
            TrainingDummy::default(),
        ))
        .id();
    let player = app
        .world_mut()
        .spawn((
            pos(0.0, 0.0, 0.0),
            crate::maps::InMap::continent(0),
            Player {
                name: "Human".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            UnitFactionTemplate(1),
            CombatTarget(Some(dummy)),
            AutoAttack::new(dummy),
        ))
        .id();
    app.world_mut().run_schedule(FixedUpdate);
    for _ in 0..3 {
        app.world_mut().get_mut::<AutoAttack>(player).unwrap().timer = 0.0;
        app.world_mut().run_schedule(FixedUpdate);
    }

    let log = app.world().resource::<CombatLog>().range(0.0, f64::MAX);
    let swings: Vec<f32> = log
        .iter()
        .filter(|entry| {
            entry.event.target == dummy.to_bits()
                && entry.event.event_type == CombatEventType::MeleeDamage
        })
        .map(|entry| entry.event.amount)
        .collect();
    // The first swing lands in the first tick.
    assert_eq!(
        swings,
        vec![BASE_DAMAGE; 4],
        "each swing logs its full damage"
    );
    assert!(
        log.iter()
            .all(|entry| entry.event.event_type != CombatEventType::Death)
    );
    let entity = app.world().entity(dummy);
    assert_eq!(*entity.get::<Health>().unwrap(), health);
    assert!(entity.get::<DeathState>().is_none());
    assert!(
        entity.get::<AutoAttack>().is_none(),
        "the dummy never swings back"
    );
    assert!(entity.get::<CombatStatus>().is_some_and(|status| status.0));
}

#[test]
fn looting_a_corpse_empty_halves_its_remaining_decay_and_respawn() {
    // Kobold Vermin: spawntimesecs 180 + 60 s decay; looted 20 s after death.
    let mut corpse = Corpse { remaining: 40.0 };
    let mut death = DeathState {
        respawn_timer: 220.0,
    };
    corpse_looted(&mut corpse, &mut death);
    assert_eq!(corpse, Corpse { remaining: 20.0 });
    assert_eq!(death.respawn_timer, 200.0);
}
