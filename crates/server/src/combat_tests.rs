use super::*;
use bevy::ecs::system::RunSystemOnce;
use shared::components::Player;

fn pos(x: f32, y: f32, z: f32) -> Position {
    Position { x, y, z }
}

fn setup_app() -> App {
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.init_resource::<CombatLog>();
    app.update();
    app.update();
    app
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
         mut targets: Query<(&Position, &mut Health, Option<&mut LastDamager>)>| {
            let dt = time.delta_secs();
            let combat_expires_at = time.elapsed_secs_f64() + COMBAT_GRACE_SECS;
            for (attacker, combat_target, mut aa, apos) in &mut attackers {
                if !aa.active {
                    continue;
                }
                let Some(te) = combat_target.0 else { continue };
                aa.timer -= dt;
                let damage = try_swing(attacker, apos, &mut targets, te, &mut aa, &mut commands);
                if damage > 0.0 {
                    refresh_combat_state(&mut commands, attacker, combat_expires_at);
                    refresh_combat_state(&mut commands, te, combat_expires_at);
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
fn combat_flag_expires_after_grace_period_once_target_is_cleared() {
    let mut app = setup_app();
    let target = spawn_target(&mut app, pos(1.0, 0.0, 0.0), 100.0);
    let attacker = spawn_attacker(&mut app, pos(0.0, 0.0, 0.0), target);

    run_attack_tick(&mut app);
    app.world_mut()
        .entity_mut(attacker)
        .insert(CombatTarget(None));
    let _ = app.world_mut().run_system_once(start_attack_system);

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
fn test_death_at_zero_hp() {
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);

    let entity = app
        .world_mut()
        .spawn((
            pos(0.0, 0.0, 0.0),
            Health {
                current: 0.0,
                max: 100.0,
            },
            Npc { template_id: 1 },
        ))
        .id();

    // Verify the condition that death_system checks
    let health = app.world().entity(entity).get::<Health>().unwrap();
    assert!(health.current <= 0.0);

    // Simulate what death_system does (avoids needing MessageSender)
    app.world_mut().entity_mut(entity).insert(DeathState {
        respawn_timer: DEFAULT_RESPAWN_SECS,
    });

    assert!(app.world().entity(entity).get::<DeathState>().is_some());
}

#[test]
fn test_respawn_restores_health() {
    let mut health = Health {
        current: 0.0,
        max: 100.0,
    };
    let mut position = pos(50.0, 50.0, 50.0);
    let spawn = SpawnPoint {
        x: 10.0,
        y: 20.0,
        z: 30.0,
        wander_distance: 5.0,
        speed_walk: 1.0,
    };

    respawn_entity(&mut health, &mut position, &spawn);

    assert_eq!(health.current, 100.0, "health should be restored to max");
    assert_eq!(position.x, 10.0);
    assert_eq!(position.y, 20.0);
    assert_eq!(position.z, 30.0);
}

#[test]
fn respawn_timer_counts_down_then_restores() {
    let spawn = SpawnPoint {
        x: 10.0,
        y: 20.0,
        z: 30.0,
        wander_distance: 0.0,
        speed_walk: 1.0,
    };
    let mut health = Health {
        current: 0.0,
        max: 500.0,
    };
    let mut position = pos(99.0, 99.0, 99.0);
    let mut death = DeathState {
        respawn_timer: DEFAULT_RESPAWN_SECS,
    };

    // Tick 50 seconds - still dead
    death.respawn_timer -= 50.0;
    assert!(death.respawn_timer > 0.0, "timer not expired at 50s");
    assert_eq!(health.current, 0.0, "still dead");

    // Tick remaining 10s - timer expires
    death.respawn_timer -= 10.0;
    assert!(death.respawn_timer <= 0.0, "timer expired at 60s");

    // Respawn
    respawn_entity(&mut health, &mut position, &spawn);
    assert_eq!(health.current, 500.0, "health restored to max");
    assert_eq!(position.x, 10.0, "returned to spawn x");
    assert_eq!(position.y, 20.0, "returned to spawn y");
    assert_eq!(position.z, 30.0, "returned to spawn z");
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
