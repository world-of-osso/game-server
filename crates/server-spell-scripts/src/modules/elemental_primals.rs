//! Native Primal Elemental action conditions from the pinned pet APLs.
use bevy::prelude::*;

use super::combat;
use combat::auras::ActiveAuras;
use combat::spell_cast::{self, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::GuardianActionEligible(action_eligible)];

const STORMFURY: u32 = 157375;
const CALL_LIGHTNING: u32 = 157348;
const IMMOLATE: u32 = 118297;
const STORMFURY_MIN_CALL_LIGHTNING_SECONDS: f64 = 10.0;

fn action_eligible(world: &World, actor: Entity, spell: u32, target: Entity) -> bool {
    match spell {
        // SimC76f8118 sc_shaman.cpp4275–4285: stormfury,if=buff.call_lightning.remains>=10.
        STORMFURY => {
            aura_remains(world, actor, actor, CALL_LIGHTNING) >= STORMFURY_MIN_CALL_LIGHTNING_SECONDS
        }
        // sc_shaman.cpp4109–4117: immolate,target_if=!ticking.
        IMMOLATE => aura_remains(world, target, actor, IMMOLATE) <= 0.0,
        _ => true,
    }
}

fn aura_remains(world: &World, unit: Entity, caster: Entity, spell: u32) -> f64 {
    let now = spell_cast::now(world);
    world
        .get::<ActiveAuras>(unit)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == spell && aura.caster == caster)
        })
        .and_then(|aura| aura.expires_at)
        .map_or(0.0, |at| (at - now).max(0.0))
}
