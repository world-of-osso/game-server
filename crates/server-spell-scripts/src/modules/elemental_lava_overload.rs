//! Native Elemental Lava Burst and its overload scale with the caster's crit chance.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::ActiveSpec;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetDamageMultiplier(multiplier)];

fn multiplier(world: &World, caster: Entity, _target: Entity, spell: &SpellInfo) -> f32 {
    // SimC76f8118 sc_shaman.cpp6835–6855/7197–7211: Elemental Lava Burst
    // and its overload gain 1 + caster crit chance, not the guaranteed target crit.
    if !matches!(spell.id, 285452 | 285466)
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
