//! Feral Omen of Clarity rolls from landed Cat melee swings.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraInstance, procs::AuraEvent, scripts::AuraScripts};
use combat::random::Rng;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use shared::components::ActiveSpec;

const OMEN: u32 = 16864;
const CLEARCASTING: u32 = 135700;
const MOMENT: u32 = 236068;
// SimC76f8118 sc_druid.cpp9788–9794: observed Cat Omen rate, not RPPM.
const CAT_PPM: f32 = 7.0;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::AfterWhiteHit(roll_white_hit),
];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(OMEN).or_default().check_proc = Some(non_feral_only);
}

fn non_feral_only(world: &World, aura: &AuraInstance, _: &AuraEvent) -> bool {
    !world
        .get::<ActiveSpec>(aura.caster)
        .is_some_and(|spec| spec.0 == 103)
}

fn roll_white_hit(world: &mut World, caster: Entity, _: bool) {
    if !world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 103)
        || !auras::has_aura(world, caster, OMEN)
    {
        return;
    }
    let Some(form) = auras::shapeshift::attack_timing(world, caster) else {
        return;
    };
    if !auras::has_aura(world, caster, 768) {
        return;
    }
    let moment = if auras::has_aura(world, caster, MOMENT) {
        spell_cast::scripts::script_effect_value(world, caster, MOMENT, 1) / 100.0
    } else {
        0.0
    };
    // SimC weapon.hpp79–85 uses the unhasted authored swing time per hit.
    let chance = CAT_PPM * (1.0 + moment) * form.seconds / 60.0;
    if combat::random::thread_rng().gen_range(0.0..1.0) < chance {
        spell_cast::cast_triggered(world, caster, CLEARCASTING, Some(caster), 1);
    }
}
