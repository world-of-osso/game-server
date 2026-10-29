//! SimC76f8118 sc_monk.cpp CAST_DURING: admitted Monk actions during selected channels.
use super::{SpellScriptHook, combat};
use bevy::prelude::*;
use combat::spell_info::SpellInfo;
use shared::casting::{CastState, CastType};
use shared::components::ActiveSpec;

pub const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AllowCastDuringChannel(
    allowed_during_monk_channel,
)];

fn allowed_during_monk_channel(world: &World, caster: Entity, spell: &SpellInfo) -> bool {
    let Some(spec) = world.get::<ActiveSpec>(caster).map(|spec| spec.0) else {
        return false;
    };
    if !matches!(spec, 268..=270) {
        return false;
    }
    let Some(channel) = world
        .get::<CastState>(caster)
        .filter(|cast| cast.cast_type == CastType::Channel)
    else {
        return false;
    };
    if channel.spell_id == 443028 {
        return spell.name == "Zenith Stomp";
    }
    if !matches!(channel.spell_id, 101546 | 322729) {
        return false;
    }
    matches!(
        spell.name.as_str(),
        "Tiger Palm"
            | "Rising Sun Kick"
            | "Blackout Kick"
            | "Rushing Jade Wind"
            | "Whirling Dragon Punch"
            | "Strike of the Windlord"
            | "Keg Smash"
            | "Touch of Death"
            | "Touch of Karma"
            | "Breath of Fire"
            | "Fortifying Brew"
            | "Exploding Keg"
            | "Purifying Brew"
            | "Invoke Xuen, the White Tiger"
            | "Invoke Niuzao, the Black Ox"
            | "Zenith Stomp"
            | "Expel Harm"
            | "Celestial Brew"
            | "Celestial Infusion"
    )
}
