//! Narrow `SpellInfo` to the shared `SpellData` used by `validate_cast`.
//!
//! Shared `SpellData` holds one school, one cost and three effects, so this
//! conversion is lossy; `SpellInfo` keeps the full data.

use shared::components::SpellSchool;
use shared::spell_data::{ResourceType, SpellCost, SpellData, SpellEffectDef, SpellTarget};

use super::{SpellEffect, SpellInfo, SpellPowerCost};

const SPELL_ATTR0_CASTABLE_WHILE_MOVING: u32 = 0x10000000;

const EFFECT_SCHOOL_DAMAGE: u32 = 2;
const EFFECT_APPLY_AURA: u32 = 6;
const EFFECT_HEAL: u32 = 10;
const EFFECT_ENERGIZE: u32 = 30;
const EFFECT_DISPEL: u32 = 38;
const EFFECT_INTERRUPT_CAST: u32 = 68;

/// Why a spell's cost cannot be expressed as a shared `SpellCost`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostError {
    /// Raw DB2 power type with no shared `ResourceType`.
    UnsupportedPowerType(i32),
    /// Percent cost, but the caster has no pool of this resource to scale from.
    MissingPool(ResourceType),
}

impl std::fmt::Display for CostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPowerType(power_type) => {
                write!(f, "power type {power_type} has no ResourceType")
            }
            Self::MissingPool(resource) => write!(f, "caster has no {resource:?} pool"),
        }
    }
}

/// Map a raw DB2 Powers ID to shared `ResourceType`.
pub fn resource_type(power_type: i32) -> Option<ResourceType> {
    match power_type {
        0 => Some(ResourceType::Mana),
        1 => Some(ResourceType::Rage),
        2 => Some(ResourceType::Focus),
        3 => Some(ResourceType::Energy),
        4 => Some(ResourceType::ComboPoints),
        6 => Some(ResourceType::RunicPower),
        9 => Some(ResourceType::HolyPower),
        _ => None,
    }
}

/// Shared `SpellSchool` has one school: the highest-priority set bit wins.
fn school_from_mask(mask: u32) -> SpellSchool {
    const ORDER: [(u32, SpellSchool); 6] = [
        (0x20, SpellSchool::Shadow),
        (0x40, SpellSchool::Arcane),
        (0x10, SpellSchool::Frost),
        (0x08, SpellSchool::Nature),
        (0x04, SpellSchool::Fire),
        (0x02, SpellSchool::Holy),
    ];
    ORDER
        .iter()
        .find(|(bit, _)| mask & bit != 0)
        .map_or(SpellSchool::Physical, |&(_, school)| school)
}

fn target_from_implicit(target_a: u32) -> SpellTarget {
    match target_a {
        1 => SpellTarget::Self_,              // TARGET_UNIT_CASTER
        5 | 21 | 25 => SpellTarget::Friendly, // TARGET_UNIT_TARGET_ALLY variants
        _ => SpellTarget::Hostile,
    }
}

impl SpellInfo {
    pub fn school_mask(&self) -> u32 {
        self.misc.as_ref().map_or(0, |misc| misc.school_mask)
    }

    pub fn cast_time_secs(&self) -> f32 {
        self.misc
            .as_ref()
            .and_then(|misc| misc.cast_time)
            .map_or(0.0, |cast| cast.base_ms as f32 / 1000.0)
    }

    fn duration_secs(&self) -> f32 {
        self.misc
            .as_ref()
            .and_then(|misc| misc.duration)
            .map_or(0.0, |duration| duration.duration_ms.max(0) as f32 / 1000.0)
    }

    /// First cost with no required aura (caster aura checks are not implemented).
    pub fn unconditional_cost(&self) -> Option<&SpellPowerCost> {
        self.power_costs
            .iter()
            .find(|cost| cost.required_aura_spell_id == 0)
    }

    /// Convert for `validate_cast`. `base_pool` gives the caster's base pool for
    /// the cost's resource (`None` = no pool), used for percent costs.
    pub fn to_spell_data(
        &self,
        base_pool: impl FnOnce(ResourceType) -> Option<f32>,
    ) -> Result<SpellData, CostError> {
        let cast_time = self.cast_time_secs();
        let target = self
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .map_or(SpellTarget::Hostile, |effect| {
                target_from_implicit(effect.implicit_targets[0])
            });
        let range = self.misc.as_ref().and_then(|misc| misc.range);
        let attributes_0 = self.misc.as_ref().map_or(0, |misc| misc.attributes[0]);
        Ok(SpellData {
            id: self.id,
            name: self.name.clone(),
            school: school_from_mask(self.school_mask()),
            cast_time,
            cooldown: self.cooldown_secs(),
            cost: self.shared_cost(base_pool)?,
            target,
            range: range.map_or(0.0, |range| match target {
                SpellTarget::Friendly => range.max_friendly,
                _ => range.max_hostile,
            }),
            cast_while_moving: attributes_0 & SPELL_ATTR0_CASTABLE_WHILE_MOVING != 0,
            interruptible: cast_time > 0.0,
            effects: self.shared_effects(),
        })
    }

    fn cooldown_secs(&self) -> f32 {
        self.cooldowns.as_ref().map_or(0.0, |cd| {
            cd.recovery_ms.max(cd.category_recovery_ms) as f32 / 1000.0
        })
    }

    /// Raw amount: flat cost plus percent of `base_pool`. Rage/Runic Power stay in tenths.
    fn shared_cost(
        &self,
        base_pool: impl FnOnce(ResourceType) -> Option<f32>,
    ) -> Result<Option<SpellCost>, CostError> {
        let Some(cost) = self.unconditional_cost() else {
            return Ok(None);
        };
        if cost.mana_cost <= 0 && cost.power_cost_pct <= 0.0 {
            return Ok(None);
        }
        let resource = resource_type(cost.power_type)
            .ok_or(CostError::UnsupportedPowerType(cost.power_type))?;
        let percent = if cost.power_cost_pct > 0.0 {
            base_pool(resource).ok_or(CostError::MissingPool(resource))? * cost.power_cost_pct
                / 100.0
        } else {
            0.0
        };
        Ok(Some(SpellCost {
            resource,
            amount: cost.mana_cost.max(0) as f32 + percent,
        }))
    }

    fn shared_effects(&self) -> [Option<SpellEffectDef>; 3] {
        let mut effects = [None; 3];
        for effect in &self.effects {
            if let Some(slot) = effects.get_mut(effect.index as usize) {
                *slot = self.shared_effect(effect);
            }
        }
        effects
    }

    fn shared_effect(&self, effect: &SpellEffect) -> Option<SpellEffectDef> {
        match effect.effect {
            EFFECT_SCHOOL_DAMAGE => Some(SpellEffectDef::SchoolDamage {
                base_min: effect.base_points,
                base_max: effect.base_points,
                ap_coefficient: effect.bonus_coefficient_from_ap,
                sp_coefficient: effect.bonus_coefficient,
            }),
            EFFECT_HEAL => Some(SpellEffectDef::Heal {
                base_min: effect.base_points,
                base_max: effect.base_points,
                ap_coefficient: effect.bonus_coefficient_from_ap,
                sp_coefficient: effect.bonus_coefficient,
            }),
            // Energize names its power type in EffectMiscValue_0.
            EFFECT_ENERGIZE => {
                resource_type(effect.misc_values[0]).map(|resource| SpellEffectDef::Energize {
                    resource,
                    amount: effect.base_points,
                })
            }
            EFFECT_APPLY_AURA => Some(SpellEffectDef::ApplyAura {
                aura_spell_id: self.id,
            }),
            // Shared Dispel carries a school; the DispelType is EffectMiscValue_0.
            EFFECT_DISPEL => Some(SpellEffectDef::Dispel {
                school: school_from_mask(self.school_mask()),
            }),
            // The school lockout lasts the spell duration (Kick 1766: 6 s).
            EFFECT_INTERRUPT_CAST => Some(SpellEffectDef::Interrupt {
                lockout_duration: self.duration_secs(),
            }),
            _ => None,
        }
    }
}
