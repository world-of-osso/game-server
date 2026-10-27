//! Native Catalyze: SimC76f8118 sc_evoker.cpp5365–5374.
//! The selected passive's effect0 is the Fire Breath tick-rate bonus.
use bevy::prelude::*;

use super::combat;
use combat::auras::ActiveAuras;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;

const CATALYZE: u32 = 386283;
const FIRE_DAMAGE: u32 = 357209;
const DISINTEGRATE: u32 = 356995;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::PeriodicTickRate(tick_rate)];

/// Fire Breath ticks faster while the same caster's Disintegrate ticks on its target.
fn tick_rate(world: &World, caster: Entity, target: Entity, spell: &SpellInfo) -> f32 {
    let selected = combat::auras::has_aura(world, caster, CATALYZE);
    if spell.id != FIRE_DAMAGE || !selected || !disintegrating(world, caster, target) {
        return 1.0;
    }
    let bonus = combat::spell_cast::scripts::script_effect_value(world, caster, CATALYZE, 0);
    1.0 + bonus / 100.0
}

fn disintegrating(world: &World, caster: Entity, target: Entity) -> bool {
    world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.spell_id == DISINTEGRATE && aura.caster == caster)
    })
}
