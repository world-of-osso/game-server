//! Aura runtime: aura instances on units, stacking/refresh, expiry and
//! removal, periodic ticks, procs, spell modifiers, stat modifiers and
//! `UnitAuras` replication. See `docs/wiki/systems/auras.md`.
//!
//! Stacking rule (TrinityCore `Aura::CanStackWith`, simplified):
//! - same spell and caster: refresh duration, +1 stack up to `CumulativeAura`
//!   (`SPELL_ATTR1_AURA_UNIQUE` auras are not refreshed);
//! - same spell, other caster: coexist when the aura is harmful, has a
//!   periodic effect or `SPELL_ATTR3_DOT_STACKING_RULE`; otherwise the new
//!   aura replaces the old one.
//!
//! Harmful: `SPELL_ATTR0_AURA_IS_DEBUFF`, else hostile caster (without
//! `SPELL_ATTR4_AURA_IS_BUFF`). Hidden: passive or
//! `SPELL_ATTR0_DO_NOT_DISPLAY_SPELLBOOK_AURA_ICON_COMBAT_LOG`.

mod handlers;
mod periodic;
pub mod procs;
pub mod scripts;
pub mod spell_mods;
pub mod stat_mods;
mod unit_state;

use std::collections::HashSet;

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{AuraView, Npc, Player, UnitAuras};
use shared::protocol::{CancelAura, CombatLogEvent, CombatLogKind};

use crate::auth::{Authenticated, InWorld};
use crate::class_progression::KnownSpells;
use crate::grid_activity::GridDormant;
use crate::networking::PlayerLink;
use crate::spell_cast::{log_event, targets};
use crate::spell_info::{SpellEffect, SpellInfo, SpellStore};
use crate::stats::StatModifiers;
pub use spell_mods::SpellModOp;
pub use stat_mods::AuraStatMods;
pub use unit_state::Invisible;

/// SPELL_AURA_PERIODIC_DAMAGE.
pub const PERIODIC_DAMAGE_AURA: u32 = handlers::PERIODIC_DAMAGE;

/// SpellEffects IDs that apply an aura (`SpellEffect.aura` is the AuraType).
/// 35 (party) and 65 (raid) area auras apply to their resolved targets only:
/// there is no group backend. 27 PERSISTENT_AREA_AURA applies through its
/// dynamic object (`spell_cast::area_aura`).
pub const AURA_EFFECTS: [u32; 4] = [6, 27, 35, 65];

const ATTR0_PASSIVE: u32 = 0x40;
const ATTR0_DO_NOT_DISPLAY: u32 = 0x80;
const ATTR0_AURA_IS_DEBUFF: u32 = 0x0400_0000;
const ATTR0_NO_AURA_CANCEL: u32 = 0x8000_0000;
const ATTR1_AURA_UNIQUE: u32 = 0x800;
const ATTR3_DOT_STACKING_RULE: u32 = 0x80;
const ATTR3_ALLOW_AURA_WHILE_DEAD: u32 = 0x0010_0000;
const ATTR4_AURA_IS_BUFF: u32 = 0x1000;

/// `DispelType` 7 matches every dispel type.
const DISPEL_ALL: u32 = 7;

pub struct AuraPlugin;

impl Plugin for AuraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<procs::PendingAuraEvents>()
            .init_resource::<UnhandledAuras>()
            .init_resource::<scripts::AuraScripts>()
            .init_resource::<PendingPassiveSync>()
            .add_systems(
                Update,
                (
                    receive_cancel_aura.after(crate::spell_cast::receive_cast_requests),
                    queue_passive_sync,
                ),
            )
            .add_systems(
                FixedUpdate,
                (
                    init_unit_auras,
                    sync_passive_auras,
                    update_auras,
                    procs::process_aura_events,
                    sync_unit_auras,
                )
                    .chain()
                    .after(crate::spell_cast::advance_casts)
                    .after(crate::combat::auto_attack_system),
            );
    }
}

/// Seconds of virtual time; aura timers use it in every schedule.
pub fn aura_now(world: &World) -> f64 {
    world.resource::<Time<Virtual>>().elapsed_secs_f64()
}

/// Who put an aura on the unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuraSource {
    /// Applied by a resolving spell effect.
    Cast,
    /// Passive spell in the unit's `KnownSpells` (talent passives apply
    /// once the trait system adds talent spells to `KnownSpells`).
    KnownSpell,
}

/// One aura application request: the aura effects of `spell_id` that hit
/// `target` in one resolution (`effect_mask` bit = `EffectIndex`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuraApplication {
    pub caster: Entity,
    pub target: Entity,
    pub spell_id: u32,
    pub effect_mask: u32,
    pub source: AuraSource,
}

/// One aura effect of an instance.
#[derive(Debug, Clone, PartialEq)]
pub struct AuraEffectState {
    pub index: u8,
    pub aura_type: u32,
    /// Per-stack amount computed at application: `BasePoints` with the
    /// caster's Points mods, plus SP/AP coefficients for damage, heal and
    /// absorb types (`handlers::scales_with_power`).
    pub amount: f32,
    pub misc_values: [i32; 2],
    pub period_ms: u32,
    pub trigger_spell: u32,
    pub class_mask: [u32; 4],
    next_tick_at: f64,
    /// SCHOOL_ABSORB points left (whole aura, not per stack).
    absorb_remaining: f32,
}

/// Proc timers (TrinityCore `Aura` proc fields).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProcTimers {
    next_ready_at: f64,
    last_attempt_at: f64,
    last_success_at: f64,
}

/// One aura on a unit.
#[derive(Debug, Clone, PartialEq)]
pub struct AuraInstance {
    pub instance_id: u32,
    pub spell_id: u32,
    pub caster: Entity,
    pub source: AuraSource,
    pub from_player: bool,
    pub effects: Vec<AuraEffectState>,
    /// Full duration in ms; 0 is permanent.
    pub duration_ms: u32,
    pub expires_at: Option<f64>,
    pub stacks: u8,
    /// `CumulativeAura` (+ MaxAuraStacks mods); 0 does not stack.
    pub max_stacks: u8,
    /// Proc charges left; 0 has no charges.
    pub charges: u8,
    pub passive: bool,
    pub hidden: bool,
    pub harmful: bool,
    pub dispel_type: u8,
    /// `SpellInterrupts.AuraInterruptFlags_0/_1`.
    pub interrupt_flags: [u32; 2],
    /// `SpellClassOptions.SpellClassSet` of the aura spell.
    pub class_set: u32,
    proc: ProcTimers,
}

impl AuraInstance {
    fn remaining_ms(&self, now: f64) -> u32 {
        self.expires_at
            .map_or(0, |at| ((at - now).max(0.0) * 1000.0).round() as u32)
    }

    fn stack_multiplier(&self) -> f32 {
        f32::from(self.stacks.max(1))
    }

    fn view(&self, now: f64) -> AuraView {
        let mut flags = 0;
        if self.passive {
            flags |= AuraView::FLAG_PASSIVE;
        }
        if self.hidden {
            flags |= AuraView::FLAG_HIDDEN;
        }
        if self.from_player {
            flags |= AuraView::FLAG_FROM_PLAYER;
        }
        AuraView {
            instance_id: self.instance_id,
            spell_id: self.spell_id,
            caster: Some(self.caster.to_bits()),
            stacks: self.stacks,
            charges: self.charges,
            duration_ms: self.duration_ms,
            remaining_ms: self.remaining_ms(now),
            harmful: self.harmful,
            dispel_type: self.dispel_type,
            flags,
        }
    }

    /// Take over a reapplication's duration, amounts and charges; add a stack.
    fn refresh_from(&mut self, new: AuraInstance) {
        self.duration_ms = new.duration_ms;
        self.expires_at = new.expires_at;
        self.max_stacks = new.max_stacks;
        if self.max_stacks > 0 {
            self.stacks = (self.stacks + 1).min(self.max_stacks);
        }
        self.charges = new.charges;
        for fresh in new.effects {
            match self
                .effects
                .iter_mut()
                .find(|effect| effect.index == fresh.index)
            {
                Some(effect) => {
                    effect.amount = fresh.amount;
                    effect.absorb_remaining = fresh.absorb_remaining;
                }
                None => self.effects.push(fresh),
            }
        }
    }
}

/// Server-side auras of a unit. `UnitAuras` mirrors it on change.
#[derive(Component, Debug, Default)]
pub struct ActiveAuras {
    auras: Vec<AuraInstance>,
    next_instance_id: u32,
    /// The replicated view is stale.
    dirty: bool,
    /// Last seen `CombatStatus`, for EnteringCombat/LeavingCombat interrupts.
    in_combat: bool,
    /// Last seen `ActiveSpec`, for the ChangeSpec interrupt.
    spec: Option<u32>,
}

/// What reapplying did.
#[derive(Debug, Clone, PartialEq)]
enum Stacking {
    Applied(u32),
    Refreshed(u32),
    Replaced { id: u32, removed: AuraInstance },
    Ignored,
}

impl ActiveAuras {
    pub fn iter(&self) -> impl Iterator<Item = &AuraInstance> {
        self.auras.iter()
    }

    /// Under a `SPELL_AURA_MOD_STUN` effect (AzerothCore `UNIT_STATE_STUNNED`).
    pub fn is_stunned(&self) -> bool {
        self.auras
            .iter()
            .flat_map(|aura| &aura.effects)
            .any(|effect| effect.aura_type == handlers::MOD_STUN)
    }

    pub fn has_spell(&self, spell_id: u32) -> bool {
        self.auras.iter().any(|aura| aura.spell_id == spell_id)
    }

    pub fn get(&self, instance_id: u32) -> Option<&AuraInstance> {
        self.auras
            .iter()
            .find(|aura| aura.instance_id == instance_id)
    }

    fn get_mut(&mut self, instance_id: u32) -> Option<&mut AuraInstance> {
        self.auras
            .iter_mut()
            .find(|aura| aura.instance_id == instance_id)
    }

    fn push(&mut self, mut aura: AuraInstance) -> u32 {
        self.next_instance_id += 1;
        aura.instance_id = self.next_instance_id;
        self.auras.push(aura);
        self.next_instance_id
    }

    /// Apply the stacking rule (module docs) to a new instance.
    fn stack_or_add(&mut self, new: AuraInstance, coexist: bool, unique: bool) -> Stacking {
        let same_caster = |aura: &AuraInstance| {
            aura.spell_id == new.spell_id && aura.caster == new.caster && aura.source == new.source
        };
        if let Some(existing) = self.auras.iter_mut().find(|aura| same_caster(aura)) {
            if unique {
                return Stacking::Ignored;
            }
            let id = existing.instance_id;
            existing.refresh_from(new);
            return Stacking::Refreshed(id);
        }
        if !coexist
            && let Some(position) = self
                .auras
                .iter()
                .position(|aura| aura.spell_id == new.spell_id)
        {
            let removed = self.auras.remove(position);
            let id = self.push(new);
            return Stacking::Replaced { id, removed };
        }
        Stacking::Applied(self.push(new))
    }

    fn take_where(&mut self, remove: impl Fn(&AuraInstance) -> bool) -> Vec<AuraInstance> {
        let (removed, kept) = std::mem::take(&mut self.auras)
            .into_iter()
            .partition(|aura| remove(aura));
        self.auras = kept;
        removed
    }
}

/// `(spell id, aura type)` pairs already logged as unhandled.
#[derive(Resource, Debug, Default)]
pub struct UnhandledAuras(HashSet<(u32, u32)>);

fn attributes(spell: &SpellInfo, index: usize) -> u32 {
    spell.misc.as_ref().map_or(0, |misc| misc.attributes[index])
}

pub fn is_passive(spell: &SpellInfo) -> bool {
    attributes(spell, 0) & ATTR0_PASSIVE != 0
}

pub fn is_aura_effect(effect: &SpellEffect) -> bool {
    AURA_EFFECTS.contains(&effect.effect)
}

/// Every aura effect index of `spell` as an `AuraApplication` mask.
pub fn aura_effect_mask(spell: &SpellInfo) -> u32 {
    spell
        .effects
        .iter()
        .filter(|effect| is_aura_effect(effect))
        .fold(0, |mask, effect| mask | 1 << effect.index)
}

fn lookup_spell(world: &World, spell_id: u32) -> Option<SpellInfo> {
    world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(spell_id))
        .cloned()
}

fn is_harmful(world: &World, spell: &SpellInfo, caster: Entity, target: Entity) -> bool {
    if attributes(spell, 0) & ATTR0_AURA_IS_DEBUFF != 0 {
        return true;
    }
    attributes(spell, 4) & ATTR4_AURA_IS_BUFF == 0 && targets::is_hostile(world, caster, target)
}

fn duration_ms(world: &World, caster: Entity, spell: &SpellInfo, passive: bool) -> u32 {
    if passive {
        return 0;
    }
    let base = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .map_or(0, |duration| duration.duration_ms);
    if base <= 0 {
        return 0;
    }
    spell_mods::modify(world, caster, spell, SpellModOp::Duration, base as f32).max(0.0) as u32
}

fn build_effect(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
    now: f64,
) -> AuraEffectState {
    let amount = if handlers::scales_with_power(effect.aura) {
        crate::spell_cast::effects::scaled_amount(world, caster, spell, effect)
    } else {
        crate::spell_cast::effects::modded_base_points(world, caster, spell, effect)
    };
    let period_ms = if effect.aura_period_ms > 0 {
        spell_mods::modify(
            world,
            caster,
            spell,
            SpellModOp::Period,
            effect.aura_period_ms as f32,
        )
        .max(1.0) as u32
    } else {
        0
    };
    AuraEffectState {
        index: effect.index,
        aura_type: effect.aura,
        amount,
        misc_values: effect.misc_values,
        period_ms,
        trigger_spell: effect.trigger_spell,
        class_mask: effect.spell_class_mask,
        next_tick_at: now + f64::from(period_ms) / 1000.0,
        absorb_remaining: amount.max(0.0),
    }
}

fn build_instance(
    world: &World,
    application: &AuraApplication,
    spell: &SpellInfo,
    now: f64,
) -> AuraInstance {
    let caster = application.caster;
    let passive = is_passive(spell);
    let duration_ms = duration_ms(world, caster, spell, passive);
    let options = spell.aura_options.as_deref();
    let max_stacks = options.map_or(0.0, |options| options.cumulative_aura as f32);
    let max_stacks =
        spell_mods::modify(world, caster, spell, SpellModOp::MaxAuraStacks, max_stacks);
    let charges = options.map_or(0.0, |options| options.proc_charges.max(0) as f32);
    let charges = spell_mods::modify(world, caster, spell, SpellModOp::ProcCharges, charges);
    let effects = spell
        .effects
        .iter()
        .filter(|effect| {
            is_aura_effect(effect) && application.effect_mask & (1 << effect.index) != 0
        })
        .map(|effect| build_effect(world, caster, spell, effect, now))
        .collect();
    AuraInstance {
        instance_id: 0,
        spell_id: spell.id,
        caster,
        source: application.source,
        from_player: world.get::<Player>(caster).is_some(),
        effects,
        duration_ms,
        expires_at: (duration_ms > 0).then(|| now + f64::from(duration_ms) / 1000.0),
        stacks: 1,
        max_stacks: max_stacks.clamp(0.0, 255.0) as u8,
        charges: charges.clamp(0.0, 255.0) as u8,
        passive,
        hidden: passive || attributes(spell, 0) & ATTR0_DO_NOT_DISPLAY != 0,
        harmful: is_harmful(world, spell, caster, application.target),
        dispel_type: spell
            .categories
            .as_ref()
            .map_or(0, |categories| categories.dispel_type.min(255) as u8),
        interrupt_flags: spell
            .interrupts
            .as_ref()
            .map_or([0; 2], |interrupts| interrupts.aura_interrupt_flags),
        class_set: spell
            .class_options
            .as_ref()
            .map_or(0, |options| options.spell_class_set),
        proc: ProcTimers {
            next_ready_at: now,
            last_attempt_at: now - 10.0,
            last_success_at: now - 120.0,
        },
    }
}

fn log_unhandled_types(world: &mut World, aura: &AuraInstance) {
    for effect in &aura.effects {
        if handlers::is_handled(effect.aura_type) {
            continue;
        }
        let mut logged = world.resource_mut::<UnhandledAuras>();
        if logged.0.insert((aura.spell_id, effect.aura_type)) {
            warn!(
                "Spell {}: aura type {} has no handler; tracked only",
                aura.spell_id, effect.aura_type
            );
        }
    }
}

fn ensure_aura_components(world: &mut World, unit: Entity) {
    if world.get::<ActiveAuras>(unit).is_none() {
        world.entity_mut(unit).insert((
            ActiveAuras::default(),
            UnitAuras::default(),
            AuraStatMods::default(),
        ));
    }
}

fn aura_event(aura: &AuraInstance, target: Entity, kind: CombatLogKind) -> CombatLogEvent {
    CombatLogEvent {
        source: Some(aura.caster.to_bits()),
        target: Some(target.to_bits()),
        spell_id: Some(aura.spell_id),
        school_mask: 0,
        amount: i32::from(aura.stacks),
        overflow: 0,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit: false,
        periodic: false,
        kind,
    }
}

/// A persistent area aura's application shares its dynamic object's expiry and
/// tick phase (`DynObjAura` is one aura for every unit inside): the unit's aura of
/// `spell_id` from `caster` ends at `expires_at` and ticks at `started_at + k × period`.
pub fn align_area_aura(
    world: &mut World,
    unit: Entity,
    caster: Entity,
    spell_id: u32,
    started_at: f64,
    expires_at: f64,
) {
    let now = aura_now(world);
    let Some(mut auras) = world.get_mut::<ActiveAuras>(unit) else {
        return;
    };
    for aura in auras
        .auras
        .iter_mut()
        .filter(|aura| aura.spell_id == spell_id && aura.caster == caster)
    {
        aura.expires_at = Some(expires_at);
        for effect in aura
            .effects
            .iter_mut()
            .filter(|effect| effect.period_ms > 0)
        {
            let period = f64::from(effect.period_ms) / 1000.0;
            let elapsed_ticks = ((now - started_at) / period).floor().max(0.0);
            effect.next_tick_at = started_at + (elapsed_ticks + 1.0) * period;
        }
    }
    auras.dirty = true;
}

/// Apply (or refresh) the aura effects of one resolution on a target.
pub fn apply_aura(world: &mut World, application: &AuraApplication) {
    let Some(spell) = lookup_spell(world, application.spell_id) else {
        warn!("Aura spell {} not found", application.spell_id);
        return;
    };
    let target = application.target;
    if world.get_entity(target).is_err() || world.get_entity(application.caster).is_err() {
        return;
    }
    if !targets::is_alive(world, target) && attributes(&spell, 3) & ATTR3_ALLOW_AURA_WHILE_DEAD == 0
    {
        return;
    }
    let now = aura_now(world);
    let instance = build_instance(world, application, &spell, now);
    if instance.effects.is_empty() {
        return;
    }
    log_unhandled_types(world, &instance);
    ensure_aura_components(world, target);
    let coexist = instance.harmful
        || instance.effects.iter().any(|effect| effect.period_ms > 0)
        || attributes(&spell, 3) & ATTR3_DOT_STACKING_RULE != 0;
    let unique = attributes(&spell, 1) & ATTR1_AURA_UNIQUE != 0;
    let stacking = world
        .get_mut::<ActiveAuras>(target)
        .expect("aura components ensured")
        .stack_or_add(instance, coexist, unique);
    let (id, kind) = match stacking {
        Stacking::Ignored => return,
        Stacking::Applied(id) => (id, CombatLogKind::AuraApplied),
        Stacking::Refreshed(id) => (id, CombatLogKind::AuraRefreshed),
        Stacking::Replaced { id, removed } => {
            announce_removed(world, target, &removed);
            (id, CombatLogKind::AuraApplied)
        }
    };
    let aura = world
        .get::<ActiveAuras>(target)
        .and_then(|auras| auras.get(id))
        .cloned()
        .expect("aura just stored");
    if kind == CombatLogKind::AuraApplied {
        scripts::run_apply(world, target, &aura);
    }
    log_event(world, aura_event(&aura, target, kind));
    auras_changed(world, target);
}

/// Why an aura left its unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveReason {
    Expired,
    Cancelled,
    Dispelled,
    Interrupted,
    Death,
    ChannelEnded,
    /// Charges or absorb points used up.
    Consumed,
    /// The passive spell is no longer known.
    Unlearned,
    CasterGone,
    /// The creature evaded (`RemoveEvadeAuras`).
    Evade,
}

fn announce_removed(world: &mut World, unit: Entity, aura: &AuraInstance) {
    scripts::run_remove(world, unit, aura);
    log_event(world, aura_event(aura, unit, CombatLogKind::AuraRemoved));
}

/// Remove every aura of `unit` matching `remove`; logs `AuraRemoved` for each.
pub fn remove_auras(
    world: &mut World,
    unit: Entity,
    reason: RemoveReason,
    remove: impl Fn(&AuraInstance) -> bool,
) -> Vec<AuraInstance> {
    let Some(mut auras) = world.get_mut::<ActiveAuras>(unit) else {
        return Vec::new();
    };
    let removed = auras.take_where(remove);
    if removed.is_empty() {
        return removed;
    }
    for aura in &removed {
        debug!("Aura {} removed from {unit}: {reason:?}", aura.spell_id);
        announce_removed(world, unit, aura);
    }
    auras_changed(world, unit);
    removed
}

/// One charge used (proc or spell modifier); the aura ends at zero charges.
pub fn spend_charge(world: &mut World, unit: Entity, instance_id: u32) {
    let Some(mut auras) = world.get_mut::<ActiveAuras>(unit) else {
        return;
    };
    let Some(aura) = auras.get_mut(instance_id) else {
        return;
    };
    aura.charges = aura.charges.saturating_sub(1);
    if aura.charges == 0 {
        remove_instance(world, unit, instance_id, RemoveReason::Consumed);
    } else {
        auras_changed(world, unit);
    }
}

pub fn remove_instance(world: &mut World, unit: Entity, instance_id: u32, reason: RemoveReason) {
    remove_auras(world, unit, reason, |aura| aura.instance_id == instance_id);
}

/// Recompute the unit's `StatModifiers` and `AuraStatMods` and mark the
/// replicated view stale. Components are written only on change, so
/// `update_character_stats` recomputes only when aura stats moved.
pub fn auras_changed(world: &mut World, unit: Entity) {
    let Some(mut auras) = world.get_mut::<ActiveAuras>(unit) else {
        return;
    };
    auras.dirty = true;
    let (stats, combat) = stat_mods::compute(&auras);
    if let Some(mut stored) = world.get_mut::<AuraStatMods>(unit)
        && *stored != combat
    {
        *stored = combat;
    }
    let current = world
        .get::<StatModifiers>(unit)
        .copied()
        .unwrap_or_default();
    if current != stats {
        world.entity_mut(unit).insert(stats);
    }
    unit_state::reconcile(world, unit);
}

pub fn has_aura(world: &World, unit: Entity, spell_id: u32) -> bool {
    world
        .get::<ActiveAuras>(unit)
        .is_some_and(|auras| auras.has_spell(spell_id))
}

fn school_or_physical(school_mask: u32) -> u32 {
    if school_mask == 0 { 1 } else { school_mask }
}

/// SCHOOL_IMMUNITY 39: an aura whose MiscValue school mask covers the damage.
pub fn is_immune(world: &World, unit: Entity, school_mask: u32) -> bool {
    let school = school_or_physical(school_mask);
    world.get::<ActiveAuras>(unit).is_some_and(|auras| {
        auras.iter().flat_map(|aura| &aura.effects).any(|effect| {
            effect.aura_type == handlers::SCHOOL_IMMUNITY
                && effect.misc_values[0] as u32 & school != 0
        })
    })
}

/// Damage taken multiplier of `target`: MOD_DAMAGE_PERCENT_TAKEN (through
/// `AuraStatMods`) × MOD_SPELL_DAMAGE_FROM_CASTER auras cast by `attacker`.
pub fn damage_taken_multiplier(
    world: &World,
    target: Entity,
    attacker: Entity,
    school_mask: u32,
) -> f32 {
    let taken = world
        .get::<AuraStatMods>(target)
        .map_or(1.0, |mods| mods.damage_taken(school_mask));
    let from_caster: f32 = world.get::<ActiveAuras>(target).map_or(1.0, |auras| {
        auras
            .iter()
            .filter(|aura| aura.caster == attacker)
            .flat_map(|aura| {
                let stacks = aura.stack_multiplier();
                aura.effects
                    .iter()
                    .filter(|effect| effect.aura_type == handlers::MOD_SPELL_DAMAGE_FROM_CASTER)
                    .map(move |effect| 1.0 + effect.amount * stacks / 100.0)
            })
            .product()
    });
    taken * from_caster
}

/// SCHOOL_ABSORB 69: matching absorbs (MiscValue school mask) soak damage,
/// oldest aura first; a depleted absorb removes its aura. Returns
/// `(damage left, absorbed)`.
pub fn absorb_damage(world: &mut World, unit: Entity, school_mask: u32, amount: f32) -> (f32, f32) {
    let school = school_or_physical(school_mask);
    let Some(mut auras) = world.get_mut::<ActiveAuras>(unit) else {
        return (amount, 0.0);
    };
    let mut left = amount;
    let mut depleted = Vec::new();
    let mut touched = false;
    for aura in auras.bypass_change_detection().auras.iter_mut() {
        for effect in aura.effects.iter_mut().filter(|effect| {
            effect.aura_type == handlers::SCHOOL_ABSORB
                && effect.misc_values[0] as u32 & school != 0
                && effect.absorb_remaining > 0.0
        }) {
            if left <= 0.0 {
                break;
            }
            let soaked = left.min(effect.absorb_remaining);
            effect.absorb_remaining -= soaked;
            left -= soaked;
            touched = true;
            if effect.absorb_remaining <= 0.0 {
                depleted.push(aura.instance_id);
            }
        }
    }
    if !touched {
        return (amount, 0.0);
    }
    if !depleted.is_empty() {
        remove_auras(world, unit, RemoveReason::Consumed, |aura| {
            depleted.contains(&aura.instance_id)
        });
    }
    (left, amount - left)
}

/// Why crowd control forbids `spell` now: MOD_STUN 12 blocks every spell
/// without `SPELL_ATTR5_ALLOW_WHILE_STUNNED`; MOD_SILENCE 27 blocks
/// `PreventionType` silence (1) spells and MOD_PACIFY 25 pacify (2) spells;
/// MOD_PACIFY_SILENCE 60 does both.
pub fn cast_blocker(world: &World, caster: Entity, spell: &SpellInfo) -> Option<CastBlock> {
    const ATTR5_ALLOW_WHILE_STUNNED: u32 = 0x8;
    const PREVENTION_SILENCE: u32 = 1;
    const PREVENTION_PACIFY: u32 = 2;
    let auras = world.get::<ActiveAuras>(caster)?;
    let has = |types: &[u32]| {
        auras
            .iter()
            .flat_map(|aura| &aura.effects)
            .any(|effect| types.contains(&effect.aura_type))
    };
    let prevention = spell
        .categories
        .as_ref()
        .map_or(0, |categories| categories.prevention_type);
    if attributes(spell, 5) & ATTR5_ALLOW_WHILE_STUNNED == 0 && has(&[handlers::MOD_STUN]) {
        return Some(CastBlock::Stunned);
    }
    if prevention & PREVENTION_SILENCE != 0
        && has(&[handlers::MOD_SILENCE, handlers::MOD_PACIFY_SILENCE])
    {
        return Some(CastBlock::Silenced);
    }
    if prevention & PREVENTION_PACIFY != 0
        && has(&[handlers::MOD_PACIFY, handlers::MOD_PACIFY_SILENCE])
    {
        return Some(CastBlock::Pacified);
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastBlock {
    Stunned,
    Silenced,
    Pacified,
}

/// Charge auras of one charge category (TrinityCore `SpellHistory`
/// `GetMaxCharges`/`GetChargeRecoveryTime`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChargeMods {
    /// MOD_MAX_CHARGES 411.
    pub extra_charges: i32,
    /// CHARGE_RECOVERY_MOD 453, ms.
    pub recovery_flat_ms: f32,
    /// CHARGE_RECOVERY_MULTIPLIER 454.
    pub recovery_mult: f32,
}

/// Charge auras of `unit` whose MiscValue is `charge_category`.
pub fn charge_mods(world: &World, unit: Entity, charge_category: u32) -> ChargeMods {
    let mut mods = ChargeMods {
        extra_charges: 0,
        recovery_flat_ms: 0.0,
        recovery_mult: 1.0,
    };
    let Some(auras) = world.get::<ActiveAuras>(unit) else {
        return mods;
    };
    for aura in auras.iter() {
        let stacks = aura.stack_multiplier();
        for effect in aura
            .effects
            .iter()
            .filter(|effect| effect.misc_values[0] as u32 == charge_category)
        {
            let amount = effect.amount * stacks;
            match effect.aura_type {
                handlers::MOD_MAX_CHARGES => mods.extra_charges += amount.round() as i32,
                handlers::CHARGE_RECOVERY_MOD => mods.recovery_flat_ms += amount,
                handlers::CHARGE_RECOVERY_MULTIPLIER => mods.recovery_mult *= 1.0 + amount / 100.0,
                _ => {}
            }
        }
    }
    mods
}

/// A channel ended: its caster's auras of that spell end with it.
pub fn channel_ended(world: &mut World, caster: Entity, spell_id: u32) {
    let mut units = world.query_filtered::<Entity, With<ActiveAuras>>();
    let units: Vec<Entity> = units.iter(world).collect();
    for unit in units {
        remove_auras(world, unit, RemoveReason::ChannelEnded, |aura| {
            aura.spell_id == spell_id && aura.caster == caster && aura.source == AuraSource::Cast
        });
    }
}

/// SPELL_EFFECT_DISPEL: remove up to `count` non-passive auras of
/// `dispel_type` — buffs from a hostile target, debuffs from a friendly one.
/// Logs `Dispel` (source = dispeller, spell = removed aura) per aura.
pub fn dispel(
    world: &mut World,
    dispeller: Entity,
    target: Entity,
    dispel_type: u32,
    count: usize,
) {
    let hostile = targets::is_hostile(world, dispeller, target);
    let Some(auras) = world.get::<ActiveAuras>(target) else {
        return;
    };
    let chosen: Vec<u32> = auras
        .iter()
        .filter(|aura| {
            !aura.passive
                && aura.dispel_type != 0
                && (dispel_type == DISPEL_ALL || u32::from(aura.dispel_type) == dispel_type)
                && aura.harmful != hostile
        })
        .take(count)
        .map(|aura| aura.instance_id)
        .collect();
    let removed = remove_auras(world, target, RemoveReason::Dispelled, |aura| {
        chosen.contains(&aura.instance_id)
    });
    for aura in removed {
        let mut event = aura_event(&aura, target, CombatLogKind::Dispel);
        event.source = Some(dispeller.to_bits());
        log_event(world, event);
    }
}

/// Remove auras whose `AuraInterruptFlags_0/_1` intersect `flags`, except
/// auras of `except_spell` (the spell causing the interrupt).
pub fn interrupt_auras(
    world: &mut World,
    unit: Entity,
    flags: [u32; 2],
    except_spell: Option<u32>,
) {
    if flags == [0; 2] {
        return;
    }
    remove_auras(world, unit, RemoveReason::Interrupted, |aura| {
        (aura.interrupt_flags[0] & flags[0] != 0 || aura.interrupt_flags[1] & flags[1] != 0)
            && Some(aura.spell_id) != except_spell
    });
}

// --- Systems ---

/// Every unit (player or NPC) carries `ActiveAuras`, `UnitAuras` and `AuraStatMods`.
fn init_unit_auras(
    mut commands: Commands,
    units: Query<Entity, (Or<(With<Player>, With<Npc>)>, Without<ActiveAuras>)>,
) {
    for unit in &units {
        commands.entity(unit).insert((
            ActiveAuras::default(),
            UnitAuras::default(),
            AuraStatMods::default(),
        ));
    }
}

/// Players whose `KnownSpells` changed since the last passive sync.
#[derive(Resource, Debug, Default)]
struct PendingPassiveSync(Vec<Entity>);

fn queue_passive_sync(
    changed: Query<Entity, Changed<KnownSpells>>,
    mut pending: ResMut<PendingPassiveSync>,
) {
    pending.0.extend(changed.iter());
}

fn sync_passive_auras(world: &mut World) {
    let units = std::mem::take(&mut world.resource_mut::<PendingPassiveSync>().0);
    for unit in units {
        let Some(known) = world.get::<KnownSpells>(unit) else {
            continue;
        };
        let spells: Vec<u32> = known.iter().collect();
        sync_passives(world, unit, AuraSource::KnownSpell, &spells);
    }
}

/// Keep the passive auras of `source` on `unit` equal to the passive spells
/// in `spells`: apply missing ones (permanent, self-cast) and remove the rest.
pub fn sync_passives(world: &mut World, unit: Entity, source: AuraSource, spells: &[u32]) {
    let passives: Vec<SpellInfo> = spells
        .iter()
        .filter_map(|&id| lookup_spell(world, id))
        .filter(|spell| is_passive(spell) && aura_effect_mask(spell) != 0)
        .collect();
    let ids: Vec<u32> = passives.iter().map(|spell| spell.id).collect();
    remove_auras(world, unit, RemoveReason::Unlearned, |aura| {
        aura.source == source && !ids.contains(&aura.spell_id)
    });
    for spell in passives {
        let applied = world.get::<ActiveAuras>(unit).is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == spell.id && aura.source == source)
        });
        if !applied {
            apply_aura(
                world,
                &AuraApplication {
                    caster: unit,
                    target: unit,
                    spell_id: spell.id,
                    effect_mask: aura_effect_mask(&spell),
                    source,
                },
            );
        }
    }
}

type AuraUnit = (
    Entity,
    &'static mut ActiveAuras,
    Option<&'static shared::components::Health>,
    Option<&'static shared::components::CombatStatus>,
    Option<&'static crate::class_progression::ActiveSpec>,
);

/// Units that have auras to update. A living unit without auras has nothing to
/// interrupt, tick or expire, so only its combat and spec tracking advances here
/// (as `state_interrupts` would), keeping a later aura from seeing a stale
/// transition.
fn units_with_auras(
    world: &mut World,
    units: &mut QueryState<AuraUnit, Without<GridDormant>>,
) -> Vec<Entity> {
    let mut with_auras = Vec::new();
    for (entity, mut auras, health, combat, spec) in units.iter_mut(world) {
        if !auras.auras.is_empty() {
            with_auras.push(entity);
            continue;
        }
        if !health.is_some_and(|health| health.current > 0.0) {
            continue;
        }
        let in_combat = combat.is_some_and(|status| status.0);
        let spec = spec.map(|spec| spec.0);
        if auras.in_combat != in_combat || auras.spec != spec {
            let tracking = auras.bypass_change_detection();
            tracking.in_combat = in_combat;
            tracking.spec = spec;
        }
    }
    with_auras
}

/// Death, state interrupts (movement, combat, spec), lost casters,
/// periodic ticks, then expiry. Dormant creatures catch up on reactivation
/// (expiry and tick times are absolute).
fn update_auras(world: &mut World, units: &mut QueryState<AuraUnit, Without<GridDormant>>) {
    let now = aura_now(world);
    let units = units_with_auras(world, units);
    for unit in units {
        if !targets::is_alive(world, unit) {
            remove_on_death(world, unit);
            continue;
        }
        state_interrupts(world, unit);
        remove_casterless(world, unit);
        periodic::run_due_ticks(world, unit, now);
        remove_auras(world, unit, RemoveReason::Expired, |aura| {
            aura.expires_at.is_some_and(|at| at <= now)
        });
    }
}

/// Auras whose caster entity despawned end (their ticks need a source).
fn remove_casterless(world: &mut World, unit: Entity) {
    let gone: Vec<u32> = world
        .get::<ActiveAuras>(unit)
        .map(|auras| {
            auras
                .iter()
                .filter(|aura| world.get_entity(aura.caster).is_err())
                .map(|aura| aura.instance_id)
                .collect()
        })
        .unwrap_or_default();
    if !gone.is_empty() {
        remove_auras(world, unit, RemoveReason::CasterGone, |aura| {
            gone.contains(&aura.instance_id)
        });
    }
}

fn remove_on_death(world: &mut World, unit: Entity) {
    let store = world.get_resource::<SpellStore>();
    let persistent: Vec<u32> = world
        .get::<ActiveAuras>(unit)
        .map(|auras| {
            auras
                .iter()
                .filter(|aura| {
                    aura.passive
                        || store
                            .and_then(|store| store.get(aura.spell_id))
                            .is_some_and(|spell| {
                                attributes(spell, 3) & ATTR3_ALLOW_AURA_WHILE_DEAD != 0
                            })
                })
                .map(|aura| aura.instance_id)
                .collect()
        })
        .unwrap_or_default();
    remove_auras(world, unit, RemoveReason::Death, |aura| {
        !persistent.contains(&aura.instance_id)
    });
}

/// Moving (0x8), EnteringCombat (0x10000000), LeavingCombat (0x80000000)
/// and ChangeSpec (flags 2: 0x40) interrupts from unit state.
fn state_interrupts(world: &mut World, unit: Entity) {
    const MOVING: u32 = 0x8;
    const ENTERING_COMBAT: u32 = 0x1000_0000;
    const LEAVING_COMBAT: u32 = 0x8000_0000;
    const CHANGE_SPEC: u32 = 0x40;
    let moving = world
        .get::<crate::networking::Moving>(unit)
        .is_some_and(|moving| moving.0);
    let in_combat = world
        .get::<shared::components::CombatStatus>(unit)
        .is_some_and(|status| status.0);
    let spec = world
        .get::<crate::class_progression::ActiveSpec>(unit)
        .map(|spec| spec.0);
    let Some(mut auras) = world.get_mut::<ActiveAuras>(unit) else {
        return;
    };
    let mut flags = [0u32; 2];
    if moving {
        flags[0] |= MOVING;
    }
    if in_combat != auras.in_combat {
        flags[0] |= if in_combat {
            ENTERING_COMBAT
        } else {
            LEAVING_COMBAT
        };
        auras.bypass_change_detection().in_combat = in_combat;
    }
    if spec != auras.spec {
        if auras.spec.is_some() {
            flags[1] |= CHANGE_SPEC;
        }
        auras.bypass_change_detection().spec = spec;
    }
    interrupt_auras(world, unit, flags, None);
}

/// Rebuild `UnitAuras` for units whose auras changed; remaining times are
/// taken at send time and the client counts down.
fn sync_unit_auras(
    world: &mut World,
    units: &mut QueryState<(Entity, &mut ActiveAuras, &mut UnitAuras), Without<GridDormant>>,
) {
    let now = aura_now(world);
    let mut changed = Vec::new();
    for (entity, mut auras, mut view) in units.iter_mut(world) {
        if !auras.dirty {
            continue;
        }
        auras.bypass_change_detection().dirty = false;
        let views: Vec<AuraView> = auras.iter().map(|aura| aura.view(now)).collect();
        if view.auras != views {
            view.auras = views;
            changed.push(entity);
        }
    }
    if let Some(dirty) = world.get_resource::<ReplicationDirtyQueue>() {
        for entity in changed {
            dirty.mark(entity);
        }
    }
}

/// `CancelAura`: the owner removes its own positive, non-passive,
/// cancelable auras of that spell.
fn receive_cancel_aura(world: &mut World) {
    let mut receivers = world.query::<(
        &mut MessageReceiver<CancelAura>,
        Option<&PlayerLink>,
        Has<Authenticated>,
        Has<InWorld>,
    )>();
    let requests: Vec<(Entity, u32)> = receivers
        .iter_mut(world)
        .flat_map(|(mut receiver, link, auth, in_world)| {
            let owner = link.filter(|_| auth && in_world).map(|link| link.0);
            receiver
                .receive()
                .filter_map(move |request| owner.map(|owner| (owner, request.spell_id)))
                .collect::<Vec<_>>()
        })
        .collect();
    for (owner, spell_id) in requests {
        let cancelable = lookup_spell(world, spell_id)
            .is_some_and(|spell| attributes(&spell, 0) & ATTR0_NO_AURA_CANCEL == 0);
        if !cancelable {
            continue;
        }
        remove_auras(world, owner, RemoveReason::Cancelled, |aura| {
            aura.spell_id == spell_id && !aura.harmful && !aura.passive
        });
    }
}

#[cfg(test)]
mod tests;
