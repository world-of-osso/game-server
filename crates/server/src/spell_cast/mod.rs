//! Authoritative spell casting: validation, cast bars and channels, costs,
//! cooldowns/charges/GCD, implicit targets and effect resolution.
//!
//! Pipeline: `SpellCastIntent` → validate → start (instant: resolve now;
//! cast time: replicated `CastState`, resolve on completion; channel: resolve
//! when the cast time, if any, is over, then `CastState` ticks per aura
//! period, as TrinityCore `Spell::prepare` / `handle_immediate`) → spend costs,
//! start cooldown/charges → resolve effects. The GCD starts when the cast starts.
//! Game objects cast for their user without a spellbook check (`cast_granted`).
//! Cast time, duration, cooldowns, GCD, costs and range take the caster's
//! spell modifiers (`crate::auras::spell_mods`); aura effects resolve through
//! `crate::auras::apply_aura`.
//! See `docs/wiki/systems/spell-casting.md`.

pub mod area_aura;
pub mod creature;
pub mod effects;
mod missiles;
pub mod powers;
pub mod scripts;
pub mod targets;
#[cfg(test)]
pub(crate) mod test_harness;

use std::collections::HashMap;

use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::casting::{CastState, CastTickResult, CastType};
use shared::components::{Health, Npc, Player, PowerType};
use shared::protocol::{
    CastFailed, CombatChannel, CombatLogEvent, CombatLogKind, SpellCastIntent, SpellChargesUpdate,
    SpellCooldownUpdate, SpellGo, StopSpellCast,
};
use shared::spell_data::CastFailReason;

use crate::auras::procs::{self, AuraEvent};
use crate::auras::{AuraApplication, AuraSource, ChargeMods, SpellModOp, spell_mods};
use crate::auth::{Authenticated, InWorld};
use crate::class_progression::KnownSpells;
use crate::interest::NetworkVisibility;
use crate::networking::{CombatTarget, Moving, PlayerLink};
use crate::spell_info::{SpellInfo, SpellStore};
use crate::stats::CharacterStats;
pub(crate) use creature::{AiTarget, ai_spell_target, creature_cast};
use effects::{EffectCtx, UnhandledEffects};
use targets::{Dest, Relation};

/// `SpellInterruptFlags::Movement`.
const SPELL_INTERRUPT_MOVEMENT: u32 = 0x1;
/// `SpellAuraInterruptFlags::Moving | Turning`.
const CHANNEL_INTERRUPT_MOVING: u32 = 0x8 | 0x10;
/// SPELL_ATTR0_CANCELS_AUTO_ATTACK_COMBAT (Polymorph, Gouge, Blind, ...).
const SPELL_ATTR0_CANCELS_AUTO_ATTACK_COMBAT: u32 = 0x0010_0000;
/// SPELL_ATTR1_IS_CHANNELLED / SPELL_ATTR1_IS_SELF_CHANNELLED.
const SPELL_ATTR1_CHANNELED: u32 = 0x4 | 0x40;
/// `SpellInfo::HasInitialAggro` (SpellInfo.cpp:1925-1928): none of these.
const SPELL_ATTR1_NO_THREAT: u32 = 0x0000_0400;
const SPELL_ATTR2_NO_INITIAL_THREAT: u32 = 0x0040_0000;
const SPELL_ATTR4_NO_HARMFUL_THREAT: u32 = 0x0000_0010;
/// `SpellPower.PowerType` -2 costs health.
const POWER_HEALTH: i32 = -2;

pub struct SpellCastPlugin;

impl Plugin for SpellCastPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UnhandledEffects>()
            .init_resource::<ResolvingHits>()
            .init_resource::<missiles::PendingSpellHits>()
            .add_systems(Startup, powers::load_power_type_table_startup)
            .add_systems(
                Update,
                (receive_cast_requests, receive_stop_requests).chain(),
            )
            .add_systems(
                FixedUpdate,
                (
                    powers::init_player_powers,
                    powers::regenerate_powers,
                    advance_casts.after(crate::networking::process_player_inputs),
                    missiles::resolve_due_hits,
                    powers::sync_unit_powers,
                )
                    .chain(),
            )
            .add_systems(
                FixedUpdate,
                (
                    powers::regenerate_creature_mana,
                    powers::regenerate_creature_health,
                    area_aura::update_dynamic_objects.after(advance_casts),
                ),
            );
    }
}

/// Why a cast was refused, mapped to the contract's `CastFailReason`.
/// `NotSupported` carries the specific cause in `detail`.
#[derive(Debug, Clone, PartialEq)]
pub enum CastFailure {
    UnknownSpell,
    NotKnown,
    CasterDead,
    CrowdControlled(crate::auras::CastBlock),
    InProgress,
    SchoolLockedOut,
    OnGlobalCooldown,
    OnCooldown,
    NoCharges,
    Moving,
    NoTarget,
    InvalidTarget,
    OutOfRange,
    TooClose,
    NotEnoughPower(PowerType),
    NotEnoughHealth,
    /// Missing `SpellReagents` item (its name).
    Reagents(String),
    /// Missing `SpellTotems` tool (item or tool category name).
    Totems(String),
    /// A created item does not fit in the bags.
    InventoryFull,
    Unsupported(String),
}

impl CastFailure {
    fn to_contract(&self) -> (CastFailReason, Option<String>) {
        use CastFailReason as R;
        match self {
            Self::UnknownSpell => (R::NotSupported, Some("unknown spell".into())),
            Self::NotKnown => (R::NotSupported, Some("spell not known".into())),
            Self::CasterDead => (R::CasterDead, None),
            Self::CrowdControlled(block) => match block {
                crate::auras::CastBlock::Stunned => (R::Stunned, None),
                crate::auras::CastBlock::Silenced => (R::Silenced, None),
                crate::auras::CastBlock::Pacified => (R::Pacified, None),
                // SPELL_FAILED_CONFUSED has no CastFailReason in shared-protocol yet.
                crate::auras::CastBlock::Confused => {
                    (R::NotSupported, Some("Can't do that while confused".into()))
                }
            },
            Self::InProgress => (R::SpellInProgress, None),
            Self::SchoolLockedOut => (R::SchoolLockedOut, None),
            Self::OnGlobalCooldown => (R::OnGlobalCooldown, None),
            Self::OnCooldown => (R::OnCooldown, None),
            Self::NoCharges => (R::NoChargesRemain, None),
            Self::Moving => (R::CantCastWhileMoving, None),
            Self::NoTarget => (R::NoTarget, None),
            Self::InvalidTarget => (R::InvalidTarget, None),
            Self::OutOfRange => (R::OutOfRange, None),
            Self::TooClose => (R::TooClose, None),
            Self::NotEnoughPower(power) => (R::NotEnoughResource, Some(format!("{power:?}"))),
            Self::NotEnoughHealth => (R::NotEnoughResource, Some("Health".into())),
            // SPELL_FAILED_REAGENTS, SPELL_FAILED_TOTEM_CATEGORY, ERR_INV_FULL
            Self::Reagents(item) => (R::Reagents, Some(format!("Missing reagent: {item}"))),
            Self::Totems(tool) => (R::Totems, Some(format!("Requires {tool}"))),
            Self::InventoryFull => (R::InventoryFull, Some("Inventory is full.".into())),
            Self::Unsupported(detail) => (R::NotSupported, Some(detail.clone())),
        }
    }
}

/// Charge state of one charge category.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ChargeState {
    max: u8,
    used: u8,
    recharge_secs: f64,
    /// When the next used charge returns (valid while `used > 0`).
    next_ready_at: f64,
}

impl ChargeState {
    fn refresh(&mut self, now: f64) {
        while self.used > 0 && now >= self.next_ready_at {
            self.used -= 1;
            self.next_ready_at += self.recharge_secs;
        }
    }
}

/// Per-unit cooldown state in absolute server seconds (`Time` elapsed).
#[derive(Component, Debug, Clone, Default)]
pub struct SpellHistory {
    spell_ready_at: HashMap<u32, f64>,
    category_ready_at: HashMap<u32, f64>,
    /// By `StartRecoveryCategory`.
    gcd_ready_at: HashMap<u32, f64>,
    /// By `ChargeCategory`.
    charges: HashMap<u32, ChargeState>,
    /// By school bit index (0 Physical .. 6 Arcane).
    school_locked_until: [f64; 7],
}

impl SpellHistory {
    fn school_locked(&self, school_mask: u32, now: f64) -> bool {
        (0..7).any(|bit| school_mask & (1 << bit) != 0 && self.school_locked_until[bit] > now)
    }

    pub fn lock_schools(&mut self, school_mask: u32, until: f64) {
        for bit in 0..7 {
            if school_mask & (1 << bit) != 0 {
                let locked = &mut self.school_locked_until[bit];
                *locked = locked.max(until);
            }
        }
    }

    fn on_cooldown(&self, spell: &SpellInfo, now: f64) -> bool {
        let category = spell.categories.as_ref().map_or(0, |c| c.category);
        self.spell_ready_at
            .get(&spell.id)
            .is_some_and(|&at| at > now)
            || (category != 0
                && self
                    .category_ready_at
                    .get(&category)
                    .is_some_and(|&at| at > now))
    }

    fn on_gcd(&self, spell: &SpellInfo, now: f64) -> bool {
        let category = gcd_category(spell);
        category != 0 && self.gcd_ready_at.get(&category).is_some_and(|&at| at > now)
    }

    /// Charge state of the spell's charge category; max charges and recharge
    /// time follow the caster's current charge auras (`mods`).
    fn charges_mut(
        &mut self,
        spell: &SpellInfo,
        now: f64,
        mods: ChargeMods,
    ) -> Option<&mut ChargeState> {
        let categories = spell.categories.as_ref()?;
        if categories.charge_category == 0 || categories.max_charges == 0 {
            return None;
        }
        let max =
            (categories.max_charges as i32 + mods.extra_charges).clamp(0, u8::MAX as i32) as u8;
        let recharge_ms = ((categories.charge_recovery_ms as f32 + mods.recovery_flat_ms)
            * mods.recovery_mult)
            .max(0.0);
        let state = self
            .charges
            .entry(categories.charge_category)
            .or_insert(ChargeState {
                max,
                used: 0,
                recharge_secs: 0.0,
                next_ready_at: 0.0,
            });
        state.max = max;
        state.recharge_secs = f64::from(recharge_ms) / 1000.0;
        state.refresh(now);
        Some(state)
    }
}

fn charge_mods(world: &World, caster: Entity, spell: &SpellInfo) -> ChargeMods {
    let category = spell.categories.as_ref().map_or(0, |c| c.charge_category);
    crate::auras::charge_mods(world, caster, category)
}

fn gcd_category(spell: &SpellInfo) -> u32 {
    spell
        .categories
        .as_ref()
        .map_or(0, |categories| categories.start_recovery_category)
}

pub(crate) fn now(world: &World) -> f64 {
    world.resource::<Time>().elapsed_secs_f64()
}

fn attributes(spell: &SpellInfo, index: usize) -> u32 {
    spell.misc.as_ref().map_or(0, |misc| misc.attributes[index])
}

fn is_channeled(spell: &SpellInfo) -> bool {
    attributes(spell, 1) & SPELL_ATTR1_CHANNELED != 0
}

/// Moving leaves the cast bar alone (`Spell::CheckMovement`: no
/// `SpellInterruptFlags::Movement`).
fn cast_allows_moving(spell: &SpellInfo) -> bool {
    spell
        .interrupts
        .as_ref()
        .is_none_or(|interrupts| interrupts.interrupt_flags & SPELL_INTERRUPT_MOVEMENT == 0)
}

/// Moving leaves the channel alone (`SpellInfo::IsMoveAllowedChannel`).
fn channel_allows_moving(spell: &SpellInfo) -> bool {
    spell.interrupts.as_ref().is_none_or(|interrupts| {
        interrupts.channel_interrupt_flags[0] & CHANNEL_INTERRUPT_MOVING == 0
    })
}

fn modify(world: &World, caster: Entity, spell: &SpellInfo, op: SpellModOp, base: f32) -> f32 {
    spell_mods::modify(world, caster, spell, op, base)
}

/// Cast time after ChangeCastTime mods, divided by `1 + CharacterStats.haste_pct`
/// (haste auras fold in through `StatModifiers`).
fn cast_time_ms(world: &World, caster: Entity, spell: &SpellInfo) -> i32 {
    let base = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.cast_time)
        .map_or(0, |cast| cast.base_ms.max(0));
    if base == 0 {
        return 0;
    }
    let haste = world
        .get::<CharacterStats>(caster)
        .map_or(1.0, |stats| 1.0 + stats.haste_pct / 100.0);
    let modded = modify(
        world,
        caster,
        spell,
        SpellModOp::ChangeCastTime,
        base as f32,
    );
    let slow = world
        .get::<crate::auras::AuraStatMods>(caster)
        .map_or(1.0, |mods| mods.attack_cast_time_mult);
    (modded / haste.max(0.01) * slow).max(0.0).round() as i32
}

/// Channel duration after Duration mods.
pub(crate) fn duration_ms(world: &World, caster: Entity, spell: &SpellInfo) -> i32 {
    let base = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .map_or(0, |duration| duration.duration_ms);
    if base <= 0 {
        return base;
    }
    modify(world, caster, spell, SpellModOp::Duration, base as f32).round() as i32
}

/// Effects that have a handler or apply an aura; only these select targets.
fn handled_effects(spell: &SpellInfo) -> impl Iterator<Item = &crate::spell_info::SpellEffect> {
    spell
        .effects
        .iter()
        .filter(|effect| effects::is_handled(effect))
}

fn max_targets(spell: &SpellInfo) -> u32 {
    spell
        .target_restrictions
        .as_ref()
        .map_or(0, |restrictions| restrictions.max_targets)
}

// --- Messaging ---

fn owner_connection(world: &mut World, player: Entity) -> Option<Entity> {
    let mut connections = world.query_filtered::<(Entity, &PlayerLink), With<Authenticated>>();
    connections
        .iter(world)
        .find_map(|(connection, link)| (link.0 == player).then_some(connection))
}

pub(crate) fn send_to_owner<M: lightyear::prelude::Message + Clone>(
    world: &mut World,
    player: Entity,
    message: M,
) {
    let Some(connection) = owner_connection(world, player) else {
        return;
    };
    if let Some(mut sender) = world.get_mut::<MessageSender<M>>(connection) {
        sender.send::<CombatChannel>(message);
    }
}

/// Send to the owners of the event's source and target (once each); damage also counts
/// on the damage meters.
pub(crate) fn log_event(world: &mut World, event: CombatLogEvent) {
    crate::damage_meter::record_damage(world, &event);
    let mut units: Vec<Entity> = [event.source, event.target]
        .into_iter()
        .flatten()
        .filter_map(Entity::try_from_bits)
        .collect();
    units.dedup();
    for unit in units {
        send_to_owner(world, unit, event.clone());
    }
}

/// Send to every client that replicates `unit` (TrinityCore `SendMessageToSet`): the
/// connections that see it through interest, or every authenticated client for a unit
/// replicated without `NetworkVisibility`.
pub(crate) fn send_to_set<M: lightyear::prelude::Message + Clone>(
    world: &mut World,
    unit: Entity,
    message: M,
) {
    let receivers: Vec<Entity> = match world.get::<NetworkVisibility>(unit) {
        Some(visibility) => visibility.visible_senders().collect(),
        None => world
            .query_filtered::<Entity, With<Authenticated>>()
            .iter(world)
            .collect(),
    };
    for connection in receivers {
        if let Some(mut sender) = world.get_mut::<MessageSender<M>>(connection) {
            sender.send::<CombatChannel>(message.clone());
        }
    }
}

/// `SMSG_SPELL_GO` to every client that replicates the caster.
fn broadcast_spell_go(world: &mut World, message: SpellGo) {
    if let Some(caster) = Entity::try_from_bits(message.caster) {
        send_to_set(world, caster, message);
    }
}

/// The units a planned cast lands on, in plan order, each once.
fn hit_targets(plan: &TargetPlan) -> Vec<u64> {
    let mut hits = Vec::new();
    for unit in plan.iter().flat_map(|effect| &effect.units) {
        let bits = unit.to_bits();
        if !hits.contains(&bits) {
            hits.push(bits);
        }
    }
    hits
}

fn cast_event(
    caster: Entity,
    target: Option<Entity>,
    spell: &SpellInfo,
    kind: CombatLogKind,
) -> CombatLogEvent {
    CombatLogEvent {
        source: Some(caster.to_bits()),
        target: target.map(Entity::to_bits),
        spell_id: Some(spell.id),
        school_mask: spell.school_mask(),
        amount: 0,
        overflow: 0,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit: false,
        periodic: false,
        kind,
    }
}

fn mark_dirty(world: &World, entity: Entity) {
    if let Some(dirty) = world.get_resource::<ReplicationDirtyQueue>() {
        dirty.mark(entity);
    }
}

// --- Requests ---

pub(crate) fn receive_cast_requests(world: &mut World) {
    let mut receivers = world.query::<(
        &mut MessageReceiver<SpellCastIntent>,
        Option<&PlayerLink>,
        Has<Authenticated>,
        Has<InWorld>,
    )>();
    let requests: Vec<_> = receivers
        .iter_mut(world)
        .flat_map(|(mut receiver, link, auth, in_world)| {
            let owner = link.filter(|_| auth && in_world).map(|link| link.0);
            receiver
                .receive()
                .map(move |request| (owner, request))
                .collect::<Vec<_>>()
        })
        .collect();
    for (owner, request) in requests {
        let Some(owner) = owner else {
            warn!("Cast rejected: connection has no authenticated in-world owner");
            continue;
        };
        if let Err((spell_id, failure)) = request_cast(world, owner, &request) {
            debug!("Cast of {spell_id} by {owner} failed: {failure:?}");
            let (reason, detail) = failure.to_contract();
            send_to_owner(
                world,
                owner,
                CastFailed {
                    spell_id,
                    reason,
                    detail,
                },
            );
        }
    }
}

/// Cast a spell for a player as its own request would, answering `CastFailed`
/// on refusal (profession crafts).
pub(crate) fn cast_for_player(
    world: &mut World,
    player: Entity,
    spell_id: u32,
) -> Result<(), CastFailure> {
    let request = SpellCastIntent {
        spell_id: Some(spell_id),
        spell: String::new(),
        target_entity: None,
    };
    request_cast(world, player, &request).map_err(|(spell_id, failure)| {
        let (reason, detail) = failure.to_contract();
        send_to_owner(
            world,
            player,
            CastFailed {
                spell_id,
                reason,
                detail,
            },
        );
        failure
    })
}

/// Cast a spell a game object grants its user (TrinityCore `GameObject::Use`
/// casts as the user, spellbook aside), answering `CastFailed` on refusal.
pub(crate) fn cast_granted(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    target: Option<Entity>,
) -> Result<(), CastFailure> {
    let result = world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(spell_id))
        .cloned()
        .ok_or(CastFailure::UnknownSpell)
        .and_then(|spell| start_cast(world, caster, &spell, target));
    if let Err(failure) = &result {
        let (reason, detail) = failure.to_contract();
        send_to_owner(
            world,
            caster,
            CastFailed {
                spell_id,
                reason,
                detail,
            },
        );
    }
    result
}

fn request_cast(
    world: &mut World,
    caster: Entity,
    request: &SpellCastIntent,
) -> Result<(), (u32, CastFailure)> {
    let requested_id = request.spell_id.unwrap_or(0);
    let spell = world
        .get_resource::<SpellStore>()
        .and_then(|store| store.resolve(request.spell_id, &request.spell))
        .cloned()
        .ok_or((requested_id, CastFailure::UnknownSpell))?;
    if world.get::<Player>(caster).is_none() {
        return Err((
            spell.id,
            CastFailure::Unsupported("caster is not a player".into()),
        ));
    }
    if !caster_knows_spell(world, caster, spell.id) {
        return Err((spell.id, CastFailure::NotKnown));
    }
    let explicit = request
        .target_entity
        .and_then(Entity::try_from_bits)
        .filter(|&target| world.get_entity(target).is_ok())
        .or_else(|| {
            world
                .get::<CombatTarget>(caster)
                .and_then(|target| target.0)
        });
    start_cast(world, caster, &spell, explicit).map_err(|failure| (spell.id, failure))
}

/// Player requests are limited to known class and profession spells; NPC
/// casters are exempt.
fn caster_knows_spell(world: &mut World, caster: Entity, spell_id: u32) -> bool {
    if world.get::<Npc>(caster).is_some()
        || world
            .get::<crate::profession::PlayerProfessions>(caster)
            .is_some_and(|professions| professions.knows(spell_id))
    {
        return true;
    }
    let mut known = SystemState::<Query<&KnownSpells>>::new(world);
    let known = known
        .get(world)
        .expect("plain component query always validates");
    crate::class_progression::knows_spell(&known, caster, spell_id)
}

/// Validate everything checked before a cast starts; returns the explicit
/// target the spell will use.
fn validate_start(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> Result<Option<Entity>, CastFailure> {
    if !targets::is_alive(world, caster) {
        return Err(CastFailure::CasterDead);
    }
    if let Some(block) = crate::auras::cast_blocker(world, caster, spell) {
        return Err(CastFailure::CrowdControlled(block));
    }
    if world.get::<CastState>(caster).is_some() {
        return Err(CastFailure::InProgress);
    }
    validate_history(world, caster, spell)?;
    let blocked_by_moving = (cast_time_ms(world, caster, spell) > 0 && !cast_allows_moving(spell))
        || (is_channeled(spell) && !channel_allows_moving(spell));
    if blocked_by_moving && world.get::<Moving>(caster).is_some_and(|moving| moving.0) {
        return Err(CastFailure::Moving);
    }
    if let Some(id) = handled_effects(spell).find_map(targets::unsupported_target) {
        return Err(CastFailure::Unsupported(format!(
            "implicit target {id} not supported"
        )));
    }
    let target = validate_target(world, caster, spell, explicit)?;
    spell_costs(world, caster, spell)?;
    crate::profession::check_cast_items(world, caster, spell)?;
    Ok(target)
}

fn validate_history(world: &World, caster: Entity, spell: &SpellInfo) -> Result<(), CastFailure> {
    let Some(history) = world.get::<SpellHistory>(caster) else {
        return Ok(());
    };
    let now = now(world);
    if history.school_locked(spell.school_mask(), now) {
        return Err(CastFailure::SchoolLockedOut);
    }
    if history.on_gcd(spell, now) {
        return Err(CastFailure::OnGlobalCooldown);
    }
    if history.on_cooldown(spell, now) {
        return Err(CastFailure::OnCooldown);
    }
    let mods = charge_mods(world, caster, spell);
    let mut history = history.clone();
    if history
        .charges_mut(spell, now, mods)
        .is_some_and(|charges| charges.used >= charges.max)
    {
        return Err(CastFailure::NoCharges);
    }
    Ok(())
}

/// Explicit target rules: required relation, alive, range. Friendly spells
/// with no valid ally target fall back to the caster (retail auto self-cast).
fn validate_target(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> Result<Option<Entity>, CastFailure> {
    let Some(relation) = targets::explicit_requirement(handled_effects(spell)) else {
        return Ok(explicit);
    };
    let target = match explicit {
        Some(target) if targets::relation_matches(world, caster, target, relation) => target,
        _ if relation == Relation::Ally => caster,
        Some(_) => return Err(CastFailure::InvalidTarget),
        None => return Err(CastFailure::NoTarget),
    };
    // A unit of another map copy is not found (`ObjectAccessor::GetUnit` searches the
    // caster's map).
    if !targets::is_alive(world, target)
        || !targets::same_map(world, caster, target)
        || !targets::creature_type_allowed(world, spell, target)
    {
        return Err(CastFailure::InvalidTarget);
    }
    if target != caster {
        let (Some(from), Some(to)) = (
            targets::position(world, caster),
            targets::position(world, target),
        ) else {
            return Err(CastFailure::InvalidTarget);
        };
        let (min, max) = targets::spell_range(spell, targets::is_hostile(world, caster, target));
        let max = modify(world, caster, spell, SpellModOp::Range, max);
        let distance = from.distance(to);
        if distance > max {
            return Err(CastFailure::OutOfRange);
        }
        if distance < min {
            return Err(CastFailure::TooClose);
        }
    }
    Ok(Some(target))
}

/// What a `SpellPower` row draws from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum CostPool {
    Power(PowerType),
    Health,
}

/// Every applicable cost in raw units: flat `ManaCost` plus `PowerCostPct` of
/// the pool's base: base mana (`CharacterStats`) for mana, the max pool
/// otherwise (TrinityCore `Spell::CalcPowerCost`), with PowerCostN mods for
/// `OrderIndex` N. Rows with `RequiredAuraSpellID` apply only while the caster
/// has that aura.
pub(crate) fn spell_costs(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<Vec<(CostPool, f32)>, CastFailure> {
    let mut costs = Vec::new();
    for cost in spell.power_costs.iter().filter(|cost| {
        cost.required_aura_spell_id == 0
            || crate::auras::has_aura(world, caster, cost.required_aura_spell_id)
    }) {
        let pool = if cost.power_type == POWER_HEALTH {
            CostPool::Health
        } else {
            CostPool::Power(PowerType::from_db(cost.power_type).ok_or_else(|| {
                CastFailure::Unsupported(format!("power type {}", cost.power_type))
            })?)
        };
        let value = match pool {
            CostPool::Health => world
                .get::<Health>(caster)
                .map(|health| (health.current, health.max)),
            CostPool::Power(power) => powers::power_value(world, caster, power),
        };
        let percent = cost.power_cost_pct.max(0.0) / 100.0;
        let amount_with = |max: f32| cost.mana_cost.max(0) as f32 + percent * max;
        if amount_with(0.0) <= 0.0 && percent <= 0.0 {
            continue;
        }
        let missing = match pool {
            CostPool::Power(power) => CastFailure::NotEnoughPower(power),
            CostPool::Health => CastFailure::NotEnoughHealth,
        };
        let (current, max) = value.ok_or(missing.clone())?;
        // Only a percentage cost reads the base; creatures have no CharacterStats.
        let base = match pool {
            CostPool::Power(PowerType::Mana) if percent > 0.0 => world
                .get::<CharacterStats>(caster)
                .map(|stats| stats.base_mana)
                .ok_or(missing.clone())?,
            _ => max,
        };
        let amount = match SpellModOp::power_cost(cost.order_index) {
            Some(op) => modify(world, caster, spell, op, amount_with(base)).max(0.0),
            None => amount_with(base),
        };
        if current < amount {
            return Err(missing);
        }
        costs.push((pool, amount));
    }
    Ok(costs)
}

fn start_cast(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> Result<(), CastFailure> {
    let target = validate_start(world, caster, spell, explicit)?;
    start_gcd(world, caster, spell);
    let cast_ms = cast_time_ms(world, caster, spell);
    if cast_ms > 0 {
        let mut cast = CastState::normal(
            spell.id,
            target.map_or(0, Entity::to_bits),
            cast_ms as f32 / 1000.0,
            true,
        );
        insert_cast_state(world, caster, spell, &mut cast);
        log_event(
            world,
            cast_event(caster, target, spell, CombatLogKind::CastStart),
        );
        return Ok(());
    }
    execute_and_channel(world, caster, spell, target)
}

/// Resolve the spell; a channeled spell then channels for its duration.
fn execute_and_channel(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    target: Option<Entity>,
) -> Result<(), CastFailure> {
    execute_cast(world, caster, spell, target)?;
    let channel_ms = duration_ms(world, caster, spell);
    if !is_channeled(spell) || channel_ms <= 0 {
        return Ok(());
    }
    let tick_ms = spell
        .effects
        .iter()
        .map(|effect| effect.aura_period_ms)
        .filter(|&period| period > 0)
        .min()
        .unwrap_or(0);
    let mut cast = CastState::channel(
        spell.id,
        target.map_or(0, Entity::to_bits),
        channel_ms as f32 / 1000.0,
        tick_ms as f32 / 1000.0,
        true,
    );
    insert_cast_state(world, caster, spell, &mut cast);
    Ok(())
}

fn insert_cast_state(world: &mut World, caster: Entity, spell: &SpellInfo, cast: &mut CastState) {
    cast.spell_name = spell.name.clone();
    cast.cast_while_moving = match cast.cast_type {
        CastType::Normal => cast_allows_moving(spell),
        CastType::Channel => channel_allows_moving(spell),
    };
    world.entity_mut(caster).insert(cast.clone());
    mark_dirty(world, caster);
}

fn start_gcd(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // Spell::CanHaveGlobalCooldown: only players and their controlled units.
    if world.get::<Npc>(caster).is_some() {
        return;
    }
    let base_gcd_ms = spell
        .cooldowns
        .as_ref()
        .map_or(0, |cd| cd.start_recovery_ms);
    let gcd_ms = modify(
        world,
        caster,
        spell,
        SpellModOp::StartCooldown,
        base_gcd_ms as f32,
    )
    .max(0.0)
    .round() as u32;
    let category = gcd_category(spell);
    if gcd_ms == 0 || category == 0 {
        return;
    }
    let until = now(world) + gcd_ms as f64 / 1000.0;
    world
        .entity_mut(caster)
        .entry::<SpellHistory>()
        .or_default()
        .get_mut()
        .gcd_ready_at
        .insert(category, until);
    send_to_owner(
        world,
        caster,
        SpellCooldownUpdate {
            spell_id: spell.id,
            category,
            duration_ms: gcd_ms,
            remaining_ms: gcd_ms,
            is_gcd: true,
        },
    );
}

/// Recheck, plan targets, spend, start cooldowns, log CastSuccess, resolve.
fn execute_cast(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> Result<(), CastFailure> {
    if !targets::is_alive(world, caster) {
        return Err(CastFailure::CasterDead);
    }
    let explicit = validate_target(world, caster, spell, explicit)?;
    let costs = spell_costs(world, caster, spell)?;
    crate::profession::check_cast_items(world, caster, spell)?;
    let plan = plan_targets(world, caster, spell, explicit)?;
    spend_costs(world, caster, &costs);
    crate::profession::take_reagents(world, caster, spell);
    start_cooldowns(world, caster, spell);
    log_event(
        world,
        cast_event(caster, explicit, spell, CombatLogKind::CastSuccess),
    );
    broadcast_spell_go(
        world,
        SpellGo {
            caster: caster.to_bits(),
            target: explicit.map(Entity::to_bits),
            hit_targets: hit_targets(&plan),
            spell_id: spell.id,
        },
    );
    procs::push_event(
        world,
        AuraEvent {
            actor: caster,
            target: None,
            actor_mask: [0, procs::PROC2_CAST_SUCCESSFUL],
            target_mask: [0; 2],
            spell_id: Some(spell.id),
            can_proc: true,
            periodic: false,
            damage: 0.0,
        },
    );
    missiles::launch(world, caster, spell, plan, 0);
    spell_mods::consume_charges(world, caster, spell);
    // Spell::finish (Spell.cpp:4448-4450): some spells stop the caster's auto-attack.
    if attributes(spell, 0) & SPELL_ATTR0_CANCELS_AUTO_ATTACK_COMBAT != 0 {
        crate::combat::attack_stop(world, caster);
    }
    Ok(())
}

/// One handled effect's unit targets and destination.
#[derive(Debug)]
struct PlannedEffect {
    index: usize,
    units: Vec<Entity>,
    dest: Option<Dest>,
}

type TargetPlan = Vec<PlannedEffect>;

/// Targets of every handled effect, selected before any effect resolves. A
/// creature's melee-defense spell misses units by its miss chance
/// (`creature::roll_spell_misses`).
fn plan_targets(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> Result<TargetPlan, CastFailure> {
    let cap = max_targets(spell);
    let mut plan = Vec::new();
    for (index, effect) in spell.effects.iter().enumerate() {
        if !effects::is_handled(effect) {
            continue;
        }
        let units =
            targets::select_targets(world, caster, explicit, effect, cap).map_err(|id| {
                CastFailure::Unsupported(format!("implicit target {id} not supported"))
            })?;
        let dest = targets::effect_dest(world, caster, explicit, effect);
        plan.push(PlannedEffect { index, units, dest });
    }
    creature::roll_spell_misses(world, caster, spell, &mut plan);
    Ok(plan)
}

fn resolve_effects(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    plan: &TargetPlan,
    depth: u8,
) {
    for effect in &spell.effects {
        if !effects::is_handled(effect) {
            effects::log_unhandled(world, spell.id, effect.effect);
        }
    }
    if let Some(mut resolving) = world.get_resource_mut::<ResolvingHits>() {
        resolving.0.push(HitFrame {
            caster,
            spell_id: spell.id,
            damage: Vec::new(),
        });
    }
    // Aura effects hitting the same unit form one aura (per-unit effect mask).
    let mut aura_masks: Vec<(Entity, u32)> = Vec::new();
    for PlannedEffect { index, units, dest } in plan {
        let effect = &spell.effects[*index];
        let ctx = EffectCtx {
            caster,
            spell,
            depth,
            dest: *dest,
        };
        if effect.effect == effects::EFFECT_PERSISTENT_AREA_AURA {
            area_aura::create(world, &ctx, effect);
            continue;
        }
        if crate::auras::is_aura_effect(effect) {
            for &unit in units {
                match aura_masks.iter_mut().find(|(target, _)| *target == unit) {
                    Some((_, mask)) => *mask |= 1 << effect.index,
                    None => aura_masks.push((unit, 1 << effect.index)),
                }
            }
            continue;
        }
        if let Some(handler) = effects::cast_handler(effect.effect) {
            handler(world, &ctx, effect);
            continue;
        }
        let Some(handler) = effects::handler(effect.effect) else {
            continue;
        };
        for &unit in units {
            if world.get_entity(unit).is_ok() {
                handler(world, &ctx, effect, unit);
            }
        }
    }
    for (target, effect_mask) in aura_masks {
        crate::auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target,
                spell_id: spell.id,
                effect_mask,
                source: AuraSource::Cast,
            },
        );
    }
    run_hit_scripts(world, caster, spell, plan, depth);
    let damage = world
        .get_resource_mut::<ResolvingHits>()
        .and_then(|mut resolving| resolving.0.pop())
        .map_or_else(Vec::new, |frame| frame.damage);
    push_hit_events(world, caster, spell, plan, depth, &damage);
}

/// `Spell::PreprocessSpellLaunch` (Spell.cpp:8538-8541) puts the caster in combat with
/// each unit its spell launches at, and `TargetInfo::PreprocessTarget`
/// (Spell.cpp:2763-2764) the unit with the caster when it lands: a unit the caster can
/// attack (TC: not `IsFriendlyTo`), a harmful spell (or one that dispels), with initial
/// aggro or at an engaged unit. Both sides enter at launch (`missiles::launch`); TC
/// suppresses the target's side until the missile lands, which is not modelled.
pub(super) fn starts_combat(
    world: &World,
    caster: Entity,
    target: Entity,
    spell: &SpellInfo,
) -> bool {
    let initial_aggro = attributes(spell, 1) & SPELL_ATTR1_NO_THREAT == 0
        && attributes(spell, 2) & SPELL_ATTR2_NO_INITIAL_THREAT == 0
        && attributes(spell, 4) & SPELL_ATTR4_NO_HARMFUL_THREAT == 0;
    let engaged = world
        .get::<shared::components::CombatStatus>(target)
        .is_some_and(|status| status.0);
    let harmful = world
        .get_resource::<SpellStore>()
        .is_some_and(|store| crate::spell_info::positivity::negative_effects(store, spell) != 0)
        || spell
            .effects
            .iter()
            .any(|effect| effect.effect == effects::EFFECT_DISPEL);
    caster != target
        && targets::relation_matches(world, caster, target, Relation::Enemy)
        && harmful
        && (initial_aggro || engaged)
}

/// Distinct units of the plan, in plan order.
fn hit_units(plan: &TargetPlan) -> Vec<Entity> {
    let mut hit: Vec<Entity> = Vec::new();
    for unit in plan.iter().flat_map(|planned| &planned.units) {
        if !hit.contains(unit) {
            hit.push(*unit);
        }
    }
    hit
}

/// The spell's `OnHit` script on every unit it landed on (`Spell::DoSpellHitOnUnit`).
fn run_hit_scripts(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    plan: &TargetPlan,
    depth: u8,
) {
    let Some(hook) = scripts::on_hit(spell.id) else {
        return;
    };
    for unit in hit_units(plan) {
        if world.get_entity(unit).is_ok() {
            hook(world, caster, unit, depth);
        }
    }
}

/// Direct damage of the spells resolving now, gathered per unit for their hit events
/// (TrinityCore's hit proc carries the damage with TAKE_ANY_DAMAGE,
/// `Spell::TargetInfo::DoDamageAndTriggers`). One frame per resolution in progress,
/// innermost last: a triggered spell resolves inside its trigger's frame.
#[derive(Resource, Debug, Default)]
pub(crate) struct ResolvingHits(Vec<HitFrame>);

#[derive(Debug)]
struct HitFrame {
    caster: Entity,
    spell_id: u32,
    damage: Vec<(Entity, f32)>,
}

/// Add `amount` dealt to `target` to the innermost resolution of `spell_id` by
/// `caster`; false when that spell is not resolving (its damage gets its own event).
pub(crate) fn record_hit_damage(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    target: Entity,
    amount: f32,
) -> bool {
    let Some(mut resolving) = world.get_resource_mut::<ResolvingHits>() else {
        return false;
    };
    let Some(frame) = resolving
        .0
        .last_mut()
        .filter(|frame| frame.caster == caster && frame.spell_id == spell_id)
    else {
        return false;
    };
    match frame.damage.iter_mut().find(|(unit, _)| *unit == target) {
        Some((_, total)) => *total += amount,
        None => frame.damage.push((target, amount)),
    }
    true
}

/// One proc event per distinct unit the spell landed on, with the direct damage it
/// took (TAKE_ANY_DAMAGE when above 0); damage to a unit outside the plan gets a
/// damage-only event.
fn push_hit_events(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    plan: &TargetPlan,
    depth: u8,
    damage: &[(Entity, f32)],
) {
    let hit = hit_units(plan);
    let damage_to = |unit: Entity| {
        damage
            .iter()
            .find(|(target, _)| *target == unit)
            .map_or(0.0, |(_, amount)| *amount)
    };
    for &(target, amount) in damage.iter().filter(|(unit, _)| !hit.contains(unit)) {
        procs::push_event(
            world,
            AuraEvent {
                actor: caster,
                target: Some(target),
                actor_mask: [0; 2],
                target_mask: [procs::PROC_TAKE_ANY_DAMAGE, 0],
                spell_id: Some(spell.id),
                can_proc: depth == 0,
                periodic: false,
                damage: amount,
            },
        );
    }
    for target in hit {
        // Spell::DoSpellHitOnUnit (Spell.cpp:2879-2882): a spell landing on a
        // unit the caster can attack starts combat with it.
        if targets::relation_matches(world, caster, target, Relation::Enemy) {
            crate::creature_aggro::add_threat(world, target, caster, 0.0);
        }
        let harmful = targets::is_hostile(world, caster, target);
        let (actor_mask, mut target_mask) = procs::spell_hit_masks(spell, harmful);
        let damage = damage_to(target);
        if damage > 0.0 {
            target_mask |= procs::PROC_TAKE_ANY_DAMAGE;
        }
        procs::push_event(
            world,
            AuraEvent {
                actor: caster,
                target: Some(target),
                actor_mask: [actor_mask, 0],
                target_mask: [target_mask, 0],
                spell_id: Some(spell.id),
                can_proc: depth == 0,
                periodic: false,
                damage,
            },
        );
    }
}

/// `TRIGGER_SPELL`: resolve `spell_id` immediately at `explicit`, ignoring
/// costs, cooldowns, GCD and cast time. Depth is bounded by `MAX_TRIGGER_DEPTH`.
pub(crate) fn cast_triggered(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    explicit: Option<Entity>,
    depth: u8,
) {
    if depth > effects::MAX_TRIGGER_DEPTH {
        warn!(
            "Triggered spell {spell_id} skipped: depth {depth} exceeds {}",
            effects::MAX_TRIGGER_DEPTH
        );
        return;
    }
    let Some(spell) = world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(spell_id))
        .cloned()
    else {
        warn!("Triggered spell {spell_id} not found");
        return;
    };
    match plan_targets(world, caster, &spell, explicit) {
        Ok(plan) => missiles::launch(world, caster, &spell, plan, depth),
        Err(failure) => warn!("Triggered spell {spell_id} skipped: {failure:?}"),
    }
}

fn spend_costs(world: &mut World, caster: Entity, costs: &[(CostPool, f32)]) {
    for &(pool, amount) in costs {
        match pool {
            CostPool::Power(power) => {
                powers::change_power(world, caster, power, -amount);
            }
            CostPool::Health => {
                if let Some(mut health) = world.get_mut::<Health>(caster) {
                    health.current = (health.current - amount).max(1.0);
                }
            }
        }
    }
    mark_dirty(world, caster);
}

fn start_cooldowns(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let now = now(world);
    let (base_recovery_ms, base_category_ms) = spell
        .cooldowns
        .as_ref()
        .map_or((0, 0), |cd| (cd.recovery_ms, cd.category_recovery_ms));
    let cooldown = |base: u32| {
        if base == 0 {
            return 0;
        }
        modify(world, caster, spell, SpellModOp::Cooldown, base as f32)
            .max(0.0)
            .round() as u32
    };
    let (recovery_ms, category_ms) = (cooldown(base_recovery_ms), cooldown(base_category_ms));
    let category = spell.categories.as_ref().map_or(0, |c| c.category);
    let mods = charge_mods(world, caster, spell);
    let mut entity = world.entity_mut(caster);
    let mut history = entity.entry::<SpellHistory>().or_default();
    let mut history = history.get_mut();
    if recovery_ms > 0 {
        history
            .spell_ready_at
            .insert(spell.id, now + recovery_ms as f64 / 1000.0);
    }
    if category != 0 && category_ms > 0 {
        history
            .category_ready_at
            .insert(category, now + category_ms as f64 / 1000.0);
    }
    let charges = history.charges_mut(spell, now, mods).map(|charges| {
        if charges.used == 0 {
            charges.next_ready_at = now + charges.recharge_secs;
        }
        charges.used = (charges.used + 1).min(charges.max);
        *charges
    });
    let cooldown_ms = recovery_ms.max(if category != 0 { category_ms } else { 0 });
    if cooldown_ms > 0 {
        send_to_owner(
            world,
            caster,
            SpellCooldownUpdate {
                spell_id: spell.id,
                category: if category_ms > 0 { category } else { 0 },
                duration_ms: cooldown_ms,
                remaining_ms: cooldown_ms,
                is_gcd: false,
            },
        );
    }
    if let Some(charges) = charges {
        let remaining = ((charges.next_ready_at - now).max(0.0) * 1000.0) as u32;
        send_to_owner(
            world,
            caster,
            SpellChargesUpdate {
                spell_id: spell.id,
                current: charges.max - charges.used,
                max: charges.max,
                recharge_ms: (charges.recharge_secs * 1000.0) as u32,
                remaining_ms: remaining,
            },
        );
    }
}

// --- Cast progress ---

/// Remove an active cast (stop, movement, death, interrupt).
pub(crate) fn cancel_cast(world: &mut World, caster: Entity) {
    world
        .entity_mut(caster)
        .remove::<crate::profession::CraftQueue>();
    let Some(cast) = world.entity_mut(caster).take::<CastState>() else {
        return;
    };
    mark_dirty(world, caster);
    if cast.cast_type == CastType::Channel {
        crate::auras::channel_ended(world, caster, cast.spell_id);
        crate::meeting_stone::ritual::channel_ended(world, caster, cast.spell_id);
    }
}

pub(crate) fn receive_stop_requests(world: &mut World) {
    let mut receivers = world.query::<(
        &mut MessageReceiver<StopSpellCast>,
        Option<&PlayerLink>,
        Has<Authenticated>,
        Has<InWorld>,
    )>();
    let owners: Vec<_> = receivers
        .iter_mut(world)
        .filter_map(|(mut receiver, link, auth, in_world)| {
            let requested = receiver.receive().count() > 0;
            if requested && (!auth || !in_world || link.is_none()) {
                warn!("Stop cast rejected: no authenticated in-world owner");
            }
            link.filter(|_| requested && auth && in_world)
                .map(|link| link.0)
        })
        .collect();
    for owner in owners {
        cancel_cast(world, owner);
    }
}

pub(crate) fn advance_casts(world: &mut World) {
    let dt = world.resource::<Time>().delta_secs();
    let mut casts = world.query::<(Entity, &CastState)>();
    let casters: Vec<Entity> = casts.iter(world).map(|(entity, _)| entity).collect();
    for caster in casters {
        let moving = world.get::<Moving>(caster).is_some_and(|moving| moving.0);
        let Some(cast_while_moving) = world
            .get::<CastState>(caster)
            .map(|cast| cast.cast_while_moving)
        else {
            continue;
        };
        if (!cast_while_moving && moving) || !targets::is_alive(world, caster) {
            cancel_cast(world, caster);
            continue;
        }
        let Some(mut cast) = world.get_mut::<CastState>(caster) else {
            continue;
        };
        let result = cast.tick(dt);
        let spell_id = cast.spell_id;
        let target = (cast.target != 0)
            .then(|| Entity::try_from_bits(cast.target))
            .flatten();
        mark_dirty(world, caster);
        match result {
            // Channel periodic effects tick as auras (`crate::auras`).
            CastTickResult::InProgress | CastTickResult::ChannelTick => {}
            CastTickResult::ChannelComplete => cancel_cast(world, caster),
            CastTickResult::Completed => {
                world.entity_mut(caster).remove::<CastState>();
                complete_cast(world, caster, spell_id, target);
            }
        }
    }
}

fn complete_cast(world: &mut World, caster: Entity, spell_id: u32, target: Option<Entity>) {
    let Some(spell) = world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(spell_id))
        .cloned()
    else {
        return;
    };
    if let Err(failure) = execute_and_channel(world, caster, &spell, target) {
        debug!("Cast of {spell_id} by {caster} failed on completion: {failure:?}");
        let (reason, detail) = failure.to_contract();
        send_to_owner(
            world,
            caster,
            CastFailed {
                spell_id,
                reason,
                detail,
            },
        );
    }
}

#[cfg(test)]
mod tests;
