//! TC a352b1fa spell_item_goblin_weather_machine (46203): the hit unit casts one
//! uniformly chosen Personalized Weather spell on itself. TDB1210 binds no script
//! to any weather spell, so 46736's PERIODIC_DUMMY ticks are no-ops (PeriodicTick
//! leaves226 to scripts); all four are authored presentation auras.
use bevy::prelude::*;

use super::effects::EffectCtx;
use super::toy_scripts::single_effect;
use crate::auras::handlers::{DUMMY, PERIODIC_DUMMY};
use crate::spell_info::{SpellEffect, SpellInfo, SpellStore};

pub(super) const WEATHER_MACHINE: u32 = 46203;
/// SPELL_PERSONALIZED_WEATHER1..4.
const PERSONALIZED_WEATHER: [u32; 4] = [46740, 46739, 46738, 46736];
const EFFECT_APPLY_AURA: u32 = 6;
const TARGET_UNIT_CASTER: u32 = 1;

pub(super) fn unsupported(world: &World) -> Option<String> {
    let store = world.resource::<SpellStore>();
    let audited = PERSONALIZED_WEATHER
        .iter()
        .all(|id| store.get(*id).is_some_and(is_presentation_weather));
    (!audited).then(|| "weather machine source weather spells differ".into())
}

fn is_presentation_weather(spell: &SpellInfo) -> bool {
    let aura = if spell.id == PERSONALIZED_WEATHER[3] {
        PERIODIC_DUMMY
    } else {
        DUMMY
    };
    let self_aura = single_effect(spell).is_some_and(|effect| is_self_weather_aura(effect, aura));
    spell.has_spell_visual && self_aura
}

fn is_self_weather_aura(effect: &SpellEffect, aura: u32) -> bool {
    let shape = (
        effect.index,
        effect.effect,
        effect.aura,
        effect.implicit_targets,
        effect.trigger_spell,
    ) == (0, EFFECT_APPLY_AURA, aura, [TARGET_UNIT_CASTER, 0], 0);
    let no_payload = effect.misc_values == [0, 0]
        && effect.base_points == 0.0
        && effect.item_type == 0
        && effect.mechanic == 0;
    shape && no_payload
}

pub(super) fn cast_personal_weather(world: &mut World, ctx: &EffectCtx, _effect: &SpellEffect) {
    let choice = rand::Rng::gen_range(&mut rand::thread_rng(), 0..PERSONALIZED_WEATHER.len());
    let weather = PERSONALIZED_WEATHER[choice];
    super::cast_triggered(world, ctx.caster, weather, Some(ctx.caster), ctx.depth + 1);
}
