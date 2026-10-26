//! Just Warming Up grants native raw Rage once per Colossus Smash execution.
use super::{combat, selected};
use bevy::prelude::*;
use combat::spell_cast::effects::{self, EffectCtx};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::PowerType;

pub(super) fn after_smash(world: &mut World, caster: Entity, spell: &SpellInfo, _: bool) {
    if spell.id != 167105 || !selected(world, caster, 1261048) {
        return;
    }
    // SimC76f8118 sc_warrior.cpp3781–3782 uses the talent's resource
    // amount. PowerPools are raw units, so retain native150, not displayed15.
    let amount = combat::spell_cast::scripts::script_effect_value(world, caster, 1261048, 0);
    let talent = world
        .resource::<SpellStore>()
        .get(1261048)
        .expect("native Just Warming Up talent")
        .clone();
    let ctx = EffectCtx {
        caster,
        spell: &talent,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    effects::energize_unit(world, &ctx, caster, PowerType::Rage, amount, false);
}
