//! SimC76f8118 sc_monk.cpp1380–1390: Blackout Kick grants Blackout Combo.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    self, AuraApplication, AuraInstance, AuraSource,
    procs::AuraEvent,
    scripts::{AuraScript, AuraScripts},
};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::MidAuraRegistration(register),
    SpellScriptHook::AfterResourceValues(cast_hook),
];

fn register(scripts: &mut AuraScripts) {
    for spell in [196736, 228563] {
        scripts.0.insert(
            spell,
            AuraScript {
                check_proc: Some(blackout_combo_is_cast_driven),
                ..Default::default()
            },
        );
    }
}

// The unmasked catalog procs otherwise grant or consume Combo on unrelated
// events. SimC sc_monk.cpp865,2606 consumes it only on Tiger Palm or Keg Smash.
fn blackout_combo_is_cast_driven(_: &World, _: &AuraInstance, _: &AuraEvent) -> bool {
    false
}

fn cast_hook(spell: u32) -> Option<AfterCastHook> {
    (spell == 205523).then_some(grant_blackout_combo)
}

fn grant_blackout_combo(world: &mut World, caster: Entity, _: &SpellInfo) {
    if !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == 196736)
    }) {
        return;
    }
    auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: 228563,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}
