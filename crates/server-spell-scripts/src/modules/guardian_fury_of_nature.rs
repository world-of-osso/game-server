//! SimC76f8118 sc_druid.cpp10806–10820: selected talent enables its passive effects.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    self, AuraApplication, AuraInstance, AuraSource, RemoveReason, scripts::AuraScripts,
};
use combat::spell_cast::script_registry::SpellScriptHook;

const TALENT: u32 = 370695;
const PASSIVE: u32 = 370701;
const LUNAR_CALLING: u32 = 429523;
pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EarlyAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    for id in [TALENT, LUNAR_CALLING] {
        let script = scripts.0.entry(id).or_default();
        script.on_apply = Some(update);
        script.on_remove = Some(remove_update);
    }
}

fn update(world: &mut World, unit: Entity, _: &AuraInstance) {
    sync(world, unit);
}
fn remove_update(world: &mut World, unit: Entity, _: &AuraInstance, _: RemoveReason) {
    sync(world, unit);
}
fn sync(world: &mut World, unit: Entity) {
    auras::remove_auras(world, unit, RemoveReason::Cancelled, |a| {
        a.spell_id == PASSIVE && a.caster == unit
    });
    if !auras::has_aura(world, unit, TALENT) {
        return;
    }
    let effect_mask = if auras::has_aura(world, unit, LUNAR_CALLING) {
        0b1111
    } else {
        0b1001
    };
    auras::apply_aura(
        world,
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: PASSIVE,
            effect_mask,
            source: AuraSource::Cast,
        },
    );
}
