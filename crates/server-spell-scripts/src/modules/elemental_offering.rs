//! Native Offering from Beyond advances the running Stormkeeper cooldown.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterElementalLaunch(after_summon)];
const SUMMON: u32 = 445624;
const OFFERING: u32 = 443451;
const STORMKEEPER: u32 = 191634;

fn after_summon(world: &mut World, caster: Entity, spell: &SpellInfo, _: bool) {
    if spell.id != SUMMON
        || !world
            .get::<combat::class_progression::ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 262)
        || !auras::has_aura(world, caster, OFFERING)
    {
        return;
    }
    // SimC76f8118 sc_shaman.cpp11579–11596: native effect0 adjusts the active timer per summon.
    let milliseconds = spell_cast::scripts::script_effect_value(world, caster, OFFERING, 0);
    spell_cast::scripts::reduce_cooldown(
        world,
        caster,
        STORMKEEPER,
        -f64::from(milliseconds) / 1000.0,
    );
}
