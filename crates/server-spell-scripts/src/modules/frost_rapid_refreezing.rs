//! Frost's sourced four-piece proc, distinct from the catalog's broad damage hint.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraApplication, AuraSource};
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OwnsPolicyProc(owns_proc),
    SpellScriptHook::AuraRollAfterCast(after_cast_hook),
];
const FOUR_PIECE: u32 = 1296586;
const RAPID_REFREEZING: u32 = 1310248;
// SimC76f8118 sc_mage.cpp6643–6646: this custom accumulated curve is
// explicitly authored in source, not the unrelated native5% damage effect.
const INITIAL_CHANCE: f64 = 0.2;
const FAILURE_INCREMENT: f64 = 0.020116149192034555;

#[derive(Component, Default)]
struct RapidAttempts(u32);

fn owns_proc(spell: u32) -> bool {
    spell == FOUR_PIECE
}

fn after_cast_hook(spell: u32) -> Option<AfterCastHook> {
    (spell == 199786).then_some(roll_rapid_refreezing)
}

fn roll_rapid_refreezing(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if world
        .get::<ActiveSpec>(caster)
        .is_none_or(|spec| spec.0 != 64)
        || !auras::has_aura(world, caster, FOUR_PIECE)
    {
        return;
    }
    // sc_mage.cpp4296–4297: only an actual Glacial Spike execution attempts
    // the roll. Lance, its Shatter and other harmful successes do not attempt it.
    let attempts = world
        .entity_mut(caster)
        .entry::<RapidAttempts>()
        .or_default()
        .into_mut()
        .0
        + 1;
    let chance = INITIAL_CHANCE + f64::from(attempts - 1) * FAILURE_INCREMENT;
    let success = chance >= 1.0 || combat::random::random::<f64>() < chance;
    world.get_mut::<RapidAttempts>(caster).unwrap().0 = if success { 0 } else { attempts };
    if success {
        auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target: caster,
                spell_id: RAPID_REFREEZING,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
}
