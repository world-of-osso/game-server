//! Selected Lunar Inspiration owns the native Cat Moonfire override.
use super::combat;
use bevy::prelude::*;
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::auras::{AuraInstance, RemoveReason};
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EarlyAuraRegistration(register)];
const LUNAR_INSPIRATION: u32 = 155580;
const LUNAR_OVERRIDE: u32 = 155627;

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        LUNAR_INSPIRATION,
        AuraScript {
            on_apply: Some(apply_override),
            on_remove: Some(remove_override),
            ..Default::default()
        },
    );
}

// SimC76f8118 sc_druid.cpp4457–4465 selects155625 only with the talent.
// TC a352b1fa spell_dru_lunar_inspiration1543–1567 supplies the missing
// native155580→155627 aura lifecycle;155627 owns the actual override metadata.
fn apply_override(world: &mut World, unit: Entity, _: &AuraInstance) {
    combat::spell_cast::cast_triggered(world, unit, LUNAR_OVERRIDE, Some(unit), 1);
}

fn remove_override(world: &mut World, unit: Entity, _: &AuraInstance, _: RemoveReason) {
    combat::auras::remove_auras(world, unit, RemoveReason::Cancelled, |aura| {
        aura.spell_id == LUNAR_OVERRIDE && aura.caster == unit
    });
}
