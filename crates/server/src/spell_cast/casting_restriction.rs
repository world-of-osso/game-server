//! TC a352b1fa Spell.cpp5767 / Unit.cpp766: player-only aura263 cast checks.
use bevy::prelude::*;
use shared::components::Player;

use super::{CastFailure, attributes};
use crate::auras::ActiveAuras;
use crate::auras::handlers::DISABLE_CASTING_EXCEPT_ABILITIES;
use crate::spell_info::{ClassSpells, SpellInfo, SpellStore};

const USES_RANGED_SLOT: u32 = 0x0000_0002;
const IGNORE_CASTING_DISABLED: u32 = 0x0000_0200;
const EFFECT_ATTACK: u32 = 78;

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
    if !auras.has_type(DISABLE_CASTING_EXCEPT_ABILITIES) || bypasses_restriction(spell) {
        return Ok(());
    }
    let class_set = read_class_set(world, player.class)?;
    if has_class_family_exception(world, auras, class_set, spell)? {
        Ok(())
    } else {
        Err(CastFailure::CantDoThatRightNow)
    }
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
                "missing native class metadata for aura263 caster class{class_id}"
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

fn has_class_family_exception(
    world: &World,
    auras: &ActiveAuras,
    class_set: u32,
    spell: &SpellInfo,
) -> Result<bool, CastFailure> {
    let Some(cast_options) = spell.class_options.as_ref() else {
        return Ok(false);
    };
    for aura in auras.iter() {
        let restricts_casts = aura
            .effects
            .iter()
            .any(|effect| effect.aura_type == DISABLE_CASTING_EXCEPT_ABILITIES);
        if aura.class_set != class_set || !restricts_casts {
            continue;
        }
        let parent = world
            .resource::<SpellStore>()
            .get(aura.spell_id)
            .ok_or_else(|| {
                CastFailure::Unsupported(format!("missing aura263 parent spell{}", aura.spell_id))
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
