//! Local Mainline106951s2: an additional point from combo-generating abilities.
use bevy::prelude::*;

use crate::auras::ActiveAuras;
use crate::spell_info::{SpellInfo, SpellStore};

const BERSERK: u32 = 106_951;
const ADDITIONAL_COMBO_POINTS: u8 = 1;

/// The active authored amount belongs to Feral's family, not unrelated
/// generators or343216's independent periodic point. The native ENERGIZE
/// handler admits only positive self-grants from untriggered casts here.
pub(in crate::spell_cast) fn berserk_combo_bonus(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> f32 {
    let Some(aura) = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == BERSERK))
    else {
        return 0.0;
    };
    let berserk = world
        .resource::<SpellStore>()
        .get(BERSERK)
        .expect("active Berserk requires native data");
    let same_family = spell
        .class_options
        .as_ref()
        .zip(berserk.class_options.as_ref())
        .is_some_and(|(builder, buff)| builder.spell_class_set == buff.spell_class_set);
    if !same_family {
        return 0.0;
    }
    aura.effects
        .iter()
        .find(|effect| effect.index == ADDITIONAL_COMBO_POINTS)
        .expect("Berserk requires its authored additional-point effect")
        .amount
}
