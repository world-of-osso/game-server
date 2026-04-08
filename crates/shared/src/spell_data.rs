//! Static spell definitions — the data backbone for abilities.
//!
//! Each spell has an ID, name, school, cast time, cooldown, resource cost,
//! up to 3 effects, and AP/SP scaling coefficients. This data comes from
//! retail WoW DB2 tables (SpellName, SpellMisc, SpellEffect, SpellPower)
//! imported into world.db.

use serde::{Deserialize, Serialize};

use crate::components::SpellSchool;

/// Resource type consumed by a spell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceType {
    Mana,
    Rage,
    Energy,
    ComboPoints,
    HolyPower,
    RunicPower,
    Focus,
}

/// A single spell effect within a spell definition.
///
/// Each spell can have up to 3 effects (matching WoW's 3-effect-per-spell limit).
/// Ref: AzerothCore `SpellEffects.cpp`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SpellEffectDef {
    /// Direct damage: `base_min..base_max + AP * ap_coeff + SP * sp_coeff`.
    SchoolDamage {
        base_min: f32,
        base_max: f32,
        ap_coefficient: f32,
        sp_coefficient: f32,
    },
    /// Direct heal: `base_min..base_max + AP * ap_coeff + SP * sp_coeff`.
    Heal {
        base_min: f32,
        base_max: f32,
        ap_coefficient: f32,
        sp_coefficient: f32,
    },
    /// Apply an aura (buff/debuff/DoT/HoT) by referencing an aura definition.
    ApplyAura {
        /// The aura spell_id to apply on the target.
        aura_spell_id: u32,
    },
    /// Energize: restore resource to the caster or target.
    Energize { resource: ResourceType, amount: f32 },
    /// Dispel auras of a specific school from the target.
    Dispel { school: SpellSchool },
    /// Interrupt the target's current cast and lock out the school.
    Interrupt {
        /// School lockout duration in seconds.
        lockout_duration: f32,
    },
}

/// Whether a spell targets the caster, a friendly, or an enemy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpellTarget {
    /// Targets self (buffs, self-heals).
    Self_,
    /// Targets a friendly unit (heals, buffs).
    Friendly,
    /// Targets a hostile unit (damage, debuffs).
    Hostile,
}

/// Static definition of a spell.
///
/// This is **immutable data** loaded at startup from the spell database.
/// Runtime state (cooldowns, cast progress) lives elsewhere.
///
/// Ref: AzerothCore `SpellInfo`, SimC `spell_data_t`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpellData {
    /// Unique spell identifier (matches WoW spell IDs).
    pub id: u32,
    /// Display name.
    pub name: String,
    /// Damage/healing school.
    pub school: SpellSchool,
    /// Base cast time in seconds (0.0 = instant).
    pub cast_time: f32,
    /// Cooldown in seconds (0.0 = no cooldown, GCD still applies).
    pub cooldown: f32,
    /// Resource cost. `None` = no cost (e.g. warrior abilities that generate rage).
    pub cost: Option<SpellCost>,
    /// Targeting type.
    pub target: SpellTarget,
    /// Maximum range in yards (0.0 = melee range, typically 5y).
    pub range: f32,
    /// Whether the spell can be cast while moving.
    pub cast_while_moving: bool,
    /// Whether the cast can be interrupted by damage.
    pub interruptible: bool,
    /// Up to 3 effects (matches WoW's 3-effect-per-spell limit).
    pub effects: [Option<SpellEffectDef>; 3],
}

impl SpellData {
    /// Whether this spell is instant (cast_time == 0).
    /// Instant spells bypass the cast bar but still trigger the GCD.
    pub fn is_instant(&self) -> bool {
        self.cast_time <= 0.0
    }
}

/// Resource cost for casting a spell.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpellCost {
    pub resource: ResourceType,
    /// Flat amount (e.g. 3000 mana, 30 rage, 40 energy).
    pub amount: f32,
}

// --- Cast validation ---

/// Why a spell cast was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastFailReason {
    /// No target selected (for targeted spells).
    NoTarget,
    /// Target is out of range.
    OutOfRange,
    /// Target is not valid for this spell (hostile vs friendly mismatch).
    InvalidTarget,
    /// Not enough resource (mana, rage, energy, etc.).
    NotEnoughResource,
    /// Spell is on cooldown.
    OnCooldown,
    /// Global cooldown is active.
    OnGlobalCooldown,
    /// Caster is dead.
    CasterDead,
    /// Caster is moving and the spell requires standing still.
    CantCastWhileMoving,
}

/// Context needed to validate a cast attempt.
#[derive(Debug, Clone, Copy)]
pub struct CastContext {
    /// Distance from caster to target in yards. Ignored for Self_ spells.
    pub distance: f32,
    /// Whether the target is friendly to the caster.
    pub target_is_friendly: bool,
    /// Whether a target entity exists.
    pub has_target: bool,
    /// Whether the caster is alive.
    pub caster_alive: bool,
    /// Current resource amount for the spell's cost type. `None` if no pool.
    pub resource_available: Option<f32>,
    /// Remaining cooldown in seconds (0.0 = ready).
    pub cooldown_remaining: f32,
    /// Remaining GCD in seconds (0.0 = ready).
    pub gcd_remaining: f32,
    /// Whether the caster is currently moving.
    pub caster_moving: bool,
}

/// Effective melee range used when spell range is 0.
const MELEE_RANGE: f32 = 5.0;

/// Validate whether a spell can be cast. Returns `Ok(())` or the first
/// failure reason.
///
/// Checks in order (matching WoW priority):
/// 1. Caster alive
/// 2. GCD ready
/// 3. Moving check (non-instant, non-cast-while-moving)
/// 4. Target exists (for non-self spells)
/// 5. Target validity (hostile/friendly match)
/// 6. Range
/// 7. Resource cost
/// 8. Cooldown
///
/// Ref: AzerothCore `Spell::CheckCast()`.
pub fn validate_cast(spell: &SpellData, ctx: &CastContext) -> Result<(), CastFailReason> {
    if !ctx.caster_alive {
        return Err(CastFailReason::CasterDead);
    }

    if ctx.gcd_remaining > 0.0 {
        return Err(CastFailReason::OnGlobalCooldown);
    }

    if ctx.caster_moving && !spell.is_instant() && !spell.cast_while_moving {
        return Err(CastFailReason::CantCastWhileMoving);
    }

    if spell.target != SpellTarget::Self_ {
        if !ctx.has_target {
            return Err(CastFailReason::NoTarget);
        }
        let target_ok = match spell.target {
            SpellTarget::Hostile => !ctx.target_is_friendly,
            SpellTarget::Friendly => ctx.target_is_friendly,
            SpellTarget::Self_ => true,
        };
        if !target_ok {
            return Err(CastFailReason::InvalidTarget);
        }
        let max_range = if spell.range > 0.0 {
            spell.range
        } else {
            MELEE_RANGE
        };
        if ctx.distance > max_range {
            return Err(CastFailReason::OutOfRange);
        }
    }

    if let Some(cost) = &spell.cost {
        let available = ctx.resource_available.unwrap_or(0.0);
        if available < cost.amount {
            return Err(CastFailReason::NotEnoughResource);
        }
    }

    if ctx.cooldown_remaining > 0.0 {
        return Err(CastFailReason::OnCooldown);
    }

    Ok(())
}

// --- Effect processing ---

/// Caster stats snapshot for effect calculations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CasterStats {
    pub attack_power: f32,
    pub spell_power: f32,
}

/// Result of processing a single spell effect.
///
/// The server applies these to the game world; the client uses them for
/// prediction and combat text display.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EffectResult {
    /// Deal damage to the target.
    Damage { amount: f32 },
    /// Heal the target.
    Heal { amount: f32 },
    /// Apply an aura (buff/debuff/DoT/HoT) to the target.
    ApplyAura { aura_spell_id: u32 },
    /// Restore a resource to the target.
    Energize { resource: ResourceType, amount: f32 },
    /// Dispel auras of the given school from the target.
    Dispel { school: SpellSchool },
    /// Interrupt the target's cast and lock out the school.
    Interrupt { lockout_duration: f32 },
}

/// Process a single spell effect, returning what should happen.
///
/// `roll` is a random value in 0.0–1.0 used to pick within damage/heal ranges.
///
/// Ref: AzerothCore `Spell::EffectSchoolDMG`, `Spell::EffectHeal`, etc.
pub fn process_effect(effect: &SpellEffectDef, caster: &CasterStats, roll: f32) -> EffectResult {
    match *effect {
        SpellEffectDef::SchoolDamage {
            base_min,
            base_max,
            ap_coefficient,
            sp_coefficient,
        } => {
            let base = base_min + (base_max - base_min) * roll;
            let bonus = caster.attack_power * ap_coefficient + caster.spell_power * sp_coefficient;
            EffectResult::Damage {
                amount: base + bonus,
            }
        }
        SpellEffectDef::Heal {
            base_min,
            base_max,
            ap_coefficient,
            sp_coefficient,
        } => {
            let base = base_min + (base_max - base_min) * roll;
            let bonus = caster.attack_power * ap_coefficient + caster.spell_power * sp_coefficient;
            EffectResult::Heal {
                amount: base + bonus,
            }
        }
        SpellEffectDef::ApplyAura { aura_spell_id } => EffectResult::ApplyAura { aura_spell_id },
        SpellEffectDef::Energize { resource, amount } => {
            EffectResult::Energize { resource, amount }
        }
        SpellEffectDef::Dispel { school } => EffectResult::Dispel { school },
        SpellEffectDef::Interrupt { lockout_duration } => {
            EffectResult::Interrupt { lockout_duration }
        }
    }
}

/// Process all effects of a spell, returning results for each non-None effect.
pub fn process_spell_effects(
    spell: &SpellData,
    caster: &CasterStats,
    roll: f32,
) -> Vec<EffectResult> {
    spell
        .effects
        .iter()
        .flatten()
        .map(|effect| process_effect(effect, caster, roll))
        .collect()
}

#[cfg(test)]
#[path = "spell_data_tests.rs"]
mod tests;
