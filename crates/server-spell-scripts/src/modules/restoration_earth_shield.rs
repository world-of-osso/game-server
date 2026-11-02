//! Earth Shield heals its holder when damage triggers its native cooldown.
use super::combat;
use bevy::prelude::*;
use combat::auras::procs::AuraEvent;
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::auras::AuraInstance;
use combat::spell_cast::{self, script_registry::SpellScriptHook};

const EARTH_SHIELD: u32 = 974;
const EARTH_SHIELD_HEAL: u32 = 379;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::SupportAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(EARTH_SHIELD, AuraScript {
        check_proc: Some(took_damage),
        on_damage_proc: Some(heal_holder),
        ..Default::default()
    });
}

fn took_damage(_world: &World, _aura: &AuraInstance, event: &AuraEvent) -> bool {
    event.damage > 0.0
}

fn heal_holder(world: &mut World, holder: Entity, aura: &AuraInstance, _event: &AuraEvent) {
    // WCL cqFgBM3zTvVrRa6j f35 a280: 379 heals each damaged 974 holder;
    // native 974 supplies its own three-second proc cooldown.
    spell_cast::cast_triggered(world, aura.caster, EARTH_SHIELD_HEAL, Some(holder), 1);
}
