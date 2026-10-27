//! Native Primal Elemental action conditions from the pinned pet APLs.
use bevy::prelude::*;

use super::combat;
use combat::auras::ActiveAuras;
use combat::spell_cast::{self, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::GuardianActionEligible(action_eligible)];

const STORMFURY: u32 = 157375;
const CALL_LIGHTNING: u32 = 157348;
const STORMFURY_MIN_CALL_LIGHTNING_SECONDS: f64 = 10.0;

fn action_eligible(world: &World, actor: Entity, spell: u32, _target: Entity) -> bool {
    // SimC76f8118 sc_shaman.cpp4275–4285: stormfury,if=buff.call_lightning.remains>=10.
    spell != STORMFURY || call_lightning_remains(world, actor) >= STORMFURY_MIN_CALL_LIGHTNING_SECONDS
}

fn call_lightning_remains(world: &World, actor: Entity) -> f64 {
    let now = spell_cast::now(world);
    world
        .get::<ActiveAuras>(actor)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == CALL_LIGHTNING && aura.caster == actor)
        })
        .and_then(|aura| aura.expires_at)
        .map_or(0.0, |at| (at - now).max(0.0))
}
