//! Selected Tactical Edge grants native Sudden Death uses after Colossus Smash.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::AfterElementalLaunch(grant_sudden_death)];

fn grant_sudden_death(world: &mut World, caster: Entity, spell: &SpellInfo, _: bool) {
    // SimC76f8118 sc_warrior.cpp3758–3759: the completed CS execution,
    // not each AoE hit, grants the selected talent's effect1 use count.
    if spell.id != 167105
        || world
            .get::<ActiveSpec>(caster)
            .is_none_or(|spec| spec.0 != 71)
        || !combat::auras::has_aura(world, caster, 1261051)
    {
        return;
    }
    let uses = script_effect_value(world, caster, 1261051, 0) as u8;
    assert!(uses > 0, "native Tactical Edge requires positive SD uses");
    let values = combat::spell_cast::TriggeredSpellOverrides {
        aura_stacks: Some(uses),
        ..Default::default()
    };
    if !combat::spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        52437,
        Some(caster),
        1,
        values,
    ) {
        warn!(?caster, "Native Tactical Edge Sudden Death grant refused");
    }
}
