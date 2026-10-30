//! SimC76f8118 sc_druid.cpp10806–10820: the selected talent supplies its passive's value.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraInstance, AuraSource, RemoveReason, scripts::AuraScripts};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellStore;

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
    let amount = world.get::<ActiveAuras>(unit).and_then(|auras| {
        auras.iter().find(|a| a.spell_id == TALENT && a.caster == unit)
            .and_then(|a| a.effects.iter().find(|e| e.index == 0)).map(|e| e.amount)
    });
    auras::remove_auras(world, unit, RemoveReason::Cancelled, |a| a.spell_id == PASSIVE && a.caster == unit);
    let Some(amount) = amount else { return };
    let lunar = auras::has_aura(world, unit, LUNAR_CALLING);
    let mut passive = world.resource::<SpellStore>().get(PASSIVE).expect("Fury of Nature passive").clone();
    for effect in &mut passive.effects {
        effect.base_points = amount;
    }
    let effect_mask = if lunar { 0b1111 } else { 0b1001 };
    auras::apply_aura_with_spell(world, &AuraApplication { caster: unit, target: unit, spell_id: PASSIVE, effect_mask, source: AuraSource::Cast }, &passive);
}
