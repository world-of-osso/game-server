//! TC a352b1fa Spell.cpp5767–5791 / Unit.cpp766: player-only aura263/264 checks.
use bevy::prelude::*;
use shared::components::Player;

use super::effects::{
    EFFECT_NORMALIZED_WEAPON_DMG, EFFECT_WEAPON_DAMAGE, EFFECT_WEAPON_PERCENT_DAMAGE,
};
use super::{CastFailure, attributes};
use crate::auras::ActiveAuras;
use crate::auras::handlers::{
    DISABLE_ATTACKING_EXCEPT_ABILITIES, DISABLE_CASTING_EXCEPT_ABILITIES,
};
use crate::spell_info::{ClassSpells, SpellInfo, SpellStore};

const USES_RANGED_SLOT: u32 = 0x0000_0002;
const IGNORE_CASTING_DISABLED: u32 = 0x0000_0200;
const EFFECT_ATTACK: u32 = 78;
const EFFECT_WEAPON_DAMAGE_NOSCHOOL: u32 = 17;
const ON_NEXT_SWING: u32 = 0x0000_0404;
const INITIATES_AUTO_ATTACK: u32 = 0x0000_0200;
const POST_CAST_AUTO_ATTACK: u32 = 0x0010_0000;

pub(super) fn check_cast(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<(), CastFailure> {
    let Some(player) = world.get::<Player>(caster) else {
        return Ok(());
    };
    let Some(auras) = world.get::<ActiveAuras>(caster) else {
        return Ok(());
    };
    let restrictions = [
        (
            DISABLE_CASTING_EXCEPT_ABILITIES,
            !bypasses_restriction(spell),
        ),
        (
            DISABLE_ATTACKING_EXCEPT_ABILITIES,
            !crate::auras::is_passive(spell) && is_attack_ability(spell),
        ),
    ];
    for (aura_type, applies) in restrictions {
        if !applies || !auras.has_type(aura_type) {
            continue;
        }
        let class_set = read_class_set(world, player.class)?;
        if !has_class_family_exception(world, auras, class_set, spell, aura_type)? {
            return Err(CastFailure::CantDoThatRightNow);
        }
    }
    Ok(())
}

fn read_class_set(world: &World, class_id: u8) -> Result<u32, CastFailure> {
    world
        .get_resource::<ClassSpells>()
        .and_then(|classes| {
            classes
                .classes
                .iter()
                .find(|class| class.class_id == class_id)
        })
        .map(|class| class.spell_class_set)
        .ok_or_else(|| {
            CastFailure::Unsupported(format!(
                "missing native class metadata for restricted caster class{class_id}"
            ))
        })
}

fn bypasses_restriction(spell: &SpellInfo) -> bool {
    attributes(spell, 0) & USES_RANGED_SLOT != 0
        || attributes(spell, 12) & IGNORE_CASTING_DISABLED != 0
        || spell
            .effects
            .iter()
            .any(|effect| effect.effect == EFFECT_ATTACK)
}

fn is_attack_ability(spell: &SpellInfo) -> bool {
    attributes(spell, 0) & (USES_RANGED_SLOT | ON_NEXT_SWING) != 0
        || attributes(spell, 1) & INITIATES_AUTO_ATTACK != 0
        || attributes(spell, 2) & POST_CAST_AUTO_ATTACK != 0
        || spell.effects.iter().any(|effect| {
            matches!(
                effect.effect,
                EFFECT_WEAPON_DAMAGE_NOSCHOOL
                    | EFFECT_WEAPON_PERCENT_DAMAGE
                    | EFFECT_WEAPON_DAMAGE
                    | EFFECT_ATTACK
                    | EFFECT_NORMALIZED_WEAPON_DMG
            )
        })
}

fn has_class_family_exception(
    world: &World,
    auras: &ActiveAuras,
    class_set: u32,
    spell: &SpellInfo,
    aura_type: u32,
) -> Result<bool, CastFailure> {
    let Some(cast_options) = spell.class_options.as_ref() else {
        return Ok(false);
    };
    for aura in auras.iter() {
        let restricts_cast = aura
            .effects
            .iter()
            .any(|effect| effect.aura_type == aura_type);
        if aura.class_set != class_set || !restricts_cast {
            continue;
        }
        let parent = world
            .resource::<SpellStore>()
            .get(aura.spell_id)
            .ok_or_else(|| {
                CastFailure::Unsupported(format!(
                    "missing aura{aura_type} parent spell{}",
                    aura.spell_id
                ))
            })?;
        if parent_flags_overlap(parent, cast_options.spell_class_mask) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// HasAuraTypeWithFamilyFlags uses the parent spell's flags, NOT the effect mask.
fn parent_flags_overlap(parent: &SpellInfo, cast_flags: [u32; 4]) -> bool {
    parent.class_options.as_ref().is_some_and(|options| {
        (0..4).any(|word| options.spell_class_mask[word] & cast_flags[word] != 0)
    })
}
