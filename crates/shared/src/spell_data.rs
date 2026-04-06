//! Static spell definitions — the data backbone for abilities.
//!
//! Each spell has an ID, name, school, cast time, cooldown, resource cost,
//! up to 3 effects, and AP/SP scaling coefficients. This data comes from
//! AzerothCore `spell_dbc` as a baseline, overlaid with retail values from
//! SimC `engine/dbc/`.

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
    Energize {
        resource: ResourceType,
        amount: f32,
    },
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
        let max_range = if spell.range > 0.0 { spell.range } else { MELEE_RANGE };
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
pub fn process_effect(
    effect: &SpellEffectDef,
    caster: &CasterStats,
    roll: f32,
) -> EffectResult {
    match *effect {
        SpellEffectDef::SchoolDamage {
            base_min,
            base_max,
            ap_coefficient,
            sp_coefficient,
        } => {
            let base = base_min + (base_max - base_min) * roll;
            let bonus = caster.attack_power * ap_coefficient
                + caster.spell_power * sp_coefficient;
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
            let bonus = caster.attack_power * ap_coefficient
                + caster.spell_power * sp_coefficient;
            EffectResult::Heal {
                amount: base + bonus,
            }
        }
        SpellEffectDef::ApplyAura { aura_spell_id } => {
            EffectResult::ApplyAura { aura_spell_id }
        }
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
mod tests {
    use super::*;

    #[test]
    fn spell_data_fireball() {
        let fireball = SpellData {
            id: 133,
            name: "Fireball".into(),
            school: SpellSchool::Fire,
            cast_time: 2.0,
            cooldown: 0.0,
            cost: Some(SpellCost {
                resource: ResourceType::Mana,
                amount: 3000.0,
            }),
            target: SpellTarget::Hostile,
            range: 40.0,
            cast_while_moving: false,
            interruptible: true,
            effects: [
                Some(SpellEffectDef::SchoolDamage {
                    base_min: 800.0,
                    base_max: 1000.0,
                    ap_coefficient: 0.0,
                    sp_coefficient: 1.0,
                }),
                None,
                None,
            ],
        };
        assert_eq!(fireball.id, 133);
        assert_eq!(fireball.school, SpellSchool::Fire);
        assert!(fireball.cost.is_some());
        assert_eq!(fireball.effects.iter().flatten().count(), 1);
    }

    #[test]
    fn spell_data_mortal_strike() {
        let ms = SpellData {
            id: 12294,
            name: "Mortal Strike".into(),
            school: SpellSchool::Physical,
            cast_time: 0.0,
            cooldown: 6.0,
            cost: Some(SpellCost {
                resource: ResourceType::Rage,
                amount: 30.0,
            }),
            target: SpellTarget::Hostile,
            range: 0.0, // melee
            cast_while_moving: true,
            interruptible: false,
            effects: [
                Some(SpellEffectDef::SchoolDamage {
                    base_min: 500.0,
                    base_max: 500.0,
                    ap_coefficient: 1.68,
                    sp_coefficient: 0.0,
                }),
                None,
                None,
            ],
        };
        assert_eq!(ms.id, 12294);
        assert_eq!(ms.cast_time, 0.0);
        assert_eq!(ms.cooldown, 6.0);
    }

    #[test]
    fn spell_data_flash_heal() {
        let fh = SpellData {
            id: 2061,
            name: "Flash Heal".into(),
            school: SpellSchool::Holy,
            cast_time: 1.5,
            cooldown: 0.0,
            cost: Some(SpellCost {
                resource: ResourceType::Mana,
                amount: 4500.0,
            }),
            target: SpellTarget::Friendly,
            range: 40.0,
            cast_while_moving: false,
            interruptible: true,
            effects: [
                Some(SpellEffectDef::Heal {
                    base_min: 2000.0,
                    base_max: 2400.0,
                    ap_coefficient: 0.0,
                    sp_coefficient: 1.2,
                }),
                None,
                None,
            ],
        };
        assert_eq!(fh.target, SpellTarget::Friendly);
        assert!(fh.interruptible);
    }

    #[test]
    fn spell_data_kick_interrupt() {
        let kick = SpellData {
            id: 1766,
            name: "Kick".into(),
            school: SpellSchool::Physical,
            cast_time: 0.0,
            cooldown: 15.0,
            cost: Some(SpellCost {
                resource: ResourceType::Energy,
                amount: 25.0,
            }),
            target: SpellTarget::Hostile,
            range: 0.0,
            cast_while_moving: true,
            interruptible: false,
            effects: [
                Some(SpellEffectDef::Interrupt {
                    lockout_duration: 5.0,
                }),
                None,
                None,
            ],
        };
        assert_eq!(kick.cooldown, 15.0);
        if let Some(SpellEffectDef::Interrupt { lockout_duration }) = kick.effects[0] {
            assert_eq!(lockout_duration, 5.0);
        } else {
            panic!("expected interrupt effect");
        }
    }

    #[test]
    fn spell_effect_all_variants() {
        let effects: [SpellEffectDef; 6] = [
            SpellEffectDef::SchoolDamage {
                base_min: 100.0,
                base_max: 200.0,
                ap_coefficient: 0.5,
                sp_coefficient: 0.0,
            },
            SpellEffectDef::Heal {
                base_min: 300.0,
                base_max: 400.0,
                ap_coefficient: 0.0,
                sp_coefficient: 1.0,
            },
            SpellEffectDef::ApplyAura {
                aura_spell_id: 12345,
            },
            SpellEffectDef::Energize {
                resource: ResourceType::Rage,
                amount: 20.0,
            },
            SpellEffectDef::Dispel {
                school: SpellSchool::Shadow,
            },
            SpellEffectDef::Interrupt {
                lockout_duration: 3.0,
            },
        ];
        assert_eq!(effects.len(), 6);
    }

    #[test]
    fn resource_types_all_variants() {
        let types: [ResourceType; 7] = [
            ResourceType::Mana,
            ResourceType::Rage,
            ResourceType::Energy,
            ResourceType::ComboPoints,
            ResourceType::HolyPower,
            ResourceType::RunicPower,
            ResourceType::Focus,
        ];
        assert_eq!(types.len(), 7);
    }

    #[test]
    fn spell_data_serialization_round_trip() {
        let spell = SpellData {
            id: 100,
            name: "Test Spell".into(),
            school: SpellSchool::Arcane,
            cast_time: 1.5,
            cooldown: 8.0,
            cost: Some(SpellCost {
                resource: ResourceType::Mana,
                amount: 2000.0,
            }),
            target: SpellTarget::Hostile,
            range: 30.0,
            cast_while_moving: false,
            interruptible: true,
            effects: [
                Some(SpellEffectDef::SchoolDamage {
                    base_min: 500.0,
                    base_max: 600.0,
                    ap_coefficient: 0.0,
                    sp_coefficient: 0.8,
                }),
                Some(SpellEffectDef::ApplyAura {
                    aura_spell_id: 200,
                }),
                None,
            ],
        };
        let json = serde_json::to_string(&spell).unwrap();
        let decoded: SpellData = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, spell);
    }

    // --- Effect processing tests ---

    fn test_caster() -> CasterStats {
        CasterStats {
            attack_power: 1000.0,
            spell_power: 3000.0,
        }
    }

    #[test]
    fn process_school_damage_min_roll() {
        let effect = SpellEffectDef::SchoolDamage {
            base_min: 800.0,
            base_max: 1000.0,
            ap_coefficient: 0.0,
            sp_coefficient: 1.0,
        };
        let result = process_effect(&effect, &test_caster(), 0.0);
        // base_min + SP * coeff = 800 + 3000 * 1.0 = 3800
        assert_eq!(result, EffectResult::Damage { amount: 3800.0 });
    }

    #[test]
    fn process_school_damage_max_roll() {
        let effect = SpellEffectDef::SchoolDamage {
            base_min: 800.0,
            base_max: 1000.0,
            ap_coefficient: 0.0,
            sp_coefficient: 1.0,
        };
        let result = process_effect(&effect, &test_caster(), 1.0);
        // base_max + SP * coeff = 1000 + 3000 = 4000
        assert_eq!(result, EffectResult::Damage { amount: 4000.0 });
    }

    #[test]
    fn process_school_damage_with_ap() {
        let effect = SpellEffectDef::SchoolDamage {
            base_min: 500.0,
            base_max: 500.0,
            ap_coefficient: 1.68,
            sp_coefficient: 0.0,
        };
        let result = process_effect(&effect, &test_caster(), 0.5);
        // 500 + 1000 * 1.68 = 2180
        assert_eq!(result, EffectResult::Damage { amount: 2180.0 });
    }

    #[test]
    fn process_heal() {
        let effect = SpellEffectDef::Heal {
            base_min: 2000.0,
            base_max: 2400.0,
            ap_coefficient: 0.0,
            sp_coefficient: 1.2,
        };
        let result = process_effect(&effect, &test_caster(), 0.5);
        // base: 2000 + (400 * 0.5) = 2200, bonus: 3000 * 1.2 = 3600
        assert_eq!(result, EffectResult::Heal { amount: 5800.0 });
    }

    #[test]
    fn process_apply_aura() {
        let effect = SpellEffectDef::ApplyAura { aura_spell_id: 42 };
        let result = process_effect(&effect, &test_caster(), 0.0);
        assert_eq!(result, EffectResult::ApplyAura { aura_spell_id: 42 });
    }

    #[test]
    fn process_energize() {
        let effect = SpellEffectDef::Energize {
            resource: ResourceType::Rage,
            amount: 20.0,
        };
        let result = process_effect(&effect, &test_caster(), 0.0);
        assert_eq!(
            result,
            EffectResult::Energize {
                resource: ResourceType::Rage,
                amount: 20.0
            }
        );
    }

    #[test]
    fn process_interrupt() {
        let effect = SpellEffectDef::Interrupt {
            lockout_duration: 5.0,
        };
        let result = process_effect(&effect, &test_caster(), 0.0);
        assert_eq!(
            result,
            EffectResult::Interrupt {
                lockout_duration: 5.0
            }
        );
    }

    #[test]
    fn process_spell_effects_multi() {
        let spell = SpellData {
            id: 100,
            name: "Multi".into(),
            school: SpellSchool::Fire,
            cast_time: 2.0,
            cooldown: 0.0,
            cost: None,
            target: SpellTarget::Hostile,
            range: 40.0,
            cast_while_moving: false,
            interruptible: true,
            effects: [
                Some(SpellEffectDef::SchoolDamage {
                    base_min: 100.0,
                    base_max: 100.0,
                    ap_coefficient: 0.0,
                    sp_coefficient: 0.5,
                }),
                Some(SpellEffectDef::ApplyAura { aura_spell_id: 200 }),
                None,
            ],
        };
        let results = process_spell_effects(&spell, &test_caster(), 0.5);
        assert_eq!(results.len(), 2);
        // 100 + 3000 * 0.5 = 1600
        assert_eq!(results[0], EffectResult::Damage { amount: 1600.0 });
        assert_eq!(results[1], EffectResult::ApplyAura { aura_spell_id: 200 });
    }

    #[test]
    fn process_spell_no_effects() {
        let spell = SpellData {
            id: 1,
            name: "Empty".into(),
            school: SpellSchool::Physical,
            cast_time: 0.0,
            cooldown: 0.0,
            cost: None,
            target: SpellTarget::Self_,
            range: 0.0,
            cast_while_moving: false,
            interruptible: false,
            effects: [None, None, None],
        };
        let results = process_spell_effects(&spell, &test_caster(), 0.0);
        assert!(results.is_empty());
    }

    // --- Cast validation tests ---

    fn valid_hostile_ctx() -> CastContext {
        CastContext {
            distance: 10.0,
            target_is_friendly: false,
            has_target: true,
            caster_alive: true,
            resource_available: Some(5000.0),
            cooldown_remaining: 0.0,
            gcd_remaining: 0.0,
            caster_moving: false,
        }
    }

    fn fireball_spell() -> SpellData {
        SpellData {
            id: 133,
            name: "Fireball".into(),
            school: SpellSchool::Fire,
            cast_time: 2.0,
            cooldown: 0.0,
            cost: Some(SpellCost {
                resource: ResourceType::Mana,
                amount: 3000.0,
            }),
            target: SpellTarget::Hostile,
            range: 40.0,
            cast_while_moving: false,
            interruptible: true,
            effects: [None, None, None],
        }
    }

    #[test]
    fn validate_cast_success() {
        assert!(validate_cast(&fireball_spell(), &valid_hostile_ctx()).is_ok());
    }

    #[test]
    fn validate_cast_dead_caster() {
        let mut ctx = valid_hostile_ctx();
        ctx.caster_alive = false;
        assert_eq!(
            validate_cast(&fireball_spell(), &ctx),
            Err(CastFailReason::CasterDead)
        );
    }

    #[test]
    fn validate_cast_no_target() {
        let mut ctx = valid_hostile_ctx();
        ctx.has_target = false;
        assert_eq!(
            validate_cast(&fireball_spell(), &ctx),
            Err(CastFailReason::NoTarget)
        );
    }

    #[test]
    fn validate_cast_friendly_target_for_hostile_spell() {
        let mut ctx = valid_hostile_ctx();
        ctx.target_is_friendly = true;
        assert_eq!(
            validate_cast(&fireball_spell(), &ctx),
            Err(CastFailReason::InvalidTarget)
        );
    }

    #[test]
    fn validate_cast_out_of_range() {
        let mut ctx = valid_hostile_ctx();
        ctx.distance = 50.0;
        assert_eq!(
            validate_cast(&fireball_spell(), &ctx),
            Err(CastFailReason::OutOfRange)
        );
    }

    #[test]
    fn validate_cast_melee_range_default() {
        let ms = super::super::spell_catalog::mortal_strike(); // range=0.0 → 5y melee
        let mut ctx = valid_hostile_ctx();
        ctx.distance = 4.0;
        assert!(validate_cast(&ms, &ctx).is_ok());
        ctx.distance = 6.0;
        assert_eq!(
            validate_cast(&ms, &ctx),
            Err(CastFailReason::OutOfRange)
        );
    }

    #[test]
    fn validate_cast_not_enough_resource() {
        let mut ctx = valid_hostile_ctx();
        ctx.resource_available = Some(1000.0); // need 3000
        assert_eq!(
            validate_cast(&fireball_spell(), &ctx),
            Err(CastFailReason::NotEnoughResource)
        );
    }

    #[test]
    fn validate_cast_on_cooldown() {
        let mut ctx = valid_hostile_ctx();
        ctx.cooldown_remaining = 3.5;
        assert_eq!(
            validate_cast(&fireball_spell(), &ctx),
            Err(CastFailReason::OnCooldown)
        );
    }

    #[test]
    fn validate_cast_self_spell_needs_no_target() {
        let self_spell = SpellData {
            id: 1,
            name: "Buff".into(),
            school: SpellSchool::Holy,
            cast_time: 0.0,
            cooldown: 0.0,
            cost: None,
            target: SpellTarget::Self_,
            range: 0.0,
            cast_while_moving: true,
            interruptible: false,
            effects: [None, None, None],
        };
        let mut ctx = valid_hostile_ctx();
        ctx.has_target = false;
        assert!(validate_cast(&self_spell, &ctx).is_ok());
    }

    #[test]
    fn validate_cast_no_cost_spell_ignores_resources() {
        let vr = super::super::spell_catalog::victory_rush(); // cost=None
        let mut ctx = valid_hostile_ctx();
        ctx.distance = 3.0; // melee range for Victory Rush
        ctx.resource_available = None; // no resource pool — doesn't matter
        assert!(validate_cast(&vr, &ctx).is_ok());
    }

    #[test]
    fn validate_cast_on_gcd() {
        let mut ctx = valid_hostile_ctx();
        ctx.gcd_remaining = 0.8;
        assert_eq!(
            validate_cast(&fireball_spell(), &ctx),
            Err(CastFailReason::OnGlobalCooldown)
        );
    }

    #[test]
    fn spell_is_instant() {
        let ms = super::super::spell_catalog::mortal_strike();
        assert!(ms.is_instant());
        let fb = fireball_spell();
        assert!(!fb.is_instant());
    }

    #[test]
    fn validate_cast_moving_blocks_non_instant() {
        let mut ctx = valid_hostile_ctx();
        ctx.caster_moving = true;
        // Fireball: cast_time=2.0, cast_while_moving=false → blocked
        assert_eq!(
            validate_cast(&fireball_spell(), &ctx),
            Err(CastFailReason::CantCastWhileMoving)
        );
    }

    #[test]
    fn validate_cast_moving_allows_instant() {
        let ms = super::super::spell_catalog::mortal_strike(); // instant
        let mut ctx = valid_hostile_ctx();
        ctx.caster_moving = true;
        ctx.distance = 3.0;
        assert!(validate_cast(&ms, &ctx).is_ok());
    }

    #[test]
    fn validate_cast_moving_allows_cast_while_moving_flag() {
        let mut spell = fireball_spell();
        spell.cast_while_moving = true; // e.g. Scorch with talent
        let mut ctx = valid_hostile_ctx();
        ctx.caster_moving = true;
        assert!(validate_cast(&spell, &ctx).is_ok());
    }

    #[test]
    fn validate_cast_stationary_ignores_flag() {
        let mut ctx = valid_hostile_ctx();
        ctx.caster_moving = false;
        // Fireball with cast_while_moving=false, but caster is stationary → ok
        assert!(validate_cast(&fireball_spell(), &ctx).is_ok());
    }
}
