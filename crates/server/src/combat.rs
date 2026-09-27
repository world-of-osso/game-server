use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{CombatStatus, Health, Npc, Player, Position, UnitFactionTemplate};
use shared::protocol::{
    CombatChannel, CombatEvent, CombatEventType, CombatLogEvent, CombatLogKind, MissKind,
};

use crate::auras::AuraStatMods;
use crate::auras::procs::{self, AuraEvent, PendingAuraEvents};
use crate::creature_ai::{SpawnPoint, WaypointState};
use crate::creature_scripts::{TrainingDummy, health_loss};
use crate::duel::DuelOpponent;
use crate::faction::{FactionTemplates, UnitFaction};
use crate::interest::{InterestDirtyQueue, mark_interest_position_changed};
use crate::item_stats::WeaponDamage;
use crate::level_scaling::{ArmorConstants, CreatureArmor, ScalingItem, UnitScaling};
use crate::melee::{self, Hand, MeleeOutcome, Victim};
use crate::networking::CombatTarget;
use crate::spell_cast::powers::PowerPools;
use crate::stats::CharacterStats;
use shared::components::PowerType;

use std::collections::VecDeque;

/// Melee attack range in yards.
const MELEE_RANGE: f32 = 5.0;
/// Seconds between the auto-attack swings of units without `CharacterStats`
/// (creatures, `BaseAttackTime` 2000 for every Stockade creature); players swing
/// at their weapon speed.
pub(crate) const AUTO_ATTACK_INTERVAL: f32 = 2.0;
/// `Unit::GetUnitMissChance`: every victim's base melee miss chance, percent.
const UNIT_MISS_CHANCE_PCT: f32 = 5.0;
/// A swing of one hand pushes the other hand's timer to at least this
/// (`ATTACK_DISPLAY_DELAY`, Unit.h:643; Unit::AttackerStateUpdate, Unit.cpp:2203-2230).
const ATTACK_DISPLAY_DELAY: f32 = 0.2;
/// Base melee damage per swing of units without ExpectedStat damage.
const BASE_DAMAGE: f32 = 10.0;
const COMBAT_GRACE_SECS: f64 = 5.0;
/// Retail rage per second of swing time for each auto-attack dealt (SimC
/// `sc_warrior.cpp` `melee_t::base_rage_generation`), before
/// MOD_RAGE_FROM_DAMAGE_DEALT (the spec passives' rage multipliers).
const RAGE_PER_SWING_SECOND: f32 = 1.75;
/// Retail rage per damaging auto-attack taken, at most once per
/// `RAGE_FROM_HITS_TAKEN_COOLDOWN_SECS` (SimC `warrior_t::assess_damage`).
const RAGE_PER_HIT_TAKEN: f32 = 3.0;
const RAGE_FROM_HITS_TAKEN_COOLDOWN_SECS: f64 = 1.0;
/// Rage pools are stored in tenths (`power_type.DisplayModifier` 10).
const RAGE_RAW_PER_RAGE: f32 = 10.0;

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
                // A target set in the tick of a kill is attacking before the
                // kill's `CombatStop` runs.
                start_attack_system.before(auto_attack_system),
                auto_attack_system,
                maintain_in_combat_state_system,
                corpse_decay_system.before(respawn_system),
                respawn_system,
            ),
        );
    }
}

/// Tracks the swing timers for auto-attacks: the main hand and, for a unit
/// with an off-hand weapon, the off hand.
#[derive(Component, Debug)]
pub struct AutoAttack {
    pub timer: f32,
    pub off_hand_timer: f32,
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

/// Seconds a corpse stays visible after death (AzerothCore `m_corpseDelay`).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct CorpseDelay(pub f32);

impl CorpseDelay {
    pub fn for_rank(rank: u8) -> Self {
        Self(corpse_decay_secs(rank) as f32)
    }
}

/// A dead NPC still shown to clients: AzerothCore `CORPSE` death state, until
/// `m_corpseRemoveTime`. Its removal stops replication (`RemoveCorpse`).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Corpse {
    pub remaining: f32,
}

/// AzerothCore `Rate.Corpse.Decay.Looted` default (worldserver.conf.dist).
const CORPSE_DECAY_LOOTED_RATE: f32 = 0.5;

/// AzerothCore `Creature::AllLootRemovedFromCorpse` (Creature.cpp:2954-2984): once
/// the corpse has no loot left, the remaining decay shrinks by the looted rate and
/// the respawn moves earlier by the same amount.
pub(crate) fn corpse_looted(corpse: &mut Corpse, death: &mut DeathState) {
    let diff = corpse.remaining.max(0.0) * CORPSE_DECAY_LOOTED_RATE;
    corpse.remaining -= diff;
    death.respawn_timer -= diff;
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
/// Dead units neither attack nor are attacked (AzerothCore `Unit::Attack`,
/// Unit.cpp:7241-7243).
fn start_attack_system(
    query: Query<(Entity, &CombatTarget), Changed<CombatTarget>>,
    factions: Query<(
        Option<&UnitFactionTemplate>,
        Has<Player>,
        Option<&DuelOpponent>,
    )>,
    health: Query<&Health>,
    unit_flags: Query<&shared::components::UnitFlags>,
    templates: Res<FactionTemplates>,
    mut commands: Commands,
    time: Res<Time>,
) {
    let combat_expires_at = time.elapsed_secs_f64() + COMBAT_GRACE_SECS;
    let unit_faction = |entity: Entity| {
        factions
            .get(entity)
            .ok()
            .map(|(template, is_player, _)| UnitFaction {
                template: template.map_or(0, |template| template.0),
                is_player,
            })
    };
    let dueling = |attacker: Entity, target: Entity| {
        factions
            .get(attacker)
            .is_ok_and(|(_, _, opponent)| crate::duel::dueling(opponent, target))
    };
    let dead = |entity: Entity| health.get(entity).is_ok_and(|health| health.current <= 0.0);
    // WorldObject::IsValidAttackTarget (Object.cpp:2374): not an uninteractible unit.
    let uninteractible = |entity: Entity| {
        unit_flags
            .get(entity)
            .is_ok_and(|flags| !flags.is_selectable())
    };
    for (entity, target) in &query {
        let attackable = target.0.is_some_and(|target| {
            !(dead(entity) || dead(target) || uninteractible(target))
                && (dueling(entity, target)
                    || match (unit_faction(entity), unit_faction(target)) {
                        (Some(attacker), Some(target)) => templates.can_attack(attacker, target),
                        _ => false,
                    })
        });
        if attackable {
            commands.entity(entity).insert((
                AutoAttack {
                    timer: 0.0,
                    off_hand_timer: 0.0,
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
        Option<&'static CharacterStats>,
        SwingState<'static>,
    ),
>;

/// What else a swing reads of its attacker: a creature's haste (players fold it
/// into `CharacterStats`) and the states that stop auto-attacks
/// (`UNIT_STATE_CASTING`, `UNIT_STATE_STUNNED`).
type SwingState<'a> = (
    Option<&'a crate::stats::StatModifiers>,
    Has<shared::casting::CastState>,
    Option<&'a crate::auras::ActiveAuras>,
);

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
        Option<&'static DuelOpponent>,
        TargetArmor<'static>,
        Option<&'static shared::components::UnitFlags>,
        Has<crate::auras::Invisible>,
    ),
>;

/// A creature's own armor or a player's gear armor.
type TargetArmor<'a> = (Option<&'a CreatureArmor>, Option<&'a CharacterStats>);

#[derive(SystemParam)]
pub(crate) struct AutoAttackParams<'w, 's> {
    attackers: AutoAttackers<'w, 's>,
    targets: AutoAttackTargets<'w, 's>,
    scaling: Query<'w, 's, ScalingItem<'static>>,
    armor_constants: Option<Res<'w, ArmorConstants>>,
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
    let mut swings = Vec::new();

    for (
        attacker_entity,
        combat_target,
        mut auto_attack,
        position,
        mods,
        stats,
        (stat_mods, casting, auras),
    ) in &mut params.attackers
    {
        let Some(target_entity) = ready_auto_attack_target(combat_target, &mut auto_attack, dt)
        else {
            continue;
        };
        // Unit::DoMeleeAttackIfReady / AttackerStateUpdate: no swing while casting
        // or stunned; the swing timers keep running.
        if casting || auras.is_some_and(|auras| auras.is_stunned()) {
            continue;
        }
        let attacker = Attacker {
            entity: attacker_entity,
            position,
            mods,
            stats,
            haste_pct: stat_mods.map_or(0.0, |mods| mods.haste_pct),
        };
        for hand in [Hand::Main, Hand::Off] {
            let Some(outcome) = try_swing(
                &attacker,
                hand,
                &mut params.targets,
                &SwingContext {
                    scaling: &params.scaling,
                    armor_constants: params.armor_constants.as_deref(),
                },
                target_entity,
                &mut auto_attack,
                &mut params.commands,
            ) else {
                continue;
            };
            swings.push((attacker_entity, target_entity, outcome));
        }
    }
    for (attacker, target, outcome) in swings {
        record_swing(
            &mut params,
            &mut events,
            attacker,
            target,
            outcome,
            combat_expires_at,
        );
    }

    broadcast_events(
        &events,
        &mut params.senders,
        &mut params.combat_log,
        elapsed,
    );
}

fn record_swing(
    params: &mut AutoAttackParams,
    events: &mut Vec<CombatEvent>,
    attacker_entity: Entity,
    target_entity: Entity,
    outcome: SwingOutcome,
    combat_expires_at: f64,
) {
    refresh_combatants(
        &mut params.commands,
        &params.combat_statuses,
        attacker_entity,
        target_entity,
        combat_expires_at,
    );
    mark_replication_changed(params.replication_dirty.as_deref(), target_entity);
    // Procs default to normal and critical hits (SpellMgr::LoadSpellProcs
    // HitMask 0); dodge- and parry-triggered procs are not modelled.
    if !outcome.kind.avoided()
        && let Some(pending) = params.aura_events.as_deref_mut()
    {
        pending
            .0
            .extend(swing_aura_events(attacker_entity, target_entity, &outcome));
    }
    record_swing_events(
        &mut params.commands,
        events,
        &mut params.player_deaths,
        attacker_entity,
        target_entity,
        outcome,
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
    auto_attack.off_hand_timer -= delta_seconds;
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
    let hand_swing = match outcome.hand {
        Hand::Main => procs::PROC_MAIN_HAND_WEAPON_SWING,
        Hand::Off => procs::PROC_OFF_HAND_WEAPON_SWING,
    };
    let mut events = vec![AuraEvent {
        actor: attacker,
        target: Some(target),
        actor_mask: [procs::PROC_DEAL_MELEE_SWING | hand_swing, 0],
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

/// The swinging unit.
pub(crate) struct Attacker<'a> {
    pub entity: Entity,
    pub position: &'a Position,
    pub mods: Option<&'a AuraStatMods>,
    /// Players: weapon, attack power, crit, haste and versatility. Units
    /// without it (creatures) swing their ExpectedStat or flat damage.
    pub stats: Option<&'a CharacterStats>,
    /// Aura haste of a unit without `CharacterStats` (MOD_MELEE_HASTE).
    pub haste_pct: f32,
}

impl Attacker<'_> {
    fn weapon(&self, hand: Hand) -> Option<WeaponDamage> {
        match hand {
            Hand::Main => Some(
                self.stats
                    .map_or(WeaponDamage::UNARMED, |stats| stats.main_hand),
            ),
            Hand::Off => self.stats.and_then(|stats| stats.off_hand),
        }
    }

    /// Seconds to the hand's next swing: the weapon speed over haste for
    /// players, the flat interval over aura haste otherwise
    /// (Unit::ApplyAttackTimePercentMod), times the MOD_SPEED_SLOW_ALL multiplier.
    fn swing_time(&self, weapon: &WeaponDamage) -> f32 {
        let slow = self.mods.map_or(1.0, |mods| mods.attack_cast_time_mult);
        self.stats.map_or(
            AUTO_ATTACK_INTERVAL / (1.0 + self.haste_pct / 100.0).max(0.01),
            |stats| melee::swing_time(weapon.speed, stats.haste_pct),
        ) * slow
    }
}

/// World data a swing reads besides the two units.
pub(crate) struct SwingContext<'a, 'w, 's> {
    pub scaling: &'a Query<'w, 's, ScalingItem<'static>>,
    pub armor_constants: Option<&'a ArmorConstants>,
}

/// Swing damage before armor and outcome: a player's weapon roll
/// (`melee::weapon_damage_range`), a tuned creature's ExpectedStat roll or the
/// flat `BASE_DAMAGE`; then × attacker physical damage done and auto-attack
/// multipliers × target physical damage taken (`AuraStatMods`, Unit::MeleeDamageBonusDone/Taken).
fn swing_damage(
    attacker: &Attacker,
    weapon: &WeaponDamage,
    hand: Hand,
    scaling: &UnitScaling,
    target_mods: Option<&AuraStatMods>,
) -> f32 {
    const PHYSICAL: u32 = 1;
    let mut rng = rand::thread_rng();
    let base = match attacker.stats {
        Some(stats) => melee::roll_weapon_damage(
            melee::weapon_damage_range(weapon, stats.attack_power, stats.versatility_pct, hand),
            rand::Rng::r#gen(&mut rng),
        ),
        None => creature_weapon_damage(scaling),
    };
    let done = attacker.mods.map_or(1.0, |mods| {
        mods.damage_done(PHYSICAL) * mods.autoattack_damage_mult
    });
    let taken = target_mods.map_or(1.0, |mods| mods.damage_taken(PHYSICAL));
    (base * done * taken).max(0.0).round()
}

/// A creature's main-hand weapon damage roll (Unit::CalculateDamage): its
/// ExpectedStat range at `AUTO_ATTACK_INTERVAL`, `BASE_DAMAGE` when untuned.
pub(crate) fn creature_weapon_damage(scaling: &UnitScaling) -> f32 {
    scaling
        .swing_damage_range(AUTO_ATTACK_INTERVAL)
        .map_or(BASE_DAMAGE, |(min, max)| {
            rand::Rng::gen_range(&mut rand::thread_rng(), min..=max)
        })
}

/// A creature's melee spell miss chance in percent (`Unit::MeleeSpellMissChance`):
/// the victim's `GetUnitMissChance` minus its MOD_HIT_CHANCE auras. (Creature
/// auto-attacks do not roll the hit table.)
pub(crate) fn creature_miss_chance(mods: Option<&AuraStatMods>) -> f32 {
    (UNIT_MISS_CHANCE_PCT - mods.map_or(0.0, |mods| mods.melee_hit_chance_pct)).clamp(0.0, 100.0)
}

/// The target's armor against `attacker` (Unit::CalcArmorReducedDamage): a
/// creature's own armor × its level-scaling armor multiplier, a player's gear armor.
fn target_armor(
    (creature, player): TargetArmor,
    target: &UnitScaling,
    attacker: &UnitScaling,
) -> f32 {
    match (creature, player) {
        (Some(armor), _) => target.armor_against(armor.0, attacker),
        (None, Some(stats)) => stats.armor,
        (None, None) => 0.0,
    }
}

/// Physical `damage` after the target's armor at the attacker's level
/// against it; without ExpectedStat data there is no mitigation.
pub(crate) fn armor_mitigated(
    damage: f32,
    armor: f32,
    attacker: &UnitScaling,
    target: &UnitScaling,
    constants: Option<&ArmorConstants>,
) -> f32 {
    constants
        .and_then(|constants| constants.at(attacker.level_against(target)))
        .map_or(damage, |constant| {
            melee::armor_reduced_damage(damage, armor, constant)
        })
}

/// The hit-table result of a player's swing; units without `CharacterStats`
/// always hit. `level_difference` is the target's level against the
/// attacker minus the attacker's against it.
fn roll_outcome(attacker: &Attacker, target_is_npc: bool, level_difference: i32) -> MeleeOutcome {
    let Some(stats) = attacker.stats else {
        return MeleeOutcome::Hit;
    };
    let victim = if target_is_npc {
        Victim::Creature { level_difference }
    } else {
        Victim::Player
    };
    let chances = melee::player_swing_chances(victim, stats.crit_pct, stats.off_hand.is_some());
    melee::roll_melee_outcome(
        &chances,
        rand::Rng::gen_range(&mut rand::thread_rng(), 0..10_000),
    )
}

struct SwingOutcome {
    hand: Hand,
    kind: MeleeOutcome,
    damage: f32,
    blocked: f32,
    /// Damage beyond the target's health before the swing.
    overkill: f32,
    /// Unhasted weapon speed: rage per swing (CalcMeleeAttackRageGain, Unit.cpp:2239-2249).
    weapon_speed: f32,
    npc_died: bool,
    player_died: bool,
    /// The duel opponent's swing took the target to 1 health.
    duel_defeated: bool,
}

/// Swing `hand` if its timer is ready, it has a weapon and the target is in
/// range. Players roll the hit table, and every swing takes the target's armor
/// (Unit::CalculateMeleeDamage, Unit.cpp:1387-1495).
fn try_swing(
    attacker: &Attacker,
    hand: Hand,
    targets: &mut AutoAttackTargets,
    context: &SwingContext,
    target_entity: Entity,
    auto_attack: &mut AutoAttack,
    commands: &mut Commands,
) -> Option<SwingOutcome> {
    let timer = match hand {
        Hand::Main => auto_attack.timer,
        Hand::Off => auto_attack.off_hand_timer,
    };
    if timer > 0.0 {
        return None;
    }
    let weapon = attacker.weapon(hand)?;
    let Ok((
        target_pos,
        mut health,
        last_damager,
        is_npc,
        target_mods,
        training_dummy,
        duel,
        armor,
        flags,
        invisible,
    )) = targets.get_mut(target_entity)
    else {
        return None;
    };
    // IsValidAttackTarget: an invisible or NON_ATTACKABLE_2 unit is not attacked.
    let attackable = !invisible && flags.is_none_or(|flags| flags.is_attackable());
    if !in_melee_range(attacker.position, target_pos) || health.current <= 0.0 || !attackable {
        return None;
    }
    let health_before = health.current;
    let unit = |entity| {
        context.scaling.get(entity).map_or(
            UnitScaling::from_item((None, None, None)),
            UnitScaling::from_item,
        )
    };
    let (attacker_scaling, target) = (unit(attacker.entity), unit(target_entity));
    let level_difference = i32::from(target.level_against(&attacker_scaling))
        - i32::from(attacker_scaling.level_against(&target));
    let armored = armor_mitigated(
        swing_damage(attacker, &weapon, hand, &attacker_scaling, target_mods),
        target_armor(armor, &target, &attacker_scaling),
        &attacker_scaling,
        &target,
        context.armor_constants,
    );
    let kind = roll_outcome(attacker, is_npc, level_difference);
    let (outcome_damage, blocked) = melee::outcome_damage(armored, kind, level_difference);
    let damage = crate::level_scaling::damage_dealt(&attacker_scaling, &target, outcome_damage);
    let scaled_loss = crate::level_scaling::health_loss(&attacker_scaling, &target, damage);
    let overkill = (scaled_loss - health.current).max(0.0);
    let (loss, duel_defeated) = crate::duel::duel_health_loss(
        duel,
        attacker.entity,
        health.current,
        health_loss(training_dummy, scaled_loss),
    );
    let became_lethal = crate::death::apply_health_damage(&mut health, loss);
    if is_npc && loss > 0.0 {
        let attacker_entity = attacker.entity;
        commands.queue(move |world: &mut World| {
            crate::scripted_ai::damage_taken(
                world,
                target_entity,
                Some(attacker_entity),
                health_before,
                loss,
            );
        });
    }
    if let Some(mut last_damager) = last_damager {
        last_damager.0 = attacker.entity;
    } else {
        commands
            .entity(target_entity)
            .insert(LastDamager(attacker.entity));
    }
    reset_swing_timers(auto_attack, hand, attacker.swing_time(&weapon));
    Some(SwingOutcome {
        hand,
        kind,
        damage,
        blocked,
        overkill,
        weapon_speed: weapon.speed,
        npc_died: is_npc && became_lethal,
        player_died: !is_npc && became_lethal,
        duel_defeated,
    })
}

/// Restart `hand`'s timer and hold the other hand at least
/// `ATTACK_DISPLAY_DELAY` away, so both hands never land together.
fn reset_swing_timers(auto_attack: &mut AutoAttack, hand: Hand, swing_time: f32) {
    let (swung, other) = match hand {
        Hand::Main => (&mut auto_attack.timer, &mut auto_attack.off_hand_timer),
        Hand::Off => (&mut auto_attack.off_hand_timer, &mut auto_attack.timer),
    };
    *swung = swing_time;
    *other = other.max(ATTACK_DISPLAY_DELAY);
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

/// A swing's `CombatEvent`: melee damage, or the avoidance without an amount.
fn build_swing_event(attacker: Entity, target: Entity, outcome: &SwingOutcome) -> CombatEvent {
    let event_type = match outcome.kind {
        MeleeOutcome::Miss => CombatEventType::Miss,
        MeleeOutcome::Dodge => CombatEventType::Dodge,
        MeleeOutcome::Parry => CombatEventType::Parry,
        _ => CombatEventType::MeleeDamage,
    };
    CombatEvent {
        attacker: attacker.to_bits(),
        target: target.to_bits(),
        amount: outcome.damage,
        spell_id: 0,
        event_type,
    }
}

/// The combat log `MissKind` of an avoided swing.
fn swing_miss_kind(outcome: MeleeOutcome) -> Option<MissKind> {
    match outcome {
        MeleeOutcome::Miss => Some(MissKind::Miss),
        MeleeOutcome::Dodge => Some(MissKind::Dodge),
        MeleeOutcome::Parry => Some(MissKind::Parry),
        _ => None,
    }
}

/// The swing in the attacker's and target's combat log (AzerothCore
/// `SendAttackStateUpdate`): physical Damage without a spell, with its crit
/// and blocked amount, or the Miss, Dodge or Parry.
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
        blocked: outcome.blocked as i32,
        crit: outcome.kind == MeleeOutcome::Crit,
        periodic: false,
        kind: swing_miss_kind(outcome.kind).map_or(CombatLogKind::Damage, CombatLogKind::Miss),
    }
}

/// The killing blow's Death line in the killer's and victim's combat log,
/// for melee and spell kills alike (Unit::DealDamage → Unit::Kill).
pub(crate) fn death_log_event(killer: Entity, target: Entity) -> CombatLogEvent {
    CombatLogEvent {
        source: Some(killer.to_bits()),
        target: Some(target.to_bits()),
        spell_id: None,
        school_mask: 0,
        amount: 0,
        overflow: 0,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit: false,
        periodic: false,
        kind: CombatLogKind::Death,
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

/// Start the NPC's respawn timer and leave its corpse replicated for its
/// `CorpseDelay` (AzerothCore `setDeathState(CORPSE)`: `m_corpseRemoveTime = now +
/// m_corpseDelay`). NPCs without a `RespawnDelay` (not world spawns) have no
/// `SpawnPoint` and never respawn; without a `CorpseDelay` the corpse goes at once.
/// Dying stops combat (`setDeathState` → `CombatStop`), so its killers do not
/// swing at it again when it respawns.
fn insert_npc_death(mut entity: EntityWorldMut) {
    let respawn_timer = entity.get::<RespawnDelay>().map_or(0.0, |delay| delay.0);
    let remaining = entity.get::<CorpseDelay>().map_or(0.0, |delay| delay.0);
    let killer = entity.get::<LastDamager>().map(|damager| damager.0);
    entity.insert((DeathState { respawn_timer }, Corpse { remaining }));
    // A corpse does not run on (the movement systems skip it).
    if let Some(mut motion) = entity.get_mut::<shared::components::CreatureMotion>()
        && motion.set_if_neq(shared::components::CreatureMotion::Still)
    {
        let npc = entity.id();
        entity.world_scope(|world| {
            mark_replication_changed(world.get_resource::<ReplicationDirtyQueue>(), npc);
        });
    }
    crate::creature_aggro::forget_combat(&mut entity);
    let npc = entity.id();
    entity.world_scope(|world| {
        combat_stop(world, npc);
        crate::scripted_ai::dispatch(world, npc, crate::scripted_ai::Hook::JustDied(killer));
    });
}

/// `Unit::DealDamage(attacker, victim, damage)` without a spell (`DIRECT_DAMAGE`,
/// school Normal; the `.damage` command): the victim loses `damage` over its health
/// multiplier for the attacker, both enter combat, the victim's threat and
/// `DamageTaken` follow, and a lethal hit kills it.
pub(crate) fn deal_direct_damage(world: &mut World, attacker: Entity, victim: Entity, damage: f32) {
    if !crate::spell_cast::targets::is_alive(world, victim) {
        return;
    }
    let (attacker_scaling, victim_scaling) = (
        UnitScaling::of(world, attacker),
        UnitScaling::of(world, victim),
    );
    let loss = crate::level_scaling::health_loss(&attacker_scaling, &victim_scaling, damage);
    let Some(mut health) = world.get_mut::<Health>(victim) else {
        return;
    };
    let before = health.current;
    let lethal = crate::death::apply_health_damage(&mut health, loss);
    mark_replication_changed(world.get_resource::<ReplicationDirtyQueue>(), victim);
    world.entity_mut(victim).insert(LastDamager(attacker));
    enter_combat(world, attacker);
    enter_combat(world, victim);
    crate::creature_aggro::add_threat(world, victim, attacker, damage);
    crate::spell_cast::log_event(
        world,
        CombatLogEvent {
            source: Some(attacker.to_bits()),
            target: Some(victim.to_bits()),
            spell_id: None,
            school_mask: 1,
            amount: damage as i32,
            overflow: (loss - before).max(0.0) as i32,
            absorbed: 0,
            resisted: 0,
            blocked: 0,
            crit: false,
            periodic: false,
            kind: CombatLogKind::Damage,
        },
    );
    if loss > 0.0 {
        crate::scripted_ai::damage_taken(world, victim, Some(attacker), before, loss);
    }
    if !lethal {
        return;
    }
    if world.get::<Npc>(victim).is_some() {
        mark_npc_dead_world(world, victim);
    } else {
        world.write_message(crate::death::PlayerDeathTransition { player: victim });
    }
    crate::spell_cast::log_event(world, death_log_event(attacker, victim));
}

/// `Unit::Kill(killer, victim)` outside a damage path (`Unit::KillSelf`): the
/// victim dies at once.
pub(crate) fn kill_unit(world: &mut World, killer: Entity, victim: Entity) {
    let Some(mut health) = world.get_mut::<Health>(victim) else {
        return;
    };
    if health.current <= 0.0 {
        return;
    }
    health.current = 0.0;
    mark_replication_changed(world.get_resource::<ReplicationDirtyQueue>(), victim);
    world.entity_mut(victim).insert(LastDamager(killer));
    if world.get::<Npc>(victim).is_some() {
        mark_npc_dead_world(world, victim);
    } else {
        world.write_message(crate::death::PlayerDeathTransition { player: victim });
    }
    crate::spell_cast::log_event(world, death_log_event(killer, victim));
}

/// AzerothCore `Creature::Update` in `CORPSE` state: past `m_corpseRemoveTime` the
/// corpse is removed from clients (`RemoveCorpse`) until it respawns. Interest
/// hides it from every observer, including players who arrive later (replicon
/// shows an entity without `NetworkVisibility` to every new client); the respawn
/// shows the NPC again.
/// A dead summon goes with its corpse (`TempSummon::Update`: `DEAD` → `UnSummon`).
pub(crate) fn corpse_decay_system(
    time: Res<Time>,
    mut corpses: Query<(Entity, &mut Corpse, Has<crate::zones::TempSummon>)>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    for (entity, mut corpse, summon) in &mut corpses {
        corpse.remaining -= dt;
        if corpse.remaining > 0.0 {
            continue;
        }
        if summon {
            commands.queue(move |world: &mut World| crate::zones::unsummon(world, entity));
        } else {
            commands.entity(entity).remove::<Corpse>();
            commands.queue(move |world: &mut World| {
                crate::interest::set_hidden(world, entity, true);
            });
        }
    }
}

/// AzerothCore `Unit::CombatStop`: `unit` stops attacking (`AttackStop`), its
/// attackers stop attacking it (`RemoveAllAttackers`) and it leaves combat
/// (`ClearInCombat`). The attackers leave combat once their grace runs out.
pub(crate) fn combat_stop(world: &mut World, unit: Entity) {
    let attackers: Vec<Entity> = world
        .query_filtered::<(Entity, &CombatTarget), With<AutoAttack>>()
        .iter(world)
        .filter(|(_, target)| target.0 == Some(unit))
        .map(|(attacker, _)| attacker)
        .collect();
    for attacker in attackers {
        world.entity_mut(attacker).remove::<AutoAttack>();
    }
    world
        .entity_mut(unit)
        .remove::<(AutoAttack, CombatState)>()
        .insert(CombatStatus(false));
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
    events.push(build_swing_event(attacker, target, &outcome));
    let damage = outcome.damage;
    commands.queue(move |world: &mut World| {
        crate::creature_aggro::add_threat(world, target, attacker, damage);
    });
    let rage_secs = match outcome.hand {
        Hand::Main => outcome.weapon_speed,
        Hand::Off => outcome.weapon_speed / 2.0,
    };
    commands.queue(move |world: &mut World| {
        reward_swing_rage(world, attacker, target, rage_secs, damage)
    });
    let swing = swing_log_event(attacker, target, &outcome);
    commands.queue(move |world: &mut World| crate::spell_cast::log_event(world, swing));
    if outcome.npc_died || outcome.player_died {
        let death = death_log_event(attacker, target);
        commands.queue(move |world: &mut World| crate::spell_cast::log_event(world, death));
    }
    if outcome.npc_died {
        mark_npc_dead(commands, target);
        events.push(build_death_event(target));
    }
    if outcome.player_died {
        player_deaths.write(crate::death::PlayerDeathTransition { player: target });
    }
    if outcome.duel_defeated {
        commands.entity(target).insert(crate::duel::DuelDefeated);
    }
}

/// When the unit next gains rage from an auto-attack taken.
#[derive(Component, Debug)]
struct RageFromHitsTakenReadyAt(f64);

/// Rage for a swing: the attacker gains `swing_secs` (the unhasted weapon
/// speed, half for the off hand) × 1.75 rage scaled by
/// MOD_RAGE_FROM_DAMAGE_DEALT, rounded to 0.1 rage (SimC `trigger_rage_gain`;
/// CalcMeleeAttackRageGain, Unit.cpp:2239-2249); the victim gains 3 rage when
/// the swing did damage. Only units whose primary power is rage gain it
/// (AzerothCore `Unit::DealDamage` `HasActivePowerType(POWER_RAGE)`).
fn reward_swing_rage(
    world: &mut World,
    attacker: Entity,
    target: Entity,
    swing_secs: f32,
    damage: f32,
) {
    if has_rage_primary_power(world, attacker) {
        let bonus_pct = world
            .get::<AuraStatMods>(attacker)
            .map_or(0.0, |mods| mods.rage_from_damage_dealt_pct);
        let rage = swing_secs * RAGE_PER_SWING_SECOND * (1.0 + bonus_pct / 100.0);
        change_rage(world, attacker, (rage * RAGE_RAW_PER_RAGE).round());
    }
    if damage > 0.0
        && has_rage_primary_power(world, target)
        && take_hit_rage_cooldown(world, target)
    {
        change_rage(world, target, RAGE_PER_HIT_TAKEN * RAGE_RAW_PER_RAGE);
    }
}

fn has_rage_primary_power(world: &World, unit: Entity) -> bool {
    world
        .get::<PowerPools>(unit)
        .is_some_and(|pools| pools.order.first() == Some(&PowerType::Rage))
}

/// Start the hit-taken rage cooldown; false while it is running.
fn take_hit_rage_cooldown(world: &mut World, unit: Entity) -> bool {
    let now = world.resource::<Time>().elapsed_secs_f64();
    if world
        .get::<RageFromHitsTakenReadyAt>(unit)
        .is_some_and(|ready| now < ready.0)
    {
        return false;
    }
    world.entity_mut(unit).insert(RageFromHitsTakenReadyAt(
        now + RAGE_FROM_HITS_TAKEN_COOLDOWN_SECS,
    ));
    true
}

fn change_rage(world: &mut World, unit: Entity, raw: f32) {
    crate::spell_cast::powers::change_power(world, unit, PowerType::Rage, raw);
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
    mut query: Query<
        (
            Entity,
            &mut DeathState,
            &mut Health,
            &SpawnPoint,
            &mut Position,
            Option<&mut WaypointState>,
        ),
        Without<crate::zones::TempSummon>,
    >,
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
        commands
            .entity(entity)
            .remove::<(DeathState, Corpse)>()
            .insert(Replicate::to_clients(NetworkTarget::All));
        commands.queue(move |world: &mut World| {
            crate::interest::set_hidden(world, entity, false);
        });
        commands.entity(entity).remove::<LastDamager>();
        commands.queue(move |world: &mut World| crate::scripted_ai::reinitialize(world, entity));
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
