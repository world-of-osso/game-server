//! Runic Power spend earns the native rune-regeneration buff.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::script_effect_value};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::BeforeNativeOnCast(on_cast)];

fn on_cast(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    if depth != 0
        || world
            .get::<ActiveSpec>(caster)
            .is_none_or(|spec| spec.0 != 252)
    {
        return;
    }
    let Some(cost) = spell
        .power_costs
        .iter()
        .find(|cost| cost.power_type == 6 && cost.mana_cost > 0)
    else {
        return;
    };
    // SimC76f8118 sc_death_knight.cpp13131–13147/13486–13500: Runic
    // Corruption rolls native effect1 per Runic Power of the base cost.
    let base_rp = cost.mana_cost as f32 / 10.0;
    let chance = script_effect_value(world, caster, 51462, 0) * base_rp / 10000.0;
    if combat::random::random::<f32>() >= chance {
        return;
    }
    let duration = world
        .resource::<SpellStore>()
        .get(51460)
        .and_then(|buff| buff.misc.as_ref())
        .and_then(|misc| misc.duration)
        .expect("native Runic Corruption duration")
        .duration_ms;
    assert!(
        duration > 0,
        "native Runic Corruption duration must be positive"
    );
    // SimC76f8118 sc_death_knight.cpp7310–7328 uses EXTEND, with the
    // pinned bugs=true duration; native aura110 supplies rune regen.
    if combat::auras::has_aura(world, caster, 51460) {
        combat::auras::extend_owned_duration(world, caster, caster, &[51460], duration as u32);
    } else if !spell_cast::try_cast_triggered(world, caster, 51460, Some(caster), 1) {
        warn!(?caster, "native Runic Corruption buff refused");
    }
}
