//! TC a352b1fa spell_rog_improved_garrote and spell_rog_improved_garrote_damage.
use bevy::prelude::*;

use super::{ActiveAuras, RemoveReason, has_aura, remove_auras};

pub(super) const GARROTE: u32 = 703;
const TALENT: u32 = 381632;
const STEALTH_BONUS: u32 = 392403;
const AFTER_STEALTH_BONUS: u32 = 392401;

pub(super) fn apply_stealth(world: &mut World, unit: Entity) {
    swap_bonus(world, unit, STEALTH_BONUS, AFTER_STEALTH_BONUS);
}

pub(super) fn remove_stealth(world: &mut World, unit: Entity) {
    swap_bonus(world, unit, AFTER_STEALTH_BONUS, STEALTH_BONUS);
}

fn swap_bonus(world: &mut World, unit: Entity, apply: u32, remove: u32) {
    if !has_aura(world, unit, TALENT) {
        return;
    }
    remove_auras(world, unit, RemoveReason::Cancelled, |aura| {
        aura.spell_id == remove
    });
    crate::spell_cast::cast_triggered(world, unit, apply, Some(unit), 1);
}

/// Effect1's native percentage is frozen when Garrote's periodic amount is built.
/// TC checks the after-stealth buff first; overlapping children never multiply.
pub(super) fn snapshot_amount(world: &World, caster: Entity, amount: f32) -> f32 {
    let Some(auras) = world.get::<ActiveAuras>(caster) else {
        return amount;
    };
    let bonus = [AFTER_STEALTH_BONUS, STEALTH_BONUS]
        .into_iter()
        .find_map(|spell| {
            auras
                .iter()
                .find(|aura| aura.spell_id == spell)
                .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 1))
                .map(|effect| effect.amount)
        });
    amount * (1.0 + bonus.unwrap_or(0.0) / 100.0)
}
