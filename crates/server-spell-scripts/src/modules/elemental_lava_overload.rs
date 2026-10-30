//! Native Elemental Lava Burst Overload scales with the caster's crit chance.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::ActiveSpec;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetDamageMultiplier(multiplier)];

fn multiplier(world: &World, caster: Entity, _target: Entity, spell: &SpellInfo) -> f32 {
    // SimC76f8118 sc_shaman.cpp6835–6855: Elemental overload gains 1 + composite crit chance.
    if spell.id != 285466
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 262)
    {
        return 1.0;
    }
    let crit = world
        .get::<combat::stats::CharacterStats>(caster)
        .expect("native Elemental stats")
        .crit_pct;
    1.0 + crit / 100.0
}
