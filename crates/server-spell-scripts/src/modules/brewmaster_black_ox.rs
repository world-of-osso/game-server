//! SimC76f8118 sc_monk.cpp3122–3137: Black Ox Brew restores Brew charges.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    restore_charges, script_registry::SpellScriptHook, scripts::AfterCastHook,
};
use combat::spell_info::{SpellInfo, SpellStore};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterResourceValues(cast_hook)];

fn cast_hook(spell: u32) -> Option<AfterCastHook> {
    (spell == 115399).then_some(restore_brews)
}

fn restore_brews(world: &mut World, caster: Entity, _: &SpellInfo) {
    for (spell, count) in [(119582, 2), (1241059, 1)] {
        let category = world
            .resource::<SpellStore>()
            .get(spell)
            .expect("Brew spell exists")
            .categories
            .as_ref()
            .expect("Brew charge category exists")
            .charge_category;
        assert_ne!(category, 0, "Brew must have a charge category");
        restore_charges(world, caster, category, count);
    }
}
