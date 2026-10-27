//! Source-qualified Devourer Metamorphosis entry.
use super::combat;
use bevy::prelude::*;
use combat::auras::AuraInstance;
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::spell_cast::{powers, script_registry::SpellScriptHook};
use shared::components::{ActiveSpec, PowerType};

const META: u32 = 1217607;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EarlyAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        META,
        AuraScript {
            on_apply: Some(grant_entry_fury),
            ..Default::default()
        },
    );
}

fn grant_entry_fury(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if unit != aura.caster || world.get::<ActiveSpec>(unit).map(|spec| spec.0) != Some(1480) {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp12445–12452 grants 200 Fury on Meta entry.
    // Native PowerPools owns the cap and the actual applied amount.
    if powers::change_power(world, unit, PowerType::Fury, 200.0).is_none() {
        error!(?unit, "Devourer Meta entry has no Fury pool");
    }
}
