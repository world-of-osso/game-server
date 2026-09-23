//! Authoritative spell casting: validation, cast bars and channels, costs,
//! cooldowns/charges/GCD, implicit targets and effect resolution.
//!
//! Pipeline: `SpellCastIntent` → validate → start (instant: resolve now;
//! cast time: replicated `CastState`, resolve on completion; channel: resolve
//! at start, `CastState` ticks per aura period) → spend costs, start
//! cooldown/charges → resolve effects. The GCD starts when the cast starts.
//! See `docs/wiki/systems/spell-casting.md`.

pub mod effects;
pub mod powers;
pub mod targets;

use std::collections::HashMap;

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::casting::{CastState, CastTickResult, CastType};
use shared::components::{Health, Player, PowerType};
use shared::protocol::{
    CastFailed, CombatChannel, CombatLogEvent, CombatLogKind, SpellCastIntent, SpellChargesUpdate,
    SpellCooldownUpdate, StopSpellCast,
};
use shared::spell_data::CastFailReason;

use crate::auth::{Authenticated, InWorld};
use crate::networking::{CombatTarget, Moving, PlayerLink};
use crate::spell_info::{SpellInfo, SpellStore};
use effects::{AuraApplierHook, EffectCtx, UnhandledEffects, with_aura_hook};
use targets::Relation;

const SPELL_ATTR0_CASTABLE_WHILE_MOVING: u32 = 0x1000_0000;
/// SPELL_ATTR1_IS_CHANNELLED / SPELL_ATTR1_IS_SELF_CHANNELLED.
const SPELL_ATTR1_CHANNELED: u32 = 0x4 | 0x40;
/// `SpellPower.PowerType` -2 costs health.
const POWER_HEALTH: i32 = -2;

pub struct SpellCastPlugin;

impl Plugin for SpellCastPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UnhandledEffects>()
            .init_resource::<AuraApplierHook>()
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
                    powers::sync_unit_powers,
                    powers::sync_unit_level,
                )
                    .chain(),
            );
    }
}

/// Why a cast was refused. Mapped to the contract's `CastFailReason`; cases
/// without a contract variant use the nearest one plus `detail` (contract
/// gap reported: TooClose, SchoolLockedOut, SpellInProgress, NoCharges,
/// UnknownSpell/Unsupported).
#[derive(Debug, Clone, PartialEq)]
pub enum CastFailure {
    UnknownSpell,
    CasterDead,
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
    Unsupported(String),
}

impl CastFailure {
    fn to_contract(&self) -> (CastFailReason, Option<String>) {
        use CastFailReason as R;
        match self {
            Self::UnknownSpell => (R::InvalidTarget, Some("unknown spell".into())),
            Self::CasterDead => (R::CasterDead, None),
            Self::InProgress => (R::InvalidTarget, Some("another cast is in progress".into())),
            Self::SchoolLockedOut => (R::OnCooldown, Some("spell school locked out".into())),
            Self::OnGlobalCooldown => (R::OnGlobalCooldown, None),
            Self::OnCooldown => (R::OnCooldown, None),
            Self::NoCharges => (R::OnCooldown, Some("no charges".into())),
            Self::Moving => (R::CantCastWhileMoving, None),
            Self::NoTarget => (R::NoTarget, None),
            Self::InvalidTarget => (R::InvalidTarget, None),
            Self::OutOfRange => (R::OutOfRange, None),
            Self::TooClose => (R::OutOfRange, Some("too close".into())),
            Self::NotEnoughPower(power) => (R::NotEnoughResource, Some(format!("{power:?}"))),
            Self::NotEnoughHealth => (R::NotEnoughResource, Some("Health".into())),
            Self::Unsupported(detail) => (R::InvalidTarget, Some(detail.clone())),
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

    fn charges_mut(&mut self, spell: &SpellInfo, now: f64) -> Option<&mut ChargeState> {
        let categories = spell.categories.as_ref()?;
        if categories.charge_category == 0 || categories.max_charges == 0 {
            return None;
        }
        let state = self
            .charges
            .entry(categories.charge_category)
            .or_insert(ChargeState {
                max: categories.max_charges.min(u8::MAX as u32) as u8,
                used: 0,
                recharge_secs: categories.charge_recovery_ms as f64 / 1000.0,
                next_ready_at: 0.0,
            });
        state.refresh(now);
        Some(state)
    }
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

fn cast_time_ms(spell: &SpellInfo) -> i32 {
    spell
        .misc
        .as_ref()
        .and_then(|misc| misc.cast_time)
        .map_or(0, |cast| cast.base_ms.max(0))
}

fn duration_ms(spell: &SpellInfo) -> i32 {
    spell
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .map_or(0, |duration| duration.duration_ms)
}

/// Effects that have a handler; only these select targets.
fn handled_effects(spell: &SpellInfo) -> impl Iterator<Item = &crate::spell_info::SpellEffect> {
    spell
        .effects
        .iter()
        .filter(|effect| effects::handler(effect.effect).is_some())
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

fn send_to_owner<M: lightyear::prelude::Message + Clone>(
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

/// Send to the owners of the event's source and target (once each).
pub(crate) fn log_event(world: &mut World, event: CombatLogEvent) {
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
    if world.get::<CastState>(caster).is_some() {
        return Err(CastFailure::InProgress);
    }
    validate_history(world, caster, spell)?;
    let has_cast_bar = cast_time_ms(spell) > 0 || is_channeled(spell);
    let may_move = attributes(spell, 0) & SPELL_ATTR0_CASTABLE_WHILE_MOVING != 0;
    if has_cast_bar && !may_move && world.get::<Moving>(caster).is_some_and(|moving| moving.0) {
        return Err(CastFailure::Moving);
    }
    if let Some(id) = handled_effects(spell).find_map(targets::unsupported_target) {
        return Err(CastFailure::Unsupported(format!(
            "implicit target {id} not supported"
        )));
    }
    let target = validate_target(world, caster, spell, explicit)?;
    spell_costs(world, caster, spell)?;
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
    let mut history = history.clone();
    if history
        .charges_mut(spell, now)
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
    if !targets::is_alive(world, target) {
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
enum CostPool {
    Power(PowerType),
    Health,
}

/// Every non-aura-gated cost in raw units: flat `ManaCost` plus
/// `PowerCostPct` of the caster's max pool (base pools are not modeled).
fn spell_costs(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<Vec<(CostPool, f32)>, CastFailure> {
    let mut costs = Vec::new();
    for cost in spell
        .power_costs
        .iter()
        .filter(|cost| cost.required_aura_spell_id == 0)
    {
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
        let amount = amount_with(max);
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
    let cast_ms = cast_time_ms(spell);
    let channel_ms = duration_ms(spell);
    if is_channeled(spell) && channel_ms > 0 {
        execute_cast(world, caster, spell, target)?;
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
        return Ok(());
    }
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
    execute_cast(world, caster, spell, target)
}

fn insert_cast_state(world: &mut World, caster: Entity, spell: &SpellInfo, cast: &mut CastState) {
    cast.spell_name = spell.name.clone();
    cast.cast_while_moving = attributes(spell, 0) & SPELL_ATTR0_CASTABLE_WHILE_MOVING != 0;
    world.entity_mut(caster).insert(cast.clone());
    mark_dirty(world, caster);
}

fn start_gcd(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let gcd_ms = spell
        .cooldowns
        .as_ref()
        .map_or(0, |cd| cd.start_recovery_ms);
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
    let plan = plan_targets(world, caster, spell, explicit)?;
    spend_costs(world, caster, &costs);
    start_cooldowns(world, caster, spell);
    log_event(
        world,
        cast_event(caster, explicit, spell, CombatLogKind::CastSuccess),
    );
    resolve_effects(world, caster, spell, &plan, 0);
    Ok(())
}

type TargetPlan = Vec<(usize, Vec<Entity>)>;

/// Targets of every handled effect, selected before any effect resolves.
fn plan_targets(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> Result<TargetPlan, CastFailure> {
    let cap = max_targets(spell);
    let mut plan = Vec::new();
    for (index, effect) in spell.effects.iter().enumerate() {
        if effects::handler(effect.effect).is_none() {
            continue;
        }
        let units =
            targets::select_targets(world, caster, explicit, effect, cap).map_err(|id| {
                CastFailure::Unsupported(format!("implicit target {id} not supported"))
            })?;
        plan.push((index, units));
    }
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
        if effects::handler(effect.effect).is_none() {
            effects::log_unhandled(world, spell.id, effect.effect);
        }
    }
    let ctx = EffectCtx {
        caster,
        spell,
        depth,
    };
    for (index, units) in plan {
        let effect = &spell.effects[*index];
        let Some(handler) = effects::handler(effect.effect) else {
            continue;
        };
        for &unit in units {
            if world.get_entity(unit).is_ok() {
                handler(world, &ctx, effect, unit);
            }
        }
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
        Ok(plan) => resolve_effects(world, caster, &spell, &plan, depth),
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
    let (recovery_ms, category_ms) = spell
        .cooldowns
        .as_ref()
        .map_or((0, 0), |cd| (cd.recovery_ms, cd.category_recovery_ms));
    let category = spell.categories.as_ref().map_or(0, |c| c.category);
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
    let charges = history.charges_mut(spell, now).map(|charges| {
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
    let Some(cast) = world.entity_mut(caster).take::<CastState>() else {
        return;
    };
    mark_dirty(world, caster);
    if cast.cast_type == CastType::Channel {
        with_aura_hook(world, |hook, world| {
            hook.channel_ended(world, caster, cast.spell_id)
        });
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
            CastTickResult::InProgress => {}
            CastTickResult::ChannelTick => {
                with_aura_hook(world, |hook, world| {
                    hook.channel_tick(world, caster, spell_id)
                });
            }
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
    if let Err(failure) = execute_cast(world, caster, &spell, target) {
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
