//! SimC76f8118 sc_druid.cpp5023–5037: paid Rage accumulates toward After the Wildfire.
use super::combat;
use bevy::prelude::*;
use combat::auras::{AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::spell_cast::{self, CostPool, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;
use shared::components::PowerType;

const TALENT: u32 = 371905;
const HEAL: u32 = 371982;

#[derive(Component, Default)]
struct RageSpent(f32);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterSpellPayment(on_payment),
    SpellScriptHook::LateAuraRegistration(register),
];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(TALENT).or_default().on_remove = Some(clear_progress);
}

fn clear_progress(world: &mut World, unit: Entity, _: &AuraInstance, _: RemoveReason) {
    world.entity_mut(unit).remove::<RageSpent>();
}

fn on_payment(world: &mut World, caster: Entity, spell: &SpellInfo, costs: &[(CostPool, f32)]) {
    if !matches!(spell.id, 192081 | 6807 | 400254 | 22842)
        || !combat::auras::has_aura(world, caster, TALENT)
    {
        return;
    }
    let paid = spell_cast::spent(costs, PowerType::Rage);
    if paid <= 0.0 {
        return;
    }
    // Rage is stored in tenths; effect 1 is the native threshold in Rage.
    let threshold = spell_cast::scripts::script_effect_value(world, caster, TALENT, 1) * 10.0;
    assert!(
        threshold > 0.0,
        "After the Wildfire requires its native Rage threshold"
    );
    let trigger = {
        let mut entity = world.entity_mut(caster);
        let mut entry = entity.entry::<RageSpent>().or_default();
        let mut progress = entry.get_mut();
        progress.0 += paid;
        if progress.0 < threshold {
            false
        } else {
            progress.0 -= threshold;
            true
        }
    };
    if trigger {
        let cap = spell_cast::scripts::script_effect_value(world, caster, TALENT, 2) as u32;
        let mut heal = spell_cast::load_triggered_spell(world, HEAL, 1)
            .expect("native After the Wildfire heal");
        heal.target_restrictions
            .get_or_insert_with(Default::default)
            .max_targets = cap;
        if !spell_cast::launch_triggered_spell(world, caster, &heal, Some(caster), 1) {
            warn!(?caster, "After the Wildfire heal failed");
        }
    }
}
