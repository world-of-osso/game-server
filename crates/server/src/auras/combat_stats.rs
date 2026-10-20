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

/// SpellInfo.cpp1979-1990: family0 and an empty mask are wildcards.
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

/// Unit.cpp7505-7509: each matched354 scales by the recipient's live missing HP.
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
