//! TC a352b1fa spell_rog_deadly_poison: own pre-existing2818 adds113780.
use bevy::prelude::*;

pub(in crate::spell_cast) const DEADLY_POISON: u32 = 2818;
const INSTANT_DAMAGE: u32 = 113780;

/// Native2818 has speed0: launch-target and aura resolution share this boundary.
/// Inspect ownership before refresh installs the new caster's aura.
pub(in crate::spell_cast) fn before_refresh(
    world: &mut World,
    caster: Entity,
    target: Entity,
    depth: u8,
) {
    let already_poisoned = world
        .get::<crate::auras::ActiveAuras>(target)
        .is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == DEADLY_POISON && aura.caster == caster)
        });
    if already_poisoned {
        crate::spell_cast::cast_triggered(world, caster, INSTANT_DAMAGE, Some(target), depth + 1);
    }
}
