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

/// Resource cost for casting a spell.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpellCost {
    pub resource: ResourceType,
    /// Flat amount (e.g. 3000 mana, 30 rage, 40 energy).
    pub amount: f32,
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
}
