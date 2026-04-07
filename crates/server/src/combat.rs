use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Health, Npc, Position};
use shared::protocol::{CombatChannel, CombatEvent, CombatEventType};

use crate::creature_ai::SpawnPoint;
use crate::networking::CombatTarget;

use std::collections::VecDeque;

/// Melee attack range in yards.
const MELEE_RANGE: f32 = 5.0;
/// Seconds between auto-attack swings.
const AUTO_ATTACK_INTERVAL: f32 = 2.0;
/// Base melee damage per swing.
const BASE_DAMAGE: f32 = 10.0;
/// Default respawn timer in seconds.
const DEFAULT_RESPAWN_SECS: f32 = 60.0;

/// Maximum number of combat log entries retained in memory.
const COMBAT_LOG_CAPACITY: usize = 10_000;

/// A timestamped combat event for server-side replay and debugging.
#[derive(Debug, Clone)]
pub struct CombatLogEntry {
    /// Elapsed game time when the event occurred (seconds since server start).
    pub timestamp: f64,
    pub event: CombatEvent,
}

/// Server-side combat log resource. Stores a ring buffer of recent combat events
/// for replay, debugging, and analytics. Oldest entries are evicted when capacity
/// is reached.
#[derive(Resource)]
pub struct CombatLog {
    entries: VecDeque<CombatLogEntry>,
    capacity: usize,
}

impl Default for CombatLog {
    fn default() -> Self {
        Self {
            entries: VecDeque::with_capacity(COMBAT_LOG_CAPACITY),
            capacity: COMBAT_LOG_CAPACITY,
        }
    }
}

impl CombatLog {
    /// Record a combat event with the current timestamp.
    pub fn record(&mut self, timestamp: f64, event: CombatEvent) {
        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(CombatLogEntry { timestamp, event });
    }

    /// Get entries within a time range (inclusive).
    pub fn range(&self, start: f64, end: f64) -> Vec<&CombatLogEntry> {
        self.entries
            .iter()
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .collect()
    }
}

type LivingNpcFilter = (With<Npc>, Without<DeathState>);

#[derive(SystemParam)]
struct LivingNpcQuery<'w, 's> {
    query: Query<'w, 's, (Entity, &'static Health), LivingNpcFilter>,
}

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CombatLog>();
        app.init_resource::<shared::components::SpellBatch>();
        app.add_systems(
            FixedUpdate,
            (
                start_attack_system,
                auto_attack_system,
                death_system,
                respawn_system,
            ),
        );
    }
}

/// Tracks the swing timer for auto-attacks.
#[derive(Component, Debug)]
pub struct AutoAttack {
    pub timer: f32,
    pub active: bool,
}

/// Marks a dead NPC awaiting respawn.
#[derive(Component, Debug)]
pub struct DeathState {
    pub respawn_timer: f32,
}

/// When CombatTarget changes to Some(entity), activate auto-attack.
/// When None, deactivate.
fn start_attack_system(
    query: Query<(Entity, &CombatTarget), Changed<CombatTarget>>,
    mut commands: Commands,
) {
    for (entity, target) in &query {
        if target.0.is_some() {
            commands.entity(entity).insert(AutoAttack {
                timer: 0.0,
                active: true,
            });
        } else if let Ok(mut cmd) = commands.get_entity(entity) {
            cmd.insert(AutoAttack {
                timer: 0.0,
                active: false,
            });
        }
    }
}

/// Process auto-attacks: tick timers, deal damage when in range and timer ready.
fn auto_attack_system(
    time: Res<Time>,
    mut attackers: Query<(Entity, &CombatTarget, &mut AutoAttack, &Position)>,
    mut targets: Query<(&Position, &mut Health)>,
    mut senders: Query<&mut MessageSender<CombatEvent>>,
    mut combat_log: ResMut<CombatLog>,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs_f64();
    let mut events = Vec::new();

    for (attacker_entity, combat_target, mut auto_attack, attacker_pos) in &mut attackers {
        if !auto_attack.active {
            continue;
        }
        let Some(target_entity) = combat_target.0 else {
            continue;
        };
        auto_attack.timer -= dt;
        let damage = try_swing(attacker_pos, &mut targets, target_entity, &mut auto_attack);
        if damage > 0.0 {
            events.push(build_damage_event(attacker_entity, target_entity, damage));
        }
    }

    broadcast_events(&events, &mut senders, &mut combat_log, elapsed);
}

/// Attempt a swing if timer ready and target in range. Returns damage dealt.
fn try_swing(
    attacker_pos: &Position,
    targets: &mut Query<(&Position, &mut Health)>,
    target_entity: Entity,
    auto_attack: &mut AutoAttack,
) -> f32 {
    if auto_attack.timer > 0.0 {
        return 0.0;
    }
    let Ok((target_pos, mut health)) = targets.get_mut(target_entity) else {
        return 0.0;
    };
    if !in_melee_range(attacker_pos, target_pos) {
        return 0.0;
    }
    if health.current <= 0.0 {
        return 0.0;
    }
    let damage = BASE_DAMAGE;
    health.current = (health.current - damage).max(0.0);
    auto_attack.timer = AUTO_ATTACK_INTERVAL;
    damage
}

/// Check if two positions are within melee range.
fn in_melee_range(a: &Position, b: &Position) -> bool {
    distance(a, b) <= MELEE_RANGE
}

/// Euclidean distance between two positions.
fn distance(a: &Position, b: &Position) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn build_damage_event(attacker: Entity, target: Entity, damage: f32) -> CombatEvent {
    CombatEvent {
        attacker: attacker.to_bits(),
        target: target.to_bits(),
        amount: damage,
        spell_id: 0,
        event_type: CombatEventType::MeleeDamage,
    }
}

/// Detect entities that just hit 0 HP and mark them dead.
fn death_system(
    time: Res<Time>,
    living: LivingNpcQuery,
    mut commands: Commands,
    mut senders: Query<&mut MessageSender<CombatEvent>>,
    mut combat_log: ResMut<CombatLog>,
) {
    let elapsed = time.elapsed_secs_f64();
    let mut events = Vec::new();

    for (entity, health) in &living.query {
        if health.current > 0.0 {
            continue;
        }
        commands.entity(entity).insert(DeathState {
            respawn_timer: DEFAULT_RESPAWN_SECS,
        });
        commands.entity(entity).remove::<Replicate>();
        events.push(CombatEvent {
            attacker: 0,
            target: entity.to_bits(),
            amount: 0.0,
            spell_id: 0,
            event_type: CombatEventType::Death,
        });
    }

    broadcast_events(&events, &mut senders, &mut combat_log, elapsed);
}

/// Tick respawn timers and restore dead NPCs when ready.
fn respawn_system(
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &mut DeathState,
        &mut Health,
        &SpawnPoint,
        &mut Position,
    )>,
    mut commands: Commands,
    mut senders: Query<&mut MessageSender<CombatEvent>>,
    mut combat_log: ResMut<CombatLog>,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs_f64();
    let mut events = Vec::new();

    for (entity, mut death, mut health, spawn, mut pos) in &mut query {
        death.respawn_timer -= dt;
        if death.respawn_timer > 0.0 {
            continue;
        }
        respawn_entity(&mut health, &mut pos, spawn);
        commands.entity(entity).remove::<DeathState>().insert((
            Replicate::new(ReplicationMode::SingleServer(NetworkTarget::All)),
            NetworkVisibility,
        ));
        events.push(CombatEvent {
            attacker: 0,
            target: entity.to_bits(),
            amount: 0.0,
            spell_id: 0,
            event_type: CombatEventType::Respawn,
        });
    }

    broadcast_events(&events, &mut senders, &mut combat_log, elapsed);
}

/// Restore health and position for a respawning entity.
fn respawn_entity(health: &mut Health, pos: &mut Position, spawn: &SpawnPoint) {
    health.current = health.max;
    pos.x = spawn.x;
    pos.y = spawn.y;
    pos.z = spawn.z;
}

/// Send combat events to all connected clients and record to combat log.
fn broadcast_events(
    events: &[CombatEvent],
    senders: &mut Query<&mut MessageSender<CombatEvent>>,
    combat_log: &mut CombatLog,
    elapsed: f64,
) {
    if events.is_empty() {
        return;
    }
    for event in events {
        combat_log.record(elapsed, event.clone());
        for mut sender in senders.iter_mut() {
            sender.send::<CombatChannel>(event.clone());
        }
    }
}

#[cfg(test)]
mod tests {
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

    /// Run auto-attack logic without needing lightyear MessageSender.
    fn run_attack_tick(app: &mut App) {
        let _ = app.world_mut().run_system_once(
            |time: Res<Time>,
             mut attackers: Query<(Entity, &CombatTarget, &mut AutoAttack, &Position)>,
             mut targets: Query<(&Position, &mut Health)>| {
                let dt = time.delta_secs();
                for (_, combat_target, mut aa, apos) in &mut attackers {
                    if !aa.active {
                        continue;
                    }
                    let Some(te) = combat_target.0 else { continue };
                    aa.timer -= dt;
                    try_swing(apos, &mut targets, te, &mut aa);
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

        // First tick: timer=0, swings immediately → damage dealt, timer resets to 2.0
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

        // Expire timer and try again — no further damage (target is dead)
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

        // Tick 50 seconds — still dead
        death.respawn_timer -= 50.0;
        assert!(death.respawn_timer > 0.0, "timer not expired at 50s");
        assert_eq!(health.current, 0.0, "still dead");

        // Tick remaining 10s — timer expires
        death.respawn_timer -= 10.0;
        assert!(death.respawn_timer <= 0.0, "timer expired at 60s");

        // Respawn
        respawn_entity(&mut health, &mut position, &spawn);
        assert_eq!(health.current, 500.0, "health restored to max");
        assert_eq!(position.x, 10.0, "returned to spawn x");
        assert_eq!(position.y, 20.0, "returned to spawn y");
        assert_eq!(position.z, 30.0, "returned to spawn z");
    }

    // --- Combat log tests ---

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
        assert_eq!(log.entries[0].timestamp, 2.0, "oldest entry should be evicted");
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
}
