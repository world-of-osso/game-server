//! Native Burnout: TC a352b1fa spell_evoker.cpp161–183;
//! SimC76f8118 sc_evoker.cpp5393/7028–7029/10553–10555.
use bevy::prelude::*;

use super::combat;
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::auras::{ActiveAuras, AuraInstance, procs::AuraEvent};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::AfterCastHook;
use combat::spell_info::SpellInfo;

const BURNOUT: u32 = 375801;
const INSTANT_FLAME: u32 = 375802;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::MidAuraRegistration(register),
    SpellScriptHook::AfterResourceValues(cast_hook),
];

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        BURNOUT,
        AuraScript {
            check_proc: Some(check_burnout),
            on_proc: Some(grant_burnout),
            ..Default::default()
        },
    );
}

fn check_burnout(_world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    let own_fire_tick =
        event.periodic && event.spell_id == Some(357209) && event.actor == aura.caster;
    if !own_fire_tick {
        return false;
    }
    // Native proc admission owns the mask/ICD; TC rolls the actual effect0 amount.
    let chance = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Burnout chance effect0")
        .amount;
    combat::random::random::<f32>() * 100.0 < chance
}

fn grant_burnout(world: &mut World, unit: Entity, _aura: &AuraInstance, _other: Option<Entity>) {
    if !combat::spell_cast::try_cast_triggered(world, unit, INSTANT_FLAME, Some(unit), 1) {
        warn!(?unit, "Native Burnout buff refused");
    }
}

fn cast_hook(spell: u32) -> Option<AfterCastHook> {
    (spell == 361469).then_some(spend_burnout)
}

fn spend_burnout(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    let instance = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == INSTANT_FLAME && aura.caster == caster)
            .map(|aura| aura.instance_id)
    });
    if let Some(instance) = instance {
        combat::auras::spend_stack(world, caster, instance);
    }
}
