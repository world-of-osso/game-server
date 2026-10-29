//! Modern Water Shield uses its native melee proc mask and cooldown.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    AuraInstance,
    scripts::{AuraScript, AuraScripts},
};
use combat::spell_cast::{self, script_registry::SpellScriptHook};

const WATER_SHIELD: u32 = 52_127;
const ENERGIZE: u32 = 52_128;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::SupportAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        WATER_SHIELD,
        AuraScript {
            on_proc: Some(restore_mana),
            ..Default::default()
        },
    );
}

fn restore_mana(world: &mut World, unit: Entity, _aura: &AuraInstance, _other: Option<Entity>) {
    spell_cast::cast_triggered(world, unit, ENERGIZE, Some(unit), 1);
}
