//! Spell modifiers: ADD_FLAT_MODIFIER 107 / ADD_PCT_MODIFIER 108 (matched by
//! `SpellClassSet` + `SpellClassMask`) and ADD_*_MODIFIER_BY_SPELL_LABEL
//! 219/218 (matched by `SpellLabel`). `EffectMiscValue_0` is the
//! `SpellModOp`; label mods carry the label in `EffectMiscValue_1`.
//! Combined as TrinityCore `Unit::ApplySpellMod`: `(base + Σflat) × Π(1 + pct/100)`.

use bevy::prelude::*;

use super::handlers::{
    ADD_FLAT_MODIFIER, ADD_FLAT_MODIFIER_BY_SPELL_LABEL, ADD_PCT_MODIFIER,
    ADD_PCT_MODIFIER_BY_SPELL_LABEL,
};
use super::{ActiveAuras, AuraEffectState, AuraInstance};
use crate::spell_info::SpellInfo;

/// TrinityCore `SpellModOp` values read by this server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpellModOp {
    HealingAndDamage = 0,
    Duration = 1,
    PointsIndex0 = 3,
    ProcCharges = 4,
    Range = 5,
    CritChance = 7,
    Points = 8,
    ChangeCastTime = 10,
    Cooldown = 11,
    PointsIndex1 = 12,
    PowerCost0 = 14,
    CritDamageAndHealing = 15,
    ProcChance = 18,
    Period = 19,
    StartCooldown = 21,
    PeriodicHealingAndDamage = 22,
    PointsIndex2 = 23,
    BonusCoefficient = 24,
    PointsIndex3 = 32,
    PointsIndex4 = 33,
    PowerCost1 = 34,
    MaxAuraStacks = 37,
    PowerCost2 = 39,
}

impl SpellModOp {
    /// PointsIndexN for effect index N (0-4).
    pub fn points_index(index: u8) -> Option<Self> {
        Some(match index {
            0 => Self::PointsIndex0,
            1 => Self::PointsIndex1,
            2 => Self::PointsIndex2,
            3 => Self::PointsIndex3,
            4 => Self::PointsIndex4,
            _ => return None,
        })
    }

    /// PowerCostN for `SpellPower.OrderIndex` N (0-2).
    pub fn power_cost(order_index: u8) -> Option<Self> {
        Some(match order_index {
            0 => Self::PowerCost0,
            1 => Self::PowerCost1,
            2 => Self::PowerCost2,
            _ => return None,
        })
    }
}

fn is_spell_mod(aura_type: u32) -> bool {
    matches!(
        aura_type,
        ADD_FLAT_MODIFIER
            | ADD_PCT_MODIFIER
            | ADD_FLAT_MODIFIER_BY_SPELL_LABEL
            | ADD_PCT_MODIFIER_BY_SPELL_LABEL
    )
}

fn is_pct(aura_type: u32) -> bool {
    matches!(
        aura_type,
        ADD_PCT_MODIFIER | ADD_PCT_MODIFIER_BY_SPELL_LABEL
    )
}

/// Whether a spell-mod effect of `aura` applies to `spell`.
fn affects(aura: &AuraInstance, effect: &AuraEffectState, spell: &SpellInfo) -> bool {
    match effect.aura_type {
        ADD_FLAT_MODIFIER | ADD_PCT_MODIFIER => class_mask_affects(aura, effect, spell),
        _ => u32::try_from(effect.misc_values[1])
            .is_ok_and(|label| spell.labels.binary_search(&label).is_ok()),
    }
}

/// `AuraEffect::IsAffectingSpell` (SpellAuraEffects.cpp:1286-1296): `spell` is of the
/// aura's `SpellClassSet` and shares a bit of the effect's `SpellClassMask`.
fn class_mask_affects(aura: &AuraInstance, effect: &AuraEffectState, spell: &SpellInfo) -> bool {
    let Some(options) = spell.class_options.as_ref() else {
        return false;
    };
    aura.class_set != 0
        && options.spell_class_set == aura.class_set
        && (0..4).any(|word| options.spell_class_mask[word] & effect.class_mask[word] != 0)
}

/// `Unit::HasAuraTypeWithAffectMask` (Unit.cpp:4852-4858): `unit` has an `aura_type`
/// effect whose class mask affects `spell`.
pub fn has_aura_type_affecting(
    world: &World,
    unit: Entity,
    aura_type: u32,
    spell: &SpellInfo,
) -> bool {
    world.get::<ActiveAuras>(unit).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.effects.iter().any(|effect| {
                effect.aura_type == aura_type && class_mask_affects(aura, effect, spell)
            })
        })
    })
}

/// Spell-mod effects of `auras` for `op` that affect `spell`.
fn matching<'a>(
    auras: &'a ActiveAuras,
    spell: &'a SpellInfo,
    op: SpellModOp,
) -> impl Iterator<Item = (&'a AuraInstance, &'a AuraEffectState)> {
    auras.iter().flat_map(move |aura| {
        aura.effects
            .iter()
            .filter(move |effect| {
                is_spell_mod(effect.aura_type)
                    && effect.misc_values[0] == op as i32
                    && affects(aura, effect, spell)
            })
            .map(move |effect| (aura, effect))
    })
}

/// `base` modified by `caster`'s spell modifiers for `op` on `spell`.
pub fn modify(world: &World, caster: Entity, spell: &SpellInfo, op: SpellModOp, base: f32) -> f32 {
    let Some(auras) = world.get::<ActiveAuras>(caster) else {
        return base;
    };
    let (flat, mult) =
        matching(auras, spell, op).fold((0.0, 1.0), |(flat, mult), (aura, effect)| {
            let amount = effect.amount * aura.stack_multiplier();
            if is_pct(effect.aura_type) {
                (flat, mult * (1.0 + amount / 100.0))
            } else {
                (flat + amount, mult)
            }
        });
    (base + flat) * mult
}

/// Spell modifiers with charges are spent by the spell they modify: each
/// charged mod aura affecting `spell` (any op) loses one charge and is
/// removed at zero.
pub fn consume_charges(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let Some(auras) = world.get::<ActiveAuras>(caster) else {
        return;
    };
    let used: Vec<u32> = auras
        .iter()
        .filter(|aura| {
            aura.charges > 0
                && aura
                    .effects
                    .iter()
                    .any(|effect| is_spell_mod(effect.aura_type) && affects(aura, effect, spell))
        })
        .map(|aura| aura.instance_id)
        .collect();
    for id in used {
        super::spend_charge(world, caster, id);
    }
}
