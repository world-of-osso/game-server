//! Aura runtime: aura instances on units, stacking/refresh, expiry and
//! removal, periodic ticks, procs, spell modifiers, stat modifiers and
//! `UnitAuras` replication. See `docs/wiki/systems/auras.md`.
//!
//! Stacking rule (TrinityCore `Aura::CanStackWith`, simplified):
//! - same spell and caster: refresh duration, +1 stack up to `CumulativeAura`
//!   (`SPELL_ATTR1_AURA_UNIQUE` auras are not refreshed);
//! - native stackable, non-channel/non-DOT/non-multislot spells share their stack
//!   slot across casters, preserving the existing aura caster;
//! - otherwise same spell, other caster: coexist when the aura is harmful, has a
//!   periodic effect, vehicle-control ownership or `SPELL_ATTR3_DOT_STACKING_RULE`; otherwise the new
//!   aura replaces the old one.
//!
//! Harmful (`AuraApplication::_InitFlags`): from itself or a non-friendly caster, an aura
//! with a negative applied effect (`spell_info::positivity`); from a friendly caster, one
//! without a positive applied effect. Hidden: passive or
//! `SPELL_ATTR0_DO_NOT_DISPLAY_SPELLBOOK_AURA_ICON_COMBAT_LOG`.

pub(crate) mod area_avoidance;
pub(crate) mod base_armor;
mod charred_warblades;
pub(crate) mod damage_shield;
#[cfg(test)]
mod damage_shield_tests;
pub(crate) mod damage_transfer;
pub(crate) mod effect_immunity;
mod fear;
mod fixate;
mod fleeing;
pub(crate) mod handlers;
pub(crate) mod health_pct_trigger;
pub mod invisibility;
mod linked;
pub(crate) mod linked_summon;
pub(crate) mod mastery;
mod mechanic_duration;
pub(crate) mod mechanic_immunity;
pub mod mount;
mod periodic;
pub mod procs;
mod refresh;
pub mod scripts;
pub mod shapeshift;
pub mod spell_mods;
pub mod stat_mods;
pub(crate) mod stealth;
pub(crate) mod summon_damage;
mod unit_state;
pub use invisibility::Invisibility;

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
pub use unit_state::{Confused, Feared, is_polymorphed};
pub(crate) use unit_state::{orb_deception_display, supports_zero_entry_costume};

/// SPELL_AURA_PERIODIC_DAMAGE.
pub const PERIODIC_DAMAGE_AURA: u32 = handlers::PERIODIC_DAMAGE;

/// SpellEffects IDs that apply an aura (`SpellEffect.aura` is the AuraType).
/// 35 (party) and 65 (raid) area auras apply to their resolved targets only:
/// there is no group backend. 27 PERSISTENT_AREA_AURA applies through its
/// dynamic object (`spell_cast::area_aura`).
pub const AURA_EFFECTS: [u32; 4] = [6, 27, 35, 65];

const ATTR0_PASSIVE: u32 = 0x40;
const ATTR4_NOT_IN_ARENA_OR_RATED_BATTLEGROUND: u32 = 0x0001_0000;
const ATTR8_NOT_IN_BATTLEGROUND: u32 = 0x1000_0000;
const ATTR0_DO_NOT_DISPLAY: u32 = 0x80;
const ATTR0_NO_AURA_CANCEL: u32 = 0x8000_0000;
const ATTR1_AURA_UNIQUE: u32 = 0x800;
const ATTR3_DOT_STACKING_RULE: u32 = 0x80;
const ATTR3_ALLOW_AURA_WHILE_DEAD: u32 = 0x0010_0000;
const ATTR5_EXTRA_INITIAL_PERIOD: u32 = 0x200;

/// `DispelType` 7 matches every dispel type.
const DISPEL_ALL: u32 = 7;

pub(crate) mod override_spell_data;
pub(crate) mod overrides;

pub struct AuraPlugin;

impl Plugin for AuraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<procs::PendingAuraEvents>()
            .init_resource::<UnhandledAuras>()
            .init_resource::<scripts::AuraScripts>()
            .add_systems(
                Startup,
                (
                    scripts::register_class_scripts,
                    crate::seething_parachute::register_aura_scripts,
                ),
            )
            .init_resource::<PendingPassiveSync>()
            .add_systems(
                Startup,
                (
                    shapeshift::load_shapeshift_forms_startup,
                    mount::load_mount_store_startup,
                    override_spell_data::load_startup,
                ),
            )
            .add_systems(
                Update,
                (
                    receive_cancel_aura.after(crate::spell_cast::receive_cast_requests),
                    mount::receive_cancel_mount_aura
                        .after(crate::spell_cast::receive_cast_requests),
                    queue_passive_sync,
                    mastery::refresh_amounts.after(crate::stats::update_character_stats),
                ),
            )
            .add_systems(
                FixedUpdate,
                (
                    init_unit_auras,
                    mount::apply_flight_style,
                    sync_passive_auras,
                    update_auras,
                    procs::process_aura_events,
                    health_pct_trigger::trigger_on_health_pct,
                    sync_unit_auras,
                    fleeing::update,
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
    /// Authored spell_area autocast; cannot be canceled by the player.
    SpellArea,
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
    pub immunity: Option<Box<crate::spell_info::SpellImmunityDefinition>>,
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

    pub(crate) fn stack_multiplier(&self) -> f32 {
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
            overrides: Vec::new(),
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
        self.has_type(handlers::MOD_STUN)
    }

    /// Under a `SPELL_AURA_MOD_CONFUSE` effect (TrinityCore `UNIT_STATE_CONFUSED`).
    pub fn is_confused(&self) -> bool {
        self.has_type(handlers::MOD_CONFUSE)
    }

    /// Under a `SPELL_AURA_MOD_FEAR` effect (TrinityCore `UNIT_STATE_FLEEING`).
    pub fn is_feared(&self) -> bool {
        self.has_type(handlers::MOD_FEAR)
    }

    /// `UNIT_STATE_CANNOT_AUTOATTACK`: stunned, confused or fleeing (charges are not
    /// modelled).
    pub fn cannot_auto_attack(&self) -> bool {
        self.is_stunned() || self.is_confused() || self.is_feared() || self.is_pacified()
    }

    pub fn is_pacified(&self) -> bool {
        self.has_type(handlers::MOD_PACIFY) || self.has_type(handlers::MOD_PACIFY_SILENCE)
    }

    /// Additive aura points, outside rating diminishing returns.
    pub(crate) fn modifier(&self, aura_type: u32) -> f32 {
        self.iter()
            .map(|aura| {
                aura.effects
                    .iter()
                    .filter(|effect| effect.aura_type == aura_type)
                    .map(|effect| effect.amount * aura.stack_multiplier())
                    .sum::<f32>()
            })
            .sum()
    }

    pub(crate) fn has_type(&self, aura_type: u32) -> bool {
        self.auras
            .iter()
            .flat_map(|aura| &aura.effects)
            .any(|effect| effect.aura_type == aura_type)
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
    fn stack_or_add(
        &mut self,
        new: AuraInstance,
        coexist: bool,
        unique: bool,
        shared_slot: bool,
    ) -> Stacking {
        let same_slot = |aura: &AuraInstance| {
            aura.spell_id == new.spell_id
                && aura.source == new.source
                && (shared_slot || aura.caster == new.caster)
        };
        if let Some(existing) = self.auras.iter_mut().find(|aura| same_slot(aura)) {
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

#[cfg(test)]
pub(crate) fn take_unhandled_auras(world: &mut World) -> Vec<(u32, u32)> {
    let mut types: Vec<_> = world.resource_mut::<UnhandledAuras>().0.drain().collect();
    types.sort_unstable();
    types
}

fn attributes(spell: &SpellInfo, index: usize) -> u32 {
    spell.misc.as_ref().map_or(0, |misc| misc.attributes[index])
}

/// TC SpellInfo.cpp:1807–1815 and Unit.cpp:3403–3465 share these stack slots.
fn shares_stack_slot_between_casters(spell: &SpellInfo) -> bool {
    let stackable = spell
        .aura_options
        .as_ref()
        .is_some_and(|options| options.cumulative_aura > 1);
    let per_caster = crate::spell_cast::is_channeled(spell)
        || attributes(spell, 3) & ATTR3_DOT_STACKING_RULE != 0;
    // Native IsMultiSlotAura: passive plus Power Spark/Flak Fire/Incanter's Absorption.
    let multi_slot = is_passive(spell) || matches!(spell.id, 55849 | 40075 | 44413);
    stackable && !per_caster && !multi_slot
}

pub fn is_passive(spell: &SpellInfo) -> bool {
    attributes(spell, 0) & ATTR0_PASSIVE != 0
}

/// TC SpellInfo.cpp2218 and Spell.cpp7158; map changes must not erase knowledge.
fn passive_map_allowed(world: &World, unit: Entity, spell: &SpellInfo) -> bool {
    use crate::maps::store::InstanceType;
    let no_battleground = attributes(spell, 8) & ATTR8_NOT_IN_BATTLEGROUND != 0;
    let no_arena = attributes(spell, 4) & ATTR4_NOT_IN_ARENA_OR_RATED_BATTLEGROUND != 0;
    if !no_battleground && !no_arena {
        return true;
    }
    let map = world
        .get::<crate::maps::InMap>(unit)
        .expect("Map-restricted passive requires the unit's actual map");
    let maps = world
        .get_resource::<crate::maps::MapStore>()
        .expect("Map-restricted passive requires installed MapStore metadata");
    let entry = maps.map(map.map_id).unwrap_or_else(|| {
        panic!(
            "Map-restricted passive{} has no installed map{}",
            spell.id, map.map_id
        )
    });
    match entry.instance_type {
        InstanceType::Battleground => !no_battleground,
        InstanceType::Arena => !no_arena,
        _ => true,
    }
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

/// `AuraApplication::_InitFlags` (SpellAuras.cpp:113-147) for the applied `effect_mask`.
fn is_harmful(
    world: &World,
    spell: &SpellInfo,
    caster: Entity,
    target: Entity,
    effect_mask: u32,
) -> bool {
    let negative = world.get_resource::<SpellStore>().map_or(0, |store| {
        crate::spell_info::positivity::negative_effects(store, spell)
    });
    if caster == target || !targets::is_friendly(world, caster, target) {
        negative & effect_mask != 0
    } else {
        effect_mask & !negative == 0
    }
}

fn duration_ms(world: &World, application: &AuraApplication, spell: &SpellInfo) -> u32 {
    if is_passive(spell) {
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
    let duration = spell_mods::modify(
        world,
        application.caster,
        spell,
        SpellModOp::Duration,
        base as f32,
    );
    let extension = scripts::self_regrowth_duration_bonus(world, application);
    let duration = (duration + extension).max(0.0) as u32;
    mechanic_duration::modify_duration(world, application, spell, duration)
}

fn build_effect(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
    now: f64,
) -> AuraEffectState {
    let amount = if let Some(amount) = scripts::absorb_amount(world, caster, spell, effect) {
        amount
    } else if handlers::scales_with_power(effect.aura) {
        crate::spell_cast::effects::scaled_amount(world, caster, spell, effect)
    } else {
        crate::spell_cast::effects::modded_base_points(world, caster, spell, effect)
    };
    let amount = scripts::snapshot_periodic_amount(world, caster, spell, effect, amount);
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
        immunity: effect.immunity.clone(),
        aura_type: effect.aura,
        amount,
        misc_values: effect.misc_values,
        period_ms,
        trigger_spell: effect.trigger_spell,
        class_mask: effect.spell_class_mask,
        // `AuraEffect::ResetPeriodic` (SpellAuraEffects.cpp:957-960): with
        // SPELL_ATTR5_EXTRA_INITIAL_PERIOD the first tick lands on apply.
        next_tick_at: if attributes(spell, 5) & ATTR5_EXTRA_INITIAL_PERIOD != 0 {
            now
        } else {
            now + f64::from(period_ms) / 1000.0
        },
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
    let options = spell.aura_options.as_deref();
    let max_stacks = options.map_or(0.0, |options| options.cumulative_aura as f32);
    let max_stacks =
        spell_mods::modify(world, caster, spell, SpellModOp::MaxAuraStacks, max_stacks);
    let charges = options.map_or(0.0, |options| options.proc_charges.max(0) as f32);
    let charges = crate::spell_cast::scripts::proc_charges(world, caster, spell, charges);
    let charges = spell_mods::modify(world, caster, spell, SpellModOp::ProcCharges, charges);
    let mut effects: Vec<AuraEffectState> = spell
        .effects
        .iter()
        .filter(|effect| {
            is_aura_effect(effect)
                && application.effect_mask & (1 << effect.index) != 0
                && scripts::effect_enabled(world, caster, spell.id, effect.index)
        })
        .map(|effect| build_effect(world, caster, spell, effect, now))
        .collect();
    let applied_mask = effects
        .iter()
        .fold(0, |mask, effect| mask | (1 << effect.index));
    let effective_application = AuraApplication {
        effect_mask: applied_mask,
        ..*application
    };
    let duration_ms = duration_ms(world, &effective_application, spell);
    // AuraEffect::CalculateAmount: breakable crowd control of a spell with proc flags
    // holds 10% of the target's max health (CountPctFromMaxHealth) against damage.
    if options.is_some_and(|options| options.proc_type_mask != [0; 2]) {
        let max_health = world
            .get::<shared::components::Health>(application.target)
            .map_or(0.0, |health| health.max);
        for effect in effects
            .iter_mut()
            .filter(|effect| handlers::breaks_on_damage(effect.aura_type))
        {
            effect.amount = (max_health * 0.1).floor();
        }
    }
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
        harmful: is_harmful(
            world,
            spell,
            caster,
            application.target,
            application.effect_mask & aura_effect_mask(spell),
        ),
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
        let supported_amount = !linked::is_linked(effect.aura_type) || effect.amount == 0.0;
        let duration_gap = mechanic_duration::positive_source_gap(effect)
            || mechanic_duration::invalid_mechanic_source_gap(effect);
        let duration_supported = !duration_gap;
        let immunity_supported = effect.aura_type != handlers::MECHANIC_IMMUNITY_MASK
            || effect
                .immunity
                .as_ref()
                .is_some_and(|definition| !definition.has_unmodeled_fields());
        let handled = handlers::is_handled(effect.aura_type)
            && supported_amount
            && duration_supported
            && immunity_supported;
        if handled
            || scripts::consumes_named_field(aura.spell_id, effect.index)
            || crate::scripted_ai::kings_rest::handles_molten_gold_effect(aura.spell_id, effect)
        {
            continue;
        }
        let mut logged = world.resource_mut::<UnhandledAuras>();
        if logged.0.insert((aura.spell_id, effect.aura_type)) {
            let reason = describe_unhandled_aura(effect);
            warn!(
                "Spell {}: aura type {} {reason}; tracked only",
                aura.spell_id, effect.aura_type
            );
        }
    }
}

fn describe_unhandled_aura(effect: &AuraEffectState) -> String {
    if effect.aura_type == handlers::MECHANIC_IMMUNITY_MASK {
        return if effect.immunity.is_some() {
            format!(
                "has unmodeled school/dispel/aura/other fields in definition {}",
                effect.misc_values[0]
            )
        } else {
            format!(
                "requires missing native creature_immunities definition {}",
                effect.misc_values[0]
            )
        };
    }
    if linked::is_linked(effect.aura_type) {
        format!(
            "nonzero linked amount {} for child {} remains a source gap",
            effect.amount, effect.trigger_spell
        )
    } else if mechanic_duration::invalid_mechanic_source_gap(effect) {
        format!(
            "duration mechanic {} does not fit the pinned mask; source gap",
            effect.misc_values[0]
        )
    } else if mechanic_duration::positive_source_gap(effect) {
        format!(
            "positive duration amount {} remains a source gap; pinned rule cannot extend positive-only values",
            effect.amount
        )
    } else {
        "has no handler".into()
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
        glancing: false,
        periodic: false,
        extra_spell_id: None,
        timestamp_unix_ms: 0,
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

/// Damage pushback, shared by spell and melee events. Periodic/absorbed hits
/// are excluded by the event consumer before reaching this function.
pub(super) fn reduce_cast_pushback(world: &mut World, target: Entity) {
    use shared::casting::{CastState, CastType};
    const MAX_PUSHBACKS: u8 = 2;
    const CAST_DELAY: f32 = 0.5;
    const CHANNEL_FRACTION: f32 = 0.25;
    let reduction = world
        .get::<AuraStatMods>(target)
        .map_or(0.0, |mods| mods.pushback_reduction_pct)
        .clamp(0.0, 100.0);
    let Some(mut cast) = world.get_mut::<CastState>(target) else {
        return;
    };
    if reduction == 100.0 || !cast.interruptible || cast.pushback_count >= MAX_PUSHBACKS {
        return;
    }
    let factor = 1.0 - reduction / 100.0;
    cast.pushback_count += 1;
    cast.duration = match cast.cast_type {
        CastType::Normal => cast.duration + CAST_DELAY * factor,
        CastType::Channel => (cast.duration * (1.0 - CHANNEL_FRACTION * factor)).max(cast.elapsed),
    };
    crate::spell_cast::mark_dirty(world, target);
}

/// `Aura::SetStackAmount` on the unit's own cast, then refresh derived stats/views.
/// A non-stacking aura has a single stack; zero removes it in TC and is not a
/// stack assignment. Callers needing removal use remove_instance instead.
pub(crate) fn set_self_cast_stacks(
    world: &mut World,
    unit: Entity,
    spell_id: u32,
    stacks: u8,
) -> bool {
    assert!(stacks > 0, "zero aura stack assignment requires removal");
    let Some(mut auras) = world.get_mut::<ActiveAuras>(unit) else {
        return false;
    };
    let Some(aura) = auras
        .auras
        .iter_mut()
        .find(|aura| aura.spell_id == spell_id && aura.caster == unit)
    else {
        return false;
    };
    aura.stacks = stacks.min(aura.max_stacks.max(1));
    auras.dirty = true;
    drop(auras);
    auras_changed(world, unit);
    true
}

/// Apply (or refresh) the aura effects of one resolution on a target.
pub fn apply_aura(world: &mut World, application: &AuraApplication) {
    let Some(spell) = lookup_spell(world, application.spell_id) else {
        warn!("Aura spell {} not found", application.spell_id);
        return;
    };
    apply_aura_with_spell(world, application, &spell);
}

/// Casts supply their per-cast spell clone so item-level amounts are snapshotted
/// before the casting item is consumed. Other aura sources use the catalog above.
pub(crate) fn apply_aura_with_spell(
    world: &mut World,
    application: &AuraApplication,
    spell: &SpellInfo,
) {
    let target = application.target;
    if world.get_entity(target).is_err() || world.get_entity(application.caster).is_err() {
        return;
    }
    if !targets::is_alive(world, target) && attributes(&spell, 3) & ATTR3_ALLOW_AURA_WHILE_DEAD == 0
    {
        return;
    }
    let now = aura_now(world);
    let mut instance = build_instance(world, application, &spell, now);
    instance.effects.retain(|stored| {
        let effect = spell
            .effects
            .iter()
            .find(|effect| effect.index == stored.index)
            .expect("applied aura effect belongs to spell");
        let immune = effect_immunity::blocks_effect(world, target, spell, effect)
            || mechanic_immunity::blocks_mechanic(world, target, spell, effect.mechanic);
        if immune {
            let ctx = crate::spell_cast::effects::EffectCtx {
                caster: application.caster,
                spell,
                depth: 0,
                dest: None,
                chain_multiplier: 1.0,
            };
            crate::spell_cast::effects::log_mechanic_immune(world, &ctx, target);
        }
        !immune
    });
    if instance.effects.is_empty() {
        return;
    }
    refresh::prepare_application(world, target, &mut instance);
    log_unhandled_types(world, &instance);
    ensure_aura_components(world, target);
    mechanic_immunity::remove_matching(world, target, &instance);
    if let Some(specific) = food_drink(&spell) {
        let excluded: Vec<u32> = world
            .get::<ActiveAuras>(target)
            .expect("aura components ensured")
            .iter()
            .map(|aura| aura.spell_id)
            .filter(|&id| id != spell.id)
            .filter(|&id| {
                lookup_spell(world, id)
                    .and_then(|other| food_drink(&other))
                    .is_some_and(|other| specific.excludes(other))
            })
            .collect();
        remove_auras(world, target, RemoveReason::Replaced, |aura| {
            excluded.contains(&aura.spell_id)
        });
    }
    // Atonement marks from different priests remain independently owned; other
    // casters' channelled auras stack (Aura::CanStackWith, SpellAuras.cpp:1695).
    let coexist = instance.harmful
        || spell.id == 194384
        || crate::spell_cast::is_channeled(&spell)
        // TC Aura::CanStackWith keeps each vehicle occupant's control aura;
        // vehicle seat admission, not generic beneficial-aura replacement, limits capacity.
        || instance.effects.iter().any(|effect| {
            effect.period_ms > 0 || effect.aura_type == handlers::CONTROL_VEHICLE
        })
        || attributes(&spell, 3) & ATTR3_DOT_STACKING_RULE != 0;
    let unique = attributes(&spell, 1) & ATTR1_AURA_UNIQUE != 0;
    let shared_slot = shares_stack_slot_between_casters(spell);
    let previous_rate =
        crate::spell_cast::recovery_rate::capture_previous_rate_aura(world, target, &instance);
    let stacking = world
        .get_mut::<ActiveAuras>(target)
        .expect("aura components ensured")
        .stack_or_add(instance, coexist, unique, shared_slot);
    let (id, kind) = match stacking {
        Stacking::Ignored => return,
        Stacking::Applied(id) => (id, CombatLogKind::AuraApplied),
        Stacking::Refreshed(id) => (id, CombatLogKind::AuraRefreshed),
        Stacking::Replaced { id, removed } => {
            announce_removed(world, target, &removed, RemoveReason::Replaced);
            (id, CombatLogKind::AuraApplied)
        }
    };
    let aura = world
        .get::<ActiveAuras>(target)
        .and_then(|auras| auras.get(id))
        .cloned()
        .expect("aura just stored");
    if kind == CombatLogKind::AuraApplied {
        crate::achievement::record_gain_aura(world, target, aura.spell_id);
        crate::quest::record_gain_aura(world, target, spell.id);
        if shapeshift::aura_form(&aura).is_some() {
            shapeshift::remove_other_forms(world, target, id);
        }
        scripts::run_apply(world, target, &aura);
        linked::on_applied(world, target, &aura);
        linked_summon::on_applied(world, target, &aura);
        fixate::on_applied(world, target, &aura);
        crate::spell_cast::recovery_rate::on_applied(world, target, &aura);
        interrupt_no_actions_cast(world, target, &aura);
        mount::on_applied(world, target, &aura);
        crate::vehicle::isle::on_applied(world, target, &aura);
        let stuns = aura
            .effects
            .iter()
            .any(|effect| effect.aura_type == handlers::MOD_STUN);
        crate::stand_state::on_aura_applied(world, target, aura.interrupt_flags, stuns);
    }
    if kind == CombatLogKind::AuraRefreshed {
        linked::on_reapplied(world, target, &aura);
        crate::spell_cast::recovery_rate::on_refreshed(
            world,
            target,
            previous_rate.as_ref(),
            &aura,
        );
    }
    log_event(world, aura_event(&aura, target, kind));
    auras_changed(world, target);
}

/// TC HandleAuraModNoActions interrupts only spells with PreventionType bit4.
fn interrupt_no_actions_cast(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if !aura
        .effects
        .iter()
        .any(|effect| effect.aura_type == handlers::MOD_NO_ACTIONS)
    {
        return;
    }
    let prevented = world
        .get::<shared::casting::CastState>(unit)
        .and_then(|cast| lookup_spell(world, cast.spell_id))
        .is_some_and(|spell| no_actions_prevention(&spell));
    if prevented {
        crate::spell_cast::cancel_cast(world, unit);
    }
}

fn no_actions_prevention(spell: &SpellInfo) -> bool {
    // SharedDefines.h SPELL_PREVENTION_TYPE_NO_ACTIONS=4. Aura MiscValue
    // is NOT this mask: authentic toy records contain0 or85.
    const PREVENTION_NO_ACTIONS: u32 = 4;
    spell
        .categories
        .as_ref()
        .is_some_and(|categories| categories.prevention_type & PREVENTION_NO_ACTIONS != 0)
}

/// Data-backed226 tick bridge, not a replacement for conditional/custom scripts.
/// TC PeriodicTick leaves226 to scripts. Retail DB2 supplies explicit triggers
/// for some toys; consuming auras (164) also need a predicate absent from DB2.
pub(crate) fn periodic_dummy_trigger(
    world: &World,
    effect: &SpellEffect,
) -> Result<u32, &'static str> {
    if effect.aura_period_ms == 0 || effect.trigger_spell == 0 {
        return Err("periodic dummy requires custom script (no trigger/period)");
    }
    let trigger = world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(effect.trigger_spell))
        .ok_or("periodic dummy missing trigger spell")?;
    const EFFECT_REMOVE_AURA: u32 = 164;
    if trigger
        .effects
        .iter()
        .any(|effect| effect.effect == EFFECT_REMOVE_AURA)
    {
        return Err("conditional periodic dummy requires custom script");
    }
    Ok(trigger.id)
}

/// `SPELL_SPECIFIC_FOOD`, `_DRINK` or `_FOOD_AND_DRINK` (SpellInfo.cpp:2815-2845).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FoodDrink {
    food: bool,
    drink: bool,
}

impl FoodDrink {
    /// `SpellInfo::IsAuraExclusiveBySpecificWith` (SpellInfo.cpp:2082-2091): food
    /// excludes food, drink drink, and food and drink both.
    fn excludes(self, other: Self) -> bool {
        (self.food && other.food) || (self.drink && other.drink)
    }
}

/// A `SPELLFAMILY_GENERIC` spell with the Standing interrupt flag is food when it has a
/// MOD_REGEN or OBS_MOD_HEALTH aura and drink with MOD_POWER_REGEN or OBS_MOD_POWER.
fn food_drink(spell: &SpellInfo) -> Option<FoodDrink> {
    let generic = spell
        .class_options
        .as_ref()
        .is_none_or(|options| options.spell_class_set == 0);
    let seated = spell.interrupts.as_ref().is_some_and(|interrupts| {
        interrupts.aura_interrupt_flags[0] & crate::stand_state::AURA_INTERRUPT_STANDING != 0
    });
    if !generic || !seated {
        return None;
    }
    let has = |types: [u32; 2]| {
        spell
            .effects
            .iter()
            .any(|effect| is_aura_effect(effect) && types.contains(&effect.aura))
    };
    let specific = FoodDrink {
        food: has([handlers::MOD_REGEN, handlers::OBS_MOD_HEALTH]),
        drink: has([handlers::MOD_POWER_REGEN, handlers::OBS_MOD_POWER]),
    };
    (specific.food || specific.drink).then_some(specific)
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
    /// Stacks, charges or absorb points used up.
    Consumed,
    /// The passive spell is no longer known.
    Unlearned,
    CasterGone,
    /// The creature evaded (`RemoveEvadeAuras`).
    Evade,
    /// A new aura its spell specific excludes took its place (food by food).
    Replaced,
}

///31850s2 rescues exactly the next otherwise fatal attack; reserve its field
///immediately so deferred melee commands cannot spend the same rescue twice.
pub(crate) fn rescue_ardent_defender(
    health: &mut shared::components::Health,
    auras: Option<&mut ActiveAuras>,
    loss: f32,
) -> Option<u32> {
    if health.current <= 0.0 || loss < health.current {
        return None;
    }
    let aura = auras?.auras.iter_mut().find(|a| {
        a.spell_id == 31850 && a.effects.iter().any(|e| e.index == 1 && e.amount > 0.0)
    })?;
    let effect = aura
        .effects
        .iter_mut()
        .find(|e| e.index == 1)
        .expect("Ardent Defender field");
    health.current = health.max * effect.amount / 100.0;
    effect.amount = 0.0;
    Some(aura.instance_id)
}

pub(crate) fn apply_attack_damage(world: &mut World, unit: Entity, loss: f32) -> bool {
    let mut query = world.query::<(&mut shared::components::Health, Option<&mut ActiveAuras>)>();
    let Ok((mut health, mut auras)) = query.get_mut(world, unit) else {
        return false;
    };
    let rescue = rescue_ardent_defender(&mut health, auras.as_deref_mut(), loss);
    let lethal = rescue.is_none() && crate::death::apply_health_damage(&mut health, loss);
    if let Some(instance) = rescue {
        remove_instance(world, unit, instance, RemoveReason::Consumed);
    }
    lethal
}

fn announce_removed(world: &mut World, unit: Entity, aura: &AuraInstance, reason: RemoveReason) {
    scripts::run_remove(world, unit, aura, reason);
    linked::on_removed(world, unit, aura);
    linked_summon::on_removed(world, unit, aura);
    fixate::on_removed(world, aura);
    crate::spell_cast::recovery_rate::on_removed(world, unit, aura);
    mount::on_removed(world, unit, aura);
    crate::vehicle::isle::on_removed(world, unit, aura);
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
        announce_removed(world, unit, aura, reason);
    }
    auras_changed(world, unit);
    removed
}

/// Reset an existing finite aura to its stored maximum duration, without
/// refilling absorb capacity or changing its periodic timer (Aura::RefreshDuration).
pub(crate) fn refresh_duration(world: &mut World, target: Entity, spell_id: u32) {
    let now = aura_now(world);
    let Some(mut auras) = world.get_mut::<ActiveAuras>(target) else {
        return;
    };
    let Some(aura) = auras
        .auras
        .iter_mut()
        .find(|aura| aura.spell_id == spell_id)
    else {
        return;
    };
    if aura.expires_at.is_some() {
        aura.expires_at = Some(now + f64::from(aura.duration_ms) / 1_000.0);
        auras.dirty = true;
    }
}

/// Extend existing finite auras owned by this caster without restarting ticks.
pub(crate) fn extend_owned_duration(
    world: &mut World,
    caster: Entity,
    target: Entity,
    spells: &[u32],
    milliseconds: u32,
) {
    let Some(mut auras) = world.get_mut::<ActiveAuras>(target) else {
        return;
    };
    let mut changed = false;
    for aura in &mut auras.auras {
        if aura.caster == caster && spells.contains(&aura.spell_id) {
            if let Some(expiry) = &mut aura.expires_at {
                *expiry += f64::from(milliseconds) / 1000.0;
                aura.duration_ms += milliseconds;
                changed = true;
            }
        }
    }
    if changed {
        auras.dirty = true;
    }
}

/// Extend owned auras without resetting tick clocks, respecting a maximum
/// remaining duration (327541 Aetherial Kindling). Never shorten longer auras.
pub(crate) fn extend_owned_duration_capped(
    world: &mut World,
    caster: Entity,
    target: Entity,
    spells: &[u32],
    milliseconds: u32,
    maximum_remaining_ms: u32,
) {
    let maximum_expiry = aura_now(world) + f64::from(maximum_remaining_ms) / 1000.0;
    let Some(mut auras) = world.get_mut::<ActiveAuras>(target) else {
        return;
    };
    let mut changed = false;
    for aura in &mut auras.auras {
        if aura.caster != caster || !spells.contains(&aura.spell_id) {
            continue;
        }
        let Some(expiry) = &mut aura.expires_at else {
            continue;
        };
        let additional_ms = ((maximum_expiry - *expiry).max(0.0) * 1000.0).round() as u32;
        let extension_ms = milliseconds.min(additional_ms);
        *expiry += f64::from(extension_ms) / 1000.0;
        aura.duration_ms += extension_ms;
        changed |= extension_ms > 0;
    }
    if changed {
        auras.dirty = true;
    }
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

/// TC Aura::ModStackAmount(-1), SpellAuras.cpp:1093–1128: no timer refresh on loss.
pub(crate) fn spend_stack(world: &mut World, unit: Entity, instance_id: u32) {
    let Some(mut auras) = world.get_mut::<ActiveAuras>(unit) else {
        return;
    };
    let Some(aura) = auras.get_mut(instance_id) else {
        return;
    };
    if aura.stacks <= 1 {
        remove_instance(world, unit, instance_id, RemoveReason::Consumed);
    } else {
        aura.stacks -= 1;
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
    let (mut stats, mut combat) = stat_mods::compute(&auras);
    stats.max_health_pct += scripts::fount_max_health_pct(unit, &auras);
    let growth = scripts::protective_growth_multiplier(unit, &auras);
    for multiplier in &mut combat.damage_taken_mult {
        *multiplier *= growth;
    }
    if let Some(mut stored) = world.get_mut::<AuraStatMods>(unit)
        && *stored != combat
    {
        *stored = combat;
    }
    let current = world
        .get::<StatModifiers>(unit)
        .copied()
        .unwrap_or_default();
    change_flat_health(world, unit, stats.max_health_flat - current.max_health_flat);
    if current != stats {
        world.entity_mut(unit).insert(stats);
    }
    unit_state::reconcile(world, unit);
    health_pct_trigger::reconcile(world, unit);
    crate::spell_cast::refresh_cast_movement_permission(world, unit);
}

/// TrinityCore AuraEffect::HandleAuraModIncreaseHealth adds/removes the fixed
/// bonus from both current and maximum health, never killing on removal:
/// https://github.com/TrinityCore/TrinityCore/blob/master/src/server/game/Spells/Auras/SpellAuraEffects.cpp
fn change_flat_health(world: &mut World, unit: Entity, delta: f32) {
    if delta == 0.0 {
        return;
    }
    let Some(mut health) = world.get_mut::<shared::components::Health>(unit) else {
        return;
    };
    health.max = (health.max + delta).max(1.0);
    if health.current > 0.0 {
        health.current = (health.current + delta).clamp(1.0, health.max);
    }
    if let Some(dirty) = world.get_resource::<ReplicationDirtyQueue>() {
        dirty.mark(unit);
    }
}

/// `Unit::GetAuraCount`: the stacks of every aura of `spell_id` on `unit`.
pub fn aura_count(world: &World, unit: Entity, spell_id: u32) -> u32 {
    world.get::<ActiveAuras>(unit).map_or(0, |auras| {
        auras
            .iter()
            .filter(|aura| aura.spell_id == spell_id)
            .map(|aura| u32::from(aura.stacks.max(1)))
            .sum()
    })
}

pub fn has_aura(world: &World, unit: Entity, spell_id: u32) -> bool {
    world
        .get::<ActiveAuras>(unit)
        .is_some_and(|auras| auras.has_spell(spell_id))
}

/// TC a352b1fa Unit.cpp:783–801: harmful + AnyDamageMask, deliberately
/// independent of aura type. SpellDefines.h:114 includes scripted damage breaks.
pub(crate) fn has_breakable_by_damage_cc(world: &World, unit: Entity) -> bool {
    const ANY_DAMAGE_MASK: u32 = 0x0900_0002;
    world.get::<ActiveAuras>(unit).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.harmful && aura.interrupt_flags[0] & ANY_DAMAGE_MASK != 0)
    })
}

fn school_or_physical(school_mask: u32) -> u32 {
    if school_mask == 0 { 1 } else { school_mask }
}

/// SCHOOL_IMMUNITY 39: an aura whose MiscValue school mask covers the damage.
pub fn is_immune(world: &World, unit: Entity, school_mask: u32) -> bool {
    immune_to_school(world.get::<ActiveAuras>(unit), school_mask)
}

/// Shared school immunity check for spell damage and queried melee targets.
pub(crate) fn immune_to_school(auras: Option<&ActiveAuras>, school_mask: u32) -> bool {
    let school = school_or_physical(school_mask);
    let immunity_mask = auras.map_or(0, |auras| {
        auras
            .iter()
            .flat_map(|aura| &aura.effects)
            .filter(|effect| effect.aura_type == handlers::SCHOOL_IMMUNITY)
            .fold(0, |mask, effect| mask | effect.misc_values[0] as u32)
    });
    // TC IsImmunedToSpell: all of a composite school must be covered;
    // separate immunity auras may supply the combined mask.
    immunity_mask & school == school
}

/// Chance in percent that `unit` reflects a spell of `school_mask`
/// (`WorldObject::SpellHitResult`, Object.cpp:1971-1978): REFLECT_SPELLS amounts plus
/// REFLECT_SPELLS_SCHOOL amounts whose MiscValue school mask covers the spell's.
pub fn reflect_chance(world: &World, unit: Entity, school_mask: u32) -> f32 {
    let school = school_or_physical(school_mask);
    world.get::<ActiveAuras>(unit).map_or(0.0, |auras| {
        auras
            .iter()
            .flat_map(|aura| {
                aura.effects
                    .iter()
                    .map(move |effect| (effect, aura.stack_multiplier()))
            })
            .filter(|(effect, _)| match effect.aura_type {
                handlers::REFLECT_SPELLS => true,
                handlers::REFLECT_SPELLS_SCHOOL => effect.misc_values[0] as u32 & school != 0,
                _ => false,
            })
            .map(|(effect, stacks)| effect.amount * stacks)
            .sum()
    })
}

/// TC Unit::CalculateSpellDamageTaken1198–1285 retains final resilience reduction
/// even for effect165, which does not call SpellDamageBonusTaken.
pub(crate) fn damage_resilience_multiplier(
    world: &World,
    target: Entity,
    spell: &SpellInfo,
) -> f32 {
    const ATTR4_IGNORE_DAMAGE_TAKEN_MODIFIERS: u32 = 0x0000_0100;
    if attributes(spell, 4) & ATTR4_IGNORE_DAMAGE_TAKEN_MODIFIERS == 0 {
        world
            .get::<crate::stats::CharacterStats>(target)
            .map_or(1.0, |stats| stats.versatility_damage_taken_multiplier())
    } else {
        1.0
    }
}

/// Damage taken multiplier of `target`: MOD_DAMAGE_PERCENT_TAKEN (through
/// `AuraStatMods`) × matching school/family/label auras cast by `attacker`.
pub fn damage_taken_multiplier(
    world: &World,
    target: Entity,
    attacker: Entity,
    spell: &SpellInfo,
) -> f32 {
    // TC Unit.cpp7033: only mechanic damage modifiers survive this attribute.
    // This function's supported modifiers are school/caster/versatility, not mechanic.
    const ATTR4_IGNORE_DAMAGE_TAKEN_MODIFIERS: u32 = 0x0000_0100;
    if attributes(spell, 4) & ATTR4_IGNORE_DAMAGE_TAKEN_MODIFIERS != 0 {
        return 1.0;
    }
    let versatility = damage_resilience_multiplier(world, target, spell);
    let school_mask = spell.school_mask();
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
                    .filter(|effect| match effect.aura_type {
                        handlers::MOD_SCHOOL_MASK_DAMAGE_FROM_CASTER => {
                            effect.misc_values[0] as u32 & school_mask != 0
                        }
                        handlers::MOD_SPELL_DAMAGE_FROM_CASTER => {
                            scripts::affects_incoming_spell(aura, effect, spell)
                        }
                        handlers::MOD_SPELL_DAMAGE_FROM_CASTER_BY_LABEL => {
                            spell.labels.contains(&(effect.misc_values[0] as u32))
                        }
                        _ => false,
                    })
                    .map(move |effect| 1.0 + effect.amount * stacks / 100.0)
            })
            .product()
    });
    taken
        * versatility
        * from_caster
        * scripts::unstable_affliction_multiplier(world, attacker, target)
        * scripts::outgoing_to_owner_multiplier(world, attacker, target)
}

/// SCHOOL_ABSORB 69: matching absorbs (MiscValue school mask) soak damage,
/// oldest aura first; a depleted absorb removes its aura. Returns
/// `(damage left, absorbed)`.
pub fn absorb_damage(world: &mut World, unit: Entity, school_mask: u32, amount: f32) -> (f32, f32) {
    let result = consume_absorbs(
        world
            .get_mut::<ActiveAuras>(unit)
            .as_mut()
            .map(|auras| auras.bypass_change_detection()),
        school_mask,
        amount,
    );
    let damage = (result.remaining, result.absorbed);
    finish_absorbs(world, unit, result);
    damage
}

/// Capacity is consumed immediately, even when melee's world-side cleanup is queued.
pub(crate) struct AbsorbResult {
    pub remaining: f32,
    pub absorbed: f32,
    depleted: Vec<u32>,
    contributions: Vec<(Entity, f32)>,
}

pub(crate) fn consume_absorbs(
    auras: Option<&mut ActiveAuras>,
    school_mask: u32,
    amount: f32,
) -> AbsorbResult {
    let school = school_or_physical(school_mask);
    let mut result = AbsorbResult {
        remaining: amount,
        absorbed: 0.0,
        depleted: Vec::new(),
        contributions: Vec::new(),
    };
    let Some(auras) = auras else {
        return result;
    };
    for aura in &mut auras.auras {
        let fraction = refresh::absorb_fraction(aura);
        for effect in aura.effects.iter_mut().filter(|effect| {
            effect.aura_type == handlers::SCHOOL_ABSORB
                && effect.misc_values[0] as u32 & school != 0
                && effect.absorb_remaining > 0.0
        }) {
            if result.remaining <= 0.0 {
                break;
            }
            let soaked = (result.remaining * fraction).min(effect.absorb_remaining);
            effect.absorb_remaining -= soaked;
            result.remaining -= soaked;
            result.contributions.push((aura.caster, soaked));
            if effect.absorb_remaining <= 0.0 {
                result.depleted.push(aura.instance_id);
            }
        }
    }
    result.absorbed = amount - result.remaining;
    result
}

pub(crate) fn finish_absorbs(world: &mut World, unit: Entity, result: AbsorbResult) {
    for (caster, soaked) in result.contributions {
        crate::damage_meter::record_absorb(world, caster, soaked);
    }
    if !result.depleted.is_empty() {
        remove_auras(world, unit, RemoveReason::Consumed, |aura| {
            result.depleted.contains(&aura.instance_id) && scripts::absorb_ends_aura(aura.spell_id)
        });
    }
}

/// Why crowd control forbids `spell` now: MOD_STUN 12 blocks every spell
/// without `SPELL_ATTR5_ALLOW_WHILE_STUNNED`, MOD_CONFUSE 5 every spell without
/// `SPELL_ATTR5_ALLOW_WHILE_CONFUSED` (`Spell::CheckCasterAuras`); MOD_SILENCE 27 blocks
/// `PreventionType` silence (1) spells and MOD_PACIFY 25 pacify (2) spells;
/// MOD_PACIFY_SILENCE 60 does both.
pub fn cast_blocker(world: &World, caster: Entity, spell: &SpellInfo) -> Option<CastBlock> {
    const ATTR5_ALLOW_WHILE_STUNNED: u32 = 0x8;
    const ATTR5_ALLOW_WHILE_CONFUSED: u32 = 0x0004_0000;
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
    if no_actions_prevention(spell) && has(&[handlers::MOD_NO_ACTIONS]) {
        return Some(CastBlock::NoActions);
    }
    if fear::blocks_cast(world, auras, spell) {
        return Some(CastBlock::Fleeing);
    }
    if attributes(spell, 5) & ATTR5_ALLOW_WHILE_CONFUSED == 0 && has(&[handlers::MOD_CONFUSE]) {
        return Some(CastBlock::Confused);
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastBlock {
    Stunned,
    Silenced,
    Pacified,
    NoActions,
    Fleeing,
    Confused,
}

/// Charge auras of one charge category (TrinityCore `SpellHistory`
/// `GetMaxCharges`/`GetChargeRecoveryTime`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChargeMods {
    /// MOD_MAX_CHARGES 411.
    pub extra_charges: i32,
    /// CHARGE_RECOVERY_MOD 453, ms.
    pub recovery_flat_ms: f32,
    /// CHARGE_RECOVERY_MULTIPLIER 454, haste457 and category rate148.
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
    let haste_affected = auras.iter().flat_map(|aura| &aura.effects).any(|effect| {
        effect.aura_type == handlers::CHARGE_RECOVERY_AFFECTED_BY_HASTE_REGEN
            && effect.misc_values[0] as u32 == charge_category
    });
    if haste_affected {
        let haste = world
            .get::<crate::stats::CharacterStats>(unit)
            .map_or(0.0, |stats| stats.haste_pct);
        mods.recovery_mult /= (1.0 + haste / 100.0).max(0.01);
    }
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
                handlers::MOD_CHARGE_RECOVERY_RATE => {
                    mods.recovery_mult *= 100.0 / (100.0 + amount.max(-99.0));
                }
                _ => {}
            }
        }
    }
    mods
}

/// A channel ended: its caster's auras of that spell end with it.
/// A channel that ran its full duration: the auras it keeps up reached their end with it,
/// so their ticks due by their expiry land while the channel still stands (the last
/// tick of a channel, Tame Beast's 13481 at its channel target) before `channel_ended`
/// removes them.
pub fn channel_completed(world: &mut World, caster: Entity, spell_id: u32) {
    let mut units = world.query::<(Entity, &ActiveAuras)>();
    let channel_auras: Vec<(Entity, u32, f64)> = units
        .iter(world)
        .flat_map(|(unit, auras)| {
            auras
                .iter()
                .filter(|aura| {
                    aura.spell_id == spell_id
                        && aura.caster == caster
                        && aura.source == AuraSource::Cast
                })
                .filter_map(move |aura| Some((unit, aura.instance_id, aura.expires_at?)))
        })
        .collect();
    for (unit, instance_id, expiry) in channel_auras {
        periodic::run_ticks_until(world, unit, instance_id, expiry);
    }
}

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
/// Logs `dispel` (dispeller, target, dispelling spell) once per removed aura, naming the
/// aura as its extra spell (`Spell::EffectDispel` → `SMSG_SPELL_DISPELL_LOG`).
pub fn dispel(world: &mut World, dispel: &CombatLogEvent, dispel_type: u32, count: usize) {
    let (Some(dispeller), Some(target)) = (
        dispel.source.and_then(Entity::try_from_bits),
        dispel.target.and_then(Entity::try_from_bits),
    ) else {
        return;
    };
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
        crate::spell_cast::scripts::on_dispelled(world, dispeller, &aura);
        let event = CombatLogEvent {
            amount: i32::from(aura.stacks),
            extra_spell_id: Some(aura.spell_id),
            ..dispel.clone()
        };
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
type UnitsMissingAuras = QueryState<Entity, (Or<(With<Player>, With<Npc>)>, Without<ActiveAuras>)>;

fn init_unit_auras(world: &mut World, missing: &mut UnitsMissingAuras) {
    let units: Vec<Entity> = missing.iter(world).collect();
    // Casts and summon cleanup mutate units immediately. Deferred defaults can
    // overwrite a freshly applied aura or insert into a unit already despawned.
    for unit in units {
        ensure_aura_components(world, unit);
    }
}

/// Owners whose knowledge or actual map changed since the last passive sync.
#[derive(Resource, Debug, Default)]
struct PendingPassiveSync(Vec<Entity>);

type ChangedPassiveOwners<'w, 's> = Query<
    'w,
    's,
    Entity,
    (
        With<KnownSpells>,
        Or<(Changed<KnownSpells>, Changed<crate::maps::InMap>)>,
    ),
>;

fn queue_passive_sync(changed: ChangedPassiveOwners, mut pending: ResMut<PendingPassiveSync>) {
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
        crate::spell_cast::publish_known_charge_snapshot(world, unit);
    }
}

/// Keep the passive auras of `source` on `unit` equal to the passive spells
/// in `spells`: apply missing ones (permanent, self-cast) and remove the rest.
pub fn sync_passives(world: &mut World, unit: Entity, source: AuraSource, spells: &[u32]) {
    let passives: Vec<SpellInfo> = spells
        .iter()
        .filter_map(|&id| lookup_spell(world, id))
        .filter(|spell| is_passive(spell) && aura_effect_mask(spell) != 0)
        .filter(|spell| passive_map_allowed(world, unit, spell))
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
    if source == AuraSource::KnownSpell {
        crate::spell_cast::control_dream::sync_known_ready_credit(world, unit, spells);
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
        crate::spell_cast::toy_area::remove_outside_auras(world, unit);
        remove_casterless(world, unit);
        periodic::run_due_ticks(world, unit, now);
        remove_auras(world, unit, RemoveReason::Expired, |aura| {
            aura.expires_at.is_some_and(|at| at <= now)
        });
    }
}

/// Source-dependent auras end when their caster despawns. Rating snapshots
/// retain their authored duration (TC Aura::Update accepts a null caster).
fn remove_casterless(world: &mut World, unit: Entity) {
    let gone: Vec<u32> = world
        .get::<ActiveAuras>(unit)
        .map(|auras| {
            auras
                .iter()
                .filter(|aura| {
                    world.get_entity(aura.caster).is_err()
                        && !stat_mods::retains_rating_snapshot(aura)
                })
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

/// Unit::RemoveAllAurasOnDeath, also used before post-lethal melee reactions.
pub(crate) fn remove_on_death(world: &mut World, unit: Entity) {
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
    // Proc-derived interrupt flags approximate other TC call sites (triggered casts
    // skip Action there), so only these state changes end matching channels.
    crate::spell_cast::interrupt_channel_with_flags(world, unit, flags);
}

/// Rebuild `UnitAuras` for units whose auras changed; remaining times are
/// taken at send time and the client counts down.
fn sync_unit_auras(
    world: &mut World,
    units: &mut QueryState<(Entity, &mut ActiveAuras, &mut UnitAuras), Without<GridDormant>>,
) {
    let now = aura_now(world);
    let mut views_by_unit: std::collections::HashMap<_, _> = units
        .iter(world)
        .filter(|(_, auras, _)| auras.dirty)
        .map(|(entity, auras, _)| {
            let views = auras
                .iter()
                .map(|aura| {
                    let mut view = aura.view(now);
                    view.overrides = overrides::view_overrides(world, entity, aura);
                    view
                })
                .collect::<Vec<_>>();
            (entity, views)
        })
        .collect();
    let mut changed = Vec::new();
    for (entity, mut auras, mut view) in units.iter_mut(world) {
        if !auras.dirty {
            continue;
        }
        auras.bypass_change_detection().dirty = false;
        let views = views_by_unit
            .remove(&entity)
            .expect("dirty unit has a prepared aura view");
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
            aura.spell_id == spell_id
                && !aura.harmful
                && !aura.passive
                && aura.source != AuraSource::SpellArea
        });
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod toy_effects_tests;

#[cfg(test)]
mod autoattack_crit_tests;
#[cfg(test)]
mod bloodsurge_tests;
#[cfg(test)]
mod charred_warblades_tests;
#[cfg(test)]
mod crit_damage_bonus_tests;
#[cfg(test)]
mod critical_chaos_tests;
#[cfg(test)]
mod defensives_tests;
#[cfg(test)]
mod enduring_torment_tests;
#[cfg(test)]
mod fear_tests;
#[cfg(test)]
mod monster_rising_tests;
#[cfg(test)]
mod power_strike_tests;
#[cfg(test)]
mod tiger_dash_tests;
#[cfg(test)]
mod versatility_tests;
#[cfg(test)]
mod yseras_gift_tests;

#[cfg(test)]
mod override_tests;
#[cfg(test)]
mod spell_auras_tests;

#[cfg(test)]
mod polymorph_tests;

#[cfg(test)]
mod mage_tests;

#[cfg(test)]
pub(crate) mod mount_tests;

#[cfg(test)]
#[test]
fn idle_aura_init_full_population_is_bounded() {
    let mut app = App::new();
    app.add_systems(Update, init_unit_auras);
    crate::test_data::populate_idle_world(app.world_mut(), || {
        (
            Npc {
                template_id: 30,
                name: String::new(),
            },
            ActiveAuras::default(),
        )
    });
    crate::test_data::assert_idle_updates_are_bounded(
        &mut app,
        "idle_aura_init_full_population_is_bounded",
    );
}
