//! SimC76f8118 sc_monk.cpp3412/4517–4522: restore Keg Smash on these triggers.
use super::combat;
use bevy::prelude::*;
use combat::auras::AuraInstance;
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{
    restore_charges, script_registry::SpellScriptHook, scripts::AfterCastHook,
};
use combat::spell_info::{SpellInfo, SpellStore};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterAuraApplication(empty_barrel),
    SpellScriptHook::AfterResourceValues(cast_hook),
];

fn restore_keg(world: &mut World, caster: Entity) {
    let category = world
        .resource::<SpellStore>()
        .get(121253)
        .expect("Keg Smash spell exists")
        .categories
        .as_ref()
        .expect("Keg Smash category exists")
        .charge_category;
    assert_ne!(category, 0, "Keg Smash must have a charge category");
    restore_charges(world, caster, category, 1);
}

fn empty_barrel(world: &mut World, target: Entity, aura: &AuraInstance) {
    if aura.spell_id != 1265307 || target != aura.caster {
        return;
    }
    if world.get::<TraitLoadout>(target).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == 1265138)
    }) {
        restore_keg(world, target);
    }
}

fn cast_hook(spell: u32) -> Option<AfterCastHook> {
    (spell == 325153).then_some(exploding_keg)
}

fn exploding_keg(world: &mut World, caster: Entity, _: &SpellInfo) {
    restore_keg(world, caster);
}
