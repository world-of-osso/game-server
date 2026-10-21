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

#[cfg(test)]
mod achievement_damage_tests;
#[cfg(test)]
mod achievement_healing_tests;
#[cfg(test)]
mod aoe_avoidance_tests;
pub mod area_aura;
pub mod area_trigger;
#[cfg(test)]
mod attack_power_tests;
#[cfg(test)]
mod attack_restriction_tests;
#[cfg(test)]
mod attack_speed_tests;
mod aura_spell_restrictions;
pub(crate) mod auto_repeat;
#[cfg(test)]
mod auto_shot_repeat_tests;
pub(crate) use aura_spell_restrictions::check_target as check_target_aura_restrictions;
#[cfg(test)]
mod autoattack_range_tests;
#[cfg(test)]
mod base_armor_tests;
pub(crate) mod blooming_infusion;
#[cfg(test)]
mod blooming_infusion_tests;
#[cfg(test)]
mod bonus_armor_tests;
#[cfg(test)]
mod bonus_stat_tests;
#[cfg(test)]
mod cast_while_moving_tests;
#[cfg(test)]
mod caster_school_damage_tests;
mod casting_restriction;
#[cfg(test)]
mod casting_restriction_tests;
#[cfg(test)]
mod category_cooldown_tests;
#[cfg(test)]
mod census_death_grip_tests;
#[cfg(test)]
mod census_e77_dk_tests;
#[cfg(test)]
mod census_e77_priest_tests;
#[cfg(test)]
mod census_generic_next_tests;
#[cfg(test)]
mod census_guardian_aura_tests;
pub(crate) mod charge;
#[cfg(test)]
mod charge_rate_tests;
#[cfg(test)]
mod chrono_flame_tests;
#[cfg(test)]
mod class_completeness_probe;
#[cfg(test)]
mod class_completeness_tests;
pub(crate) mod control_dream;
#[cfg(test)]
mod control_dream_tests;
pub(crate) mod cosmetic;
pub mod creature;
#[cfg(test)]
mod creature_trigger_aura_tests;
#[cfg(test)]
mod creature_trigger_dead_caster_tests;
#[cfg(test)]
mod creature_type_damage_tests;
#[cfg(test)]
mod critical_block_tests;
#[cfg(test)]
mod dead_cast_progress_tests;
#[cfg(test)]
mod dead_caster_channel_tests;
mod disenchant;
#[cfg(test)]
mod disenchant_tests;
mod druid_transform;
#[cfg(test)]
mod druid_transform_tests;
#[cfg(test)]
mod dummy10_tests;
#[cfg(test)]
mod dummy11_tests;
#[cfg(test)]
mod dummy12_tests;
#[cfg(test)]
mod dummy13_tests;
#[cfg(test)]
mod dummy14_tests;
#[cfg(test)]
mod dummy15_boundary_tests;
#[cfg(test)]
mod dummy15_tests;
#[cfg(test)]
mod dummy16_tests;
#[cfg(test)]
mod dummy17_tests;
#[cfg(test)]
mod dummy18_tests;
#[cfg(test)]
mod dummy19_boundary_tests;
#[cfg(test)]
mod dummy19_tests;
#[cfg(test)]
mod dummy20_tests;
#[cfg(test)]
mod dummy21_tests;
#[cfg(test)]
mod dummy22_tests;
#[cfg(test)]
mod dummy23_tests;
#[cfg(test)]
mod dummy2_tests;
#[cfg(test)]
mod dummy3_tests;
#[cfg(test)]
mod dummy4_tests;
#[cfg(test)]
mod dummy5_tests;
#[cfg(test)]
mod dummy6_tests;
#[cfg(test)]
mod dummy7_tests;
#[cfg(test)]
mod dummy8_defense_tests;
#[cfg(test)]
mod dummy8_tests;
#[cfg(test)]
mod dummy9_tests;
#[cfg(test)]
mod dummy_tests;
pub mod effects;
#[cfg(test)]
mod excluded_aura_spell_tests;
#[cfg(test)]
mod excluded_caster_type_tests;
#[cfg(test)]
mod expel_harm_tests;
#[cfg(test)]
mod fel_devastation_tests;
#[cfg(test)]
mod fount_strength_tests;
#[cfg(test)]
mod full_trigger_dead_target_tests;
pub(crate) mod guardian;
#[cfg(test)]
mod guardian_tests;
pub(crate) mod history_store;
mod hit;
#[cfg(test)]
mod impending_victory_tests;
mod interruptibility;
#[cfg(test)]
mod invigorating_fury_tests;
pub(crate) use interruptibility::can_interrupt_cast;
#[cfg(test)]
mod interruptibility_rule_tests;
#[cfg(test)]
mod interruptibility_tests;
pub(crate) mod mobile_banking;
#[cfg(test)]
mod mobile_banking_tests;
#[cfg(test)]
mod normal_allow_dead_target_tests;
pub(crate) mod omor_orbital;
#[cfg(test)]
mod ordinary_dead_caster_tests;
#[cfg(test)]
mod prayer_of_mending_tests;
#[cfg(test)]
mod queued_trigger_cost_tests;
#[cfg(test)]
mod queued_trigger_target_tests;
#[cfg(test)]
mod queued_trigger_tests;
#[cfg(test)]
mod readiness_lifetime_tests;
#[cfg(test)]
mod required_caster_aura_tests;
#[cfg(test)]
mod scourge_eruption_tests;
#[cfg(test)]
mod summon_lifetime_tests;
#[cfg(test)]
mod target_aura_type_tests;
#[cfg(test)]
mod touch_of_death_tests;
#[cfg(test)]
mod triggered_caster_aura_tests;
#[cfg(test)]
mod triggered_target_aura_tests;
#[cfg(test)]
mod warrior_stun_tests;
mod warrior_stuns;
pub(crate) use hit::damage_shield_hits;
#[cfg(test)]
mod enemy_dodge_tests;
#[cfg(test)]
mod label_recovery_tests;
#[cfg(test)]
mod linked_aura_tests;
#[cfg(test)]
mod mechanic_duration_tests;
#[cfg(test)]
mod mechanic_resistance_tests;
#[cfg(test)]
mod melee_crit_taken_tests;
#[cfg(test)]
mod melee_from_caster_tests;
#[cfg(test)]
mod melee_hit_modifier_tests;
#[cfg(test)]
mod minimum_speed_tests;
mod missiles;
#[cfg(test)]
mod npc_dodge_tests;
mod open_lock;
#[cfg(test)]
mod open_lock_tests;
#[cfg(test)]
mod portal_tests;
pub mod powers;
#[cfg(test)]
mod protective_growth_tests;
#[cfg(test)]
mod raid_area_tests;
pub(crate) mod recovery_rate;
#[cfg(test)]
mod recovery_rate_tests;
mod reflect;
#[cfg(test)]
mod resistance_pct_tests;
mod resources;
pub mod runes;
pub mod scripts;
mod shapeshift;
#[cfg(test)]
mod shapeshift_tests;
pub mod summon;
#[cfg(test)]
mod summon_at_tests;
#[cfg(test)]
mod summon_damage_tests;
pub mod targets;
#[cfg(test)]
pub(crate) mod test_harness;

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::casting::{CastState, CastTickResult, CastType};
use shared::components::{Health, Npc, Player, PowerType};
use shared::protocol::{
    CastFailed, CombatChannel, CombatLogEvent, CombatLogKind, SpellCastIntent, SpellChargesUpdate,
    SpellCooldownUpdate, SpellFailure, SpellGo, StopSpellCast,
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
pub(crate) use creature::{
    AiTarget, ai_spell_target, creature_cast, creature_cast_ignoring_cooldowns,
    creature_cast_ignoring_in_progress, creature_cast_interrupting_self,
};
use effects::{EffectCtx, UnhandledEffects};
use targets::{Dest, Relation};

/// `SpellInterruptFlags::Movement`.
const SPELL_INTERRUPT_MOVEMENT: u32 = 0x1;
/// `SpellAuraInterruptFlags::Moving | Turning`.
const CHANNEL_INTERRUPT_MOVING: u32 = 0x8 | 0x10;
/// SPELL_ATTR0_CANCELS_AUTO_ATTACK_COMBAT (Polymorph, Gouge, Blind, ...).
const SPELL_ATTR0_CANCELS_AUTO_ATTACK_COMBAT: u32 = 0x0010_0000;
/// SPELL_ATTR0_ALLOW_WHILE_MOUNTED (SharedDefines.h:460).
const SPELL_ATTR0_ALLOW_WHILE_MOUNTED: u32 = 0x0100_0000;
/// SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL (SharedDefines.h:464): mounts, food, drink.
const SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL: u32 = 0x1000_0000;
/// SPELL_ATTR1_IS_CHANNELLED / SPELL_ATTR1_IS_SELF_CHANNELLED.
const SPELL_ATTR1_CHANNELED: u32 = 0x4 | 0x40;
/// `SpellInfo::HasInitialAggro` (SpellInfo.cpp:1925-1928): none of these.
const SPELL_ATTR1_NO_THREAT: u32 = 0x0000_0400;
/// `SPELL_ATTR2_IGNORE_LINE_OF_SIGHT` (SharedDefines.h:512).
const SPELL_ATTR2_IGNORE_LINE_OF_SIGHT: u32 = 0x0000_0004;
const SPELL_ATTR2_NO_INITIAL_THREAT: u32 = 0x0040_0000;
const SPELL_ATTR4_NO_HARMFUL_THREAT: u32 = 0x0000_0010;
/// `SpellPower.PowerType` -2 costs health.
const POWER_HEALTH: i32 = -2;

mod arena_cooldowns;
pub(crate) use arena_cooldowns::reset_arena_cooldowns;
#[cfg(test)]
mod arena_cooldown_tests;
mod assisted_costs;
#[cfg(test)]
mod charge_snapshot_tests;
pub(crate) mod homebind;
pub(crate) mod jump_charge;
mod knockback;
#[cfg(test)]
mod knockback_tests;
mod school_costs;
pub(crate) mod teleport;
pub(crate) mod teleport_positions;
mod toy_coin;
mod toy_foam_sword;
pub(crate) mod toy_scripts;
mod toy_weather;
mod toy_wormhole;

pub(crate) mod queued_triggers;

pub struct SpellCastPlugin;

impl Plugin for SpellCastPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(test)]
        app.insert_resource(toy_requirements::fixture());
        app.add_plugins(crate::knockback::KnockbackPlugin);
        app.init_resource::<crate::group::GroupRegistry>()
            .init_resource::<disenchant::DisenchantData>()
            .add_systems(Startup, (disenchant::load_startup, open_lock::load_startup))
            .init_resource::<UnhandledEffects>()
            .init_resource::<ResolvingHits>()
            .init_resource::<teleport_positions::SpellTargetPositions>()
            .add_systems(Startup, teleport_positions::load_positions_startup)
            .init_resource::<missiles::PendingSpellHits>()
            .init_resource::<queued_triggers::PendingTriggers>()
            .init_resource::<guardian::GuardianSpells>()
            .add_systems(Startup, guardian::load_startup)
            .init_resource::<summon::SummonPropertiesStore>()
            .add_systems(Startup, summon::load_summon_properties_startup)
            .add_systems(Startup, powers::load_power_type_table_startup)
            .add_systems(FixedPreUpdate, charge::release_finished)
            .add_systems(
                Update,
                (receive_cast_requests, receive_stop_requests).chain(),
            )
            .add_systems(Update, history_store::queue_restore)
            .add_systems(
                FixedUpdate,
                (
                    powers::init_player_powers,
                    powers::update_max_powers,
                    powers::regenerate_powers,
                    runes::regenerate_runes,
                    sync_charge_recovery,
                    advance_casts.after(crate::networking::process_player_inputs),
                    charge::advance,
                    missiles::resolve_due_hits,
                    start_queued_casts,
                    auto_repeat::update,
                    queued_triggers::resolve_due,
                    powers::sync_unit_powers,
                    runes::sync_unit_runes,
                    assisted_costs::publish,
                )
                    .chain(),
            )
            .add_systems(
                FixedUpdate,
                (
                    powers::regenerate_creature_mana,
                    powers::regenerate_creature_health,
                    powers::regenerate_player_health,
                    area_aura::update_dynamic_objects.after(advance_casts),
                    (summon::expire_summons, guardian::update)
                        .chain()
                        .after(advance_casts),
                    crate::game_objects::summoned::expire_objects.after(advance_casts),
                    area_trigger::update_area_triggers.after(advance_casts),
                    scripts::update_ground_pulses.after(advance_casts),
                    toy_coin::catch_landed_coins.after(advance_casts),
                ),
            );
    }
}

/// Why a cast was refused, mapped to the contract's `CastFailReason`.
/// `NotSupported` carries the specific cause in `detail`.
#[derive(Debug, Clone, PartialEq)]
pub enum CastFailure {
    /// Source RequiredAreasID does not contain the current area or its ancestors.
    IncorrectArea(u32),
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
    /// `SPELL_FAILED_CASTER_AURASTATE` (also covers required/excluded aura spells).
    CasterAuraState,
    /// `SPELL_FAILED_TARGET_AURASTATE` (also covers required/excluded aura spells).
    TargetAuraState,
    OutOfRange,
    TooClose,
    /// `SPELL_FAILED_UNIT_NOT_INFRONT`.
    NotInFront,
    /// `SPELL_FAILED_LINE_OF_SIGHT`.
    LineOfSight,
    /// `SPELL_FAILED_AFFECTING_COMBAT`.
    AffectingCombat,
    /// `SPELL_FAILED_NOT_MOUNTED`.
    NotMounted,
    /// `SPELL_FAILED_NOT_SHAPESHIFT`.
    NotShapeshift,
    /// `SPELL_FAILED_ONLY_SHAPESHIFT`.
    OnlyShapeshift,
    /// `SPELL_FAILED_CANT_DO_THAT_RIGHT_NOW`.
    CantDoThatRightNow,
    NotEnoughPower(PowerType),
    NotEnoughHealth,
    /// Missing `SpellReagents` item (its name).
    Reagents(String),
    /// Missing `SpellTotems` tool (item or tool category name).
    Totems(String),
    /// A created item does not fit in the bags.
    InventoryFull,
    /// Native equip error; reported to the owner instead of a spell cast error.
    Inventory(shared::protocol::InventoryError),
    LowCastLevel,
    CantBeDisenchanted,
    CantBeDisenchantedSkill,
    DisenchantNotKnown,
    /// A Tame Beast, Call Pet or Dismiss Pet check (`crate::hunter_pet::check_cast`).
    Pet(crate::hunter_pet::PetCastFailure),
    Unsupported(String),
}

impl CastFailure {
    pub(crate) fn to_contract(
        &self,
    ) -> Result<(CastFailReason, Option<String>), shared::protocol::InventoryError> {
        use CastFailReason as R;
        let contract = match self {
            Self::IncorrectArea(group) => (
                R::IncorrectArea,
                Some(format!("Requires area group{group}")),
            ),
            Self::UnknownSpell => (R::NotSupported, Some("unknown spell".into())),
            Self::NotKnown => (R::NotSupported, Some("spell not known".into())),
            Self::DisenchantNotKnown => (R::NotKnown, None),
            Self::CasterDead => (R::CasterDead, None),
            Self::CrowdControlled(block) => match block {
                crate::auras::CastBlock::Stunned => (R::Stunned, None),
                crate::auras::CastBlock::Silenced => (R::Silenced, None),
                crate::auras::CastBlock::Pacified => (R::Pacified, None),
                // Existing detail contract, as with Confused; no protocol ABI change.
                crate::auras::CastBlock::NoActions => {
                    (R::NotSupported, Some("Can't do that right now".into()))
                }
                // SPELL_FAILED_FLEEING/CONFUSED have no CastFailReason in shared-protocol yet.
                crate::auras::CastBlock::Fleeing => {
                    (R::NotSupported, Some("Can't do that while fleeing".into()))
                }
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
            Self::CasterAuraState => (
                R::InvalidTarget,
                Some("SPELL_FAILED_CASTER_AURASTATE".into()),
            ),
            Self::TargetAuraState => (
                R::InvalidTarget,
                Some("SPELL_FAILED_TARGET_AURASTATE".into()),
            ),
            Self::OutOfRange => (R::OutOfRange, None),
            Self::TooClose => (R::TooClose, None),
            Self::NotInFront => (R::NotInFront, None),
            Self::LineOfSight => (R::LineOfSight, None),
            Self::AffectingCombat => (R::AffectingCombat, None),
            Self::NotMounted => (R::NotMounted, None),
            Self::NotShapeshift => (R::NotShapeshift, None),
            Self::OnlyShapeshift => (R::OnlyShapeshift, None),
            Self::CantDoThatRightNow => (R::CantDoThatRightNow, None),
            Self::NotEnoughPower(power) => (R::NotEnoughResource, Some(format!("{power:?}"))),
            Self::NotEnoughHealth => (R::NotEnoughResource, Some("Health".into())),
            // SPELL_FAILED_REAGENTS, SPELL_FAILED_TOTEM_CATEGORY, ERR_INV_FULL
            Self::Reagents(item) => (R::Reagents, Some(format!("Missing reagent: {item}"))),
            Self::Totems(tool) => (R::Totems, Some(format!("Requires {tool}"))),
            Self::InventoryFull => (R::InventoryFull, Some("Inventory is full.".into())),
            Self::Inventory(error) => return Err(error.clone()),
            // Existing wire contract has no LOW_CASTLEVEL variant.
            Self::LowCastLevel => (R::InvalidTarget, Some("SPELL_FAILED_LOW_CASTLEVEL".into())),
            Self::CantBeDisenchanted => (R::CantBeDisenchanted, None),
            Self::CantBeDisenchantedSkill => (R::CantBeDisenchantedSkill, None),
            Self::Pet(failure) => failure.to_contract(),
            Self::Unsupported(detail) => (R::NotSupported, Some(detail.clone())),
        };
        Ok(contract)
    }
}

/// One consumed charge: the flat-adjusted base selected at use and its deadline.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ChargeRecovery {
    /// Flat-adjusted recovery at consumption, before the changeable rate.
    recovery_ms: f32,
    ready_at: f64,
}

/// TC SpellHistory::ConsumeCharge keeps separately timed, insertion-ordered uses.
#[derive(Debug, Clone, PartialEq)]
struct ChargeState {
    spell_id: u32,
    category: u32,
    max: u8,
    /// Flat-adjusted base for new uses; existing uses retain their recorded bases.
    recovery_ms: f32,
    recovery_mult: f32,
    pending: VecDeque<ChargeRecovery>,
    /// Exact time the final missing charge returned, not the observed frame.
    fully_ready_at: f64,
}

impl ChargeState {
    fn used(&self) -> u8 {
        u8::try_from(self.pending.len()).expect("charge queue fits its source u8 count")
    }

    fn next_ready_at(&self) -> f64 {
        self.pending.front().map_or(0.0, |charge| charge.ready_at)
    }

    fn refresh(&mut self, now: f64) {
        while self
            .pending
            .front()
            .is_some_and(|charge| now >= charge.ready_at)
        {
            let recovered = self.pending.pop_front().unwrap();
            if self.pending.is_empty() {
                self.fully_ready_at = recovered.ready_at;
            }
        }
    }

    fn consume(&mut self, now: f64, ready_credit: u32) {
        // TC ConsumeCharge834–853: nonpositive recovery/max consumes nothing.
        let duration = Self::calculate_recovery_seconds(self.recovery_ms, self.recovery_mult);
        if self.max == 0 || duration <= 0.0 {
            return;
        }
        let ready_at = self.pending.back().map_or_else(
            || now + (duration - f64::from(ready_credit) / 1000.0).max(0.0),
            |previous| previous.ready_at + duration,
        );
        self.pending.push_back(ChargeRecovery {
            recovery_ms: self.recovery_ms,
            ready_at,
        });
    }

    fn retime(&mut self, now: f64, multiplier: f32) {
        // Recompute from each use's base: multiplying an already floored timer
        // loses milliseconds when removing or refreshing a rate modifier.
        let mut previous_end = now;
        for (index, charge) in self.pending.iter_mut().enumerate() {
            let old_duration =
                Self::calculate_recovery_seconds(charge.recovery_ms, self.recovery_mult);
            let new_duration = Self::calculate_recovery_seconds(charge.recovery_ms, multiplier);
            let remaining = if index == 0 {
                (charge.ready_at - now) * new_duration / old_duration
            } else {
                new_duration
            };
            charge.ready_at = previous_end + remaining;
            previous_end = charge.ready_at;
        }
    }

    fn calculate_recovery_seconds(base_ms: f32, multiplier: f32) -> f64 {
        f64::from((base_ms * multiplier).max(0.0).floor()) / 1000.0
    }

    fn reduce_recovery(&mut self, now: f64, seconds: f64) {
        for charge in &mut self.pending {
            charge.ready_at -= seconds;
        }
        self.refresh(now);
    }

    fn snapshot(&self, now: f64) -> SpellChargesUpdate {
        let next = self.pending.front();
        let base_ms = next.map_or(self.recovery_ms, |charge| charge.recovery_ms);
        let duration = Self::calculate_recovery_seconds(base_ms, self.recovery_mult);
        SpellChargesUpdate {
            spell_id: self.spell_id,
            category: self.category,
            current: self.max.saturating_sub(self.used()),
            max: self.max,
            recharge_ms: (duration * 1000.0) as u32,
            remaining_ms: ((self.next_ready_at() - now).max(0.0) * 1000.0) as u32,
        }
    }
}

/// Per-unit cooldown state in real elapsed server seconds (`history_now`).
#[derive(Component, Debug, Clone, Default)]
pub struct SpellHistory {
    spell_ready_at: HashMap<u32, f64>,
    category_ready_at: HashMap<u32, f64>,
    /// Native spell that last started each category timer, for masked rate changes.
    category_spell_ids: HashMap<u32, u32>,
    /// Actual item-use authored durations/categories, which can override SpellInfo.
    item_cooldown_sources: HashMap<u32, ItemCooldownSource>,
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

    pub(crate) fn on_cooldown(&self, spell: &SpellInfo, now: f64) -> bool {
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

    pub(crate) fn on_gcd(&self, spell: &SpellInfo, now: f64) -> bool {
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
        let recovery_ms = categories.charge_recovery_ms as f32 + mods.recovery_flat_ms;
        let state = self
            .charges
            .entry(categories.charge_category)
            .or_insert(ChargeState {
                spell_id: spell.id,
                category: categories.charge_category,
                max,
                recovery_ms: 0.0,
                recovery_mult: mods.recovery_mult,
                pending: VecDeque::new(),
                fully_ready_at: 0.0,
            });
        state.max = max;
        state.refresh(now);
        if !state.pending.is_empty() && state.recovery_mult != mods.recovery_mult {
            state.retime(now, mods.recovery_mult);
        }
        // Aura453 has no immediate handler: only future consumption changes.
        state.recovery_ms = recovery_ms;
        state.recovery_mult = mods.recovery_mult;
        Some(state)
    }
}

fn charge_mods(world: &World, caster: Entity, spell: &SpellInfo) -> ChargeMods {
    let category = spell.categories.as_ref().map_or(0, |c| c.charge_category);
    crate::auras::charge_mods(world, caster, category)
}

/// Refresh existing category clocks/maxima, never discover known spells here.
/// Initial full states belong to the character spellbook snapshot, not recovery.
fn sync_charge_recovery(world: &mut World) {
    let pending: Vec<(Entity, u32)> = world
        .query::<(Entity, &SpellHistory)>()
        .iter(world)
        .flat_map(|(unit, history)| {
            history
                .charges
                .values()
                .map(move |state| (unit, state.spell_id))
        })
        .collect();
    for (unit, spell_id) in pending {
        refresh_charge_recovery(world, unit, spell_id);
    }
}

/// World-entry/learned-spell publication after passive synchronization. TC
/// SpellHistory::WritePacket(SendSpellCharges) is separate from recovery updates;
/// our owner snapshot also supplies full-category maxima for Cooldown Manager.
/// ClientId identifies a persisted character whose spellbook has been initialized.
pub(crate) fn publish_known_charge_snapshot(world: &mut World, unit: Entity) {
    if world.get::<crate::persistence::ClientId>(unit).is_none() {
        return;
    }
    let spells = known_charge_categories(world, unit);
    let now = history_now(world);
    for spell in spells.into_values() {
        let mods = charge_mods(world, unit, &spell);
        let charges = world
            .entity_mut(unit)
            .entry::<SpellHistory>()
            .or_default()
            .get_mut()
            .charges_mut(&spell, now, mods)
            .unwrap()
            .snapshot(now);
        send_to_owner(world, unit, charges);
    }
}

fn known_charge_categories(
    world: &World,
    unit: Entity,
) -> std::collections::BTreeMap<u32, SpellInfo> {
    let store = world.resource::<SpellStore>();
    world
        .get::<KnownSpells>(unit)
        .expect("charge snapshot follows spellbook initialization")
        .iter()
        .filter_map(|id| store.get(id))
        .filter_map(|spell| {
            let categories = spell.categories.as_ref()?;
            let has_charges = categories.charge_category != 0 && categories.max_charges != 0;
            has_charges.then(|| (categories.charge_category, spell.clone()))
        })
        .collect()
}

fn refresh_charge_recovery(world: &mut World, unit: Entity, spell_id: u32) {
    let spell = world
        .resource::<SpellStore>()
        .get(spell_id)
        .expect("pending charge spell must remain in the catalog")
        .clone();
    let category = spell.categories.as_ref().unwrap().charge_category;
    let mods = charge_mods(world, unit, &spell);
    let now = history_now(world);
    if world.get::<SpellHistory>(unit).is_none() {
        world.entity_mut(unit).insert(SpellHistory::default());
    }
    let mut history = world.get_mut::<SpellHistory>(unit).unwrap();
    let before = history
        .charges
        .get(&category)
        .map(|charges| charges.snapshot(now));
    let after = history
        .charges_mut(&spell, now, mods)
        .unwrap()
        .snapshot(now);
    if before.as_ref() != Some(&after) {
        send_to_owner(world, unit, after);
    }
}

fn gcd_category(spell: &SpellInfo) -> u32 {
    spell
        .categories
        .as_ref()
        .map_or(0, |categories| categories.start_recovery_category)
}

/// TC SpellHistory uses GameTime deadlines, independent of simulation catch-up.
/// Bevy virtual/fixed time drops elapsed time when a frame exceeds its 250ms cap.
/// Keep simulation clocks for casts, auras and missiles; readiness must match the
/// real milliseconds sent to the owning client's cooldown and charge timers.
pub(crate) fn history_now(world: &World) -> f64 {
    world.resource::<Time<Real>>().elapsed_secs_f64()
}

pub(crate) fn now(world: &World) -> f64 {
    world.resource::<Time>().elapsed_secs_f64()
}

fn attributes(spell: &SpellInfo, index: usize) -> u32 {
    spell.misc.as_ref().map_or(0, |misc| misc.attributes[index])
}

pub(crate) fn is_channeled(spell: &SpellInfo) -> bool {
    attributes(spell, 1) & SPELL_ATTR1_CHANNELED != 0
}

/// Moving leaves the cast bar alone (`Spell::CheckMovement`: no
/// `SpellInterruptFlags::Movement`).
fn cast_allows_moving(world: &World, caster: Entity, spell: &SpellInfo) -> bool {
    let intrinsic = spell
        .interrupts
        .as_ref()
        .is_none_or(|interrupts| interrupts.interrupt_flags & SPELL_INTERRUPT_MOVEMENT == 0);
    intrinsic || aura_allows_moving(world, caster, spell)
}

/// Moving leaves the channel alone (`SpellInfo::IsMoveAllowedChannel`).
fn channel_allows_moving(world: &World, caster: Entity, spell: &SpellInfo) -> bool {
    let intrinsic = spell.interrupts.as_ref().is_none_or(|interrupts| {
        interrupts.channel_interrupt_flags[0] & CHANNEL_INTERRUPT_MOVING == 0
    });
    intrinsic || aura_allows_moving(world, caster, spell)
}

/// The channel half of `Unit::RemoveAurasWithInterruptFlags` (Unit.cpp:4278-4284): a
/// channel whose `ChannelInterruptFlags` share `flags` ends (`InterruptNonMeleeSpells`).
/// Moving and Turning are enforced through `CastState::cast_while_moving` instead.
pub(crate) fn interrupt_channel_with_flags(world: &mut World, caster: Entity, flags: [u32; 2]) {
    let Some((spell, CastType::Channel)) = read_active_cast_spell(world, caster) else {
        return;
    };
    let channel_flags = spell
        .interrupts
        .as_ref()
        .map_or([0; 2], |interrupts| interrupts.channel_interrupt_flags);
    let handled_elsewhere = [CHANNEL_INTERRUPT_MOVING, 0];
    let matches = (0..2).any(|i| channel_flags[i] & flags[i] & !handled_elsewhere[i] != 0);
    if matches {
        cancel_cast(world, caster);
    }
}

/// TC Unit::CanCastSpellWhileMoving:330 uses a family mask;307 uses Misc0
/// SpellLabel without a family constraint. Neither selector depends on amount.
fn aura_allows_moving(world: &World, caster: Entity, spell: &SpellInfo) -> bool {
    const NO_DISABLE_MOVEMENT_INTERRUPT: u32 = 0x0800_0000;
    if attributes(spell, 13) & NO_DISABLE_MOVEMENT_INTERRUPT != 0 {
        return false;
    }
    spell_mods::has_aura_type_affecting(
        world,
        caster,
        crate::auras::handlers::CAST_WHILE_WALKING,
        spell,
    ) || world
        .get::<crate::auras::ActiveAuras>(caster)
        .is_some_and(|auras| {
            auras.iter().any(|aura| {
                aura.effects.iter().any(|effect| {
                    effect.aura_type == crate::auras::handlers::CAST_WHILE_WALKING_BY_SPELL_LABEL
                        && u32::try_from(effect.misc_values[0])
                            .is_ok_and(|label| spell.labels.binary_search(&label).is_ok())
                })
            })
        })
}

/// Aura gain/removal/expiry updates the server cast and its replicated movement flag.
pub(crate) fn refresh_cast_movement_permission(world: &mut World, caster: Entity) {
    let Some((spell, cast_type)) = read_active_cast_spell(world, caster) else {
        return;
    };
    let allowed = match cast_type {
        CastType::Normal => cast_allows_moving(world, caster, &spell),
        CastType::Channel => channel_allows_moving(world, caster, &spell),
    };
    let mut cast = world.get_mut::<CastState>(caster).expect("cast read above");
    if cast.cast_while_moving != allowed {
        cast.cast_while_moving = allowed;
    }
}

fn read_active_cast_spell(world: &World, caster: Entity) -> Option<(SpellInfo, CastType)> {
    let (spell_id, cast_type) = world
        .get::<CastState>(caster)
        .map(|cast| (cast.spell_id, cast.cast_type))?;
    let Some(spell) = world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(spell_id))
        .cloned()
    else {
        warn!(
            "Cannot refresh movement permission for active cast {spell_id}: missing SpellStore row"
        );
        return None;
    };
    Some((spell, cast_type))
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
    let modded = scripts::blast_cast_time(world, caster, spell, modded);
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

/// Effects that have a handler, apply an aura or run a script hook; only
/// these select targets.
fn handled_effects(spell: &SpellInfo) -> impl Iterator<Item = &crate::spell_info::SpellEffect> {
    spell
        .effects
        .iter()
        .filter(|effect| selects_targets(spell, effect))
}

/// A `SPELL_EFFECT_DUMMY` effect does nothing itself but selects its targets, which the
/// spell then hits (`CreatureAI::SpellHit`); one at an unmodelled implicit target selects
/// nothing instead of failing the cast.
fn selects_targets(spell: &SpellInfo, effect: &crate::spell_info::SpellEffect) -> bool {
    effects::is_handled(effect)
        || cosmetic::is_visual_only(spell)
        || scripts::on_effect_hit(spell.id, effect.index).is_some()
        || scripts::cast_effect(spell.id, effect.index).is_some()
        || scripts::on_cast_effect(spell.id, effect.index).is_some()
        || (effect.effect == effects::EFFECT_DUMMY && targets::unsupported_target(effect).is_none())
}

fn max_targets(spell: &SpellInfo) -> u32 {
    spell
        .target_restrictions
        .as_ref()
        .map_or(0, |restrictions| restrictions.max_targets)
}

// --- Messaging ---

pub(crate) fn owner_connection(world: &mut World, player: Entity) -> Option<Entity> {
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

/// TC Spell.cpp:7648–7656 reports item-cap checks as equip errors, not CastFailed.
pub(crate) fn send_cast_rejection(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    failure: &CastFailure,
) {
    match failure.to_contract() {
        Ok((reason, detail)) => send_to_owner(
            world,
            caster,
            CastFailed {
                spell_id,
                reason,
                detail,
            },
        ),
        Err(error) => send_inventory_rejection(world, caster, error),
    }
}

fn send_inventory_rejection(
    world: &mut World,
    caster: Entity,
    error: shared::protocol::InventoryError,
) {
    let Some(connection) = owner_connection(world, caster) else {
        debug!(?caster, "Inventory rejection has no connected owner");
        return;
    };
    match world.get_mut::<MessageSender<shared::protocol::InventoryError>>(connection) {
        Some(mut sender) => sender.send::<shared::protocol::InventoryChannel>(error),
        None => warn!(
            ?connection,
            "Inventory rejection has no owner message sender"
        ),
    }
}

/// Send to the owners of the event's source and target (once each), including
/// guardian ancestry (`GetCharmerOrOwnerPlayerOrPlayerItself`), stamped with the server's
/// wall clock; damage also counts on the damage meters.
pub(crate) fn log_event(world: &mut World, mut event: CombatLogEvent) {
    event.timestamp_unix_ms = unix_millis();
    crate::followers::pull_history::record_event(world, &event);
    #[cfg(test)]
    if let Some(mut observed) = world.get_resource_mut::<test_harness::ObservedCombatEvents>() {
        observed.0.push(event.clone());
    }
    crate::scripted_ai::bossmod::cast_event(world, &event);
    crate::damage_meter::record_damage(world, &event);
    crate::pvp::record_arena_event(world, &event);
    scripts::record_health_change(world, &event);
    if event.kind == CombatLogKind::Death {
        let source = event.source.and_then(Entity::try_from_bits);
        let target = event.target.and_then(Entity::try_from_bits);
        if let (Some(killer), Some(victim)) = (source, target) {
            crate::achievement::record_delivered_killing_blow(world, killer);
            crate::achievement::record_player_killed_by(world, victim, killer);
            let owner = combat_event_owner(world, killer);
            crate::pvp::record_battleground_player_kill(world, owner, victim);
            crate::pvp::reward_world_player_kill(world, owner, victim);
            crate::quest::record_player_kill(world, owner, victim);
        }
    }
    let mut players: Vec<Entity> = Vec::new();
    for unit in [event.source, event.target]
        .into_iter()
        .flatten()
        .filter_map(Entity::try_from_bits)
    {
        let player = combat_log_recipient(world, unit);
        if !players.contains(&player) {
            players.push(player);
        }
    }
    for player in players {
        send_to_owner(world, player, event.clone());
    }
}

/// Spell constructors point to an already-existing caster, so ancestry is acyclic.
/// Broaden log delivery only; quest, PvP and meter credit retain their own policies.
fn combat_log_recipient(world: &World, unit: Entity) -> Entity {
    match world.get::<summon::SummonedCreature>(unit) {
        Some(summoned) => combat_log_recipient(world, summoned.owner),
        None => combat_event_owner(world, unit),
    }
}

pub(crate) fn combat_event_owner(world: &World, unit: Entity) -> Entity {
    if let Some(follower) = world.get::<crate::followers::Follower>(unit) {
        return follower.owner;
    }
    match world.get::<crate::hunter_pet::HunterPet>(unit) {
        Some(pet) => world
            .get::<crate::followers::Follower>(pet.owner)
            .map_or(pet.owner, |follower| follower.owner),
        None => unit,
    }
}

fn unix_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time after 1970")
        .as_millis() as u64
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

pub(crate) fn log_summon(world: &mut World, caster: Entity, actor: Entity, spell: &SpellInfo) {
    log_event(
        world,
        cast_event(caster, Some(actor), spell, CombatLogKind::Summon),
    );
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
        glancing: false,
        periodic: false,
        extra_spell_id: None,
        timestamp_unix_ms: 0,
        kind,
    }
}

pub(crate) fn mark_dirty(world: &World, entity: Entity) {
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
        debug!(caster = ?owner, request = ?request, "SpellCastRequest received (SpellCastIntent)");
        if let Err((spell_id, failure)) = request_cast(world, owner, &request) {
            debug!("Cast of {spell_id} by {owner} failed: {failure:?}");
            send_cast_rejection(world, owner, spell_id, &failure);
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
        target_item_guid: None,
        destination: None,
        spell_id: Some(spell_id),
        spell: String::new(),
        target_entity: None,
        witness: None,
    };
    request_cast(world, player, &request).map_err(|(spell_id, failure)| {
        send_cast_rejection(world, player, spell_id, &failure);
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
    cast_granted_with_item_context(world, caster, spell_id, target, None, None, None)
}

fn cast_granted_with_item_context(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    target: Option<Entity>,
    item_level: Option<u16>,
    retail_item_id: Option<u32>,
    item_cooldown: Option<ItemCooldown>,
) -> Result<(), CastFailure> {
    debug!(caster = ?caster, spell_id, target = ?target, moving = ?world.get::<Moving>(caster), active_cast = ?world.get::<CastState>(caster), "SpellCastRequest received (granted object/item cast)");
    let result = world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(spell_id))
        .cloned()
        .ok_or(CastFailure::UnknownSpell)
        .and_then(|mut spell| {
            spell.cast_item_level = item_level;
            spell.cast_retail_item_id = retail_item_id;
            if let Some(cooldown) =
                item_cooldown.filter(|cd| cd.category != 0 && cd.category_ms > 0)
            {
                spell
                    .categories
                    .get_or_insert_with(Default::default)
                    .category = cooldown.category;
                spell
                    .cooldowns
                    .get_or_insert_with(Default::default)
                    .category_recovery_ms = cooldown.category_ms;
            }
            start_cast(world, caster, &spell, target)
        });
    debug!(caster = ?caster, spell_id, result = ?result, "Granted cast acceptance result");
    if let Err(failure) = &result {
        send_cast_rejection(world, caster, spell_id, failure);
    }
    result
}

#[derive(Debug, Clone, Copy)]
struct ItemCooldownSource {
    recovery_ms: u32,
    category_ms: u32,
    category: u32,
}

/// The cooldown an item effect puts on its spell's category (`ItemEffect`
/// `SpellCategoryID` / `CategoryCoolDownMSec`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ItemCooldown {
    pub category: u32,
    pub category_ms: u32,
}

/// `Player::CastItemUseSpell` (Player.cpp:8790-8838): cast an item's on-use spell as the
/// player, spellbook aside, unless the item effect's category is cooling down
/// (`SPELL_FAILED_NOT_READY`); a started cast puts the category on that cooldown
/// (`SpellHistory::StartCooldown` with the item's `CategoryCoolDownMSec`).
pub(crate) fn cast_item_spell(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    target: Option<Entity>,
    cooldown: ItemCooldown,
    item_level: u16,
    retail_item_id: Option<u32>,
) -> Result<(), CastFailure> {
    let now = history_now(world);
    let cooling = cooldown.category != 0
        && world.get::<SpellHistory>(caster).is_some_and(|history| {
            history
                .category_ready_at
                .get(&cooldown.category)
                .is_some_and(|&ready| ready > now)
        });
    if cooling {
        send_cast_rejection(world, caster, spell_id, &CastFailure::OnCooldown);
        return Err(CastFailure::OnCooldown);
    }
    cast_granted_with_item_context(
        world,
        caster,
        spell_id,
        target,
        Some(item_level),
        retail_item_id,
        Some(cooldown),
    )
}

/// Toy casts use ItemEffect overrides (>=0) or SpellCooldowns (-1), including
/// the per-item recovery omitted by the older consumable-item path.
pub(crate) mod toy_area;
mod toy_facing;
pub(crate) mod toy_requirements;

pub(crate) fn cast_toy_spell(
    world: &mut World,
    caster: Entity,
    toy: &crate::toy_catalog::Toy,
    target: Option<Entity>,
    item_level: u16,
    request: &shared::protocol::UseToy,
) -> Result<(), CastFailure> {
    let spell_id = toy.snapshot.spell_id.ok_or(CastFailure::UnknownSpell)?;
    let mut spell = world
        .resource::<SpellStore>()
        .get(spell_id)
        .cloned()
        .ok_or(CastFailure::UnknownSpell)?;
    spell.cast_destination = request.destination;
    validate_toy_destination(world, caster, &spell)?;
    spell.cast_item_level = Some(item_level);
    let cooldowns = spell.cooldowns.get_or_insert_with(Default::default);
    if toy.item_cooldown_ms >= 0 {
        cooldowns.recovery_ms = toy.item_cooldown_ms as u32;
    }
    if toy.category_cooldown_ms >= 0 {
        cooldowns.category_recovery_ms = toy.category_cooldown_ms as u32;
    }
    if toy.category != 0 || toy.category_cooldown_ms >= 0 {
        spell
            .categories
            .get_or_insert_with(Default::default)
            .category = toy.category;
    }
    let result = start_cast(world, caster, &spell, target);
    if let Err(failure) = &result {
        send_cast_rejection(world, caster, spell_id, failure);
    }
    result
}

pub(crate) fn requires_target_witness(spell: &SpellInfo) -> bool {
    targets::explicit_requirement(spell.effects.iter()).is_some()
        && attributes(spell, 2) & SPELL_ATTR2_IGNORE_LINE_OF_SIGHT == 0
}

fn validate_toy_destination(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<(), CastFailure> {
    let cursor_target = spell
        .effects
        .iter()
        .any(|effect| effect.implicit_targets.contains(&87));
    if !cursor_target {
        return if spell.cast_destination.is_none() {
            Ok(())
        } else {
            Err(CastFailure::InvalidTarget)
        };
    }
    let point = spell
        .cast_destination
        .map(Vec3::from_array)
        .ok_or(CastFailure::NoTarget)?;
    if !point.is_finite() {
        return Err(CastFailure::InvalidTarget);
    }
    let origin = targets::position(world, caster).ok_or(CastFailure::InvalidTarget)?;
    let (min, max) = targets::spell_range(spell, false);
    let max = modify(world, caster, spell, SpellModOp::Range, max);
    let distance = origin.distance(point);
    if distance > max {
        return Err(CastFailure::OutOfRange);
    }
    if distance < min {
        return Err(CastFailure::TooClose);
    }
    Ok(())
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
        let failure = if disenchant::is_disenchant(&spell) {
            CastFailure::DisenchantNotKnown
        } else {
            CastFailure::NotKnown
        };
        return Err((spell.id, failure));
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
    let explicit = if disenchant::is_disenchant(&spell)
        || (request.target_item_guid.is_some()
            && spell.effects.iter().any(open_lock::is_item_effect))
    {
        None
    } else {
        explicit
    };
    let mut requested_spell = spell.clone();
    requested_spell.cast_destination = request.destination;
    requested_spell.cast_target_item_guid = request.target_item_guid;
    let effective_id = crate::auras::overrides::resolve_spell(world, caster, spell.id);
    let mut spell = world
        .resource::<SpellStore>()
        .get(effective_id)
        .expect("resolved spell exists")
        .clone();
    spell.cast_destination = request.destination;
    spell.cast_target_item_guid = request.target_item_guid;
    let delay = cast_queue_delay(world, caster, &spell);
    // Compare at CastState's precision: a widened f32 window can exceed its f64 value.
    if delay > 0.0 && delay as f32 <= SPELL_QUEUE_WINDOW_SECS {
        world.entity_mut(caster).insert(QueuedSpellCast {
            spell: requested_spell,
            explicit,
        });
        return Ok(());
    }
    if delay == 0.0 {
        // A newer executable request supersedes the held request too.
        world.entity_mut(caster).remove::<QueuedSpellCast>();
    }
    start_cast(world, caster, &spell, explicit).map_err(|failure| (spell.id, failure))
}

/// One request in the last 750 ms. TrinityCore a352b1fa Player.cpp:31545-31559 and Retail
/// use 400 ms; World of Osso widens it to the 750 ms hasted-GCD floor (user, 2026-10-15).
const SPELL_QUEUE_WINDOW_SECS: f32 = 0.75;

/// Player-request slot, distinct from the snapshot belonging to the active cast bar.
#[derive(Component)]
struct QueuedSpellCast {
    spell: SpellInfo,
    explicit: Option<Entity>,
}

fn cast_queue_delay(world: &World, caster: Entity, spell: &SpellInfo) -> f64 {
    let cast_remaining = world.get::<CastState>(caster).map_or(0.0, |cast| {
        f64::from((cast.duration - cast.elapsed).max(0.0))
    });
    let gcd_ready_at = world
        .get::<SpellHistory>(caster)
        .and_then(|history| history.gcd_ready_at.get(&gcd_category(spell)))
        .copied()
        .unwrap_or(0.0);
    cast_remaining.max((gcd_ready_at - history_now(world)).max(0.0))
}

/// Run after active cast completion and due missile effects, before power replication.
fn start_queued_casts(world: &mut World) {
    let mut pending = world.query::<(Entity, &QueuedSpellCast)>();
    let ready: Vec<_> = pending
        .iter(world)
        .filter(|(caster, queued)| {
            world.get::<CastState>(*caster).is_none()
                && cast_queue_delay(world, *caster, &queued.spell) == 0.0
        })
        .map(|(caster, _)| caster)
        .collect();
    for caster in ready {
        let queued = world.entity_mut(caster).take::<QueuedSpellCast>().unwrap();
        let effective_id = crate::auras::overrides::resolve_spell(world, caster, queued.spell.id);
        let mut effective_spell = world
            .resource::<SpellStore>()
            .get(effective_id)
            .expect("resolved queued spell exists")
            .clone();
        effective_spell.cast_destination = queued.spell.cast_destination;
        effective_spell.cast_target_item_guid = queued.spell.cast_target_item_guid;
        if let Err(failure) = start_cast(world, caster, &effective_spell, queued.explicit) {
            send_cast_rejection(world, caster, queued.spell.id, &failure);
        }
    }
}

/// Player requests are limited to known class and profession spells; NPC
/// casters are exempt.
pub(crate) fn caster_knows_spell(world: &mut World, caster: Entity, spell_id: u32) -> bool {
    if world.get::<Npc>(caster).is_some()
        || crate::mount_journal::knows_spell(world, caster, spell_id)
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
        || taught_by_known_spell(world, caster, spell_id)
        || crate::auras::overrides::grants_spell(world, caster, spell_id)
        || world.get::<KnownSpells>(caster).is_some_and(|known| {
            known
                .iter()
                .any(|base| crate::auras::overrides::resolve_spell(world, caster, base) == spell_id)
        })
}

/// `AuraEffect::HandleLearnSpell` (SpellAuraEffects.cpp:5248-5261): a known spell's
/// LEARN_SPELL 195 aura teaches its MiscValue (Skyriding Basics 376777 teaches Surge Forward
/// 372608 and Skyward Ascent 372610).
fn taught_by_known_spell(world: &World, caster: Entity, spell_id: u32) -> bool {
    let store = world.resource::<SpellStore>();
    let learned = world
        .get::<crate::profession::PlayerProfessions>(caster)
        .into_iter()
        .flat_map(|professions| professions.spells.iter().copied());
    let known = world
        .get::<KnownSpells>(caster)
        .into_iter()
        .flat_map(KnownSpells::iter);
    learned
        .chain(known)
        .filter_map(|id| store.get(id))
        .flat_map(|spell| &spell.effects)
        .any(|effect| {
            crate::auras::is_aura_effect(effect)
                && effect.aura == crate::auras::handlers::LEARN_SPELL
                && effect.misc_values[0] as u32 == spell_id
        })
}

fn ordinary_cast_allows_dead_caster(spell: &SpellInfo) -> bool {
    const ALLOW_CAST_WHILE_DEAD: u32 = 0x0080_0000;
    crate::auras::is_passive(spell) || attributes(spell, 0) & ALLOW_CAST_WHILE_DEAD != 0
}

/// TC CheckCast: ordinary passive spells and AllowCastWhileDead survive this check.
fn check_ordinary_caster_life(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<(), CastFailure> {
    if !targets::is_alive(world, caster) && !ordinary_cast_allows_dead_caster(spell) {
        return Err(CastFailure::CasterDead);
    }
    Ok(())
}

/// Validate everything checked before a cast starts; returns the explicit
/// target the spell will use.
#[derive(Clone, Copy, PartialEq, Eq)]
enum StartCheck {
    Native,
    IgnoreCastInProgress,
    TankHealCapability,
}

fn validate_start(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
    ignore_cast_in_progress: bool,
) -> Result<Option<Entity>, CastFailure> {
    let check = if ignore_cast_in_progress {
        StartCheck::IgnoreCastInProgress
    } else {
        StartCheck::Native
    };
    validate_start_with_policy(world, caster, spell, explicit, check)
}

/// Read ordinary native cast eligibility without casting, retargeting or spending.
#[cfg(test)]
pub(crate) fn read_native_cast_start(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    target: Entity,
) -> Result<Option<Entity>, CastFailure> {
    validate_start(world, caster, spell, Some(target), false)
}

/// Planner capability only: no cast, no spend, and no mutation of native GCD clocks.
/// All native checks remain except the in-progress veto and a bounded transient GCD.
pub(crate) fn read_tank_heal_capability(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    tank: Entity,
) -> Result<Option<Entity>, CastFailure> {
    validate_start_with_policy(
        world,
        caster,
        spell,
        Some(tank),
        StartCheck::TankHealCapability,
    )
}

pub(crate) fn read_targets_include(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    target: Entity,
) -> Result<bool, CastFailure> {
    let mut pending = vec![(spell.clone(), Some(target))];
    let mut visited = std::collections::HashSet::new();
    while let Some((spell, explicit)) = pending.pop() {
        if !visited.insert((spell.id, explicit)) {
            continue;
        }
        for effect in handled_effects(&spell) {
            let units = targets::select_targets(
                world,
                caster,
                explicit,
                &spell,
                effect,
                max_targets(&spell),
                false,
            )
            .map_err(|id| {
                CastFailure::Unsupported(format!("implicit target {id} not supported"))
            })?;
            if units.contains(&target) {
                return Ok(true);
            }
            if let Some(child) = read_periodic_trigger(world, effect)? {
                pending.extend(units.into_iter().map(|unit| (child.clone(), Some(unit))));
            }
        }
    }
    Ok(false)
}

fn read_periodic_trigger(
    world: &World,
    effect: &crate::spell_info::SpellEffect,
) -> Result<Option<SpellInfo>, CastFailure> {
    if effect.aura != crate::auras::handlers::PERIODIC_TRIGGER_SPELL || effect.trigger_spell == 0 {
        return Ok(None);
    }
    world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(effect.trigger_spell))
        .cloned()
        .map(Some)
        .ok_or_else(|| {
            CastFailure::Unsupported(format!(
                "periodic healing trigger {} lacks source metadata",
                effect.trigger_spell
            ))
        })
}

fn validate_start_with_policy(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
    check: StartCheck,
) -> Result<Option<Entity>, CastFailure> {
    check_ordinary_caster_life(world, caster, spell)?;
    casting_restriction::check_cast(world, caster, spell)?;
    shapeshift::check_cast(world, caster, spell)?;
    aura_spell_restrictions::check_caster(world, caster, spell)?;
    if let Some(block) = crate::auras::cast_blocker(world, caster, spell) {
        return Err(CastFailure::CrowdControlled(block));
    }
    toy_area::check_cast(world, caster, spell.id)?;
    validate_caster_state(world, caster, spell)?;
    toy_scripts::check_cast(world, caster, spell)?;
    mobile_banking::check_cast(world, caster, spell.id)?;
    crate::game_objects::cooking_focus::check_cast(world, caster, spell.id)?;
    teleport::check_destination(world, caster, spell)?;
    if check == StartCheck::Native && world.get::<CastState>(caster).is_some() {
        return Err(CastFailure::InProgress);
    }
    validate_history(world, caster, spell, check)?;
    let cast_blocked_by_moving =
        cast_time_ms(world, caster, spell) > 0 && !cast_allows_moving(world, caster, spell);
    let channel_blocked_by_moving =
        is_channeled(spell) && !channel_allows_moving(world, caster, spell);
    let blocked_by_moving = cast_blocked_by_moving || channel_blocked_by_moving;
    if blocked_by_moving && world.get::<Moving>(caster).is_some_and(|moving| moving.0) {
        return Err(CastFailure::Moving);
    }
    if scripts::target_plan(spell.id).is_none()
        && let Some(id) = handled_effects(spell).find_map(targets::unsupported_target)
    {
        return Err(CastFailure::Unsupported(format!(
            "implicit target {id} not supported"
        )));
    }
    let target = validate_target(world, caster, spell, explicit, Sight::Start)?;
    aura_spell_restrictions::check_target(world, caster, target, spell)?;
    toy_facing::check_cast(world, caster, spell.id, target)?;
    crate::hunter_pet::check_cast(world, caster, spell, target)?;
    spell_costs(world, caster, spell)?;
    crate::profession::check_cast_items(world, caster, spell)?;
    disenchant::check_cast(world, caster, spell)?;
    Ok(target)
}

/// `Spell::CheckCast`: a spell that cannot be used in combat fails in combat
/// (`CanBeUsedInCombat`, Spell.cpp:5891-5892); a mounted player casts only passive spells and
/// `SPELL_ATTR0_ALLOW_WHILE_MOUNTED` ones (Spell.cpp:6063-6072).
fn validate_caster_state(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<(), CastFailure> {
    let in_combat = world
        .get::<shared::components::CombatStatus>(caster)
        .is_some_and(|status| status.0);
    if in_combat && attributes(spell, 0) & SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL != 0 {
        return Err(CastFailure::AffectingCombat);
    }
    let mounted_player = world.get::<Player>(caster).is_some()
        && world.get::<shared::components::Mounted>(caster).is_some();
    if mounted_player
        && !crate::auras::is_passive(spell)
        && attributes(spell, 0) & SPELL_ATTR0_ALLOW_WHILE_MOUNTED == 0
    {
        return Err(CastFailure::NotMounted);
    }
    Ok(())
}

fn validate_history(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    check: StartCheck,
) -> Result<(), CastFailure> {
    let Some(history) = world.get::<SpellHistory>(caster) else {
        return Ok(());
    };
    let now = history_now(world);
    // TC source 0x2 skips SpellHistory::IsReady, including its school/charge
    // readiness checks; GCD and caster-aura checks are independent.
    if !spell.cast_ignore_cooldowns && history.school_locked(spell.school_mask(), now) {
        return Err(CastFailure::SchoolLockedOut);
    }
    let gcd_ready_at = history
        .gcd_ready_at
        .get(&gcd_category(spell))
        .copied()
        .unwrap_or(now);
    if history.on_gcd(spell, now)
        && !permits_transient_gcd(world, caster, spell, check, gcd_ready_at)
    {
        return Err(CastFailure::OnGlobalCooldown);
    }
    if spell.cast_ignore_cooldowns {
        return Ok(());
    }
    if history.on_cooldown(spell, now) {
        return Err(CastFailure::OnCooldown);
    }
    let mods = charge_mods(world, caster, spell);
    let mut history = history.clone();
    if history
        .charges_mut(spell, now, mods)
        .is_some_and(|charges| charges.used() >= charges.max)
    {
        return Err(CastFailure::NoCharges);
    }
    Ok(())
}

fn permits_transient_gcd(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    check: StartCheck,
    ready_at: f64,
) -> bool {
    if check != StartCheck::TankHealCapability {
        return false;
    }
    let duration = f64::from(gcd_duration_ms(world, caster, spell)) / 1000.0;
    ready_at <= history_now(world) + duration
}

/// Line of sight evidence when a cast checks its unit target (docs/specs/line-of-sight.md).
#[derive(Clone, Copy, Debug)]
pub(crate) enum Sight {
    /// Cast start: search using the caster and target's authoritative positions.
    Start,
    /// Cast finish: a creature searches again; player sight was checked at start.
    Finish,
}

/// `Spell::CheckCast` → `IsWithinLOS` (Spell.cpp:5996-6013, 9259-9272):
/// authoritative target visibility, never a client endpoint that aged in transit.
fn in_line_of_sight(world: &World, caster: Entity, target: Entity, sight: Sight) -> bool {
    if world.get::<Npc>(caster).is_some() {
        return crate::line_of_sight::sees_in_world(world, caster, target);
    }
    match sight {
        Sight::Start => {
            let held = crate::line_of_sight::sees_in_world(world, caster, target);
            debug!(?caster, ?target, held, "Authoritative cast LOS checked");
            held
        }
        Sight::Finish => true,
    }
}

/// Explicit target rules: required relation, alive, range, line of sight. Friendly
/// spells with no valid ally target fall back to the caster (retail auto self-cast).
fn validate_target(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
    sight: Sight,
) -> Result<Option<Entity>, CastFailure> {
    // TC SpellInfo::CheckTarget rejects explicit self before any selector runs.
    if explicit == Some(caster) && targets::excludes_caster(spell) {
        return Err(CastFailure::InvalidTarget);
    }
    if spell.id == shared::battleground::ALTERAC_OPENING_SPELL
        && explicit.is_some_and(|target| crate::pvp::is_alterac_banner(world, target))
    {
        let target = explicit.expect("Alterac Opening has an explicit banner");
        crate::pvp::check_banner_target(world, caster, target)?;
        return Ok(Some(target));
    }
    if spell.id == shared::battleground::AZERITE_GATHERING_SPELL {
        let target = explicit.ok_or(CastFailure::NoTarget)?;
        crate::pvp::check_azerite_target(world, caster, target)?;
        return Ok(Some(target));
    }
    if open_lock::is_item_cast(spell) {
        open_lock::check_cast(world, caster, spell)?;
        return Ok(None);
    }
    if spell.effects.iter().any(open_lock::is_item_effect)
        && explicit.is_none_or(|target| {
            world
                .get::<shared::protocol::GameObjectInfo>(target)
                .is_none()
        })
    {
        return Err(CastFailure::InvalidTarget);
    }
    if crate::gathering::is_gathering(spell) {
        let target = explicit.ok_or(CastFailure::NoTarget)?;
        crate::gathering::check_target(world, caster, spell, target)?;
        return Ok(Some(target));
    }
    let resurrects = spell
        .effects
        .iter()
        .any(crate::death::resurrection::is_resurrection);
    // Retail resurrection rows have both implicit targets zero; their effect still
    // requires an explicit dead ally and the normal range/LOS checks.
    let requirement = scripts::target_plan(spell.id)
        .map(|(relation, _)| relation)
        .or_else(|| targets::explicit_requirement(handled_effects(spell)))
        .or_else(|| {
            spell
                .effects
                .iter()
                .any(|effect| {
                    crate::death::resurrection::is_resurrection(effect)
                        && effect.implicit_targets == [0, 0]
                })
                .then_some(Relation::Ally)
        });
    let Some(relation) = requirement else {
        return Ok(explicit);
    };
    let target = match explicit {
        Some(target) if targets::relation_matches(world, caster, target, relation) => target,
        _ if relation == Relation::Ally => caster,
        Some(_) => return Err(CastFailure::InvalidTarget),
        None => return Err(CastFailure::NoTarget),
    };
    if target == caster && targets::excludes_caster(spell) {
        return Err(CastFailure::InvalidTarget);
    }
    // A unit of another map copy is not found (`ObjectAccessor::GetUnit` searches the
    // caster's map).
    let target_alive = targets::is_alive(world, target);
    let invalid_life = if target_alive {
        resurrects
    } else {
        !spell.allows_dead_target()
    };
    if invalid_life
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
        if attributes(spell, 2) & SPELL_ATTR2_IGNORE_LINE_OF_SIGHT == 0
            && !in_line_of_sight(world, caster, target, sight)
        {
            return Err(CastFailure::LineOfSight);
        }
    }
    scripts::check_target(world, caster, spell.id, target)?;
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
    applicable_costs(world, caster, spell)?
        .into_iter()
        .map(|(pool, amount, current)| {
            if current < amount {
                Err(match pool {
                    CostPool::Power(power) => CastFailure::NotEnoughPower(power),
                    CostPool::Health => CastFailure::NotEnoughHealth,
                })
            } else {
                Ok((pool, amount))
            }
        })
        .collect()
}

/// The costs `spell_costs` checks, with the caster's current value of each pool,
/// without requiring the caster to afford them.
pub(crate) fn applicable_costs(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<Vec<(CostPool, f32, f32)>, CastFailure> {
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
        let percent = cost.power_cost_pct / 100.0;
        let amount_with = |max: f32| cost.mana_cost as f32 + percent * max;
        if amount_with(0.0) == 0.0 && percent == 0.0 {
            continue;
        }
        let missing = match pool {
            CostPool::Power(power) => CastFailure::NotEnoughPower(power),
            CostPool::Health => CastFailure::NotEnoughHealth,
        };
        let (current, max) = value.ok_or(missing.clone())?;
        // Only a percentage cost reads the base; creatures have no CharacterStats.
        let base = match pool {
            CostPool::Power(PowerType::Mana) if percent != 0.0 => world
                .get::<CharacterStats>(caster)
                .map(|stats| stats.base_mana)
                .ok_or(missing.clone())?,
            _ => max,
        };
        let initial = amount_with(base);
        let school_modified =
            school_costs::modify_school_cost(world, caster, spell, cost.power_type, initial);
        let modified = match SpellModOp::power_cost(cost.order_index) {
            Some(op) => modify(world, caster, spell, op, school_modified),
            None => school_modified,
        };
        let modified = if pool == CostPool::Power(PowerType::Mana) {
            scripts::mana_cost(world, caster, spell, modified)
        } else {
            modified
        };
        // TC CalcPowerCost retains native negative generation, but modifiers
        // cannot turn an originally positive cost into a grant or vice versa.
        let amount = preserve_cost_sign(initial, modified);
        // OptionalCost is additional raw spend, never an admission requirement.
        let additional = resources::additional_optional_cost(world, caster, spell, cost.power_type);
        let optional = (cost.optional_cost as f32 + additional).max(0.0);
        let optional = if optional > 0.0 {
            match SpellModOp::power_cost(cost.order_index) {
                Some(op) => spell_mods::modify_percent(world, caster, spell, op, optional).max(0.0),
                None => optional,
            }
        } else {
            0.0
        };
        let amount = amount + optional.min((current - amount).max(0.0));
        costs.push((pool, amount, current));
    }
    Ok(costs)
}

fn preserve_cost_sign(initial: f32, modified: f32) -> f32 {
    if (initial < 0.0) == (modified < 0.0) {
        modified
    } else {
        0.0
    }
}

fn start_cast(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> Result<(), CastFailure> {
    let target = validate_start(world, caster, spell, explicit, false)?;
    begin_validated_cast(world, caster, spell, target)
}

fn begin_validated_cast(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    target: Option<Entity>,
) -> Result<(), CastFailure> {
    crate::stand_state::stand_to_cast(world, caster, attributes(spell, 0));
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
    start_channel(world, caster, spell, target);
    Ok(())
}

/// Shared by ordinary channels and the documented Penance DUMMY bridge.
fn start_channel(world: &mut World, caster: Entity, spell: &SpellInfo, target: Option<Entity>) {
    let channel_ms = duration_ms(world, caster, spell);
    if !is_channeled(spell) || channel_ms <= 0 {
        return;
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
}

fn insert_cast_state(world: &mut World, caster: Entity, spell: &SpellInfo, cast: &mut CastState) {
    cast.spell_name = spell.name.clone();
    cast.cast_while_moving = match cast.cast_type {
        CastType::Normal => cast_allows_moving(world, caster, spell),
        CastType::Channel => channel_allows_moving(world, caster, spell),
    };
    debug!(caster = ?caster, state = ?cast, "Cast state started");
    let pending = PendingCastSpell {
        spell: spell.clone(),
        caster_was_alive: targets::is_alive(world, caster),
    };
    world.entity_mut(caster).insert((cast.clone(), pending));
    mark_dirty(world, caster);
}

/// GCD bounds of `Spell::TriggerGlobalCooldown` (Spell.cpp:9192-9193).
const MIN_GCD_MS: u32 = 750;
const MAX_GCD_MS: u32 = 1500;

/// MOD_GLOBAL_COOLDOWN_BY_HASTE_REGEN 417 (Spell.cpp:9220-9224): when an aura of that
/// type affects `spell`, a GCD above 750 ms is multiplied by `ModHasteRegen`
/// (1 / (1 + haste), Unit::ApplyCastTimePercentMod, Unit.cpp:11058-11071) and kept
/// within 750-1500 ms.
fn haste_regen_gcd_ms(world: &World, caster: Entity, spell: &SpellInfo, gcd_ms: u32) -> u32 {
    if gcd_ms <= MIN_GCD_MS
        || !spell_mods::has_aura_type_affecting(
            world,
            caster,
            crate::auras::handlers::MOD_GLOBAL_COOLDOWN_BY_HASTE_REGEN,
            spell,
        )
    {
        return gcd_ms;
    }
    let haste = world
        .get::<CharacterStats>(caster)
        .map_or(1.0, |stats| 1.0 + stats.haste_pct / 100.0);
    ((gcd_ms as f32 / haste.max(0.01)) as u32).clamp(MIN_GCD_MS, MAX_GCD_MS)
}

fn start_gcd(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // Spell::CanHaveGlobalCooldown: only players and their controlled units (a creature
    // with a `CharmInfo`).
    if world.get::<Npc>(caster).is_some()
        && world.get::<crate::pet_control::PetCharm>(caster).is_none()
        && world.get::<crate::followers::Follower>(caster).is_none()
    {
        return;
    }
    let gcd_ms = gcd_duration_ms(world, caster, spell);
    let category = gcd_category(spell);
    if gcd_ms == 0 || category == 0 {
        return;
    }
    let until = history_now(world) + gcd_ms as f64 / 1000.0;
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

fn gcd_duration_ms(world: &World, caster: Entity, spell: &SpellInfo) -> u32 {
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
    // Only a 750-1500 ms GCD is modified (Spell.cpp:9195-9198).
    if (MIN_GCD_MS..=MAX_GCD_MS).contains(&base_gcd_ms) {
        haste_regen_gcd_ms(world, caster, spell, gcd_ms)
    } else {
        gcd_ms
    }
}

/// Recheck, plan targets, spend, start cooldowns, log CastSuccess, resolve.
pub(crate) fn execute_cast(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> Result<(), CastFailure> {
    check_ordinary_caster_life(world, caster, spell)?;
    mobile_banking::check_cast(world, caster, spell.id)?;
    druid_transform::before_cast(world, caster, spell, 0)?;
    casting_restriction::check_cast(world, caster, spell)?;
    aura_spell_restrictions::check_caster(world, caster, spell)?;
    toy_area::check_cast(world, caster, spell.id)?;
    crate::game_objects::cooking_focus::check_cast(world, caster, spell.id)?;
    let explicit = validate_target(world, caster, spell, explicit, Sight::Finish)?;
    aura_spell_restrictions::check_target(world, caster, explicit, spell)?;
    if cast_time_ms(world, caster, spell) > 0 {
        toy_facing::check_cast(world, caster, spell.id, explicit)?;
    }
    blooming_infusion::check_children(world, caster, spell)?;
    crate::hunter_pet::check_cast(world, caster, spell, explicit)?;
    let costs = spell_costs(world, caster, spell)?;
    crate::profession::check_cast_items(world, caster, spell)?;
    disenchant::check_cast(world, caster, spell)?;
    let mut prepared = resources::snapshot(world, caster, spell, &costs);
    prepared.cast_charge_mods = Some(spell_mods::snapshot_charged_modifiers(
        world, caster, &prepared,
    ));
    let spell = &prepared;
    let plan = plan_targets(world, caster, spell, explicit, false)?;
    if spell.id == shared::battleground::AZERITE_GATHERING_SPELL {
        debug!(caster = ?caster, spell_id = spell.id, explicit = ?explicit, plan = ?plan, "Cast execution target plan");
    }
    const DISCOUNT_POWER_ON_MISS: u32 = 0x0800_0000;
    let discounted_player =
        world.get::<Player>(caster).is_some() && attributes(spell, 1) & DISCOUNT_POWER_ON_MISS != 0;
    let primary_missed =
        explicit.is_some_and(|target| !plan.iter().any(|effect| effect.units.contains(&target)));
    let costs = discount_missed_rune_costs(costs, discounted_player && primary_missed);
    disenchant::roll_cast_loot(world, caster, spell, &mut rand::thread_rng())?;
    open_lock::unlock_cast(world, spell)?;
    crate::achievement::record_spell_cast(world, caster, spell.id);
    if let Some(item) = spell.cast_retail_item_id {
        // TC Spell.cpp3543-3546: one UseItem credit after successful cast checks.
        crate::achievement::record_item_use(world, caster, item);
    }
    take_cast_resources(world, caster, spell, &costs);
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
            reflected: false,
        },
    );
    missiles::launch(world, caster, spell, plan, 0);
    scripts::refund_execute(world, caster, spell, explicit, &costs);
    if let Some(hook) = scripts::after_cast(spell.id) {
        hook(world, caster, spell);
    }
    spell_mods::consume_charges(world, caster, spell);
    blooming_infusion::after_cast(world, caster, spell);
    druid_transform::after_cast(world, caster, spell);
    // Spell::finish (Spell.cpp:4448-4450): some spells stop the caster's auto-attack.
    if attributes(spell, 0) & SPELL_ATTR0_CANCELS_AUTO_ATTACK_COMBAT != 0 {
        crate::combat::attack_stop(world, caster);
    }
    auto_repeat::on_cast(world, caster, spell, explicit);
    Ok(())
}

/// TC TakePower/TakeRunePower: discounted primary misses neither consume
/// runes nor grant resources from negative costs. Admission still checks all costs.
fn discount_missed_rune_costs(costs: Vec<(CostPool, f32)>, missed: bool) -> Vec<(CostPool, f32)> {
    if !missed {
        return costs;
    }
    costs
        .into_iter()
        .filter(|(pool, amount)| *amount >= 0.0 && *pool != CostPool::Power(PowerType::Runes))
        .collect()
}

/// One handled effect's unit targets and destination.
#[derive(Debug, Clone)]
struct PlannedEffect {
    index: usize,
    units: Vec<Entity>,
    dest: Option<Dest>,
    /// A chained effect's damage and healing multiplier per unit: `EffectChainAmplitude`
    /// to the power of the unit's place in the chain (`m_damageMultipliers`,
    /// Spell.cpp:1840-1844, 8631-8636). Empty for an unchained effect.
    chain_multipliers: Vec<(Entity, f32)>,
    /// Native block hit result, retained across flight for these unit targets.
    blocked_units: Vec<Entity>,
    /// Source per-cast continuation state; retain hit receipts when the hook is suppressed.
    prevent_hit_script: bool,
}

impl PlannedEffect {
    fn chain_multiplier(&self, unit: Entity) -> f32 {
        self.chain_multipliers
            .iter()
            .find(|(chained, _)| *chained == unit)
            .map_or(1.0, |(_, multiplier)| *multiplier)
    }
}

type TargetPlan = Vec<PlannedEffect>;

/// The first effect whose targets `Spell::SelectSpellTargets` selects together with
/// effect `index` (Spell.cpp:741-781): an effect with the same implicit targets and, for
/// area selections, the same radii. Its selection (its chain) serves both.
fn target_group_leader(world: &World, spell: &SpellInfo, index: usize) -> usize {
    let effect = &spell.effects[index];
    spell.effects[..index]
        .iter()
        .position(|other| {
            other.effect != 0
                && targets::unsupported_target(other).is_none()
                && other.implicit_targets == effect.implicit_targets
                && world
                    .get_resource::<crate::conditions::SpellTargetConditions>()
                    .is_none_or(|conditions| {
                        conditions.share_targets(spell.id, other.index, effect.index)
                    })
                && other.attributes & EFFECT_ATTR_PLAYERS_ONLY
                    == effect.attributes & EFFECT_ATTR_PLAYERS_ONLY
                && (!targets::shared_selection_checks_radius(effect)
                    || (other.radius[0] == effect.radius[0] && other.radius[1] == effect.radius[1]))
        })
        .unwrap_or(index)
}

/// `SpellEffectAttributes::PlayersOnly` (DBCEnums.h:2432).
const EFFECT_ATTR_PLAYERS_ONLY: u32 = 0x0000_4000;

/// Targets of every handled effect, selected before any effect resolves. A
/// creature's melee-defense spell misses units by its miss chance
/// (`hit::remove_defended_targets`).
/// `ignore_target_check` (`TRIGGERED_IGNORE_TARGET_CHECK`, Spell.cpp:3489-3491): the
/// explicit unit is hit whatever its relation to the caster, as a reflected Frostbolt's
/// triggered damage hits its own caster.
fn plan_targets(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
    ignore_target_check: bool,
) -> Result<TargetPlan, CastFailure> {
    effects::check_power_drain_inputs(world, caster, spell)?;
    if open_lock::is_item_cast(spell) {
        return Ok(spell
            .effects
            .iter()
            .enumerate()
            .filter(|(_, effect)| open_lock::is_item_effect(effect))
            .map(|(index, _)| PlannedEffect {
                index,
                units: Vec::new(),
                dest: None,
                chain_multipliers: Vec::new(),
                blocked_units: Vec::new(),
                prevent_hit_script: false,
            })
            .collect());
    }
    if crate::gathering::is_gathering(spell) {
        let target = explicit.ok_or(CastFailure::NoTarget)?;
        return Ok(spell
            .effects
            .iter()
            .enumerate()
            .filter(|(_, effect)| matches!(effect.effect, 33 | 95))
            .map(|(index, _)| PlannedEffect {
                index,
                units: vec![target],
                dest: None,
                chain_multipliers: Vec::new(),
                blocked_units: Vec::new(),
                prevent_hit_script: false,
            })
            .collect());
    }
    if let Some((_, select)) = scripts::target_plan(spell.id) {
        let mut plan = select(world, caster, spell, explicit);
        hit::remove_defended_targets(world, caster, spell, &mut plan);
        return Ok(plan);
    }
    // Ebon's source damage-role filter owns its native cap after filtering;
    // retain implicit area admission instead of requiring an explicit target.
    let cap = if spell.id == crate::auras::ebon_might::EBON {
        0
    } else {
        max_targets(spell)
    };
    let mut plan = Vec::new();
    // Units per leading effect (`processedEffectsMaskForSpell`).
    let mut selected: Vec<(usize, Vec<Entity>)> = Vec::new();
    for (index, effect) in spell.effects.iter().enumerate() {
        if !selects_targets(spell, effect) {
            continue;
        }
        if effect.effect == crate::game_objects::summoned::EFFECT_TRANSMITTED {
            if let Some(reason) = crate::game_objects::summoned::unsupported_reason(world, effect) {
                return Err(CastFailure::Unsupported(reason));
            }
        }
        if effect.effect == crate::game_objects::summoned::EFFECT_SUMMON_OBJECT_WILD {
            if let Some(reason) =
                crate::game_objects::summoned::wild_unsupported_reason(world, effect)
            {
                return Err(CastFailure::Unsupported(reason));
            }
        }
        if effect.effect == disenchant::EFFECT_DISENCHANT {
            plan.push(PlannedEffect {
                index,
                units: Vec::new(),
                dest: None,
                chain_multipliers: Vec::new(),
                blocked_units: Vec::new(),
                prevent_hit_script: false,
            });
            continue;
        }
        let leader = target_group_leader(world, spell, index);
        let mut units = match selected.iter().find(|(selected, _)| *selected == leader) {
            Some((_, units)) => units.clone(),
            None => {
                let units = targets::select_targets(
                    world,
                    caster,
                    explicit,
                    spell,
                    &spell.effects[leader],
                    cap,
                    ignore_target_check,
                )
                .map_err(|id| {
                    CastFailure::Unsupported(format!("implicit target {id} not supported"))
                })?;
                selected.push((leader, units.clone()));
                units
            }
        };
        if let Some(filter) = scripts::area_target_filter(spell.id, effect.index) {
            filter(world, spell, explicit, &mut units);
        }
        warrior_stuns::filter_storm_bolt_targets(
            world, caster, spell, effect, explicit, &mut units,
        );
        let prevent_hit_script = if spell.id == omor_orbital::ORBITAL_TARGET && effect.index == 0 {
            omor_orbital::filter_targets(world, caster, &mut units)
        } else {
            false
        };
        let chain_multipliers = if spell.effects[leader].chain_targets > 1 {
            std::iter::successors(Some(1.0), |multiplier| {
                Some(multiplier * effect.chain_amplitude)
            })
            .zip(&units)
            .map(|(multiplier, &unit)| (unit, multiplier))
            .collect()
        } else {
            Vec::new()
        };
        let dest = if effect.effect == 254 && effect.implicit_targets.contains(&87) {
            spell.cast_destination.map(|point| targets::Dest {
                position: Vec3::from_array(point),
                orientation: targets::orientation(world, caster),
            })
        } else {
            targets::effect_dest(world, caster, explicit, effect, spell.cast_destination)
        };
        let mut dest = targets::ground_destination(world, caster, effect, dest)?;
        if effect.implicit_targets.contains(&32) {
            dest = targets::caster_summon_ground_destination(world, caster, spell, dest)?;
        }
        if let Some(dest) = &mut dest {
            scripts::relocate_destination(spell.id, effect, dest);
        }
        plan.push(PlannedEffect {
            index,
            units,
            dest,
            chain_multipliers,
            blocked_units: Vec::new(),
            prevent_hit_script,
        });
    }
    hit::remove_defended_targets(world, caster, spell, &mut plan);
    Ok(plan)
}

/// Generic TRIGGER_SPELL64 runs only in LAUNCH/LAUNCH_TARGET mode
/// (SpellEffects.cpp594–598); authored scripts keep their own hit handling.
fn launches(spell: &SpellInfo, effect: &crate::spell_info::SpellEffect) -> bool {
    effect.effect == effects::EFFECT_TRIGGER_SPELL
        && scripts::cast_effect(spell.id, effect.index).is_none()
        && scripts::effect_handler(spell.id, effect.index).is_none()
}

/// `Spell::HandleLaunchPhase` (Spell.cpp8493–8527): launch-mode effects resolve on
/// each planned unit when the spell launches, before any missile lands. Units
/// that missed or were immune were already removed from the plan.
fn resolve_launch_effects(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    plan: &TargetPlan,
    depth: u8,
) {
    for planned in plan {
        let effect = &spell.effects[planned.index];
        if !launches(spell, effect)
            || !scripts::effect_enabled(world, caster, spell.id, effect.index)
        {
            continue;
        }
        for &unit in &planned.units {
            if world.get_entity(unit).is_err() {
                continue;
            }
            let ctx = EffectCtx {
                caster,
                spell,
                depth,
                dest: planned.dest,
                chain_multiplier: planned.chain_multiplier(unit),
            };
            queued_triggers::schedule(world, &ctx, effect, unit);
        }
    }
}

fn resolve_effects(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    plan: &TargetPlan,
    depth: u8,
) {
    // Distinct hit units after missile/defense filtering, before effect dispatch.
    for unit in hit_units(plan) {
        crate::achievement::record_targeted_spell_hit(world, caster, unit, spell.id);
    }
    for effect in &spell.effects {
        if !selects_targets(spell, effect) {
            effects::log_unhandled(world, spell.id, effect.effect);
        }
    }
    if let Some(mut resolving) = world.get_resource_mut::<ResolvingHits>() {
        resolving.0.push(HitFrame {
            caster,
            spell_id: spell.id,
            depth,
            damage: Vec::new(),
            blocked: plan
                .iter()
                .flat_map(|effect| effect.blocked_units.iter().copied())
                .collect(),
        });
    }
    // Aura effects hitting the same unit form one aura (per-unit effect mask).
    let mut aura_masks: Vec<(Entity, u32)> = Vec::new();
    for planned in plan {
        let PlannedEffect {
            index, units, dest, ..
        } = planned;
        let effect = &spell.effects[*index];
        if spell.id == shared::battleground::AZERITE_GATHERING_SPELL {
            debug!(caster = ?caster, effect = ?effect, enabled = scripts::effect_enabled(world, caster, spell.id, effect.index), "Azerite effect dispatch");
        }
        if !scripts::effect_enabled(world, caster, spell.id, effect.index) {
            continue;
        }
        let ctx = EffectCtx {
            caster,
            spell,
            depth,
            dest: *dest,
            chain_multiplier: 1.0,
        };
        if let Some(hook) = scripts::on_cast_effect(spell.id, effect.index) {
            hook(world, &ctx, effect);
        }
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
        // Authored spell scripts own their effect before generic dispatch.
        if let Some(handler) = scripts::cast_effect(spell.id, effect.index) {
            handler(world, &ctx, effect);
            continue;
        }
        // TC EffectTriggerMissileSpell's HIT mode: destination effects have no
        // hit units, but still cast their child once with the parent's destination.
        if effect.effect == effects::EFFECT_TRIGGER_MISSILE && units.is_empty() {
            effects::trigger_missile_at_destination(world, &ctx, effect);
            continue;
        }
        // TC EffectSendEvent HANDLE_HIT runs targetless events once, not once per frame.
        // Unit-targeted events use the normal hit/immunity boundary below.
        if effect.effect == effects::EFFECT_SEND_EVENT && effect.implicit_targets == [0, 0] {
            effects::send_game_event(world, &ctx, effect, caster);
            continue;
        }
        // Item unlock is persisted before the cast success receipt; it has no
        // unit target and must not enter the gathering-node effect handler.
        if open_lock::is_item_cast(spell) && open_lock::is_item_effect(effect) {
            continue;
        }
        if let Some(handler) = effects::cast_handler(effect.effect) {
            handler(world, &ctx, effect);
            continue;
        }
        let handler = if launches(spell, effect) {
            None
        } else {
            scripts::effect_handler(spell.id, effect.index)
                .or_else(|| effects::handler(effect.effect))
        };
        let hook = scripts::on_effect_hit(spell.id, effect.index);
        for &unit in units {
            if crate::auras::effect_immunity::blocks_effect(world, unit, spell, effect) {
                effects::log_mechanic_immune(world, &ctx, unit);
                continue;
            }
            if let Some(handler) = handler
                && world.get_entity(unit).is_ok()
            {
                let ctx = EffectCtx {
                    chain_multiplier: planned.chain_multiplier(unit),
                    ..ctx
                };
                warrior_stuns::cast_hit_stun(world, &ctx, effect, unit);
                handler(world, &ctx, effect, unit);
            }
            if let Some(hook) = hook
                && !planned.prevent_hit_script
                && world.get_entity(unit).is_ok()
            {
                hook(world, caster, unit, depth);
            }
        }
    }
    for (target, effect_mask) in aura_masks {
        if scripts::redirect_rejuvenation(world, caster, target, spell.id, depth) {
            continue;
        }
        scripts::bloom_lifebloom_before_refresh(world, caster, target, spell.id, depth);
        crate::auras::apply_aura_with_spell(
            world,
            &AuraApplication {
                caster,
                target,
                spell_id: spell.id,
                effect_mask,
                source: AuraSource::Cast,
            },
            spell,
        );
    }
    run_hit_scripts(world, caster, spell, plan, depth);
    for unit in hit_units(plan) {
        crate::scripted_ai::spell_hit(world, unit, caster, spell.id);
    }
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
/// aggro or at an engaged unit. At launch the target's side is suppressed until the
/// spell lands (`missiles::launch`, `missiles::hit`).
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
    caster != target
        && targets::relation_matches(world, caster, target, Relation::Enemy)
        && attacks_with(world, spell)
        && (initial_aggro || engaged)
}

/// A harmful spell or one that dispels (`!IsPositive() || HasEffect(SPELL_EFFECT_DISPEL)`).
fn attacks_with(world: &World, spell: &SpellInfo) -> bool {
    world
        .get_resource::<SpellStore>()
        .is_some_and(|store| crate::spell_info::positivity::negative_effects(store, spell) != 0)
        || spell
            .effects
            .iter()
            .any(|effect| effect.effect == effects::EFFECT_DISPEL)
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
    depth: u8,
    damage: Vec<(Entity, f32)>,
    blocked: HashSet<Entity>,
}

/// Read the innermost matching hit's block result; nested casts own their frame.
fn hit_was_blocked(world: &World, caster: Entity, spell_id: u32, target: Entity) -> bool {
    world.get_resource::<ResolvingHits>().is_some_and(|hits| {
        hits.0.last().is_some_and(|frame| {
            frame.caster == caster && frame.spell_id == spell_id && frame.blocked.contains(&target)
        })
    })
}

/// A linked child inherits the active hit's trigger depth, without persisting cast state in an aura.
pub(crate) fn next_trigger_depth(world: &World) -> u8 {
    world
        .get_resource::<ResolvingHits>()
        .and_then(|hits| hits.0.last())
        .map_or(1, |hit| hit.depth.saturating_add(1))
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
                reflected: false,
            },
        );
    }
    for target in hit {
        // Spell::DoSpellHitOnUnit (Spell.cpp:2879-2882): a spell landing on a
        // unit the caster can attack starts combat with it.
        if targets::relation_matches(world, caster, target, Relation::Enemy) {
            crate::creature_aggro::add_threat(world, target, caster, 0.0);
            if attacks_with(world, spell) {
                crate::stand_state::stand_on_hostile_spell(world, target, attributes(spell, 3));
            }
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
                reflected: false,
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
    cast_triggered_with_item_level(world, caster, spell_id, explicit, depth, None);
}

/// `cast_triggered`, reporting whether the cast went off (TC `SPELL_CAST_OK`).
pub(crate) fn try_cast_triggered(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    explicit: Option<Entity>,
    depth: u8,
) -> bool {
    cast_triggered_with_item_level(world, caster, spell_id, explicit, depth, None)
}

pub(super) fn cast_triggered_with_item_level(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    explicit: Option<Entity>,
    depth: u8,
    item_level: Option<u16>,
) -> bool {
    let Some(mut spell) = load_triggered_spell(world, spell_id, depth) else {
        return false;
    };
    spell.cast_item_level = item_level;
    launch_triggered_spell(world, caster, &spell, explicit, depth)
}

/// The sourced CastSpellExtraArgs values carried by a real triggered cast.
#[derive(Default)]
pub(crate) struct TriggeredSpellOverrides {
    pub effect0_amount: Option<f32>,
    pub aura_stacks: Option<u8>,
    pub original_caster: Option<Entity>,
}

pub(crate) fn cast_triggered_with_overrides(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    explicit: Option<Entity>,
    depth: u8,
    values: TriggeredSpellOverrides,
) -> bool {
    let Some(mut spell) = load_triggered_spell(world, spell_id, depth) else {
        return false;
    };
    spell.cast_aura_stacks = values.aura_stacks;
    spell.cast_original_caster = values.original_caster;
    if let Some(amount) = values.effect0_amount {
        let Some(effect) = spell.effects.iter_mut().find(|effect| effect.index == 0) else {
            warn!("Triggered spell {spell_id} skipped: effect0 override has no effect0");
            return false;
        };
        effect.base_points = amount;
    }
    // The existing missile snapshot clones the whole per-cast SpellInfo.
    launch_triggered_spell(world, caster, &spell, explicit, depth)
}

/// Source AddSpellBP0 for Ysera's Gift's percent-of-owner-health children.
pub(crate) fn cast_triggered_with_effect0_amount(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    explicit: Option<Entity>,
    depth: u8,
    amount: f32,
) {
    cast_triggered_with_overrides(
        world,
        caster,
        spell_id,
        explicit,
        depth,
        TriggeredSpellOverrides {
            effect0_amount: Some(amount),
            ..Default::default()
        },
    );
}

/// Full-mask triggered cast at an authored destination, retaining native selectors and flight.
pub(crate) fn cast_triggered_at_destination(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    position: Vec3,
    depth: u8,
) {
    let Some(mut spell) = load_triggered_spell(world, spell_id, depth) else {
        return;
    };
    spell.cast_destination = Some(position.to_array());
    launch_triggered_spell(world, caster, &spell, None, depth);
}

fn load_triggered_spell(world: &World, spell_id: u32, depth: u8) -> Option<SpellInfo> {
    if depth > effects::MAX_TRIGGER_DEPTH {
        warn!(
            "Triggered spell {spell_id} skipped: depth {depth} exceeds {}",
            effects::MAX_TRIGGER_DEPTH
        );
        return None;
    }
    let spell = world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(spell_id))
        .cloned();
    if spell.is_none() {
        warn!("Triggered spell {spell_id} not found");
    }
    spell
}

fn plan_triggered_spell(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
    depth: u8,
) -> Result<TargetPlan, CastFailure> {
    druid_transform::before_cast(world, caster, spell, depth)?;
    aura_spell_restrictions::check_caster(world, caster, spell)?;
    // CheckTarget's TARGETS_DEAD is independent of BAD_TARGETS/relation bypass.
    // Non-unit objects have no Health and do not take this unit-only check.
    let target_dead = explicit
        .and_then(|target| world.get::<Health>(target))
        .is_some_and(|health| health.current <= 0.0);
    if target_dead && !spell.allows_dead_target() {
        return Err(CastFailure::InvalidTarget);
    }
    aura_spell_restrictions::check_target(world, caster, explicit, spell)?;
    plan_targets(world, caster, spell, explicit, true)
}

fn launch_triggered_spell(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
    depth: u8,
) -> bool {
    match plan_triggered_spell(world, caster, spell, explicit, depth) {
        Ok(plan) => {
            #[cfg(test)]
            if let Some(mut observed) =
                world.get_resource_mut::<test_harness::ObservedTriggeredCasts>()
            {
                observed.0.push((caster, spell.id, explicit));
            }
            crate::achievement::record_spell_cast(world, caster, spell.id);
            missiles::launch(world, caster, spell, plan, depth);
            druid_transform::after_cast(world, caster, spell);
            true
        }
        Err(failure) => {
            warn!("Triggered spell {} skipped: {failure:?}", spell.id);
            false
        }
    }
}

/// Effect64 clears IGNORE_POWER_COST/IGNORE_REAGENT_COST from FULL_MASK.
pub(super) fn cast_costed_trigger_with_context(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    explicit: Option<Entity>,
    depth: u8,
    item_level: Option<u16>,
    destination: Option<[f32; 3]>,
) -> bool {
    let Some(mut spell) = load_triggered_spell(world, spell_id, depth) else {
        return false;
    };
    spell.cast_item_level = item_level;
    spell.cast_destination = destination;
    match execute_costed_trigger(world, caster, &spell, explicit, depth) {
        Ok(()) => true,
        Err(failure) => {
            warn!("Queued spell {spell_id} skipped: {failure:?}");
            false
        }
    }
}

fn execute_costed_trigger(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
    depth: u8,
) -> Result<(), CastFailure> {
    druid_transform::before_cast(world, caster, spell, depth)?;
    aura_spell_restrictions::check_caster(world, caster, spell)?;
    let explicit = validate_target(world, caster, spell, explicit, Sight::Start)?;
    aura_spell_restrictions::check_target(world, caster, explicit, spell)?;
    let costs = spell_costs(world, caster, spell)?;
    crate::profession::check_cast_items(world, caster, spell)?;
    let prepared = resources::snapshot(world, caster, spell, &costs);
    let plan = plan_targets(world, caster, &prepared, explicit, true)?;
    take_cast_resources(world, caster, &prepared, &costs);
    crate::achievement::record_spell_cast(world, caster, prepared.id);
    missiles::launch(world, caster, &prepared, plan, depth);
    druid_transform::after_cast(world, caster, &prepared);
    Ok(())
}

/// Native TakePower/TakeReagents still check costs before this payment boundary.
fn take_cast_resources(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    costs: &[(CostPool, f32)],
) {
    const SPELL_ATTR6_DO_NOT_CONSUME_RESOURCES: u32 = 0x20;
    if attributes(spell, 6) & SPELL_ATTR6_DO_NOT_CONSUME_RESOURCES != 0 {
        return;
    }
    spend_costs(world, caster, costs);
    scripts::empower_gargoyles(world, caster, costs);
    crate::profession::take_reagents(world, caster, spell);
}

fn spend_costs(world: &mut World, caster: Entity, costs: &[(CostPool, f32)]) {
    for &(pool, amount) in costs {
        match pool {
            CostPool::Power(power) => {
                powers::change_power(world, caster, power, -amount);
            }
            CostPool::Health => {
                if let Some(mut health) = world.get_mut::<Health>(caster) {
                    health.current = (health.current - amount).clamp(1.0, health.max);
                }
            }
        }
    }
    mark_dirty(world, caster);
}

/// TC StartCooldown multiplies both spell and category cooldowns by ModHasteRegen.
fn haste_regen_cooldown_ms(world: &World, caster: Entity, spell: &SpellInfo, duration: f32) -> f32 {
    if !spell_mods::has_aura_type_affecting(
        world,
        caster,
        crate::auras::handlers::MOD_COOLDOWN_BY_HASTE_REGEN,
        spell,
    ) {
        return duration;
    }
    let haste = world
        .get::<CharacterStats>(caster)
        .map_or(1.0, |stats| 1.0 + stats.haste_pct / 100.0);
    // Native Duration(int64(...)) truncates fractional milliseconds.
    (duration / haste.max(0.01)).trunc()
}

/// TC a352b1fa SpellHistory.cpp:469-481: native category341 is independent
/// of spell class masks and applies after ordinary cooldown/haste modifiers.
fn category_cooldown_modifier_ms(world: &World, caster: Entity, category: u32) -> f32 {
    if category == 0 {
        return 0.0;
    }
    let Some(auras) = world.get::<crate::auras::ActiveAuras>(caster) else {
        return 0.0;
    };
    let mut total = 0.0;
    for aura in auras.iter() {
        for effect in &aura.effects {
            if effect.aura_type == crate::auras::handlers::MOD_SPELL_CATEGORY_COOLDOWN
                && u32::try_from(effect.misc_values[0]).ok() == Some(category)
            {
                total += (effect.amount * f32::from(aura.stacks)).trunc();
            }
        }
    }
    total
}

fn start_cooldowns(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // SpellHistory::HandleCooldowns returns before consuming a charge or starting
    // a clock when source flag 0x2 is set. The cast's GCD remains independent.
    if spell.cast_ignore_cooldowns {
        return;
    }
    let now = history_now(world);
    let ready_credit = control_dream::ready_credit_ms(world, caster, spell, now);
    let (base_recovery_ms, base_category_ms) = spell
        .cooldowns
        .as_ref()
        .map_or((0, 0), |cd| (cd.recovery_ms, cd.category_recovery_ms));
    let base_recovery_ms = if spell.id == mobile_banking::MOBILE_BANKING {
        shared::guild_perks::MOBILE_BANKING_COOLDOWN_2H
    } else {
        base_recovery_ms
    };
    let category = spell.categories.as_ref().map_or(0, |c| c.category);
    let category_modifier_ms = category_cooldown_modifier_ms(world, caster, category);
    let cooldown = |base: u32| {
        if base == 0 {
            return 0;
        }
        let modified = modify(world, caster, spell, SpellModOp::Cooldown, base as f32);
        let hasted = haste_regen_cooldown_ms(world, caster, spell, modified);
        let rate = recovery_rate::multiplier(world, caster, spell);
        let recovered = if rate == 1.0 {
            hasted
        } else {
            (f64::from(hasted) * rate).trunc() as f32
        };
        let adjusted = if recovered > 0.0 {
            recovered + category_modifier_ms
        } else {
            recovered
        };
        adjusted.max(0.0).round() as u32
    };
    let recovery_ms = cooldown(base_recovery_ms).saturating_sub(ready_credit);
    let category_ms = cooldown(base_category_ms).saturating_sub(ready_credit);
    let mods = charge_mods(world, caster, spell);
    let mut entity = world.entity_mut(caster);
    let mut history = entity.entry::<SpellHistory>().or_default();
    let mut history = history.get_mut();
    if spell.cast_item_level.is_some() {
        history.item_cooldown_sources.insert(
            spell.id,
            ItemCooldownSource {
                recovery_ms: base_recovery_ms,
                category_ms: base_category_ms,
                category,
            },
        );
    } else {
        history.item_cooldown_sources.remove(&spell.id);
    }
    // A fully discounted authored cooldown becomes ready now, not at its old deadline.
    if base_recovery_ms > 0 || base_category_ms > 0 {
        // TC SpellHistory.cpp:505: when spell recovery is zero, its entry
        // lasts until category recovery, so ResetCooldown can erase both.
        let spell_ms = if recovery_ms > 0 {
            recovery_ms
        } else {
            category_ms
        };
        history
            .spell_ready_at
            .insert(spell.id, now + spell_ms as f64 / 1000.0);
    }
    if category != 0 && base_category_ms > 0 {
        history.category_spell_ids.insert(category, spell.id);
        history
            .category_ready_at
            .insert(category, now + category_ms as f64 / 1000.0);
    }
    let charges = history.charges_mut(spell, now, mods).map(|charges| {
        charges.consume(now, ready_credit);
        charges.spell_id = spell.id;
        charges.snapshot(now)
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
        send_to_owner(world, caster, charges);
    }
}

/// `SpellHistory::RestoreCharge` (a352b1fa SpellHistory.cpp:831-840) `count` times: give back the
/// most recently used charges of `charge_category`; the oldest keeps recovering.
pub(crate) fn restore_charges(world: &mut World, unit: Entity, charge_category: u32, count: u8) {
    let now = history_now(world);
    let Some(mut history) = world.get_mut::<SpellHistory>(unit) else {
        return;
    };
    let Some(charges) = history.charges.get_mut(&charge_category) else {
        return;
    };
    charges.refresh(now);
    if charges.pending.is_empty() || count == 0 {
        return;
    }
    let retained = charges.pending.len().saturating_sub(usize::from(count));
    charges.pending.truncate(retained);
    if charges.pending.is_empty() {
        charges.fully_ready_at = now;
    }
    let update = charges.snapshot(now);
    send_to_owner(world, unit, update);
}

// --- Cast progress ---

/// Interrupt an active cast (stop, movement, death, crowd control, teleport, evade):
/// `Spell::cancel` with SPELL_FAILED_INTERRUPTED.
pub(crate) fn cancel_cast(world: &mut World, caster: Entity) {
    interrupt_cast(world, caster, CastFailReason::Interrupted, None);
}

/// `Unit::CastStop` / cancel-cast `InterruptNonMeleeSpells`: the active cast and the
/// autorepeat spell (Unit.cpp1191–1196, 3251–3259).
pub(crate) fn stop_all_casts(world: &mut World, caster: Entity) {
    auto_repeat::cancel(world, caster);
    cancel_cast(world, caster);
}

/// `Spell::cancel(result, ..., failedBy)`: end the cast and tell the caster's set why
/// (`Spell::SendInterrupted`, Spell.cpp:3613, 5177).
pub(crate) fn interrupt_cast(
    world: &mut World,
    caster: Entity,
    reason: CastFailReason,
    failed_by: Option<Entity>,
) {
    let Some(cast) = end_cast(world, caster) else {
        return;
    };
    debug!(caster = ?caster, spell_id = cast.spell_id, reason = ?reason, failed_by = ?failed_by, "Cast interrupted");
    send_spell_failure(world, caster, cast.spell_id, reason, failed_by);
}

/// `SMSG_SPELL_FAILURE` to every client that replicates the caster.
fn send_spell_failure(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    reason: CastFailReason,
    failed_by: Option<Entity>,
) {
    send_to_set(
        world,
        caster,
        SpellFailure {
            caster: caster.to_bits(),
            spell_id,
            reason,
            failed_by: failed_by.map(Entity::to_bits),
        },
    );
}

/// Remove an active cast without reporting it: a channel ran out or handed over.
pub(crate) fn end_cast(world: &mut World, caster: Entity) -> Option<CastState> {
    crate::crafting_orders::interrupt(world, caster);
    world.entity_mut(caster).remove::<PendingCastSpell>();
    world
        .entity_mut(caster)
        .remove::<crate::profession::CraftQueue>();
    let cast = world.entity_mut(caster).take::<CastState>()?;
    debug!(caster = ?caster, state = ?cast, "Cast state ended");
    mark_dirty(world, caster);
    if cast.cast_type == CastType::Channel {
        crate::auras::channel_ended(world, caster, cast.spell_id);
        crate::meeting_stone::ritual::channel_ended(world, caster, cast.spell_id);
    }
    Some(cast)
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
        // Explicit cancellation clears the held request as well as the active cast.
        // Hostile interrupts, movement and death do not: they revalidate at release.
        // TrinityCore a352b1fa SpellHandler.cpp:247-255.
        if world.get::<CastState>(owner).is_some() {
            world.entity_mut(owner).remove::<QueuedSpellCast>();
        }
        stop_all_casts(world, owner);
    }
}

/// Creature::Update does not run Unit events in DEAD (after corpse removal).
fn npc_event_updates_suspended(world: &World, caster: Entity) -> bool {
    world.get::<Npc>(caster).is_some()
        && world.get::<crate::combat::DeathState>(caster).is_some()
        && world.get::<crate::combat::Corpse>(caster).is_none()
}

fn cast_can_remain_dead(world: &World, caster: Entity) -> bool {
    world
        .get::<PendingCastSpell>(caster)
        .is_some_and(|pending| {
            !pending.caster_was_alive && ordinary_cast_allows_dead_caster(&pending.spell)
        })
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
        let alive = targets::is_alive(world, caster);
        if !alive && npc_event_updates_suspended(world, caster) {
            continue;
        }
        let movement_interrupts = !cast_while_moving && moving;
        let death_interrupts = !alive && !cast_can_remain_dead(world, caster);
        if movement_interrupts || death_interrupts {
            debug!(caster = ?caster, moving, alive, cast_while_moving, "Cast progress interrupted by movement or death");
            cancel_cast(world, caster);
            continue;
        }
        if let Some(mut pending) = world.get_mut::<PendingCastSpell>(caster)
            && pending.caster_was_alive != alive
        {
            pending.caster_was_alive = alive;
        }
        tick_and_resolve_cast(world, caster, dt);
    }
}

fn tick_and_resolve_cast(world: &mut World, caster: Entity, dt: f32) {
    let Some(mut cast) = world.get_mut::<CastState>(caster) else {
        return;
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
        CastTickResult::ChannelComplete => {
            crate::auras::channel_completed(world, caster, spell_id);
            end_cast(world, caster);
            crate::scripted_ai::dispatch(
                world,
                caster,
                crate::scripted_ai::Hook::ChannelFinished { spell_id },
            );
        }
        CastTickResult::Completed => {
            debug!(caster = ?caster, spell_id, target = ?target, "Cast state completed");
            world.entity_mut(caster).remove::<CastState>();
            complete_cast(world, caster, spell_id, target);
        }
    }
}

/// The spell snapshot belonging to a cast bar, including casting-item scaling.
#[derive(Component)]
struct PendingCastSpell {
    spell: SpellInfo,
    /// Life observed at insertion or the previous progress tick, not a native flag.
    caster_was_alive: bool,
}

fn complete_cast(world: &mut World, caster: Entity, spell_id: u32, target: Option<Entity>) {
    let Some(PendingCastSpell { spell, .. }) = world.entity_mut(caster).take::<PendingCastSpell>()
    else {
        debug!(caster = ?caster, spell_id, "Cast completion has no pending spell snapshot");
        return;
    };
    if let Err(failure) = execute_and_channel(world, caster, &spell, target) {
        crate::crafting_orders::interrupt(world, caster);
        debug!("Cast of {spell_id} by {caster} failed on completion: {failure:?}");
        // Equip errors use SPELL_FAILED_DONT_REPORT, never a fabricated spell failure.
        if let Ok((reason, _)) = failure.to_contract() {
            send_spell_failure(world, caster, spell_id, reason, None);
        }
        send_cast_rejection(world, caster, spell_id, &failure);
    }
}

#[cfg(test)]
mod additional_optional_power_tests;
#[cfg(test)]
mod class_resource_tests;
#[cfg(test)]
mod energizing_elixir_tests;
#[cfg(test)]
mod line_of_sight_tests;
#[cfg(test)]
mod live_ground_destination_tests;
#[cfg(test)]
mod mechanics_talent_tests;
#[cfg(test)]
mod mechanics_tests;
#[cfg(test)]
mod new_class_tests;
#[cfg(test)]
mod optional_flat_cost_tests;
#[cfg(test)]
mod override_queue_tests;
#[cfg(test)]
mod power_drain_tests;
#[cfg(test)]
mod prevent_regen_tests;
#[cfg(test)]
mod queue_tests;
#[cfg(test)]
mod regen_tests;
#[cfg(test)]
mod rezrtap_tests;
#[cfg(test)]
mod rogue_stealth_tests;
#[cfg(test)]
mod source_cast_flags_tests;
#[cfg(test)]
mod target32_grounding_tests;
#[cfg(test)]
mod teleport_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod water_shield_mana_tests;
