//! TC a352b1fa SpellHistory.cpp843–844: native aura500 skips charge consumption
//! only for its selected charge category. Admission and ordinary cooldowns are unchanged.
use super::*;
use crate::auras::ActiveAuras;
use crate::auras::handlers::IGNORE_SPELL_CHARGE_COOLDOWN;

pub(super) fn is_ignored(world: &World, caster: Entity, spell: &SpellInfo) -> bool {
    let category = spell
        .categories
        .as_ref()
        .map_or(0, |row| row.charge_category);
    if category == 0 {
        return false;
    }
    world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.effects.iter().any(|effect| {
                effect.aura_type == IGNORE_SPELL_CHARGE_COOLDOWN
                    && effect.misc_values[0] as u32 == category
            })
        })
    })
}

pub(super) fn consume_unless_ignored(
    charges: &mut ChargeState,
    now: f64,
    ready_credit: u32,
    ignored: bool,
) {
    if !ignored {
        charges.consume(now, ready_credit);
    }
}
