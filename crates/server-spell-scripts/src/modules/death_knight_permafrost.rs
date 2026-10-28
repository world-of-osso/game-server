//! Permafrost creates Frost Shield from landed auto-attack damage.
use bevy::prelude::*;

use super::combat;
use combat::auras::ActiveAuras;
use combat::spell_cast::{self, script_registry::SpellScriptHook};

const PERMAFROST: u32 = 207200;
const FROST_SHIELD: u32 = 207203;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LandedDamagePolicy(after_white)];

// SimC 76f8118 sc_death_knight.cpp:7704–7713 uses actual landed swing damage.
fn after_white(
    world: &mut World,
    caster: Entity,
    _: Entity,
    spell: Option<u32>,
    damage: f32,
    _: bool,
) {
    if spell.is_some() || damage <= 0.0 {
        return;
    }
    let Some(percent) = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == PERMAFROST && aura.caster == caster)
            .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
            .map(|effect| effect.amount)
    }) else {
        return;
    };
    spell_cast::cast_triggered_with_effect0_amount(
        world,
        caster,
        FROST_SHIELD,
        Some(caster),
        1,
        damage * percent / 100.0,
    );
}
