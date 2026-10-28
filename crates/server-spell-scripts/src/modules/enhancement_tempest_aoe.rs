//! SimC76f8118 sc_shaman.cpp9875–9881 and action.cpp1553–1555: Tempest secondary damage.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetDamageMultiplier(multiplier)];

fn multiplier(world: &World, caster: Entity, target: Entity, spell: &SpellInfo) -> f32 {
    if spell.id != 452201
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 263)
    {
        return 1.0;
    }
    if spell
        .cast_primary_target
        .expect("Tempest requires its native primary target")
        == target
    {
        return 1.0;
    }
    spell
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("native Tempest secondary percentage")
        .base_points
        / 100.0
}
