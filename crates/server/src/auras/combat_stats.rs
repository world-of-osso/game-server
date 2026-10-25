//! Native combat/stat aura readers, TC a352b1fa (no per-spell bindings).
use super::{ActiveAuras, AuraEffectState, AuraInstance, handlers};
use crate::spell_info::SpellInfo;
use bevy::prelude::*;
use shared::components::Health;

/// StatSystem.cpp777-803 / Unit.cpp2850: player expertise is integral and nonnegative.
pub(crate) fn expertise_reduction(auras: Option<&ActiveAuras>, player: bool) -> f32 {
    let amount = auras.map_or(0.0, |auras| auras.modifier(handlers::MOD_EXPERTISE));
    let expertise = if player {
        amount.trunc().max(0.0)
    } else {
        amount
    };
    expertise / 4.0
}

/// SimC76f8118 player.cpp1622 supplies7.5% baseline expertise. Class parity
/// followers share this player behavior; ordinary NPC aura240 remains unchanged.
pub(crate) fn class_actor_expertise_reduction(
    auras: Option<&ActiveAuras>,
    player: bool,
    parity_actor: bool,
) -> f32 {
    let baseline = if player || parity_actor { 7.5 } else { 0.0 };
    expertise_reduction(auras, player) + baseline
}

/// TC a352b1fa SpellInfo.cpp1974-1985: family0 and an empty mask are wildcards.
fn affects_spell(aura: &AuraInstance, effect: &AuraEffectState, spell: &SpellInfo) -> bool {
    if aura.class_set == 0 {
        return true;
    }
    spell.class_options.as_ref().is_some_and(|options| {
        aura.class_set == options.spell_class_set
            && (effect.class_mask == [0; 4]
                || (0..4).any(|word| effect.class_mask[word] & options.spell_class_mask[word] != 0))
    })
}

/// TC a352b1fa Unit.cpp7264–7270: recipient308 matches healer/attacker and family mask.
pub(crate) fn caster_ability_critical_chance(
    world: &World,
    caster: Entity,
    target: Entity,
    spell: &SpellInfo,
) -> f32 {
    const SPELL_DAMAGE_CLASS_MAGIC: u32 = 1;
    if spell
        .categories
        .as_ref()
        .map(|category| category.defense_type)
        != Some(SPELL_DAMAGE_CLASS_MAGIC)
    {
        return 0.0;
    }
    world.get::<ActiveAuras>(target).map_or(0.0, |auras| {
        auras
            .iter()
            .filter(|aura| aura.caster == caster)
            .flat_map(|aura| {
                aura.effects
                    .iter()
                    .filter(|effect| {
                        effect.aura_type == handlers::MOD_CRIT_CHANCE_FOR_CASTER_WITH_ABILITIES
                            && affects_spell(aura, effect, spell)
                    })
                    .map(move |effect| effect.amount * aura.stack_multiplier())
            })
            .sum()
    })
}

/// TC a352b1fa Unit.cpp1697–1703: recipient345 sums only this attacker's bypass.
pub(crate) fn armor_after_bypass(auras: Option<&ActiveAuras>, attacker: Entity, armor: f32) -> f32 {
    let bypass = auras.map_or(0.0, |auras| {
        auras
            .iter()
            .filter(|aura| aura.caster == attacker)
            .flat_map(|aura| {
                aura.effects
                    .iter()
                    .filter(|effect| effect.aura_type == handlers::BYPASS_ARMOR_FOR_CASTER)
                    .map(move |effect| effect.amount * aura.stack_multiplier())
            })
            .sum::<f32>()
    });
    armor * (100.0 - bypass.min(100.0)) / 100.0
}

/// Unit.cpp1712-1716: physical269 filters the spell, then floors armor per effect.
pub(crate) fn armor_after_ignore(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    armor: f32,
) -> f32 {
    let Some(auras) = world.get::<ActiveAuras>(caster) else {
        return armor;
    };
    auras.iter().fold(armor, |armor, aura| {
        aura.effects
            .iter()
            .filter(|effect| {
                effect.aura_type == handlers::MOD_IGNORE_TARGET_RESIST
                    && effect.misc_values[0] & 1 != 0
                    && affects_spell(aura, effect, spell)
            })
            .fold(armor, |armor, effect| {
                let remaining = 1.0 - effect.amount * aura.stack_multiplier() / 100.0;
                (armor * remaining).floor()
            })
    })
}

/// TC a352b1fa Unit.cpp7340: caster-owned50, with no MiscValue school mask.
pub fn critical_healing_multiplier(world: &World, caster: Entity) -> f32 {
    world.get::<ActiveAuras>(caster).map_or(1.0, |auras| {
        auras
            .iter()
            .flat_map(|aura| {
                aura.effects
                    .iter()
                    .filter(|effect| effect.aura_type == handlers::MOD_CRITICAL_HEALING_AMOUNT)
                    .map(move |effect| 1.0 + effect.amount * aura.stack_multiplier() / 100.0)
            })
            .product()
    })
}

/// TC a352b1fa Unit.cpp7514–7515: independently allow each healing-modifier sign.
fn healing_modifier_permissions(spell: &SpellInfo) -> (bool, bool) {
    const IGNORE_HEALING_MODIFIERS: u32 = 0x0800_0000;
    const ALWAYS_ALLOW_NEGATIVE_HEALING_PERCENT_MODIFIERS: u32 = 0x0400_0000;
    let attributes = spell.misc.as_ref().map_or([0; 17], |misc| misc.attributes);
    let allow_positive = attributes[6] & IGNORE_HEALING_MODIFIERS == 0;
    let allow_negative =
        allow_positive || attributes[13] & ALWAYS_ALLOW_NEGATIVE_HEALING_PERCENT_MODIFIERS != 0;
    (allow_positive, allow_negative)
}

/// TC a352b1fa Unit.cpp7579–7594: every504 factor belongs to this healer.
pub(crate) fn caster_healing_taken_multiplier(
    world: &World,
    caster: Entity,
    target: Entity,
    spell: &SpellInfo,
) -> f32 {
    let (allow_positive, allow_negative) = healing_modifier_permissions(spell);
    let Some(auras) = world.get::<ActiveAuras>(target) else {
        return 1.0;
    };
    auras
        .iter()
        .filter(|aura| aura.caster == caster)
        .flat_map(|aura| {
            aura.effects
                .iter()
                .filter(|effect| {
                    let allowed = if effect.amount > 0.0 {
                        allow_positive
                    } else {
                        allow_negative
                    };
                    effect.aura_type == handlers::MOD_HEALING_TAKEN_FROM_CASTER && allowed
                })
                .map(move |effect| 1.0 + effect.amount * aura.stack_multiplier() / 100.0)
        })
        .product()
}

/// TC a352b1fa Unit.cpp7512–7550: strongest259 of each allowed sign.
pub(crate) fn periodic_healing_taken_multiplier(
    world: &World,
    target: Entity,
    spell: &SpellInfo,
) -> f32 {
    let (allow_positive, allow_negative) = healing_modifier_permissions(spell);
    let Some(auras) = world.get::<ActiveAuras>(target) else {
        return 1.0;
    };
    let amounts = auras.iter().flat_map(|aura| {
        aura.effects
            .iter()
            .filter(|effect| effect.aura_type == handlers::MOD_HOT_PCT)
            .map(move |effect| effect.amount * aura.stack_multiplier())
    });
    let (negative, positive) = amounts.fold((0.0_f32, 0.0_f32), |(negative, positive), amount| {
        (negative.min(amount), positive.max(amount))
    });
    let negative_factor = if allow_negative {
        1.0 + negative / 100.0
    } else {
        1.0
    };
    let positive_factor = if allow_positive {
        1.0 + positive / 100.0
    } else {
        1.0
    };
    negative_factor * positive_factor
}

/// TC a352b1fa Unit.cpp7503-7507: each matched354 uses live missing HP.
pub(crate) fn healing_multiplier(
    world: &World,
    caster: Entity,
    target: Entity,
    spell: &SpellInfo,
) -> f32 {
    let Some(health) = world
        .get::<Health>(target)
        .filter(|health| health.max > 0.0)
    else {
        return 1.0;
    };
    let missing_fraction = 1.0 - health.current / health.max;
    world.get::<ActiveAuras>(caster).map_or(1.0, |auras| {
        auras
            .iter()
            .flat_map(|aura| {
                aura.effects
                    .iter()
                    .filter(|effect| {
                        effect.aura_type == handlers::MOD_HEALING_DONE_PCT_VERSUS_TARGET_HEALTH
                            && affects_spell(aura, effect, spell)
                    })
                    .map(move |effect| {
                        1.0 + effect.amount * aura.stack_multiplier() * missing_fraction / 100.0
                    })
            })
            .product()
    })
}
