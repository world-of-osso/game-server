//! Frost's authored crit-scaled native modifier update.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, AuraInstance};
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EarlyAuraRegistration(register)];
const DEEP_SHATTER: u32 = 1247025;

fn register(scripts: &mut combat::auras::scripts::AuraScripts) {
    let script = scripts.0.entry(DEEP_SHATTER).or_default();
    script.on_apply = Some(update_deep_shatter);
    script.on_periodic = Some(tick_deep_shatter);
}

fn tick_deep_shatter(world: &mut World, unit: Entity, aura: &AuraInstance, index: u8) {
    if index == 2 {
        update_deep_shatter(world, unit, aura);
    }
}

fn update_deep_shatter(world: &mut World, unit: Entity, aura: &AuraInstance) {
    // SimC76f8118 sc_mage.cpp4365–4373: authored crit-bonus multiplier
    // is native effect1 percent times current spell crit chance. Native effect3
    // updates the zero-valued effect2 once per second; coefficients are unchanged.
    let factor = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Deep Shatter factor")
        .amount;
    let crit = world
        .get::<combat::stats::CharacterStats>(unit)
        .map_or(0.0, |stats| stats.crit_pct);
    let amount = factor * crit / 100.0;
    let Some(effect) = world
        .get_mut::<ActiveAuras>(unit)
        .and_then(|auras| auras.into_inner().get_mut(aura.instance_id))
        .and_then(|aura| aura.effects.iter_mut().find(|effect| effect.index == 1))
    else {
        return;
    };
    effect.amount = amount;
    combat::auras::auras_changed(world, unit);
}
