//! Finish the Job follows a real Deathmark application.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    AuraApplication, AuraInstance, AuraSource,
    scripts::{AuraScript, AuraScripts},
};
use combat::spell_cast::{self, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterAuraApplication(grant_buff),
    SpellScriptHook::RogueAuraRegistration(register),
];

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        360194,
        AuraScript {
            on_natural_end: Some(grant_lingering_darkness),
            ..Default::default()
        },
    );
}

fn grant_lingering_darkness(world: &mut World, _: Entity, aura: &AuraInstance) {
    let owner = aura.caster;
    if !combat::auras::has_aura(world, owner, 457056) {
        return;
    }
    // SimC76f8118 sc_rogue.cpp4221–4225/10982–10994: Deathmark's last
    // tick grants the Assassination direct and periodic native buff effects.
    combat::auras::apply_aura(
        world,
        &AuraApplication {
            caster: owner,
            target: owner,
            spell_id: 457273,
            effect_mask: (1 << 0) | (1 << 2),
            source: AuraSource::Cast,
        },
    );
}

fn grant_buff(world: &mut World, _: Entity, aura: &AuraInstance) {
    let owner = aura.caster;
    if aura.spell_id != 360194 || !combat::auras::has_aura(world, owner, 1249809) {
        return;
    }
    // SimC76f8118 sc_rogue.cpp4208–4217: actual Deathmark impact grants
    // the native Finish the Job direct, periodic and auto-attack buff.
    if !spell_cast::try_cast_triggered(world, owner, 1249810, Some(owner), 1) {
        warn!(?owner, "Finish the Job application refused");
    }
}
