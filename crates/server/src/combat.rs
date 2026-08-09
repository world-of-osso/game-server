use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{CombatStatus, Health, Npc, Position};
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
const COMBAT_GRACE_SECS: f64 = 5.0;

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
                maintain_in_combat_state_system,
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

/// Tracks the last entity that dealt damage to a target.
#[derive(Component, Debug, Clone, Copy)]
pub struct LastDamager(pub Entity);

#[derive(Component, Debug, Clone, Copy)]
struct CombatState {
    expires_at: f64,
}

/// When CombatTarget changes to Some(entity), activate auto-attack.
/// When None, deactivate.
fn start_attack_system(
    query: Query<(Entity, &CombatTarget), Changed<CombatTarget>>,
    mut commands: Commands,
    time: Res<Time>,
) {
    let combat_expires_at = time.elapsed_secs_f64() + COMBAT_GRACE_SECS;
    for (entity, target) in &query {
        if target.0.is_some() {
            commands.entity(entity).insert((
                AutoAttack {
                    timer: 0.0,
                    active: true,
                },
                active_combat_state(combat_expires_at),
            ));
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
    mut targets: Query<(&Position, &mut Health, Option<&mut LastDamager>)>,
    mut senders: Query<&mut MessageSender<CombatEvent>>,
    mut combat_log: ResMut<CombatLog>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs_f64();
    let combat_expires_at = elapsed + COMBAT_GRACE_SECS;
    let mut events = Vec::new();

    for (attacker_entity, combat_target, mut auto_attack, attacker_pos) in &mut attackers {
        if !auto_attack.active {
            continue;
        }
        let Some(target_entity) = combat_target.0 else {
            continue;
        };
        auto_attack.timer -= dt;
        let damage = try_swing(
            attacker_entity,
            attacker_pos,
            &mut targets,
            target_entity,
            &mut auto_attack,
            &mut commands,
        );
        if damage > 0.0 {
            refresh_combat_state(&mut commands, attacker_entity, combat_expires_at);
            refresh_combat_state(&mut commands, target_entity, combat_expires_at);
            events.push(build_damage_event(attacker_entity, target_entity, damage));
        }
    }

    broadcast_events(&events, &mut senders, &mut combat_log, elapsed);
}

fn maintain_in_combat_state_system(
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &mut CombatState,
        Option<&CombatTarget>,
        Option<&AutoAttack>,
        Option<&CombatStatus>,
    )>,
    mut commands: Commands,
) {
    let combat_expires_at = time.elapsed_secs_f64() + COMBAT_GRACE_SECS;
    let now = time.elapsed_secs_f64();
    for (entity, mut combat_state, combat_target, auto_attack, combat_status) in &mut query {
        let actively_engaged = combat_target.and_then(|target| target.0).is_some()
            && auto_attack.is_some_and(|auto_attack| auto_attack.active);
        if actively_engaged {
            combat_state.expires_at = combat_expires_at;
            if !combat_status.is_some_and(|status| status.0) {
                commands.entity(entity).insert(CombatStatus(true));
            }
            continue;
        }
        if now >= combat_state.expires_at {
            commands
                .entity(entity)
                .insert(CombatStatus(false))
                .remove::<CombatState>();
        }
    }
}

/// Attempt a swing if timer ready and target in range. Returns damage dealt.
fn try_swing(
    attacker_entity: Entity,
    attacker_pos: &Position,
    targets: &mut Query<(&Position, &mut Health, Option<&mut LastDamager>)>,
    target_entity: Entity,
    auto_attack: &mut AutoAttack,
    commands: &mut Commands,
) -> f32 {
    if auto_attack.timer > 0.0 {
        return 0.0;
    }
    let Ok((target_pos, mut health, last_damager)) = targets.get_mut(target_entity) else {
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
    if let Some(mut last_damager) = last_damager {
        last_damager.0 = attacker_entity;
    } else {
        commands
            .entity(target_entity)
            .insert(LastDamager(attacker_entity));
    }
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

fn active_combat_state(expires_at: f64) -> (CombatState, CombatStatus) {
    (CombatState { expires_at }, CombatStatus(true))
}

fn refresh_combat_state(commands: &mut Commands, entity: Entity, expires_at: f64) {
    commands
        .entity(entity)
        .insert(active_combat_state(expires_at));
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
        commands.entity(entity).remove::<LastDamager>();
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
#[path = "combat_tests.rs"]
mod tests;
