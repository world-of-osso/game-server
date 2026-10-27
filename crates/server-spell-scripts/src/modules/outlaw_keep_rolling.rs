//! Keep It Rolling extends the earned Roll the Bones window.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras};
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;

const KEEP_IT_ROLLING: u32 = 381989;
const ROLL_THE_BONES: u32 = 1214909;
const STATES: &[u32] = &[1214933, 1214934, 1214935, 1214937];

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AuraRollAfterCast(after_cast)];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == KEEP_IT_ROLLING).then_some(extend_roll)
}

fn extend_roll(world: &mut World, caster: Entity, _: &SpellInfo) {
    // SimC76f8118 sc_rogue.cpp8542–8550: effect1 is the extension in ms.
    let extension =
        spell_cast::scripts::script_effect_value(world, caster, KEEP_IT_ROLLING, 0) as u32;
    auras::extend_owned_duration(world, caster, caster, &[ROLL_THE_BONES], extension);
    let now = spell_cast::now(world);
    let states: Vec<_> = world
        .get::<ActiveAuras>(caster)
        .into_iter()
        .flat_map(|auras| auras.iter())
        .filter_map(|aura| {
            let expiry = aura.expires_at?;
            (aura.caster == caster && STATES.contains(&aura.spell_id) && expiry > now)
                .then_some((aura.spell_id, expiry))
        })
        .collect();
    for (state, expiry) in states {
        // sc_rogue.cpp7640–7646: the default bugs mode caps secondary remaining time at 60s.
        let capped = ((60.0 - (expiry - now)).max(0.0) * 1000.0).round() as u32;
        auras::extend_owned_duration(world, caster, caster, &[state], extension.min(capped));
    }
}
