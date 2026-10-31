//! Protection Word of Glory rank 2 uses the recipient's missing health.
use bevy::prelude::*;
use shared::components::Health;

use super::combat;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;

const WORD_OF_GLORY: u32 = 85_673;
const RANK_TWO: u32 = 315_921;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::NamedHealingMultiplier(healing_multiplier)];

fn healing_multiplier(world: &World, caster: Entity, target: Entity, spell: &SpellInfo) -> f32 {
    if spell.id != WORD_OF_GLORY
        || caster != target
        || !combat::auras::has_aura(world, caster, RANK_TWO)
    {
        return 1.0;
    }
    let health = world
        .get::<Health>(target)
        .expect("Word of Glory target health");
    let missing = 1.0 - (health.current / health.max).clamp(0.0, 1.0);
    let percent = combat::spell_cast::scripts::script_effect_value(world, caster, RANK_TWO, 0);
    1.0 + missing * percent / 100.0
}
