use super::*;
use bevy::ecs::system::RunSystemOnce;
use shared::components::Player;

#[path = "combat_tests/death.rs"]
mod death;

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
            AutoAttack {
                timer: 0.0,
                active: true,
            },
            Player {
                name: "test".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
        ))
        .id()
}

fn run_maintain_combat_state_tick(app: &mut App) {
    let _ = app
        .world_mut()
        .run_system_once(maintain_in_combat_state_system);
}

/// Run auto-attack logic without needing lightyear MessageSender.
fn run_attack_tick(app: &mut App) {
    let _ = app.world_mut().run_system_once(
        |time: Res<Time>,
         mut commands: Commands,
         mut attackers: Query<(Entity, &CombatTarget, &mut AutoAttack, &Position)>,
         mut targets: AutoAttackTargets,
         combat_statuses: Query<&CombatStatus>| {
            let dt = time.delta_secs();
            let combat_expires_at = time.elapsed_secs_f64() + COMBAT_GRACE_SECS;
            for (attacker, combat_target, mut aa, apos) in &mut attackers {
                if !aa.active {
                    continue;
                }
                let Some(te) = combat_target.0 else { continue };
                aa.timer -= dt;
                if try_swing(
                    attacker,
                    apos,
                    None,
                    &mut targets,
                    te,
                    &mut aa,
                    &mut commands,
                )
                .is_some()
                {
                    refresh_combat_state(
                        &mut commands,
                        &combat_statuses,
                        attacker,
                        combat_expires_at,
                    );
                    refresh_combat_state(&mut commands, &combat_statuses, te, combat_expires_at);
                }
            }
        },
    );
}

#[test]
fn test_in_melee_range() {
    assert!(in_melee_range(&pos(0.0, 0.0, 0.0), &pos(3.0, 4.0, 0.0)));
    assert!(!in_melee_range(&pos(0.0, 0.0, 0.0), &pos(10.0, 0.0, 0.0)));
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
    let expires_at = app.world().resource::<Time>().elapsed_secs_f64() + COMBAT_GRACE_SECS;
    app.world_mut()
        .entity_mut(attacker)
        .insert(active_combat_state(expires_at));
    app.update();

    let tick_before = app
        .world()
        .entity(attacker)
        .get_ref::<CombatStatus>()
        .unwrap()
        .last_changed();

    run_maintain_combat_state_tick(&mut app);

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
fn combat_flag_expires_after_grace_period_once_target_is_cleared() {
    let mut app = setup_app();
    let target = spawn_target(&mut app, pos(1.0, 0.0, 0.0), 100.0);
    let attacker = spawn_attacker(&mut app, pos(0.0, 0.0, 0.0), target);

    run_attack_tick(&mut app);
    app.world_mut()
        .entity_mut(attacker)
        .insert(CombatTarget(None));
    let _ = app.world_mut().run_system_once(start_attack_system);
    assert!(
        app.world().entity(attacker).get::<AutoAttack>().is_none(),
        "clearing the combat target should remove the inactive auto-attack"
    );

    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f64(COMBAT_GRACE_SECS - 1.0));
    run_maintain_combat_state_tick(&mut app);
    assert!(
        app.world()
            .entity(attacker)
            .get::<CombatStatus>()
            .is_some_and(|flag| flag.0),
        "combat flag should stay active during grace period"
    );

    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f64(2.0));
    run_maintain_combat_state_tick(&mut app);
    assert!(
        app.world()
            .entity(attacker)
            .get::<CombatStatus>()
            .is_some_and(|flag| !flag.0),
        "combat flag should clear after grace period"
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
fn lethal_auto_attack_produces_death_state_replication_removal_and_event() {
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
        ))
        .id();
    spawn_attacker(&mut app, pos(0.0, 0.0, 0.0), target);

    app.world_mut()
        .run_system_once(auto_attack_system)
        .expect("auto-attack system should run");
    app.update();

    let entity = app.world().entity(target);
    assert!(entity.get::<DeathState>().is_some());
    assert!(entity.get::<Replicate>().is_none());
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

/// Human player (FactionTemplate 1) targeting an NPC of `template` in melee
/// range, full CombatPlugin with the world.db faction templates.
fn target_npc_with_template(template: u32) -> Option<(App, Entity, Entity)> {
    let mut app = setup_combat_app();
    app.insert_resource(crate::faction::world_db_faction_templates()?);
    let npc = app
        .world_mut()
        .spawn((
            pos(1.0, 0.0, 0.0),
            Health {
                current: 100.0,
                max: 100.0,
            },
            Npc {
                template_id: 68,
                name: "Stormwind City Guard".into(),
            },
            UnitFactionTemplate(template),
        ))
        .id();
    let player = app
        .world_mut()
        .spawn((
            pos(0.0, 0.0, 0.0),
            Player {
                name: "Human".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            UnitFactionTemplate(1),
            CombatTarget(Some(npc)),
        ))
        .id();
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
    Some((app, player, npc))
}

#[test]
fn auto_attack_on_friendly_guard_is_rejected() {
    // FactionTemplate 11: Stormwind City Guard.
    let Some((app, player, guard)) = target_npc_with_template(11) else {
        return;
    };
    assert!(app.world().get::<AutoAttack>(player).is_none());
    assert!(app.world().get::<CombatStatus>(player).is_none());
    assert_eq!(app.world().get::<Health>(guard).unwrap().current, 100.0);
}

#[test]
fn auto_attack_on_hostile_kobold_swings() {
    // FactionTemplate 25: Kobold Vermin.
    let Some((app, player, kobold)) = target_npc_with_template(25) else {
        return;
    };
    assert!(app.world().get::<AutoAttack>(player).is_some());
    assert_eq!(app.world().get::<Health>(kobold).unwrap().current, 90.0);
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
            Player {
                name: "Human".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            UnitFactionTemplate(1),
            CombatTarget(Some(dummy)),
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
    assert_eq!(
        swings,
        vec![BASE_DAMAGE; 3],
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
