//! Effect positivity: which effects of a spell are negative (TrinityCore
//! `SpellInfo::_InitializeSpellPositivity` / `_isPositiveEffectImpl`, SpellInfo.cpp:4612-5105).
//! An aura is a debuff by these per-effect marks, not by who cast it
//! (`AuraApplication::_InitFlags`, SpellAuras.cpp:113-147).
//!
//! Every effect and aura case of `_isPositiveEffectImpl` is ported. Native70448
//! aura342 shares aura192's signed haste rule; the pinned TC switch omits342
//! although both use HandleModMeleeRangedSpeedPct. Not modelled:
//! - `spell_custom_attr` negative marks (TrinityCore reads them first);
//! - the marks of already-initialized triggered spells, which TrinityCore reads in load
//!   order; here a triggered spell is always evaluated fresh;
//! - `SpellEffectInfo::CalcValue` scaling: base points are the raw `EffectBasePointsF`.

use std::collections::HashSet;

use super::{SpellEffect, SpellInfo, SpellStore};

const ATTR0_PASSIVE: u32 = 0x40;
const ATTR0_AURA_IS_DEBUFF: u32 = 0x0400_0000;
const ATTR1_AURA_UNIQUE: u32 = 0x800;
const ATTR4_AURA_IS_BUFF: u32 = 0x1000;
/// `SpellEffectAttributes::IsHarmful` (DBCEnums.h:2430).
const EFFECT_ATTR_IS_HARMFUL: u32 = 0x1000;

const SPELLFAMILY_GENERIC: u32 = 0;
const SPELLFAMILY_WARRIOR: u32 = 4;
const SPELLFAMILY_ROGUE: u32 = 8;
const MECHANIC_BANDAGE: i32 = 16;
const MECHANIC_SHIELD: i32 = 19;
const MECHANIC_MOUNT: i32 = 21;
const MECHANIC_INVULNERABILITY: i32 = 25;
const MECHANIC_IMMUNE_SHIELD: u32 = 29;
const DISPEL_STEALTH: i32 = 5;
const DISPEL_INVISIBILITY: i32 = 6;
const DISPEL_ENRAGE: i32 = 9;

/// Implicit targets with `TARGET_CHECK_ENEMY` (SpellImplicitTargetInfo::_data, SpellInfo.cpp:246+).
const ENEMY_TARGETS: [u32; 16] = [
    2, 6, 15, 16, 24, 28, 53, 54, 93, 104, 108, 115, 116, 129, 134, 151,
];

/// Area aura effects (`IsAreaAuraEffect`), APPLY_AURA 6 and APPLY_AURA_ON_PET 174.
const UNIT_AURA_EFFECTS: [u32; 10] = [6, 35, 65, 119, 128, 129, 143, 174, 202, 271];
const EFFECT_PERSISTENT_AREA_AURA: u32 = 27;

/// Negative effects of `spell`, one bit per `EffectIndex`.
pub fn negative_effects(store: &SpellStore, spell: &SpellInfo) -> u32 {
    let mut visited = HashSet::new();
    let mut negative = 0;
    for effect in &spell.effects {
        if !is_positive_effect(store, spell, effect, negative, &mut visited) {
            negative |= 1 << effect.index;
        }
    }
    // Special auras with a later negative effect on the same targets are negative too.
    for effect in &spell.effects {
        let late_check = matches!(effect.aura, 4 | 12 | 7 | 11 | 56 | 9 | 33);
        if effect.effect == 0 || negative & 1 << effect.index != 0 || !late_check {
            continue;
        }
        let later_negative = spell.effects.iter().any(|other| {
            other.index > effect.index
                && negative & 1 << other.index != 0
                && other.implicit_targets == effect.implicit_targets
        });
        if later_negative {
            negative |= 1 << effect.index;
        }
    }
    negative
}

fn attributes(spell: &SpellInfo, index: usize) -> u32 {
    spell.misc.as_ref().map_or(0, |misc| misc.attributes[index])
}

fn is_aura(effect: &SpellEffect) -> bool {
    (UNIT_AURA_EFFECTS.contains(&effect.effect) || effect.effect == EFFECT_PERSISTENT_AREA_AURA)
        && effect.aura != 0
}

/// `_isPositiveTarget`: no implicit target selects enemies.
fn positive_target(effect: &SpellEffect) -> bool {
    effect.effect == 0
        || !effect
            .implicit_targets
            .iter()
            .any(|id| ENEMY_TARGETS.contains(id))
}

/// The effects of `trigger_spell` not yet visited are all positive (`only_positive_targets`:
/// only those whose targets are positive count).
fn triggered_positive(
    store: &SpellStore,
    trigger_spell: u32,
    only_positive_targets: bool,
    visited: &mut HashSet<(u32, u8)>,
) -> bool {
    let Some(triggered) = store.get(trigger_spell) else {
        return true;
    };
    triggered.effects.iter().all(|effect| {
        visited.contains(&(triggered.id, effect.index))
            || effect.effect == 0
            || (only_positive_targets && !positive_target(effect))
            || is_positive_effect(store, triggered, effect, 0, visited)
    })
}

/// `_isPositiveEffectImpl`. `negative_so_far` is the marks already set on `spell`, read by
/// `IsPositive()`.
fn is_positive_effect(
    store: &SpellStore,
    spell: &SpellInfo,
    effect: &SpellEffect,
    negative_so_far: u32,
    visited: &mut HashSet<(u32, u8)>,
) -> bool {
    if effect.effect == 0 || attributes(spell, 0) & ATTR0_PASSIVE != 0 {
        return true;
    }
    if attributes(spell, 0) & ATTR0_AURA_IS_DEBUFF != 0 {
        return false;
    }
    if attributes(spell, 4) & ATTR4_AURA_IS_BUFF != 0 {
        return true;
    }
    if effect.attributes & EFFECT_ATTR_IS_HARMFUL != 0 {
        return false;
    }
    visited.insert((spell.id, effect.index));
    if let Some(positive) = family_override(spell) {
        return positive;
    }
    let mechanic = spell
        .categories
        .as_ref()
        .map_or(0, |categories| categories.mechanic);
    if mechanic == MECHANIC_IMMUNE_SHIELD {
        return true;
    }
    if attributes(spell, 1) & ATTR1_AURA_UNIQUE != 0 && !spell.effects.iter().all(positive_target) {
        return false;
    }
    if let Some(positive) = other_effects_decide(spell, effect) {
        return positive;
    }
    if let Some(positive) = effect_type_decides(effect) {
        return positive;
    }
    if is_aura(effect)
        && let Some(positive) = aura_decides(store, effect, negative_so_far, visited)
    {
        return positive;
    }
    if effect.aura == 0 && effect.trigger_spell != 0 {
        return triggered_positive(store, effect.trigger_spell, false, visited);
    }
    true
}

/// Spell ID and family exceptions (`SpellFamilyName` switch).
fn family_override(spell: &SpellInfo) -> Option<bool> {
    let (family, flags0) = spell.class_options.as_ref().map_or((0, 0), |options| {
        (options.spell_class_set, options.spell_class_mask[0])
    });
    match family {
        SPELLFAMILY_GENERIC => match spell.id {
            40268 | 61987 | 61988 | 64412 | 72410 | 71204 => Some(false),
            24732 | 30877 | 61716 | 61734 | 62344 | 50344 | 61819 | 61834 | 73523 => Some(true),
            _ => None,
        },
        SPELLFAMILY_ROGUE => match spell.id {
            32645 => Some(true),
            40251 => Some(false),
            _ => None,
        },
        // Slam, Execute.
        SPELLFAMILY_WARRIOR if flags0 & 0x2020_0000 != 0 => Some(false),
        _ => None,
    }
}

/// Effects of the spell that decide every effect's positivity.
fn other_effects_decide(spell: &SpellInfo, effect: &SpellEffect) -> Option<bool> {
    for other in &spell.effects {
        match other.effect {
            // HEAL, LEARN_SPELL, SKILL_STEP, HEAL_PCT.
            10 | 36 | 44 | 136 => return Some(true),
            // INSTAKILL on the same targets as another effect.
            1 if other.index != effect.index
                && other.implicit_targets == effect.implicit_targets =>
            {
                return Some(false);
            }
            _ => {}
        }
        if is_aura(other) {
            match other.aura {
                // MOD_STEALTH, MOD_UNATTACKABLE.
                16 | 93 => return Some(true),
                // SCHOOL_HEAL_ABSORB, EMPATHY, MOD_SPELL_DAMAGE_FROM_CASTER, PREVENTS_FLEEING.
                301 | 121 | 271 | 92 => return Some(false),
                _ => {}
            }
        }
    }
    None
}

/// `switch (effect.Effect)`.
fn effect_type_decides(effect: &SpellEffect) -> Option<bool> {
    let enemy = !positive_target(effect);
    let bp = effect.base_points;
    match effect.effect {
        58 | 17 | 121 | 31 | 2 | 7 | 9 | 1 | 8 | 126 | 68 | 71 | 87 | 111 | 115 | 129 | 55 | 69 => {
            Some(false)
        }
        30 | 137 | 136 | 67 | 75 => Some(true),
        // KNOCK_BACK, CHARGE, PERSISTENT_AREA_AURA, ATTACK_ME, POWER_BURN.
        98 | 96 | 27 | 114 | 62 if enemy => Some(false),
        // DISPEL.
        38 if matches!(
            effect.misc_values[0],
            DISPEL_STEALTH | DISPEL_INVISIBILITY | DISPEL_ENRAGE
        ) || enemy =>
        {
            Some(false)
        }
        // DISPEL_MECHANIC.
        108 if enemy && negative_mechanic(effect.misc_values[0]) => Some(false),
        // THREAT, MODIFY_THREAT_PERCENT.
        63 | 125 if enemy && bp > 0.0 => Some(false),
        _ => None,
    }
}

fn negative_mechanic(mechanic: i32) -> bool {
    matches!(
        mechanic,
        MECHANIC_BANDAGE | MECHANIC_SHIELD | MECHANIC_MOUNT | MECHANIC_INVULNERABILITY
    )
}

/// `switch (effect.ApplyAuraName)`.
fn aura_decides(
    store: &SpellStore,
    effect: &SpellEffect,
    negative_so_far: u32,
    visited: &mut HashSet<(u32, u8)>,
) -> Option<bool> {
    let enemy = !positive_target(effect);
    let bp = effect.base_points;
    let negative_if = |negative: bool| negative.then_some(false);
    match effect.aura {
        // Sign-dependent stats: negative base points (or per-level points) are debuffs.
        29 | 30 | 400 | 49 | 135 | 59 | 20 | 21 | 290 | 54 | 55 | 57 | 140 | 192 | 65 | 216
        | 143 | 91 | 133 | 137 | 58 | 80 | 34 | 129 | 342 => {
            negative_if(bp < 0.0 || effect.real_points_per_level < 0.0)
        }
        9 | 138 | 13 | 22 | 101 | 189 | 99 | 124 | 79 | 252 | 193 | 166 | 136 | 118 => {
            negative_if(enemy || bp < 0.0)
        }
        14 | 125 | 126 | 73 | 72 | 255 => negative_if(bp > 0.0),
        87 => negative_if(enemy && bp > 0.0),
        88 => negative_if(enemy && bp < 0.0),
        109 => Some(true),
        227 | 48 => negative_if(!triggered_positive(
            store,
            effect.trigger_spell,
            true,
            visited,
        )),
        23 | 12 | 56 | 33 | 7 | 11 | 25 | 60 | 67 | 254 | 278 | 6 | 177 | 2 | 75 | 15 | 42
        | 184 | 185 | 186 | 187 | 197 | 4 | 226 | 115 | 52 | 162 | 196 | 205 | 31 | 47 | 296
        | 24 | 37 | 112 | 36 | 10 | 231 => negative_if(enemy),
        5 | 86 | 26 | 455 | 27 | 221 | 95 | 53 | 64 | 68 | 314 | 3 | 70 | 89 | 165 | 127 => {
            Some(false)
        }
        77 => negative_if(negative_mechanic(effect.misc_values[0])),
        107 | 108 | 219 | 218 => spell_mod_decides(effect, negative_so_far == 0),
        _ => None,
    }
}

/// ADD_FLAT/PCT_MODIFIER(_BY_SPELL_LABEL) by `SpellModOp` (MiscValue).
fn spell_mod_decides(effect: &SpellEffect, spell_positive: bool) -> Option<bool> {
    let bp = effect.base_points;
    let negative_if = |negative: bool| negative.then_some(false);
    match effect.misc_values[0] {
        // ChangeCastTime, Period, PowerCostOnMiss, StartCooldown.
        10 | 19 | 30 | 21 => negative_if(bp > 0.0),
        // Cooldown, PowerCost0..2.
        11 | 14 | 34 | 39 => negative_if(!spell_positive && bp > 0.0),
        // PointsIndex0..4, Points, Hate, ChainAmplitude, Amplitude.
        3 | 12 | 23 | 32 | 33 | 8 | 2 | 20 | 27 => Some(true),
        // Duration, CritChance, HealingAndDamage, ChainTargets.
        1 | 7 | 0 | 17 => negative_if(!spell_positive && bp < 0.0),
        _ => negative_if(bp < 0.0),
    }
}
