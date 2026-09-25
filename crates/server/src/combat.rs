use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{CombatStatus, Health, Npc, Player, Position, UnitFactionTemplate};
use shared::protocol::{
    CombatChannel, CombatEvent, CombatEventType, CombatLogEvent, CombatLogKind,
};

use crate::auras::AuraStatMods;
use crate::auras::procs::{self, AuraEvent, PendingAuraEvents};
use crate::creature_ai::{SpawnPoint, WaypointState};
use crate::creature_scripts::{TrainingDummy, health_loss};
use crate::faction::{FactionTemplates, UnitFaction};
use crate::interest::{InterestDirtyQueue, NetworkVisibility, mark_interest_position_changed};
use crate::networking::CombatTarget;

use std::collections::VecDeque;

/// Melee attack range in yards.
const MELEE_RANGE: f32 = 5.0;
/// Seconds between auto-attack swings.
const AUTO_ATTACK_INTERVAL: f32 = 2.0;
/// Base melee damage per swing.
const BASE_DAMAGE: f32 = 10.0;
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

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<crate::death::PlayerDeathTransition>();
        app.init_resource::<CombatLog>();
        app.init_resource::<shared::components::SpellBatch>();
        app.add_systems(
            FixedUpdate,
            (
                start_attack_system,
                auto_attack_system,
                maintain_in_combat_state_system,
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

/// Seconds from death to respawn: AzerothCore `Creature::setDeathState`
/// `m_respawnTime = now + m_respawnDelay (spawntimesecs) + m_corpseDelay`.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct RespawnDelay(pub f32);

impl RespawnDelay {
    pub fn for_spawn(spawntimesecs: u32, rank: u8) -> Self {
        Self((spawntimesecs + corpse_decay_secs(rank)) as f32)
    }
}

/// `m_corpseDelay` by `creature_template.rank` (Creature::Create, default
/// Corpse.Decay.* config; world bosses outside instances get ELITE * 2).
fn corpse_decay_secs(rank: u8) -> u32 {
    match rank {
        1 | 2 | 4 => 300,
        3 => 600,
        _ => 60,
    }
}

/// Tracks the last entity that dealt damage to a target.
#[derive(Component, Debug, Clone, Copy)]
pub struct LastDamager(pub Entity);

#[derive(Component, Debug, Clone, Copy)]
struct CombatState {
    expires_at: f64,
}

/// When CombatTarget changes to an attackable unit, activate auto-attack.
/// When None or not attackable (for example a friendly guard), deactivate.
fn start_attack_system(
    query: Query<(Entity, &CombatTarget), Changed<CombatTarget>>,
    factions: Query<(Option<&UnitFactionTemplate>, Has<Player>)>,
    templates: Res<FactionTemplates>,
    mut commands: Commands,
    time: Res<Time>,
) {
    let combat_expires_at = time.elapsed_secs_f64() + COMBAT_GRACE_SECS;
    let unit_faction = |entity: Entity| {
        factions
            .get(entity)
            .ok()
            .map(|(template, is_player)| UnitFaction {
                template: template.map_or(0, |template| template.0),
                is_player,
            })
    };
    for (entity, target) in &query {
        let attackable =
            target.0.is_some_and(
                |target| match (unit_faction(entity), unit_faction(target)) {
                    (Some(attacker), Some(target)) => templates.can_attack(attacker, target),
                    _ => false,
                },
            );
        if attackable {
            commands.entity(entity).insert((
                AutoAttack {
                    timer: 0.0,
                    active: true,
                },
                active_combat_state(combat_expires_at),
            ));
        } else if let Ok(mut cmd) = commands.get_entity(entity) {
            cmd.remove::<AutoAttack>();
        }
    }
}

type AutoAttackers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static CombatTarget,
        &'static mut AutoAttack,
        &'static Position,
        Option<&'static AuraStatMods>,
    ),
>;

type AutoAttackTargets<'w, 's> = Query<
    'w,
    's,
    (
        &'static Position,
        &'static mut Health,
        Option<&'static mut LastDamager>,
        Has<Npc>,
        Option<&'static AuraStatMods>,
        Has<TrainingDummy>,
    ),
>;

#[derive(SystemParam)]
pub(crate) struct AutoAttackParams<'w, 's> {
    attackers: AutoAttackers<'w, 's>,
    targets: AutoAttackTargets<'w, 's>,
    senders: Query<'w, 's, &'static mut MessageSender<CombatEvent>>,
    combat_log: ResMut<'w, CombatLog>,
    commands: Commands<'w, 's>,
    combat_statuses: Query<'w, 's, &'static CombatStatus>,
    replication_dirty: Option<Res<'w, ReplicationDirtyQueue>>,
    player_deaths: MessageWriter<'w, crate::death::PlayerDeathTransition>,
    aura_events: Option<ResMut<'w, PendingAuraEvents>>,
}

/// Process auto-attacks: tick timers, deal damage when in range and timer ready.
pub(crate) fn auto_attack_system(time: Res<Time>, mut params: AutoAttackParams) {
    let dt = time.delta_secs();
    let (elapsed, combat_expires_at) = combat_timing(&time);
    let mut events = Vec::new();

    for (attacker_entity, combat_target, mut auto_attack, attacker_pos, attacker_mods) in
        &mut params.attackers
    {
        let Some(target_entity) = ready_auto_attack_target(combat_target, &mut auto_attack, dt)
        else {
            continue;
        };
        let Some(outcome) = try_swing(
            attacker_entity,
            attacker_pos,
            attacker_mods,
            &mut params.targets,
            target_entity,
            &mut auto_attack,
            &mut params.commands,
        ) else {
            continue;
        };
        refresh_combatants(
            &mut params.commands,
            &params.combat_statuses,
            attacker_entity,
            target_entity,
            combat_expires_at,
        );
        mark_replication_changed(params.replication_dirty.as_deref(), target_entity);
        if let Some(pending) = params.aura_events.as_deref_mut() {
            pending
                .0
                .extend(swing_aura_events(attacker_entity, target_entity, &outcome));
        }
        record_swing_events(
            &mut params.commands,
            &mut events,
            &mut params.player_deaths,
            attacker_entity,
            target_entity,
            outcome,
        );
    }

    broadcast_events(
        &events,
        &mut params.senders,
        &mut params.combat_log,
        elapsed,
    );
}

fn combat_timing(time: &Time) -> (f64, f64) {
    let elapsed = time.elapsed_secs_f64();
    (elapsed, elapsed + COMBAT_GRACE_SECS)
}

fn ready_auto_attack_target(
    combat_target: &CombatTarget,
    auto_attack: &mut AutoAttack,
    delta_seconds: f32,
) -> Option<Entity> {
    if !auto_attack.active {
        return None;
    }
    auto_attack.timer -= delta_seconds;
    combat_target.0
}

type CombatMaintenanceQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut CombatState,
        Option<&'static CombatTarget>,
        Option<&'static AutoAttack>,
        Option<&'static CombatStatus>,
    ),
>;

fn maintain_in_combat_state_system(
    time: Res<Time>,
    mut query: CombatMaintenanceQuery,
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
            let mut entity_commands = commands.entity(entity);
            if combat_status.is_none_or(|status| status.0) {
                entity_commands.insert(CombatStatus(false));
            }
            entity_commands.remove::<CombatState>();
        }
    }
}

/// Melee swing proc/interrupt events: DEAL/TAKE_MELEE_SWING and
/// TAKE_ANY_DAMAGE, plus KILL/DEATH on a lethal swing.
fn swing_aura_events(attacker: Entity, target: Entity, outcome: &SwingOutcome) -> Vec<AuraEvent> {
    let mut events = vec![AuraEvent {
        actor: attacker,
        target: Some(target),
        actor_mask: [
            procs::PROC_DEAL_MELEE_SWING | procs::PROC_MAIN_HAND_WEAPON_SWING,
            0,
        ],
        target_mask: [
            procs::PROC_TAKE_MELEE_SWING | procs::PROC_TAKE_ANY_DAMAGE,
            0,
        ],
        spell_id: None,
        can_proc: true,
        periodic: false,
    }];
    if outcome.npc_died || outcome.player_died {
        events.push(AuraEvent {
            actor: attacker,
            target: Some(target),
            actor_mask: [procs::PROC_KILL, 0],
            target_mask: [procs::PROC_DEATH, 0],
            spell_id: None,
            can_proc: true,
            periodic: false,
        });
    }
    events
}

/// Base swing damage × attacker physical damage done and auto-attack
/// multipliers × target physical damage taken (`AuraStatMods`).
fn swing_damage(attacker: Option<&AuraStatMods>, target: Option<&AuraStatMods>) -> f32 {
    const PHYSICAL: u32 = 1;
    let done = attacker.map_or(1.0, |mods| {
        mods.damage_done(PHYSICAL) * mods.autoattack_damage_mult
    });
    let taken = target.map_or(1.0, |mods| mods.damage_taken(PHYSICAL));
    (BASE_DAMAGE * done * taken).max(0.0).round()
}

struct SwingOutcome {
    damage: f32,
    /// Damage beyond the target's health before the swing.
    overkill: f32,
    npc_died: bool,
    player_died: bool,
}

/// Attempt a swing if timer ready and target in range.
fn try_swing(
    attacker_entity: Entity,
    attacker_pos: &Position,
    attacker_mods: Option<&AuraStatMods>,
    targets: &mut AutoAttackTargets,
    target_entity: Entity,
    auto_attack: &mut AutoAttack,
    commands: &mut Commands,
) -> Option<SwingOutcome> {
    if auto_attack.timer > 0.0 {
        return None;
    }
    let Ok((target_pos, mut health, last_damager, is_npc, target_mods, training_dummy)) =
        targets.get_mut(target_entity)
    else {
        return None;
    };
    if !in_melee_range(attacker_pos, target_pos) || health.current <= 0.0 {
        return None;
    }
    let damage = swing_damage(attacker_mods, target_mods);
    let overkill = (damage - health.current).max(0.0);
    let became_lethal =
        crate::death::apply_health_damage(&mut health, health_loss(training_dummy, damage));
    let npc_died = is_npc && became_lethal;
    let player_died = !is_npc && became_lethal;
    if let Some(mut last_damager) = last_damager {
        last_damager.0 = attacker_entity;
    } else {
        commands
            .entity(target_entity)
            .insert(LastDamager(attacker_entity));
    }
    auto_attack.timer = AUTO_ATTACK_INTERVAL;
    Some(SwingOutcome {
        damage,
        overkill,
        npc_died,
        player_died,
    })
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

/// The swing in the attacker's and target's combat log (AzerothCore
/// `SendAttackStateUpdate`): physical Damage without a spell.
fn swing_log_event(attacker: Entity, target: Entity, outcome: &SwingOutcome) -> CombatLogEvent {
    CombatLogEvent {
        source: Some(attacker.to_bits()),
        target: Some(target.to_bits()),
        spell_id: None,
        school_mask: 1,
        amount: outcome.damage as i32,
        overflow: outcome.overkill as i32,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit: false,
        periodic: false,
        kind: CombatLogKind::Damage,
    }
}

fn build_death_event(target: Entity) -> CombatEvent {
    CombatEvent {
        attacker: 0,
        target: target.to_bits(),
        amount: 0.0,
        spell_id: 0,
        event_type: CombatEventType::Death,
    }
}

fn mark_replication_changed(queue: Option<&ReplicationDirtyQueue>, entity: Entity) {
    if let Some(queue) = queue {
        queue.mark(entity);
    }
}

fn mark_npc_dead(commands: &mut Commands, entity: Entity) {
    commands.entity(entity).queue(insert_npc_death);
}

/// World-access `mark_npc_dead` for exclusive systems (spell effects).
pub(crate) fn mark_npc_dead_world(world: &mut World, entity: Entity) {
    insert_npc_death(world.entity_mut(entity));
}

/// Start the NPC's respawn timer and stop replicating it. NPCs without a
/// `RespawnDelay` (not world spawns) have no `SpawnPoint` and never respawn.
fn insert_npc_death(mut entity: EntityWorldMut) {
    let respawn_timer = entity.get::<RespawnDelay>().map_or(0.0, |delay| delay.0);
    entity
        .insert(DeathState { respawn_timer })
        .remove::<Replicate>();
}

/// Put `entity` in combat for the grace period (spell damage dealt or taken).
pub(crate) fn enter_combat(world: &mut World, entity: Entity) {
    let expires_at = world.resource::<Time>().elapsed_secs_f64() + COMBAT_GRACE_SECS;
    let in_combat = world
        .get::<CombatStatus>(entity)
        .is_some_and(|status| status.0);
    let mut entity = world.entity_mut(entity);
    entity.insert(CombatState { expires_at });
    if !in_combat {
        entity.insert(CombatStatus(true));
    }
}

fn active_combat_state(expires_at: f64) -> (CombatState, CombatStatus) {
    (CombatState { expires_at }, CombatStatus(true))
}

fn refresh_combatants(
    commands: &mut Commands,
    combat_statuses: &Query<&CombatStatus>,
    attacker: Entity,
    target: Entity,
    expires_at: f64,
) {
    refresh_combat_state(commands, combat_statuses, attacker, expires_at);
    refresh_combat_state(commands, combat_statuses, target, expires_at);
}

fn record_swing_events(
    commands: &mut Commands,
    events: &mut Vec<CombatEvent>,
    player_deaths: &mut MessageWriter<crate::death::PlayerDeathTransition>,
    attacker: Entity,
    target: Entity,
    outcome: SwingOutcome,
) {
    events.push(build_damage_event(attacker, target, outcome.damage));
    let swing = swing_log_event(attacker, target, &outcome);
    commands.queue(move |world: &mut World| crate::spell_cast::log_event(world, swing));
    if outcome.npc_died {
        mark_npc_dead(commands, target);
        events.push(build_death_event(target));
    }
    if outcome.player_died {
        player_deaths.write(crate::death::PlayerDeathTransition { player: target });
    }
}

fn refresh_combat_state(
    commands: &mut Commands,
    combat_statuses: &Query<&CombatStatus>,
    entity: Entity,
    expires_at: f64,
) {
    let mut entity_commands = commands.entity(entity);
    entity_commands.insert(CombatState { expires_at });
    if !combat_statuses.get(entity).is_ok_and(|status| status.0) {
        entity_commands.insert(CombatStatus(true));
    }
}

/// Tick respawn timers and restore dead NPCs when ready. A patroller
/// restarts its path from the first point (AzerothCore `Creature::Respawn` →
/// `Motion_Initialize` → a new WaypointMovementGenerator at `i_path->begin()`).
pub(crate) fn respawn_system(
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &mut DeathState,
        &mut Health,
        &SpawnPoint,
        &mut Position,
        Option<&mut WaypointState>,
    )>,
    mut commands: Commands,
    dirty_queue: Option<Res<InterestDirtyQueue>>,
    mut senders: Query<&mut MessageSender<CombatEvent>>,
    mut combat_log: ResMut<CombatLog>,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs_f64();
    let mut events = Vec::new();

    for (entity, mut death, mut health, spawn, mut pos, waypoint) in &mut query {
        death.respawn_timer -= dt;
        if death.respawn_timer > 0.0 {
            continue;
        }
        let (next_health, next_position) = respawned_state(*health, *pos, spawn);
        if *health != next_health {
            *health = next_health;
        }
        if *pos != next_position {
            *pos = next_position;
            mark_interest_position_changed(dirty_queue.as_deref(), entity);
        }
        if let Some(mut waypoint) = waypoint {
            waypoint.restart((spawn.x, spawn.z));
        }
        commands.entity(entity).remove::<DeathState>().insert((
            Replicate::to_clients(NetworkTarget::All),
            NetworkVisibility::default(),
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

/// Calculate restored health and position for a respawning entity.
fn respawned_state(
    mut health: Health,
    mut position: Position,
    spawn: &SpawnPoint,
) -> (Health, Position) {
    health.current = health.max;
    position.x = spawn.x;
    position.y = spawn.y;
    position.z = spawn.z;
    (health, position)
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
