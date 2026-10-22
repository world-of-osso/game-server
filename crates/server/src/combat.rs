use crate::random as rand;
use bevy::ecs::system::SystemParam;
use std::collections::HashMap;

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Health, Npc, Player, Position, UnitFactionTemplate};
use shared::protocol::{
    AttackStart, AttackStop, AttackStopped, AttackSwing, CombatChannel, CombatEvent,
    CombatEventType, CombatLogEvent, CombatLogKind, MissKind,
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
use crate::line_of_sight::{BodyParts, LineOfSight, unit_sees};
use crate::maps::InMap;
use crate::melee::{self, Hand, MeleeOutcome, PlayerDefense, Swinger, Victim};
use crate::networking::{CombatTarget, PlayerLink};
use crate::spell_cast::powers::PowerPools;
use crate::stats::CharacterStats;
use shared::components::PowerType;

use std::collections::VecDeque;

/// Melee attack range in yards.
const MELEE_RANGE: f32 = 5.0;
/// Attack's spell ID for the existing owner spell-error channel.
const ATTACK_SPELL_ID: u32 = 6603;
/// Default unit attack time and existing ExpectedStat damage normalization.
/// Source-spawned creatures carry CreatureAttackTime; players use their weapons.
pub(crate) const AUTO_ATTACK_INTERVAL: f32 = 2.0;
/// `Unit::GetUnitMissChance`: every victim's base melee miss chance, percent.
const UNIT_MISS_CHANCE_PCT: f32 = 5.0;
/// Signed attacker dodge modifier (`Unit::GetUnitDodgeChance`, Unit.cpp2775).
const MOD_ENEMY_DODGE: u32 = 251;
/// Non-player expertise (`Unit::GetUnitDodgeChance`/`GetUnitParryChance`).
/// A swing of one hand pushes the other hand's timer to at least this
/// (`ATTACK_DISPLAY_DELAY`, Unit.h:643; Unit::AttackerStateUpdate, Unit.cpp:2203-2230).
const ATTACK_DISPLAY_DELAY: f32 = 0.2;
/// Base melee damage per swing of units without ExpectedStat damage.
const BASE_DAMAGE: f32 = 10.0;
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
                // An attack started in the tick of a kill is attacking before
                // the kill's `CombatStop` runs.
                (creature_attack_system, process_attack_messages).before(auto_attack_system),
                auto_attack_system,
                crate::combat_refs::expire_combat_refs_system,
                corpse_decay_system.before(respawn_system),
                respawn_system,
            ),
        );
    }
}

/// A unit auto-attacking `victim` (TrinityCore `m_attacking` with
/// `UNIT_STATE_MELEE_ATTACKING`), and its swing timers: the main hand and, for a
/// unit with an off-hand weapon, the off hand. A player's victim is separate from
/// its selection (`Player::SetTarget` is a no-op, Player.h:1920).
#[derive(Component, Debug, Clone)]
pub struct AutoAttack {
    pub timer: f32,
    pub off_hand_timer: f32,
    pub victim: Entity,
}

impl AutoAttack {
    pub fn new(victim: Entity) -> Self {
        Self {
            timer: 0.0,
            off_hand_timer: 0.0,
            victim,
        }
    }
}

/// Marks a dead NPC awaiting respawn.
#[derive(Component, Debug)]
pub struct DeathState {
    pub respawn_timer: f32,
}

/// TC Creature::SetUnkillable, CREATURE_STATIC_FLAG_UNKILLABLE.
#[derive(Component, Debug)]
pub(crate) struct Unkillable;

/// TC Unit.cpp936–945: other units' lethal damage leaves an unkillable creature at1HP.
pub(crate) fn limit_unkillable_loss(current_health: f32, loss: f32, protected: bool) -> f32 {
    if protected && loss >= current_health {
        (current_health - 1.0).max(0.0)
    } else {
        loss
    }
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

/// Explicit cause of this pending death, never inferred from prior damage.
/// Player transitions take the value even when revived before corpse creation.
#[derive(Component, Debug)]
pub(crate) struct KillingBlow(pub(crate) Option<Entity>);

/// TC Unit::Kill notifies the actual killing creature, not its reward owner.
pub(crate) fn notify_killed_unit(world: &mut World, killer: Entity, victim: Entity) {
    if world.get::<Npc>(killer).is_none() {
        return;
    }
    let Some(map) = world.get::<crate::maps::InMap>(victim) else {
        return;
    };
    if world.get::<crate::maps::InMap>(killer) != Some(map) {
        return;
    }
    crate::scripted_ai::dispatch(world, killer, crate::scripted_ai::Hook::KilledUnit(victim));
}

/// `WorldObject::IsValidAttackTarget` and `Unit::Attack`'s own checks as far as
/// they are modelled: living units only (Object.cpp:2367, Unit.cpp:5876), no
/// uninteractible victim (Object.cpp:2374), hostile factions or duel opponents.
#[derive(SystemParam)]
struct AttackRules<'w, 's> {
    factions: Query<
        'w,
        's,
        (
            Option<&'static UnitFactionTemplate>,
            Has<Player>,
            Option<&'static DuelOpponent>,
        ),
    >,
    health: Query<'w, 's, &'static Health>,
    arenas: Query<'w, 's, &'static crate::pvp::PlayerPvpState>,
    maps: Query<'w, 's, &'static crate::maps::InMap>,
    summon_owners: Query<'w, 's, &'static shared::components::UnitSummonedBy>,
    unit_flags: Query<'w, 's, &'static shared::components::UnitFlags>,
    stealth: crate::auras::stealth::DetectionUnits<'w, 's>,
    templates: Res<'w, FactionTemplates>,
}

impl AttackRules<'_, '_> {
    fn dead(&self, unit: Entity) -> bool {
        self.health
            .get(unit)
            .is_ok_and(|health| health.current <= 0.0)
    }

    fn unit_faction(&self, unit: Entity) -> Option<UnitFaction> {
        self.factions
            .get(unit)
            .ok()
            .map(|(template, is_player, _)| UnitFaction {
                template: template.map_or(0, |template| template.0),
                is_player: crate::faction::player_affected(
                    is_player,
                    self.unit_flags.get(unit).ok(),
                ),
            })
    }

    fn arena_relation(
        &self,
        attacker: Entity,
        victim: Entity,
    ) -> Option<crate::pvp::arena_combat::ArenaRelation> {
        let controller = |entity| match self.summon_owners.get(entity) {
            Ok(owner) => Entity::try_from_bits(owner.0).expect("server-owned summon controller"),
            Err(_) => entity,
        };
        crate::pvp::arena_combat::relation_between(
            self.arenas.get(controller(attacker)).ok(),
            self.arenas.get(controller(victim)).ok(),
            self.maps.get(attacker).copied().ok(),
            self.maps.get(victim).copied().ok(),
        )
    }
    fn valid_attack_target(&self, attacker: Entity, victim: Entity) -> bool {
        let player_controlled = self
            .unit_faction(attacker)
            .is_some_and(|unit| unit.is_player);
        let blocked_by_flags = self
            .unit_flags
            .get(victim)
            .is_ok_and(|flags| !flags.can_be_attacked_by(player_controlled));
        let dueling = self
            .factions
            .get(attacker)
            .is_ok_and(|(_, _, opponent)| crate::duel::dueling(opponent, victim));
        let permitted = match self.arena_relation(attacker, victim) {
            Some(relation) => matches!(
                relation,
                crate::pvp::arena_combat::ArenaRelation::Enemy { can_attack: true }
            ),
            None => {
                dueling
                    || match (self.unit_faction(attacker), self.unit_faction(victim)) {
                        (Some(attacker), Some(victim)) => {
                            self.templates.can_attack(attacker, victim)
                        }
                        _ => false,
                    }
            }
        };
        let detected = crate::auras::stealth::query_can_detect(&self.stealth, attacker, victim);
        attacker != victim && !self.dead(victim) && !blocked_by_flags && permitted && detected
    }
}

/// Creatures whose AI target changed.
type RetargetedCreatures<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static CombatTarget, Option<&'static AutoAttack>),
    (
        Changed<CombatTarget>,
        Without<Player>,
        Without<crate::scripted_ai::caster_chase::CasterChase>,
    ),
>;

/// A creature's AI attacks by setting its `CombatTarget` (`Unit::Attack`, which
/// for creatures engages the victim, Unit.cpp:5950-5952); a target it cannot
/// attack, or none, stops the attack. Players attack only through `AttackSwing`.
fn creature_attack_system(query: RetargetedCreatures, rules: AttackRules, mut commands: Commands) {
    for (creature, target, current) in &query {
        match target.0 {
            Some(victim)
                if !rules.dead(creature) && rules.valid_attack_target(creature, victim) =>
            {
                start_attack(&mut commands, creature, victim, current);
                commands.queue(move |world: &mut World| {
                    crate::combat_refs::set_in_combat_with(world, creature, victim);
                });
            }
            _ => {
                commands.queue(move |world: &mut World| attack_stop(world, creature));
            }
        }
    }
}

/// `WorldSession::HandleAttackSwingOpcode` (CombatHandler.cpp:28-61): an unknown or
/// invalid victim answers `SMSG_ATTACK_STOP`; otherwise `Unit::Attack(victim, true)`,
/// which a dead attacker's request fails silently (Unit.cpp:5876). Combat starts
/// with the first swing (`AttackerStateUpdate` → `AtTargetAttacked`), not here.
/// `HandleAttackStopOpcode` (CombatHandler.cpp:63-66): `Unit::AttackStop`.
fn process_attack_messages(
    mut swings: Query<(&mut MessageReceiver<AttackSwing>, &PlayerLink)>,
    mut stops: Query<(&mut MessageReceiver<AttackStop>, &PlayerLink)>,
    attacking: Query<&AutoAttack>,
    rules: AttackRules,
    mut commands: Commands,
) {
    for (mut receiver, link) in &mut swings {
        let player = link.0;
        for request in receiver.receive() {
            let victim = Entity::try_from_bits(request.target)
                .filter(|&victim| rules.health.contains(victim));
            match victim {
                Some(victim) if rules.valid_attack_target(player, victim) => {
                    if !rules.dead(player) {
                        start_attack(&mut commands, player, victim, attacking.get(player).ok());
                    }
                }
                victim => commands.queue(move |world: &mut World| {
                    send_attack_stopped(world, player, victim);
                }),
            }
        }
    }
    for (mut receiver, link) in &mut stops {
        let player = link.0;
        if receiver.receive().count() > 0 {
            commands.queue(move |world: &mut World| attack_stop(world, player));
        }
    }
}

/// `Unit::Attack(victim, true)`: a new victim keeps the running swing timers and
/// is announced with `SMSG_ATTACK_START` (`SendMeleeAttackStart`, Unit.cpp:2542);
/// attacking the current victim again changes nothing (Unit.cpp:5906-5924).
pub(crate) fn start_attack(
    commands: &mut Commands,
    attacker: Entity,
    victim: Entity,
    current: Option<&AutoAttack>,
) {
    if current.is_some_and(|current| current.victim == victim) {
        return;
    }
    let auto_attack = current.map_or(AutoAttack::new(victim), |current| AutoAttack {
        victim,
        ..*current
    });
    commands.entity(attacker).insert(auto_attack);
    commands.queue(move |world: &mut World| {
        let message = AttackStart {
            attacker: attacker.to_bits(),
            victim: victim.to_bits(),
        };
        crate::spell_cast::send_to_set(world, attacker, message);
    });
}

/// `Unit::AttackStop` (Unit.cpp:5982-6009): stop swinging and announce it with
/// `SMSG_ATTACK_STOP`. A unit that is not attacking sends nothing.
pub(crate) fn attack_stop(world: &mut World, unit: Entity) {
    let Ok(mut entity) = world.get_entity_mut(unit) else {
        return;
    };
    let Some(auto_attack) = entity.take::<AutoAttack>() else {
        return;
    };
    send_attack_stopped(world, unit, Some(auto_attack.victim));
}

/// `Unit::SendMeleeAttackStop` (Unit.cpp:2550-2556): `NowDead` is the attacker's
/// own death.
fn send_attack_stopped(world: &mut World, attacker: Entity, victim: Option<Entity>) {
    let now_dead = world
        .get::<Health>(attacker)
        .is_some_and(|health| health.current <= 0.0);
    let message = AttackStopped {
        attacker: attacker.to_bits(),
        victim: victim.map(Entity::to_bits),
        now_dead,
    };
    crate::spell_cast::send_to_set(world, attacker, message);
}

/// Template-owned melee interval (TC Creature::UpdateEntry), not player or pet weapons.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub(crate) struct CreatureAttackTime(pub f32);

impl CreatureAttackTime {
    fn modified(self, haste_pct: f32, mods: Option<&AuraStatMods>) -> f32 {
        let slow = mods.map_or(1.0, |mods| {
            mods.attack_cast_time_mult * mods.attack_time_mult
        });
        self.0 * crate::auras::time_percent_mod(haste_pct) * slow
    }
}

/// Read the ordinary creature's current interval before changing cached aura modifiers.
pub(crate) fn read_creature_swing_time(world: &World, unit: Entity) -> Option<f32> {
    if world.get::<CharacterStats>(unit).is_some()
        || world
            .get::<crate::spell_cast::guardian::SpiritWolf>(unit)
            .is_some()
    {
        return None;
    }
    let base = world.get::<CreatureAttackTime>(unit)?;
    let haste = world
        .get::<crate::stats::StatModifiers>(unit)
        .map_or(0.0, |mods| mods.haste_pct);
    Some(base.modified(haste, world.get::<AuraStatMods>(unit)))
}

/// TC Unit::ApplyAttackTimePercentMod preserves each hand's remaining progress.
pub(crate) fn rescale_creature_swing_timers(
    world: &mut World,
    unit: Entity,
    previous: Option<f32>,
) {
    let Some(previous) = previous else {
        return;
    };
    let Some(current) = read_creature_swing_time(world, unit) else {
        return;
    };
    if previous == current {
        return;
    }
    if let Some(mut attack) = world.get_mut::<AutoAttack>(unit) {
        let multiplier = current / previous;
        attack.timer *= multiplier;
        attack.off_hand_timer *= multiplier;
    }
}

type AutoAttackers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut AutoAttack,
        &'static Position,
        Option<&'static AuraStatMods>,
        Option<&'static CharacterStats>,
        SwingState<'static>,
        (
            Has<Npc>,
            Option<&'static crate::zones::CreatureFlagsExtra>,
            Option<&'static crate::zones::TempSummon>,
            Option<&'static CreatureAttackTime>,
        ),
    ),
>;

/// What else a swing reads of its attacker: a creature's haste (players fold it
/// into `CharacterStats`) and the states that stop auto-attacks
/// (`UNIT_STATE_CASTING`, `UNIT_STATE_CANNOT_AUTOATTACK`).
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
        (Has<Npc>, Option<&'static crate::zones::CreatureFlagsExtra>),
        Option<&'static AuraStatMods>,
        Has<TrainingDummy>,
        Option<&'static DuelOpponent>,
        TargetArmor<'static>,
        Option<&'static shared::components::UnitFlags>,
        CanAvoid<'static>,
        Option<&'static shared::components::PlayerStandState>,
        Has<Unkillable>,
        Option<&'static crate::scripted_ai::CreatureScriptName>,
        Option<&'static shared::components::Rotation>,
    ),
>;

/// What decides whether a victim can dodge, parry or block: casting
/// (`IsNonMeleeSpellCast`) or controlled (`UNIT_STATE_CONTROLLED`: stunned,
/// confused or fleeing here) it cannot (Unit.cpp:2408-2413).
type CanAvoid<'a> = (
    Has<shared::casting::CastState>,
    Option<&'a mut crate::auras::ActiveAuras>,
);

fn can_avoid((casting, auras): (bool, Option<&crate::auras::ActiveAuras>)) -> bool {
    !casting
        && !auras
            .is_some_and(|auras| auras.is_stunned() || auras.is_confused() || auras.is_feared())
}

/// A creature's own armor or a player's gear armor.
type TargetArmor<'a> = (
    Option<&'a CreatureArmor>,
    Option<&'a CharacterStats>,
    Option<&'a crate::stats::StatModifiers>,
    Option<&'a crate::hunter_pet::HunterPet>,
    Option<&'a crate::spell_cast::summon::SummonedCreature>,
);

/// The same unit bodies and server ray search used by creature spells.
#[derive(SystemParam)]
struct SwingSight<'w, 's> {
    los: Option<Res<'w, LineOfSight>>,
    maps: Query<'w, 's, &'static InMap>,
    bodies: Query<'w, 's, BodyParts<'static>>,
}

impl SwingSight<'_, '_> {
    fn sees(&self, attacker: Entity, target: Entity) -> bool {
        // Match sees_in_world: without a map, spell LOS is not checked.
        let Ok(map) = self.maps.get(attacker) else {
            return true;
        };
        // try_swing already requires both units to have Position.
        let watcher = self.bodies.get(attacker).unwrap();
        let target = self.bodies.get(target).unwrap();
        unit_sees(self.los.as_deref(), *map, watcher, target)
    }

    fn send_blocked_swing(&self, commands: &mut Commands, attacker: Entity) {
        let (_, _, _, _, _, _, player, _) = self.bodies.get(attacker).unwrap();
        if player.is_none() {
            return;
        }
        commands.queue(move |world: &mut World| {
            crate::spell_cast::send_to_owner(
                world,
                attacker,
                shared::protocol::CastFailed {
                    spell_id: ATTACK_SPELL_ID,
                    reason: shared::spell_data::CastFailReason::LineOfSight,
                    detail: None,
                },
            );
        });
    }
}

#[derive(SystemParam)]
pub(crate) struct AutoAttackParams<'w, 's> {
    units: ParamSet<'w, 's, (AutoAttackers<'w, 's>, AutoAttackTargets<'w, 's>)>,
    scaling: Query<'w, 's, ScalingItem<'static>>,
    spirit_wolves: Query<'w, 's, &'static crate::spell_cast::guardian::SpiritWolf>,
    owner_stats: Query<'w, 's, &'static CharacterStats>,
    summon_modifiers: crate::auras::summon_damage::SummonDamageModifiers<'w, 's>,
    sight: SwingSight<'w, 's>,
    invisibility: Query<'w, 's, &'static crate::auras::Invisibility>,
    stealth: crate::auras::stealth::DetectionUnits<'w, 's>,
    armor_constants: Option<Res<'w, ArmorConstants>>,
    race_types: Option<Res<'w, crate::unit_creature_type::RaceCreatureTypes>>,
    forms: Option<Res<'w, crate::auras::shapeshift::ShapeshiftForms>>,
    world_data: Option<Res<'w, crate::world_data::WorldData>>,
    spells: Option<Res<'w, crate::spell_info::SpellStore>>,
    commands: Commands<'w, 's>,
}

/// Process auto-attacks: tick timers, deal damage when in range and timer ready.
pub(crate) fn auto_attack_system(world: &mut World) {
    let attackers: Vec<Entity> = world
        .query_filtered::<Entity, With<AutoAttack>>()
        .iter(world)
        .collect();
    for attacker in attackers {
        // Apply this attacker's commands before the next attacker reads mitigation.
        // In particular, consuming Ardent also removes its30% damage reduction now.
        world
            .run_system_cached_with(auto_attack_unit, attacker)
            .expect("auto-attack unit system must have its registered resources");
    }
}

fn auto_attack_unit(
    In(attacker_entity): In<Entity>,
    time: Res<Time>,
    mut params: AutoAttackParams,
) {
    let dt = time.delta_secs();
    let mut swings = Vec::new();
    let mut redirect_health = HashMap::new();
    // Release the attacker's read of ActiveAuras before mutating the victim's
    // absorb capacity. A player can be both an attacker and a victim.
    let Some(mut snapshot) = snapshot_attacker(&mut params.units.p0(), attacker_entity, dt) else {
        return;
    };
    let target_entity = snapshot.auto_attack.victim;
    let observer = params
        .invisibility
        .get(attacker_entity)
        .cloned()
        .unwrap_or_default();
    let target = params
        .invisibility
        .get(target_entity)
        .cloned()
        .unwrap_or_default();
    if !observer.detects(&target, true)
        || !crate::auras::stealth::query_can_detect(&params.stealth, attacker_entity, target_entity)
    {
        return;
    }
    let attacker = Attacker {
        entity: attacker_entity,
        position: &snapshot.position,
        mods: snapshot.mods.as_ref(),
        stats: snapshot.stats.as_ref(),
        haste_pct: snapshot.haste_pct,
        enemy_dodge_pct: snapshot.enemy_dodge_pct,
        expertise_pct: snapshot.expertise_pct,
        creature: snapshot.creature,
        authored_attack_time: snapshot.authored_attack_time,
        creature_flags: snapshot.creature_flags.as_ref(),
        temp_summoner: snapshot.temp_summoner,
        spirit_wolf: params.spirit_wolves.get(attacker_entity).ok().map(|wolf| {
            let owner = params
                .owner_stats
                .get(wolf.owner)
                .expect("Spirit Wolf owner requires CharacterStats for inherited power");
            (
                wolf,
                crate::spell_cast::guardian::SpiritWolf::owner_power(owner.attack_power),
            )
        }),
    };
    for hand in [Hand::Main, Hand::Off] {
        let Some(outcome) = try_swing(
            &attacker,
            hand,
            &mut params.units.p1(),
            &SwingContext {
                scaling: &params.scaling,
                summon_modifiers: &params.summon_modifiers,
                sight: &params.sight,
                armor_constants: params.armor_constants.as_deref(),
                spells: params.spells.as_deref(),
                creature_types: crate::unit_creature_type::CreatureTypes {
                    races: params.race_types.as_deref(),
                    forms: params.forms.as_deref(),
                    data: params.world_data.as_deref(),
                },
            },
            target_entity,
            &mut snapshot.auto_attack,
            &mut params.commands,
            &mut redirect_health,
        ) else {
            continue;
        };
        swings.push((attacker_entity, target_entity, outcome));
    }
    if let Ok((_, mut auto_attack, ..)) = params.units.p0().get_mut(attacker_entity) {
        *auto_attack = snapshot.auto_attack;
    }
    for (attacker, target, outcome) in swings {
        params.commands.queue(move |world: &mut World| {
            record_swing(world, attacker, target, outcome);
        });
    }
}

struct SwingSnapshot {
    auto_attack: AutoAttack,
    position: Position,
    mods: Option<AuraStatMods>,
    stats: Option<CharacterStats>,
    haste_pct: f32,
    enemy_dodge_pct: f32,
    expertise_pct: f32,
    creature: bool,
    authored_attack_time: Option<CreatureAttackTime>,
    creature_flags: Option<crate::zones::CreatureFlagsExtra>,
    temp_summoner: Option<Entity>,
}

fn snapshot_attacker(
    attackers: &mut AutoAttackers,
    entity: Entity,
    dt: f32,
) -> Option<SwingSnapshot> {
    let (
        _,
        mut auto_attack,
        position,
        mods,
        stats,
        (stat_mods, casting, auras),
        (creature, flags, summon, attack_time),
    ) = attackers.get_mut(entity).ok()?;
    auto_attack.timer -= dt;
    auto_attack.off_hand_timer -= dt;
    // Unit::DoMeleeAttackIfReady: timers run even while casting or controlled.
    if casting || auras.is_some_and(|auras| auras.cannot_auto_attack()) {
        return None;
    }
    Some(SwingSnapshot {
        auto_attack: auto_attack.clone(),
        position: *position,
        mods: mods.cloned(),
        stats: stats.cloned(),
        haste_pct: stat_mods.map_or(0.0, |mods| mods.haste_pct),
        enemy_dodge_pct: auras.map_or(0.0, |auras| auras.modifier(MOD_ENEMY_DODGE)),
        expertise_pct: crate::auras::combat_stats::expertise_reduction(auras, !creature),
        creature,
        authored_attack_time: attack_time.copied(),
        creature_flags: flags.copied(),
        temp_summoner: summon.map(|summon| summon.summoner),
    })
}

fn record_swing(
    world: &mut World,
    attacker_entity: Entity,
    target_entity: Entity,
    receipt: SwingCommitReceipt,
) {
    let Some(outcome) = *receipt
        .0
        .get()
        .expect("swing health must commit before publication")
    else {
        return;
    };
    crate::combat_refs::set_in_combat_with(world, attacker_entity, target_entity);
    mark_replication_changed(world.get_resource::<ReplicationDirtyQueue>(), target_entity);
    // Procs default to normal and critical hits (SpellMgr::LoadSpellProcs
    // HitMask 0); dodge- and parry-triggered procs are not modelled.
    if !outcome.immune
        && !outcome.kind.avoided()
        && let Some(mut pending) = world.get_resource_mut::<PendingAuraEvents>()
    {
        pending
            .0
            .extend(swing_aura_events(attacker_entity, target_entity, &outcome));
    }
    let caused_damage = outcome.damage > 0.0;
    record_swing_events(world, attacker_entity, target_entity, outcome);
    if caused_damage {
        // TC Unit::DealMeleeDamage: after health/death and only for damaging swings.
        crate::auras::damage_shield::retaliate(world, target_entity, attacker_entity);
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
        damage: outcome.scaled_loss,
        reflected: false,
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
            damage: 0.0,
            reflected: false,
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
    /// Signed attacker aura251 sum, snapshotted before mutating victim auras.
    pub enemy_dodge_pct: f32,
    /// Signed NPC aura240/4 reduction; players do not use this expertise path.
    pub expertise_pct: f32,
    /// A creature (`Npc`): it rolls the creature side of the hit table.
    pub creature: bool,
    pub authored_attack_time: Option<CreatureAttackTime>,
    /// Actual attacker's already-cached template flags, independent of victim flags.
    pub creature_flags: Option<&'a crate::zones::CreatureFlagsExtra>,
    /// Existing TempSummon direct summoner, not player owner or recursive ancestor.
    pub temp_summoner: Option<Entity>,
    pub spirit_wolf: Option<(&'a crate::spell_cast::guardian::SpiritWolf, f32)>,
}

impl Attacker<'_> {
    fn weapon(&self, hand: Hand) -> Option<WeaponDamage> {
        if let Some((wolf, _)) = self.spirit_wolf {
            return matches!(hand, Hand::Main).then_some(wolf.weapon);
        }
        match hand {
            Hand::Main => Some(
                self.stats
                    .map_or(WeaponDamage::UNARMED, |stats| stats.main_hand),
            ),
            Hand::Off => self.stats.and_then(|stats| stats.off_hand),
        }
    }

    /// Players/Spirit Wolf retain their weapon clocks; source creatures use their
    /// authored interval and TC's signed attack-time factors.
    fn swing_time(&self, weapon: &WeaponDamage) -> f32 {
        let slow = self.mods.map_or(1.0, |mods| {
            mods.attack_cast_time_mult * mods.attack_time_mult
        });
        if self.spirit_wolf.is_some() {
            return weapon.speed / (1.0 + self.haste_pct / 100.0).max(0.01) * slow;
        }
        if let Some(stats) = self.stats {
            return melee::swing_time(weapon.speed, stats.haste_pct) * slow;
        }
        if let Some(base) = self.authored_attack_time {
            return base.modified(self.haste_pct, self.mods);
        }
        // Units outside template-owned creature timing keep their existing base clock.
        AUTO_ATTACK_INTERVAL / (1.0 + self.haste_pct / 100.0).max(0.01) * slow
    }
}

/// World data a swing reads besides the two units.
pub(crate) struct SwingContext<'a, 'w, 's> {
    pub scaling: &'a Query<'w, 's, ScalingItem<'static>>,
    summon_modifiers: &'a crate::auras::summon_damage::SummonDamageModifiers<'w, 's>,
    pub armor_constants: Option<&'a ArmorConstants>,
    spells: Option<&'a crate::spell_info::SpellStore>,
    sight: &'a SwingSight<'w, 's>,
    creature_types: crate::unit_creature_type::CreatureTypes<'a>,
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
    taken: f32,
    additional_mult: f32,
) -> f32 {
    let mut rng = rand::thread_rng();
    let base = if let Some((_, attack_power)) = attacker.spirit_wolf {
        // Guardian::UpdateDamagePhysical, StatSystem.cpp:1423. Only owner AP
        // is sourced; intrinsic AP remains the named pet_levelstats gap.
        let bonus = attack_power / 3.5 * weapon.speed;
        rand::Rng::gen_range(&mut rng, weapon.min + bonus..=weapon.max + bonus)
    } else {
        match attacker.stats {
            Some(stats) => melee::roll_weapon_damage(
                melee::weapon_damage_range(weapon, stats.attack_power, stats.versatility_pct, hand),
                rand::Rng::r#gen(&mut rng),
            ),
            None => creature_weapon_damage(scaling),
        }
    };
    apply_swing_multipliers(base, attacker.mods, taken, additional_mult)
}

fn apply_swing_multipliers(
    base: f32,
    attacker_mods: Option<&AuraStatMods>,
    taken: f32,
    additional_mult: f32,
) -> f32 {
    const PHYSICAL: u32 = 1;
    let done = attacker_mods.map_or(1.0, |mods| {
        mods.damage_done(PHYSICAL) * mods.autoattack_damage_mult
    });
    let taken = attacker_mods.map_or(taken, |mods| mods.bypass_damage_reduction(PHYSICAL, taken));
    (base * done * additional_mult * taken).max(0.0).round()
}

/// TC Unit.cpp:8110, the victim type is shared with spell damage/target admission.
fn creature_type_damage_multiplier(
    mods: Option<&AuraStatMods>,
    victim: Entity,
    auras: Option<&crate::auras::ActiveAuras>,
    context: &SwingContext,
) -> f32 {
    let Some(mods) = mods.filter(|mods| mods.has_damage_done_versus()) else {
        return 1.0;
    };
    let (_, npc, _, _, _, _, player, _) =
        context.sight.bodies.get(victim).expect("swing target body");
    let kind = context
        .creature_types
        .resolve(player, npc, auras)
        .unwrap_or_else(|err| panic!("swing victim{victim}: {err}"));
    mods.damage_done_versus(crate::unit_creature_type::mask(kind))
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
    (creature, player, modifiers, hunter, summon): TargetArmor,
    target: &UnitScaling,
    attacker: &UnitScaling,
) -> f32 {
    match (creature, player) {
        (Some(armor), _) => {
            let multiplier = crate::auras::base_armor::pet_multiplier(modifiers, hunter, summon);
            target.armor_against(armor.0 * multiplier, attacker)
                * crate::auras::base_armor::total_multiplier(modifiers)
        }
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

/// The hit-table result of a swing (Unit::RollMeleeOutcomeAgainst): a creature's
/// miss chance or a player's crit chance and dual wielding, against `victim`. A player
/// without `CharacterStats` (no class stat data) always hits.
fn roll_outcome(
    attacker: &Attacker,
    victim: Victim,
    flags: Option<&crate::zones::CreatureFlagsExtra>,
    can_avoid: bool,
    in_front: bool,
    target_auras: Option<&crate::auras::ActiveAuras>,
    target_health: &Health,
) -> MeleeOutcome {
    let chances = read_outcome_chances(
        attacker,
        victim,
        flags,
        can_avoid,
        in_front,
        target_auras,
        target_health,
    );
    melee::roll_melee_outcome(
        &chances,
        rand::Rng::gen_range(&mut rand::thread_rng(), 0..10_000),
    )
}

fn read_outcome_chances(
    attacker: &Attacker,
    victim: Victim,
    flags: Option<&crate::zones::CreatureFlagsExtra>,
    can_avoid: bool,
    in_front: bool,
    target_auras: Option<&crate::auras::ActiveAuras>,
    target_health: &Health,
) -> melee::MeleeChances {
    let swinger = match (attacker.creature, attacker.stats) {
        (true, _) => Swinger::Creature {
            miss_pct: creature_miss_chance(attacker.mods),
        },
        (false, Some(stats)) => Swinger::Player {
            crit_pct: stats.crit_pct,
            dual_wield: stats.off_hand.is_some(),
        },
        (false, None) => Swinger::Player {
            crit_pct: 0.0,
            dual_wield: false,
        },
    };
    let mut chances = if !attacker.creature && attacker.stats.is_none() {
        melee::MeleeChances::default()
    } else {
        melee::swing_chances(swinger, victim, can_avoid)
    };
    // Unit.cpp2761–2861: NPC victim49/47/51; only dodge/parry meet attacker modifiers.
    // Never restore CC/casting or missing-player-stats prohibited avoidance.
    if can_avoid && (attacker.creature || attacker.stats.is_some()) {
        if matches!(victim, Victim::Creature { .. }) {
            chances.dodge += target_auras.map_or(0.0, |auras| {
                auras.modifier(crate::auras::handlers::MOD_DODGE_PERCENT)
            });
            chances.parry += target_auras.map_or(0.0, |auras| {
                auras.modifier(crate::auras::handlers::MOD_PARRY_PERCENT)
            });
            chances.block += target_auras.map_or(0.0, |auras| {
                auras.modifier(crate::auras::handlers::MOD_BLOCK_PERCENT)
            });
            chances.block = chances.block.max(0.0);
        }
        chances.dodge =
            (chances.dodge + attacker.enemy_dodge_pct - attacker.expertise_pct).max(0.0);
        chances.parry = (chances.parry - attacker.expertise_pct).max(0.0);
    }
    if let (Victim::Creature { .. }, Some(flags)) = (victim, flags) {
        const NO_PARRY: u32 = 0x4;
        const NO_BLOCK: u32 = 0x10;
        if flags.0 & NO_PARRY != 0 {
            chances.parry = 0.0;
        }
        if flags.0 & NO_BLOCK != 0 {
            chances.block = 0.0;
        }
    }
    let ignores_direction = target_auras
        .is_some_and(|auras| auras.has_type(crate::auras::handlers::IGNORE_HIT_DIRECTION));
    if !in_front && !ignores_direction {
        chances.parry = 0.0;
        chances.block = 0.0;
        if matches!(victim, Victim::Player(_)) {
            chances.dodge = 0.0;
        }
    }
    // Unit::MeleeSpellMissChance: victim's attacker-hit modifier, not attacker hit.
    let hit_modifier = target_auras.map_or(0.0, |auras| {
        auras.modifier(crate::auras::handlers::MOD_ATTACKER_MELEE_HIT_CHANCE)
    });
    let base_miss = if !attacker.creature && attacker.stats.is_none() {
        0.0
    } else {
        chances.miss
    };
    // Creature miss_pct already includes its own aura54; players must subtract
    // their caster modifier here too (Unit::MeleeSpellMissChance, Unit.cpp12525).
    let player_hit_modifier = if attacker.creature {
        0.0
    } else {
        attacker.mods.map_or(0.0, |mods| mods.melee_hit_chance_pct)
    };
    chances.miss = (base_miss - player_hit_modifier - hit_modifier).clamp(0.0, 100.0);
    let crit_modifier = target_auras.map_or(0.0, |auras| {
        auras.modifier(crate::auras::handlers::MOD_ATTACKER_MELEE_CRIT_CHANCE)
            + auras.modifier(crate::auras::handlers::MOD_ATTACKER_SPELL_AND_WEAPON_CRIT_CHANCE)
            + health_crit_modifier(auras, target_health)
            + caster_crit_modifier(
                auras,
                attacker.entity,
                crate::auras::handlers::MOD_CRIT_CHANCE_FOR_CASTER,
            )
            + attacker.temp_summoner.map_or(0.0, |summoner| {
                caster_crit_modifier(
                    auras,
                    summoner,
                    crate::auras::handlers::MOD_CRIT_CHANCE_FOR_CASTER_PET,
                )
            })
    });
    let npc_crit_modifier = if attacker.creature {
        attacker.mods.map_or(0.0, |mods| mods.npc_crit_pct)
    } else {
        0.0
    };
    // Unit.cpp2885–2893: NO_CRIT excludes only NPC base5 + source52/290.
    const NO_CRIT: u32 = 0x20000;
    let source_crit = if attacker.creature
        && attacker
            .creature_flags
            .is_some_and(|flags| flags.0 & NO_CRIT != 0)
    {
        0.0
    } else {
        chances.crit + npc_crit_modifier
    };
    // Unit.cpp2906–2926 clamps source + received183/187/197/306/339 BEFORE334 at2387.
    let crit_taken = (source_crit + crit_modifier).max(0.0);
    let autoattack_crit_modifier = attacker.mods.map_or(0.0, |mods| mods.autoattack_crit_pct);
    chances.crit = (crit_taken + autoattack_crit_modifier).max(0.0);
    chances
}

/// TC Unit.cpp2906–2909 / Unit.h804,809: !HealthBelowPct includes equality.
fn health_crit_modifier(auras: &crate::auras::ActiveAuras, health: &Health) -> f32 {
    auras
        .iter()
        .map(|aura| {
            let stacks = aura.stack_multiplier();
            aura.effects
                .iter()
                .filter(|effect| {
                    effect.aura_type == crate::auras::handlers::MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH
                })
                .filter(|effect| {
                    let threshold = (health.max * effect.misc_values[1] as f32 / 100.0).trunc();
                    health.current >= threshold
                })
                .map(|effect| effect.amount * stacks)
                .sum::<f32>()
        })
        .sum()
}

/// TC Unit.cpp2911–2921: recipient crit306/339 effects match the sourced caster.
fn caster_crit_modifier(auras: &crate::auras::ActiveAuras, caster: Entity, aura_type: u32) -> f32 {
    auras
        .iter()
        .filter(|aura| aura.caster == caster)
        .map(|aura| {
            let stacks = aura.stack_multiplier();
            aura.effects
                .iter()
                .filter(|effect| effect.aura_type == aura_type)
                .map(|effect| effect.amount * stacks)
                .sum::<f32>()
        })
        .sum()
}

#[derive(Component)]
struct MissingCreatureFlagsReported;

fn report_missing_creature_flags(commands: &mut Commands, victim: Entity) {
    commands
        .entity(victim)
        .queue(|mut actor: EntityWorldMut<'_>| {
            if actor.contains::<MissingCreatureFlagsReported>() {
                return;
            }
            error!(
                victim = ?actor.id(),
                "NPC autoattack victim has no CreatureFlagsExtra; retaining unfiltered avoidance"
            );
            actor.insert(MissingCreatureFlagsReported);
        });
}

/// Who the swing lands on: a creature (by level difference), or a player's
/// avoidance and whether it is not standing; a unit without `CharacterStats` avoids
/// nothing.
fn swing_victim(
    is_npc: bool,
    player: Option<&CharacterStats>,
    level_difference: i32,
    sitting: bool,
) -> Victim {
    if is_npc {
        Victim::Creature { level_difference }
    } else {
        Victim::Player(PlayerDefense {
            sitting,
            ..player.map_or_else(Default::default, |stats| stats.defense)
        })
    }
}

/// Share of a blocked swing the victim stops: a creature 30%, a player its shield
/// against the attacker's level armor constant.
fn blocked_pct(
    victim: Victim,
    attacker: &UnitScaling,
    target: &UnitScaling,
    constants: Option<&ArmorConstants>,
) -> f32 {
    match victim {
        Victim::Creature { .. } => melee::CREATURE_BLOCKED_PCT,
        Victim::Player(defense) => constants
            .and_then(|constants| constants.at(attacker.level_against(target)))
            .map_or(0.0, |constant| defense.blocked_pct(constant)),
    }
}

/// TC Unit::IsBlockCritical: integer aura253 amounts on the defender.
pub(crate) fn roll_critical_block(auras: Option<&crate::auras::ActiveAuras>) -> bool {
    const MOD_BLOCK_CRIT_CHANCE: u32 = 253;
    let chance = auras.map_or(0.0, |auras| {
        auras
            .iter()
            .map(|aura| {
                let stacks = aura.stack_multiplier();
                aura.effects
                    .iter()
                    .filter(|effect| effect.aura_type == MOD_BLOCK_CRIT_CHANCE)
                    .map(|effect| effect.amount.trunc() * stacks)
                    .sum::<f32>()
            })
            .sum::<f32>()
    });
    chance >= 100.0 || chance > 0.0 && rand::random::<f32>() * 100.0 < chance
}

fn roll_critical_block_damage(
    auras: Option<&crate::auras::ActiveAuras>,
    damage: f32,
    blocked: f32,
) -> (f32, f32) {
    if roll_critical_block(auras) {
        // Autoswings double AFTER the ordinary block amount was truncated.
        (damage - blocked, blocked * 2.0)
    } else {
        (damage, blocked)
    }
}

/// The health commit is queued by try_swing even when its caller discards this
/// receipt. Publication reads the actual committed result, never a second hit.
struct SwingCommitReceipt(std::sync::Arc<std::sync::OnceLock<Option<SwingOutcome>>>);

fn queue_swing_commit(
    commands: &mut Commands,
    attacker: Entity,
    target: Entity,
    mut outcome: SwingOutcome,
) -> SwingCommitReceipt {
    let receipt = std::sync::Arc::new(std::sync::OnceLock::new());
    let committed = std::sync::Arc::clone(&receipt);
    commands.queue(move |world: &mut World| {
        let result = commit_melee_loss(world, attacker, target, &mut outcome).then_some(outcome);
        committed
            .set(result)
            .expect("each swing commits exactly once");
    });
    SwingCommitReceipt(receipt)
}

#[derive(Debug, Clone, Copy)]
struct SwingOutcome {
    hand: Hand,
    kind: MeleeOutcome,
    immune: bool,
    absorbed: f32,
    damage: f32,
    /// `damage` in the target's health units (level scaling).
    scaled_loss: f32,
    /// Training-dummy adjusted loss; health-sensitive limits run at commit time.
    incoming_loss: f32,
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

fn melee_damage_splits(
    targets: &mut AutoAttackTargets,
    context: &SwingContext,
    victim: Entity,
    attacker: Entity,
) -> Vec<crate::auras::damage_transfer::Split> {
    let candidates = match targets.get_mut(victim) {
        Ok((_, _, _, _, _, _, _, _, _, avoidance, ..)) => {
            crate::auras::damage_transfer::candidates(avoidance.1.as_deref(), context.spells, 1)
        }
        Err(_) => return Vec::new(),
    };
    let mut splits = Vec::new();
    for mut split in candidates {
        if split.recipient == victim
            || context.sight.maps.get(split.recipient).ok() != context.sight.maps.get(victim).ok()
        {
            continue;
        }
        let Ok((_, health, _, _, _, _, _, _, _, avoidance, ..)) = targets.get_mut(split.recipient)
        else {
            continue;
        };
        if health.current <= 0.0 || crate::auras::immune_to_school(avoidance.1.as_deref(), 1) {
            continue;
        }
        let scaling = |entity| {
            context.scaling.get(entity).map_or(
                UnitScaling::from_item((None, None, None)),
                UnitScaling::from_item,
            )
        };
        let multiplier = scaling(split.recipient).health_multiplier(&scaling(attacker));
        split.health_at_plan = health.current;
        split.budget =
            (health.current - health.max * split.floor_pct / 100.0).max(0.0) * multiplier;
        splits.push(split);
    }
    splits
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
    redirect_health: &mut HashMap<Entity, f32>,
) -> Option<SwingCommitReceipt> {
    let timer = match hand {
        Hand::Main => auto_attack.timer,
        Hand::Off => auto_attack.off_hand_timer,
    };
    if timer > 0.0 {
        return None;
    }
    let weapon = attacker.weapon(hand)?;
    let splits = melee_damage_splits(targets, context, target_entity, attacker.entity);
    let Ok((
        target_pos,
        health,
        _last_damager,
        (is_npc, npc_flags),
        target_mods,
        training_dummy,
        _duel,
        armor,
        flags,
        mut avoidance,
        stand_state,
        _unkillable,
        _script_name,
        target_rotation,
    )) = targets.get_mut(target_entity)
    else {
        return None;
    };
    // Typed detection was checked for this attacker before the swing.
    let attackable = flags.is_none_or(|flags| flags.can_be_attacked_by(!attacker.creature));
    let range_bonus = attacker
        .mods
        .map_or(0.0, |mods| mods.autoattack_range_bonus);
    if !in_melee_range(attacker.position, target_pos, range_bonus)
        || health.current <= 0.0
        || !attackable
    {
        return None;
    }
    if !context.sight.sees(attacker.entity, target_entity) {
        context.sight.send_blocked_swing(commands, attacker.entity);
        return None;
    }
    let unit = |entity| {
        context.scaling.get(entity).map_or(
            UnitScaling::from_item((None, None, None)),
            UnitScaling::from_item,
        )
    };
    let (attacker_scaling, target) = (unit(attacker.entity), unit(target_entity));
    let level_difference = i32::from(target.level_against(&attacker_scaling))
        - i32::from(attacker_scaling.level_against(&target));
    let versus = creature_type_damage_multiplier(
        attacker.mods,
        target_entity,
        avoidance.1.as_deref(),
        context,
    );
    let taken = target_mods.map_or(1.0, |mods| mods.damage_taken(1))
        * crate::auras::melee_damage_from_caster(avoidance.1.as_deref(), attacker.entity)
        * armor
            .1
            .map_or(1.0, CharacterStats::versatility_damage_taken_multiplier);
    let armored = armor_mitigated(
        swing_damage(
            attacker,
            &weapon,
            hand,
            &attacker_scaling,
            taken,
            context.summon_modifiers.multiplier(attacker.entity)
                * context
                    .summon_modifiers
                    .taken_multiplier(attacker.entity, avoidance.1.as_deref())
                * versus,
        ),
        target_armor(armor, &target, &attacker_scaling),
        &attacker_scaling,
        &target,
        context.armor_constants,
    );
    let sitting = stand_state.is_some_and(|state| !state.0.is_stand());
    let victim = swing_victim(is_npc, armor.1, level_difference, sitting);
    if is_npc && npc_flags.is_none() {
        report_missing_creature_flags(commands, target_entity);
    }
    let victim_position = Vec3::new(target_pos.x, target_pos.y, target_pos.z);
    let attacker_position = Vec3::new(
        attacker.position.x,
        attacker.position.y,
        attacker.position.z,
    );
    let orientation = crate::spell_cast::targets::orientation_from_rotation(target_rotation);
    let in_front =
        crate::spell_cast::targets::in_front_arc(victim_position, attacker_position, orientation);
    let kind = roll_outcome(
        attacker,
        victim,
        npc_flags,
        can_avoid((avoidance.0, avoidance.1.as_deref())),
        in_front,
        avoidance.1.as_deref(),
        &health,
    );
    // Unit::DealMeleeDamage → DealDamage (Unit.cpp:1185-1186): every swing, missed or
    // not, stands a player victim up.
    if stand_state.is_some_and(|state| state.0 != shared::components::StandState::Stand) {
        commands.queue(move |world: &mut World| {
            crate::stand_state::stand_on_damage(world, target_entity)
        });
    }
    let immune = crate::auras::immune_to_school(avoidance.1.as_deref(), 1);
    let (outcome_damage, blocked) = melee::outcome_damage(
        armored,
        kind,
        level_difference,
        blocked_pct(victim, &attacker_scaling, &target, context.armor_constants),
    );
    let (outcome_damage, blocked) = if kind == MeleeOutcome::Block && !immune {
        roll_critical_block_damage(avoidance.1.as_deref(), outcome_damage, blocked)
    } else {
        (outcome_damage, blocked)
    };
    let critical_multiplier = if kind == MeleeOutcome::Crit {
        attacker.mods.map_or(1.0, |mods| mods.critical_damage(1))
    } else {
        1.0
    };
    let outcome_damage = if immune {
        0.0
    } else {
        outcome_damage * critical_multiplier
    };
    let absorption = crate::auras::consume_absorbs(
        avoidance
            .1
            .as_mut()
            .map(|auras| auras.bypass_change_detection()),
        1,
        outcome_damage,
    );
    let absorbed = absorption.absorbed;
    let mut remaining = absorption.remaining;
    for split in splits {
        let recipient_scaling = context.scaling.get(split.recipient).map_or(
            UnitScaling::from_item((None, None, None)),
            UnitScaling::from_item,
        );
        let health_left = redirect_health
            .entry(split.recipient)
            .or_insert(split.health_at_plan);
        let multiplier = recipient_scaling.health_multiplier(&attacker_scaling);
        let reserved = (split.health_at_plan - *health_left) * multiplier;
        let budget = (split.budget - reserved).max(0.0);
        let share = (remaining * split.fraction).min(budget);
        *health_left -=
            crate::level_scaling::health_loss(&attacker_scaling, &recipient_scaling, share);
        remaining -= share;
        let attacker_entity = attacker.entity;
        commands.queue(move |world: &mut World| {
            crate::auras::damage_transfer::deliver(
                world,
                attacker_entity,
                target_entity,
                1,
                split,
                share,
            );
        });
    }
    let damage = crate::level_scaling::damage_dealt(&attacker_scaling, &target, remaining);
    commands.queue(move |world: &mut World| {
        crate::auras::finish_absorbs(world, target_entity, absorption)
    });
    let scaled_loss = crate::level_scaling::health_loss(&attacker_scaling, &target, damage);
    // Commit health-sensitive limits and real rescue children in swing order.
    let incoming_loss = health_loss(training_dummy, scaled_loss);
    let pet_time = context
        .summon_modifiers
        .pet_melee_time_multiplier(attacker.entity);
    reset_swing_timers(auto_attack, hand, attacker.swing_time(&weapon) * pet_time);
    let outcome = SwingOutcome {
        hand,
        kind,
        immune,
        absorbed,
        damage,
        scaled_loss,
        incoming_loss,
        blocked,
        overkill: 0.0,
        weapon_speed: weapon.speed,
        npc_died: false,
        player_died: false,
        duel_defeated: false,
    };
    Some(queue_swing_commit(
        commands,
        attacker.entity,
        target_entity,
        outcome,
    ))
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

/// TC IsWithinMeleeRangeAt: nominal range plus attacker-owned aura153 yards.
/// Spell range does not consume this modifier.
fn in_melee_range(a: &Position, b: &Position, range_bonus: f32) -> bool {
    let range = MELEE_RANGE + range_bonus;
    distance(a, b).powi(2) <= range * range
}

/// Euclidean distance between two positions.
fn distance(a: &Position, b: &Position) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// A swing's `CombatEvent` (`SMSG_ATTACKER_STATE_UPDATE` `HitInfo`): a crit, a partial
/// block with the damage that got through, a hit (glancing included), or the avoidance
/// without an amount.
fn build_swing_event(attacker: Entity, target: Entity, outcome: &SwingOutcome) -> CombatEvent {
    let event_type = match swing_miss_kind(outcome) {
        Some(MissKind::Immune) => CombatEventType::Miss,
        Some(MissKind::Absorb) => CombatEventType::Absorb,
        _ => match outcome.kind {
            MeleeOutcome::Miss => CombatEventType::Miss,
            MeleeOutcome::Dodge => CombatEventType::Dodge,
            MeleeOutcome::Parry => CombatEventType::Parry,
            MeleeOutcome::Crit => CombatEventType::CriticalHit,
            MeleeOutcome::Block => CombatEventType::Block,
            MeleeOutcome::Glancing | MeleeOutcome::Hit => CombatEventType::MeleeDamage,
        },
    };
    let amount = if event_type == CombatEventType::Absorb {
        outcome.absorbed
    } else {
        outcome.damage
    };
    CombatEvent {
        attacker: attacker.to_bits(),
        target: target.to_bits(),
        amount,
        spell_id: 0,
        event_type,
    }
}

/// The combat log `MissKind` of an avoided swing.
fn swing_miss_kind(outcome: &SwingOutcome) -> Option<MissKind> {
    if outcome.immune {
        return Some(MissKind::Immune);
    }
    if outcome.damage == 0.0 && outcome.absorbed > 0.0 {
        return Some(MissKind::Absorb);
    }
    match outcome.kind {
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
        absorbed: outcome.absorbed as i32,
        resisted: 0,
        blocked: if outcome.immune {
            0
        } else {
            outcome.blocked as i32
        },
        crit: !outcome.immune && outcome.kind == MeleeOutcome::Crit,
        glancing: !outcome.immune && outcome.kind == MeleeOutcome::Glancing,
        periodic: false,
        extra_spell_id: None,
        timestamp_unix_ms: 0,
        kind: swing_miss_kind(outcome).map_or(CombatLogKind::Damage, CombatLogKind::Miss),
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
        glancing: false,
        periodic: false,
        extra_spell_id: None,
        timestamp_unix_ms: 0,
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

/// Commit NPC death with exclusive World access.
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
    let killing_blow = entity.take::<KillingBlow>().and_then(|cause| cause.0);
    entity.insert((DeathState { respawn_timer }, Corpse { remaining }));
    crate::vignettes::on_death(&mut entity);
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
        // TC Unit::setDeathState (Unit.cpp:9187-9190): interrupt after CombatStop,
        // before death hooks can observe or replace the old cast.
        crate::spell_cast::cancel_cast(world, npc);
        if let Some(attacker) = killing_blow {
            notify_killed_unit(world, attacker, npc);
        }
        crate::scripted_ai::dispatch(world, npc, crate::scripted_ai::Hook::JustDied(killer));
        crate::achievement::record_creature_death(world, npc, killer);
        crate::pvp::record_creature_death(world, npc, killer);
        crate::mythic_plus::record_kill(world, npc);
        crate::maps::raid::record_creature_death(world, npc);
        crate::scripted_ai::hellfire_ramparts::on_unit_death(world, npc);
        crate::zones::notify_group_depletion(world, npc);
    });
}

/// `Creature::DespawnOrUnsummon` now: a summon leaves the world (`TempSummon::UnSummon`); a
/// spawn dies without a corpse (`ForcedDespawn`: `setDeathState(JUST_DIED)`, then
/// `RemoveCorpse` skipping the corpse decay) and respawns after its spawn's respawn delay.
pub(crate) fn despawn_creature(world: &mut World, creature: Entity) {
    if world.get::<crate::zones::TempSummon>(creature).is_some() {
        crate::zones::unsummon(world, creature);
        return;
    }
    let respawn_timer = {
        let entity = world.entity(creature);
        let delay = entity.get::<RespawnDelay>().map_or(0.0, |delay| delay.0);
        let corpse = entity.get::<CorpseDelay>().map_or(0.0, |delay| delay.0);
        delay - corpse
    };
    if let Some(mut health) = world.get_mut::<Health>(creature) {
        health.current = 0.0;
    }
    let mut entity = world.entity_mut(creature);
    entity.insert(DeathState { respawn_timer });
    if let Some(mut motion) = entity.get_mut::<shared::components::CreatureMotion>() {
        motion.set_if_neq(shared::components::CreatureMotion::Still);
    }
    crate::creature_aggro::forget_combat(&mut entity);
    combat_stop(world, creature);
    crate::spell_cast::cancel_cast(world, creature);
    crate::interest::set_hidden(world, creature, true);
}

/// `Unit::DealDamage(attacker, victim, damage)` without a spell (`DIRECT_DAMAGE`,
/// school Normal; the `.damage` command): the victim loses `damage` over its health
/// multiplier for the attacker, both enter combat, the victim's threat and
/// `DamageTaken` follow, and a lethal hit kills it.
pub(crate) fn deal_direct_damage(world: &mut World, attacker: Entity, victim: Entity, damage: f32) {
    if !crate::spell_cast::targets::is_alive(world, victim) {
        return;
    }
    let damage = crate::auras::damage_transfer::route(world, attacker, victim, 1, damage);
    deal_health_damage(world, attacker, victim, damage, 1, None, 0.0);
}

/// Every swing uses the same exclusive damage commit as direct/periodic hits.
/// Rescue removal and real healing finish before the next queued swing.
fn commit_melee_loss(
    world: &mut World,
    attacker: Entity,
    victim: Entity,
    outcome: &mut SwingOutcome,
) -> bool {
    let Some(health) = world.get::<Health>(victim) else {
        return false;
    };
    if health.current <= 0.0 {
        return false;
    }
    let before = health.current;
    let scripted_loss = crate::scripted_ai::limit_scripted_loss(
        world.get::<crate::scripted_ai::CreatureScriptName>(victim),
        world
            .get::<crate::scripted_ai::hellfire_ramparts::HeraldUnkillable>(victim)
            .is_some(),
        health,
        outcome.incoming_loss,
    );
    let protected = world.get::<Unkillable>(victim).is_some() && attacker != victim;
    let (loss, defeated) = crate::duel::duel_health_loss(
        world.get::<crate::duel::DuelOpponent>(victim),
        attacker,
        before,
        limit_unkillable_loss(before, scripted_loss, protected),
    );
    let is_npc = world.get::<Npc>(victim).is_some();
    if is_npc && outcome.incoming_loss > 0.0 {
        crate::scripted_ai::damage_taken(
            world,
            victim,
            Some(attacker),
            before,
            outcome.incoming_loss,
        );
    }
    let commit = crate::auras::apply_attack_damage(world, victim, loss, 1);
    let prevented = commit.absorbed_damage(outcome.damage, loss);
    outcome.absorbed += prevented;
    outcome.damage -= prevented;
    outcome.scaled_loss = (outcome.scaled_loss - commit.absorbed_loss).max(0.0);
    let loss = (loss - commit.absorbed_loss).max(0.0);
    outcome.overkill =
        crate::level_scaling::damage_overkill(outcome.damage, outcome.scaled_loss, before);
    outcome.npc_died = is_npc && commit.lethal;
    outcome.player_died = !is_npc && commit.lethal;
    outcome.duel_defeated = defeated;
    crate::achievement::record_damage(world, attacker, victim, before, outcome.damage, loss);
    if loss > 0.0 {
        crate::creature_tap::record_damage(world, victim, attacker, before, loss);
    }
    world.entity_mut(victim).insert(LastDamager(attacker));
    true
}

/// Health application for an already-mitigated direct or redirected hit.
pub(crate) fn deal_health_damage(
    world: &mut World,
    attacker: Entity,
    victim: Entity,
    damage: f32,
    school: u32,
    spell_id: Option<u32>,
    absorbed: f32,
) {
    if !crate::spell_cast::targets::is_alive(world, victim) {
        return;
    }
    let (attacker_scaling, victim_scaling) = (
        UnitScaling::of(world, attacker),
        UnitScaling::of(world, victim),
    );
    let loss = crate::level_scaling::health_loss(&attacker_scaling, &victim_scaling, damage);
    let Some(health) = world.get::<Health>(victim) else {
        return;
    };
    let before = health.current;
    let incoming_loss = loss;
    let scripted_loss = crate::scripted_ai::limit_scripted_loss(
        world.get::<crate::scripted_ai::CreatureScriptName>(victim),
        world
            .get::<crate::scripted_ai::hellfire_ramparts::HeraldUnkillable>(victim)
            .is_some(),
        health,
        incoming_loss,
    );
    let protected = world.get::<Unkillable>(victim).is_some() && attacker != victim;
    let loss = limit_unkillable_loss(before, scripted_loss, protected);
    if incoming_loss > 0.0 {
        crate::scripted_ai::damage_taken(world, victim, Some(attacker), before, incoming_loss);
    }
    let commit = crate::auras::apply_attack_damage(world, victim, loss, school);
    let prevented = commit.absorbed_damage(damage, loss);
    let (damage, absorbed) = (damage - prevented, absorbed + prevented);
    let loss = (loss - commit.absorbed_loss).max(0.0);
    crate::achievement::record_damage(world, attacker, victim, before, damage, loss);
    mark_replication_changed(world.get_resource::<ReplicationDirtyQueue>(), victim);
    world.entity_mut(victim).insert(LastDamager(attacker));
    crate::stand_state::stand_on_damage(world, victim);
    crate::combat_refs::set_in_combat_with(world, attacker, victim);
    if loss > 0.0 {
        crate::creature_aggro::note_damage_taken(world, victim);
    }
    crate::creature_aggro::add_threat(world, victim, attacker, damage);
    crate::spell_cast::log_event(
        world,
        CombatLogEvent {
            source: Some(attacker.to_bits()),
            target: Some(victim.to_bits()),
            spell_id,
            school_mask: school,
            amount: damage as i32,
            overflow: (loss - before).max(0.0) as i32,
            absorbed: absorbed as i32,
            resisted: 0,
            blocked: 0,
            crit: false,
            glancing: false,
            periodic: false,
            extra_spell_id: None,
            timestamp_unix_ms: 0,
            kind: CombatLogKind::Damage,
        },
    );
    if loss > 0.0 {
        crate::creature_tap::record_damage(world, victim, attacker, before, loss);
    }
    if !commit.lethal {
        return;
    }
    world.entity_mut(victim).insert(KillingBlow(Some(attacker)));
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
    world
        .entity_mut(victim)
        .insert((LastDamager(killer), KillingBlow(Some(killer))));
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
    mut corpses: Query<
        (Entity, &mut Corpse, Has<crate::zones::TempSummon>),
        Without<crate::loot::PendingGroupLoot>,
    >,
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
            // TC Creature::RemoveCorpse (Creature.cpp:438): setDeathState(DEAD)
            // interrupts the current casts again.
            commands.queue(move |world: &mut World| {
                crate::spell_cast::cancel_cast(world, entity);
                crate::interest::set_hidden(world, entity, true);
            });
        }
    }
}

/// `Unit::CombatStop` (TC Unit.cpp:6029-6055): `unit` stops attacking (`AttackStop`),
/// its attackers stop attacking it (`RemoveAllAttackers`), and every combat reference
/// it holds ends (`EndAllPvECombat`, `EndAllPvPCombat`), so each unit it fought leaves
/// combat unless it still fights another.
pub(crate) fn combat_stop(world: &mut World, unit: Entity) {
    let attackers: Vec<Entity> = world
        .query::<(Entity, &AutoAttack)>()
        .iter(world)
        .filter(|(_, auto_attack)| auto_attack.victim == unit)
        .map(|(attacker, _)| attacker)
        .collect();
    for attacker in attackers {
        attack_stop(world, attacker);
    }
    attack_stop(world, unit);
    crate::combat_refs::end_all_combat(world, unit);
}

fn record_swing_events(world: &mut World, attacker: Entity, target: Entity, outcome: SwingOutcome) {
    let mut events = vec![build_swing_event(attacker, target, &outcome)];
    let damage = outcome.damage;
    if damage > 0.0 {
        crate::creature_aggro::note_damage_taken(world, target);
    }
    crate::creature_aggro::add_threat(world, target, attacker, damage);
    let rage_secs = match outcome.hand {
        Hand::Main => outcome.weapon_speed,
        Hand::Off => outcome.weapon_speed / 2.0,
    };
    reward_swing_rage(world, attacker, target, rage_secs, damage);
    crate::spell_cast::log_event(world, swing_log_event(attacker, target, &outcome));
    if outcome.npc_died || outcome.player_died {
        world.entity_mut(target).insert(KillingBlow(Some(attacker)));
        crate::auras::remove_on_death(world, target);
        crate::spell_cast::log_event(world, death_log_event(attacker, target));
    }
    if outcome.npc_died {
        mark_npc_dead_world(world, target);
        events.push(build_death_event(target));
    }
    if outcome.player_died {
        world.write_message(crate::death::PlayerDeathTransition { player: target });
    }
    if outcome.duel_defeated {
        world.entity_mut(target).insert(crate::duel::DuelDefeated);
    }
    let elapsed = world.resource::<Time>().elapsed_secs_f64();
    for event in events {
        world
            .resource_mut::<CombatLog>()
            .record(elapsed, event.clone());
        for mut sender in world
            .query::<&mut MessageSender<CombatEvent>>()
            .iter_mut(world)
        {
            sender.send::<CombatChannel>(event.clone());
        }
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

/// Tick respawn timers and restore dead NPCs when ready; a pooled creature's pool
/// re-picks instead (`spawn_pools::respawn_pooled`) and an inactive spawn group's
/// member leaves the world. A patroller
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
            Has<crate::spawn_pools::PooledSpawn>,
            Option<&crate::vignettes::CreatureVignette>,
            Option<(&crate::spawn_groups::GroupedSpawn, &crate::maps::InMap)>,
        ),
        (
            Without<crate::zones::TempSummon>,
            Without<crate::loot::PendingGroupLoot>,
        ),
    >,
    inactive_groups: Option<Res<crate::spawn_groups::InactiveGroupedSpawns>>,
    mut commands: Commands,
    dirty_queue: Option<Res<InterestDirtyQueue>>,
    mut senders: Query<&mut MessageSender<CombatEvent>>,
    mut combat_log: ResMut<CombatLog>,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs_f64();
    let mut events = Vec::new();

    for (entity, mut death, mut health, spawn, mut pos, waypoint, pooled, vignette, grouped) in
        &mut query
    {
        death.respawn_timer -= dt;
        if death.respawn_timer > 0.0 {
            continue;
        }
        if let Some((&crate::spawn_groups::GroupedSpawn(guid), map)) = grouped
            && inactive_groups
                .as_ref()
                .and_then(|inactive| inactive.0.get(map))
                .is_some_and(|guids| guids.contains(&guid))
        {
            // An inactive spawn group's member does not respawn
            // (`Map::SetSpawnGroupInactive`).
            commands.entity(entity).despawn();
            continue;
        }
        if pooled {
            commands.queue(move |world: &mut World| {
                crate::spawn_pools::respawn_pooled(world, entity);
            });
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
        let mut respawned = commands.entity(entity);
        respawned
            .remove::<(DeathState, Corpse, crate::creature_ai::HomePosition)>()
            .insert(Replicate::to_clients(NetworkTarget::All));
        if let Some(&vignette) = vignette {
            crate::vignettes::show(&mut respawned, vignette);
        }
        commands.queue(move |world: &mut World| {
            crate::interest::set_hidden(world, entity, false);
        });
        commands
            .entity(entity)
            .remove::<(LastDamager, crate::creature_tap::CreatureTap)>();
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

#[path = "combat_forecast.rs"]
pub(crate) mod forecast;

#[cfg(test)]
#[path = "combat_enemy_dodge_tests.rs"]
mod enemy_dodge_tests;
#[cfg(test)]
#[path = "combat_facing_tests.rs"]
mod facing_tests;
#[cfg(test)]
#[path = "combat_npc_cadence_tests.rs"]
mod npc_cadence_tests;
#[cfg(test)]
#[path = "combat_npc_expertise_tests.rs"]
mod npc_expertise_tests;
#[cfg(test)]
#[path = "combat_npc_flags_tests.rs"]
mod npc_flags_tests;
#[cfg(test)]
#[path = "combat_npc_general_crit_tests.rs"]
mod npc_general_crit_tests;
#[cfg(test)]
#[path = "combat_npc_no_crit_tests.rs"]
mod npc_no_crit_tests;
#[cfg(test)]
#[path = "combat_npc_victim_block_tests.rs"]
mod npc_victim_block_tests;
#[cfg(test)]
#[path = "combat_npc_victim_dodge_tests.rs"]
mod npc_victim_dodge_tests;
#[cfg(test)]
#[path = "combat_npc_victim_parry_tests.rs"]
mod npc_victim_parry_tests;
#[cfg(test)]
#[path = "combat_npc_weapon_crit_tests.rs"]
mod npc_weapon_crit_tests;
#[cfg(test)]
#[path = "combat_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "combat_victim_caster_crit_tests.rs"]
mod victim_caster_crit_tests;
#[cfg(test)]
#[path = "combat_victim_general_crit_tests.rs"]
mod victim_general_crit_tests;
#[cfg(test)]
#[path = "combat_victim_health_crit_tests.rs"]
mod victim_health_crit_tests;
#[cfg(test)]
#[path = "combat_victim_summoner_crit_tests.rs"]
mod victim_summoner_crit_tests;
