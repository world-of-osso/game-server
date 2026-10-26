//! Fire's Hot Streak conversion supplies the selected native Intellect buff.
use bevy::prelude::*;

use super::combat;
use combat::auras::{AuraInstance, scripts::{AuraScript, AuraScripts}};
use combat::class_progression::KnownSpells;
use combat::spell_cast::{self, script_registry::SpellScriptHook};

const HOT_STREAK: u32 = 48108;
const BRAINSTORM_TALENT: u32 = 461261;
const BRAINSTORM: u32 = 461531;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::FireAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(HOT_STREAK, AuraScript {
        on_apply: Some(grant_brainstorm),
        ..Default::default()
    });
}

fn grant_brainstorm(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.caster != unit || !world.get::<KnownSpells>(unit).is_some_and(|known| known.contains(BRAINSTORM_TALENT)) {
        return;
    }
    // SimC76f8118 sc_mage.cpp2154–2157/6509–6512: converting Heating Up
    // to Hot Streak grants Brainstorm. Wasted critical hits retain the existing
    // Hot Streak and do not apply it again. Native461531 owns amount/cap/duration.
    spell_cast::cast_triggered(world, unit, BRAINSTORM, Some(unit), 1);
}
